// SPDX-License-Identifier: MIT

import { useTranslation } from "react-i18next";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { SettingsSection } from "@/components/settings/SettingsSection";
import { queryKeys } from "@/lib/query-keys";
import { audit } from "@/lib/tauri";
import type { AuditEvent, AuditEventsResponse, Entry } from "@/lib/types";

interface AuditLogSectionProps {
  dbId: string | null;
}

function formatTimestamp(timestamp: string, locale: string): string {
  const parsed = new Date(timestamp);
  if (Number.isNaN(parsed.getTime())) return timestamp;
  return parsed.toLocaleString(locale, {
    dateStyle: "medium",
    timeStyle: "medium",
  });
}

// Walks every cached entries-list query for `dbId` and returns the matching
// entry's title, or `null` when the Vault is locked / not currently open in
// this session. The renderer falls back to a UUID prefix in that case so the
// on-disk log never carries entry titles outside the Vault.
function useResolveEntryTitle(
  dbId: string | null
): (entryId: string) => string | null {
  const queryClient = useQueryClient();
  return (entryId: string) => {
    if (!dbId) return null;
    const queries = queryClient
      .getQueryCache()
      .findAll({ queryKey: [...queryKeys.entries.all, dbId, "list"] });
    for (const q of queries) {
      const entries = q.state.data;
      if (!Array.isArray(entries)) continue;
      const hit = (entries as Entry[]).find((e) => e.id === entryId);
      if (hit) return hit.title;
    }
    return null;
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
  const entryId = event.entryId ?? null;
  const title = entryId ? resolveTitle(entryId) : null;
  const entryLabel = entryId ? (title ?? entryId.slice(0, 8)) : null;

  return (
    <li
      data-kind={event.kind}
      data-entry-id={entryId ?? undefined}
      className="flex flex-col gap-1 rounded-md border bg-card/50 p-3 text-sm md:flex-row md:items-center md:justify-between"
    >
      <span className="font-medium">{t(`audit.kind.${event.kind}`)}</span>
      <div className="flex items-center gap-3 text-xs text-muted-foreground">
        {entryLabel ? <span>{entryLabel}</span> : null}
        {typeof event.attemptCount === "number" ? (
          <span>{t("audit.attemptCount", { count: event.attemptCount })}</span>
        ) : null}
        <span>{formatTimestamp(event.timestamp, i18n.language)}</span>
      </div>
    </li>
  );
}

const EMPTY_RESPONSE: AuditEventsResponse = { events: [], degraded: false };

export function AuditLogSection({ dbId }: Readonly<AuditLogSectionProps>) {
  const { t } = useTranslation();
  const resolveTitle = useResolveEntryTitle(dbId);

  const query = useQuery<AuditEventsResponse, Error>({
    queryKey: queryKeys.audit.list(dbId ?? "none"),
    queryFn: () => (dbId ? audit.list(dbId) : Promise.resolve(EMPTY_RESPONSE)),
    enabled: Boolean(dbId),
  });

  const data = query.data ?? EMPTY_RESPONSE;

  return (
    <SettingsSection
      id="audit-log"
      title={t("audit.title")}
      description={t("audit.description")}
    >
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
