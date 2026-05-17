// SPDX-License-Identifier: MIT

import { useTranslation } from "react-i18next";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { ask } from "@tauri-apps/plugin-dialog";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { SettingsSection } from "@/components/settings/SettingsSection";
import { queryKeys } from "@/lib/query-keys";
import { audit, entries as entriesIpc } from "@/lib/tauri";
import type { AuditEvent, AuditEventsResponse, Entry } from "@/lib/types";

interface AuditLogSectionProps {
  dbId: string | null;
  isLocked?: boolean;
}

function formatTimestamp(timestamp: string, locale: string): string {
  const parsed = new Date(timestamp);
  if (Number.isNaN(parsed.getTime())) return timestamp;
  return parsed.toLocaleString(locale, {
    dateStyle: "medium",
    timeStyle: "medium",
  });
}

// Subscribes to the entries-list query for `dbId` so the resolver
// recomputes whenever entries load or change. While the Vault is locked
// the query stays disabled (PRD US #16: locked vaults must mask entry
// rows to a UUID prefix so the on-disk log never carries titles outside
// the unlocked Vault), and the resolver short-circuits to null.
function useResolveEntryTitle(
  dbId: string | null,
  isLocked: boolean
): (entryId: string) => string | null {
  const query = useQuery<Entry[], Error>({
    queryKey: queryKeys.entries.list(dbId ?? "none", null),
    queryFn: () => (dbId ? entriesIpc.list(dbId) : Promise.resolve([])),
    enabled: Boolean(dbId) && !isLocked,
    staleTime: 30_000,
  });
  const entriesList = query.data;
  return (entryId: string) => {
    if (isLocked || !entriesList) return null;
    return entriesList.find((e) => e.id === entryId)?.title ?? null;
  };
}

function AuditRow({
  event,
  resolveTitle,
}: Readonly<{
  event: AuditEvent;
  resolveTitle: (entryId: string) => string | null;
}>) {
  const { t, i18n } = useTranslation();
  // Row layout is uniform across kinds — kind-specific payload (attempt
  // count for failed unlocks, reason for locked, entry title for entry-
  // level kinds) hangs off the same muted-text slot so new kinds can
  // plug in without a layout rewrite.
  const entryId = event.entryId ?? null;
  const title = entryId ? resolveTitle(entryId) : null;
  const entryLabel = entryId ? (title ?? entryId.slice(0, 8)) : null;
  const settingName = event.settingName ?? null;
  return (
    <li
      data-kind={event.kind}
      data-entry-id={entryId ?? undefined}
      data-setting-name={settingName ?? undefined}
      className="flex flex-col gap-1 rounded-md border bg-card/50 p-3 text-sm md:flex-row md:items-center md:justify-between"
    >
      <span className="font-medium">{t(`audit.kind.${event.kind}`)}</span>
      <div className="flex items-center gap-3 text-xs text-muted-foreground">
        {entryLabel ? <span>{entryLabel}</span> : null}
        {settingName ? (
          <span>{t(`audit.settingName.${settingName}`)}</span>
        ) : null}
        {typeof event.attemptCount === "number" ? (
          <span>{t("audit.attemptCount", { count: event.attemptCount })}</span>
        ) : null}
        {event.reason ? <span>{t(`audit.reason.${event.reason}`)}</span> : null}
        <span>{formatTimestamp(event.timestamp, i18n.language)}</span>
      </div>
    </li>
  );
}

const EMPTY_RESPONSE: AuditEventsResponse = { events: [], degraded: false };

export function AuditLogSection({
  dbId,
  isLocked = false,
}: Readonly<AuditLogSectionProps>) {
  const { t } = useTranslation();
  const resolveTitle = useResolveEntryTitle(dbId, isLocked);
  const queryClient = useQueryClient();

  const query = useQuery<AuditEventsResponse, Error>({
    queryKey: queryKeys.audit.list(dbId ?? "none"),
    queryFn: () => (dbId ? audit.list(dbId) : Promise.resolve(EMPTY_RESPONSE)),
    enabled: Boolean(dbId),
  });

  const clearMutation = useMutation<void, Error, string>({
    mutationFn: (vaultPath) => audit.clear(vaultPath),
    onSuccess: (_data, vaultPath) => {
      void queryClient.invalidateQueries({
        queryKey: queryKeys.audit.list(vaultPath),
      });
    },
    onError: (error) => {
      toast.error(t("audit.clearError", { error: String(error) }));
    },
  });

  const data = query.data ?? EMPTY_RESPONSE;

  async function handleClear() {
    if (!dbId) return;
    const confirmed = await ask(t("audit.clearConfirm"), {
      title: t("audit.clearConfirmTitle"),
      kind: "warning",
    });
    if (!confirmed) return;
    clearMutation.mutate(dbId);
  }

  return (
    <SettingsSection
      id="audit-log"
      title={t("audit.title")}
      description={t("audit.description")}
    >
      {dbId ? (
        <div className="flex justify-end">
          <Button
            type="button"
            variant="destructive"
            size="sm"
            onClick={() => {
              void handleClear();
            }}
            disabled={clearMutation.isPending}
          >
            {t("audit.clearButton")}
          </Button>
        </div>
      ) : null}
      {!dbId ? (
        <p className="text-sm text-muted-foreground">
          {t("audit.emptyNoVault")}
        </p>
      ) : query.isLoading ? (
        <p className="text-sm text-muted-foreground">{t("common.loading")}</p>
      ) : query.error ? (
        <p className="text-sm text-destructive">
          {t("audit.loadError", { error: String(query.error) })}
        </p>
      ) : (
        <>
          {data.degraded ? (
            <p
              role="status"
              className="rounded-md border border-amber-500/40 bg-amber-500/10 p-3 text-sm text-amber-900 dark:text-amber-200"
            >
              {t("audit.degradedWarning")}
            </p>
          ) : null}
          {data.events.length === 0 ? (
            <p className="text-sm text-muted-foreground">{t("audit.empty")}</p>
          ) : (
            <ul className="flex flex-col gap-2">
              {data.events.map((event, index) => (
                <AuditRow
                  key={`${event.timestamp}-${index}`}
                  event={event}
                  resolveTitle={resolveTitle}
                />
              ))}
            </ul>
          )}
        </>
      )}
    </SettingsSection>
  );
}
