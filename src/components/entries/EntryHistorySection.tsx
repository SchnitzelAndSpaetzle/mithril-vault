// SPDX-License-Identifier: MIT

import { useCallback, useState, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import {
  ChevronDown,
  Eye,
  EyeOff,
  GitCompare,
  History,
  Loader2,
  RotateCcw,
  Trash2,
} from "lucide-react";
import { ask } from "@tauri-apps/plugin-dialog";
import { toast } from "sonner";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible";
import { Button } from "@/components/ui/button";
import { Separator } from "@/components/ui/separator.tsx";
import { entries as entriesApi } from "@/lib/tauri";
import {
  changedSince,
  EntryHistoryCompareDialog,
} from "@/components/entries/EntryHistoryCompareDialog";
import { saveWithErrorToast } from "@/lib/save-with-error-toast";
import type { EntryHistoryItem } from "@/lib/types";
import { queryKeys } from "@/lib/query-keys";
import { cn } from "@/lib/utils";

interface EntryHistorySectionProps {
  dbId: string;
  entryId: string;
}

/**
 * Renders a version's `changedFields` as a comma-separated string, replacing
 * each canonical token the backend emits with its localized label (so
 * non-English locales don't show mixed-language text) and passing user-defined
 * custom field names through verbatim.
 */
export function formatChangedFields(
  fields: string[],
  labels: Record<string, string>
): string {
  return fields.map((field) => labels[field] ?? field).join(", ");
}

/**
 * Minimal Entry History view (#322): a collapsible section in the Entry detail
 * that lists the Entry's past versions, newest-first, each with its timestamp.
 * Versions come from native KDBX history via {@link entriesApi.listHistory} and
 * carry non-secret display fields only — passwords and protected values are
 * never part of the listing (ADR-0008). Restore and per-version secret reveal
 * are later slices.
 */
export function EntryHistorySection({
  dbId,
  entryId,
}: Readonly<EntryHistorySectionProps>) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);

  // Localized labels for the standard changed-field tokens the backend emits;
  // custom field names are not listed and fall through to their raw value.
  const fieldLabels: Record<string, string> = {
    title: t("entries.detail.historyField.title"),
    username: t("entries.detail.historyField.username"),
    password: t("entries.detail.historyField.password"),
    url: t("entries.detail.historyField.url"),
    notes: t("entries.detail.historyField.notes"),
    tags: t("entries.detail.historyField.tags"),
    icon: t("entries.detail.historyField.icon"),
    attachments: t("entries.detail.historyField.attachments"),
    expiry: t("entries.detail.historyField.expiry"),
    location: t("entries.detail.historyField.location"),
  };

  const queryClient = useQueryClient();

  const { data: versions = [] } = useQuery({
    queryKey: queryKeys.entries.history(dbId, entryId),
    queryFn: () => entriesApi.listHistory(dbId, entryId),
  });

  // Restores the Entry to a chosen version after an explicit confirmation: the
  // backend snapshots the current state into history first (so the restore is
  // undoable), then overwrites the live content (ADR-0008). Secrets are read
  // backend-side and never round-trip. On success we persist via the shared
  // helper (which surfaces its own toast on a disk/backup failure) and refresh
  // the detail, list, and history views so the restored content and the new
  // pre-restore version both appear.
  const handleRestore = useCallback(
    async (version: EntryHistoryItem) => {
      const confirmed = await ask(
        t("entries.detail.restoreHistoryConfirm", {
          date: formatHistoryDate(version.modifiedAt),
        }),
        {
          title: t("entries.detail.restoreHistoryConfirmTitle"),
          kind: "warning",
        }
      );
      if (!confirmed) return;

      try {
        await entriesApi.restoreHistory(
          dbId,
          entryId,
          version.index,
          version.fingerprint
        );
      } catch (error) {
        // A version whose restorable content matches the current entry (e.g. a
        // move-only version — restore never touches the parent Group) is a
        // no-op the backend rejects rather than reporting a phantom success.
        // Surface it as a neutral info message, not a failure.
        if (String(error).includes("History version unchanged")) {
          toast.info(t("entries.detail.restoreHistoryUnchanged"));
          return;
        }
        console.error("Failed to restore entry version:", error);
        toast.error(t("entries.detail.restoreHistoryFailed"));
        return;
      }

      await saveWithErrorToast(dbId, t);
      await Promise.all([
        queryClient.invalidateQueries({
          queryKey: queryKeys.entries.detail(dbId, entryId),
        }),
        queryClient.invalidateQueries({
          predicate: (query) =>
            query.queryKey[0] === queryKeys.entries.all[0] &&
            query.queryKey[1] === dbId,
        }),
        // A restore can replace the live password/expiry, so any cached
        // password-health findings must be recomputed — same as the entry
        // mutation hook does for the same kind of content change.
        queryClient.invalidateQueries({
          queryKey: queryKeys.passwordHealth.report(dbId),
        }),
      ]);
      toast.success(t("entries.detail.restoreHistorySuccess"));
    },
    [dbId, entryId, queryClient, t]
  );

  // Clears this Entry's history after an explicit confirmation: the backend
  // empties the Entry's native KDBX history (ADR-0008) — live content is
  // untouched and the act is not audited. On success we persist via the shared
  // helper and refresh the history view so the now-empty list shows.
  const handleClear = useCallback(async () => {
    const confirmed = await ask(t("entries.detail.clearHistoryConfirm"), {
      title: t("entries.detail.clearHistoryConfirmTitle"),
      kind: "warning",
    });
    if (!confirmed) return;

    try {
      await entriesApi.clearHistory(dbId, entryId);
    } catch (error) {
      console.error("Failed to clear entry history:", error);
      toast.error(t("entries.detail.clearHistoryFailed"));
      return;
    }

    await saveWithErrorToast(dbId, t);
    await queryClient.invalidateQueries({
      queryKey: queryKeys.entries.history(dbId, entryId),
    });
    toast.success(t("entries.detail.clearHistorySuccess"));
  }, [dbId, entryId, queryClient, t]);

  return (
    <Collapsible
      open={open}
      onOpenChange={setOpen}
      className="border rounded-md"
    >
      <CollapsibleTrigger className="flex w-full items-center justify-between px-4 py-2 text-sm font-medium">
        <span className="flex items-center gap-2">
          <History className="h-4 w-4 text-muted-foreground" />
          {t("entries.detail.history")}
          {versions.length > 0 && (
            <span className="text-muted-foreground">({versions.length})</span>
          )}
        </span>
        <ChevronDown
          className={cn(
            "h-4 w-4 text-muted-foreground transition-transform",
            open && "rotate-180"
          )}
        />
      </CollapsibleTrigger>
      <CollapsibleContent>
        <Separator />
        {versions.length === 0 ? (
          <p className="px-4 py-2 text-sm text-muted-foreground">
            {t("entries.detail.noHistory")}
          </p>
        ) : (
          <ul>
            {versions.map((version, index) => (
              // Key on the fingerprint, not just index+timestamp: two snapshots
              // can share a second-precision timestamp at the same index after a
              // rapid edit + refetch. Keying on the content fingerprint forces a
              // different snapshot to remount the row, so a previously revealed
              // secret can't linger under a now-different version.
              <HistoryVersionItem
                key={`${version.index}:${version.fingerprint}`}
                dbId={dbId}
                entryId={entryId}
                version={version}
                // The oldest version (last, newest-first) carries an origin
                // label; a separator divides it from the version above.
                isOldest={index === versions.length - 1}
                showSeparator={index > 0}
                fieldLabels={fieldLabels}
                // What differs between this version and the current Entry: the
                // union of changedFields from the newest version through this
                // one (#324), used by the compare dialog (#329).
                changedSinceFields={changedSince(versions, index)}
                onRestore={handleRestore}
              />
            ))}
          </ul>
        )}
        {versions.length > 0 && (
          <>
            <Separator />
            <div className="flex justify-end px-4 py-2">
              <Button
                variant="outline"
                size="sm"
                className="gap-2 text-destructive"
                onClick={handleClear}
              >
                <Trash2 className="h-3 w-3" />
                {t("entries.detail.clearHistory")}
              </Button>
            </div>
          </>
        )}
      </CollapsibleContent>
    </Collapsible>
  );
}

/**
 * One past version's row: its title + timestamp, an origin/changed line, and a
 * per-version secret reveal for the password and each protected custom field
 * (mirroring the live Entry — the value is fetched only on the explicit action,
 * addressed by this version's index and guarded by its fingerprint, ADR-0008).
 * Extracted from {@link EntryHistorySection} so the reveal closures aren't
 * nested inside two `.map()` callbacks (keeps function nesting shallow).
 */
function HistoryVersionItem({
  dbId,
  entryId,
  version,
  isOldest,
  showSeparator,
  fieldLabels,
  changedSinceFields,
  onRestore,
}: Readonly<{
  dbId: string;
  entryId: string;
  version: EntryHistoryItem;
  isOldest: boolean;
  showSeparator: boolean;
  fieldLabels: Record<string, string>;
  changedSinceFields: string[];
  onRestore: (version: EntryHistoryItem) => void;
}>) {
  const { t } = useTranslation();
  const [compareOpen, setCompareOpen] = useState(false);
  // The changed line is shown even on the oldest "Earliest kept" version:
  // changedFields is diffed against the next-newer version, so it's accurate
  // even when the original predecessor was pruned.
  const showChanged = version.changedFields.length > 0;

  return (
    <li>
      {showSeparator && <Separator />}
      <div className="flex min-w-0 items-center justify-between gap-3 px-4 py-2">
        <span className="min-w-0 flex-1 truncate text-sm font-medium">
          {version.title}
        </span>
        <span className="shrink-0 text-sm text-muted-foreground">
          {formatHistoryDate(version.modifiedAt)}
        </span>
        <Button
          variant="outline"
          size="icon-xs"
          className="shrink-0"
          aria-label={t("entries.detail.compare.action")}
          onClick={() => setCompareOpen(true)}
        >
          <GitCompare className="h-3 w-3" />
        </Button>
        <Button
          variant="outline"
          size="icon-xs"
          className="shrink-0"
          aria-label={t("entries.detail.restoreVersion")}
          onClick={() => onRestore(version)}
        >
          <RotateCcw className="h-3 w-3" />
        </Button>
      </div>
      <EntryHistoryCompareDialog
        dbId={dbId}
        entryId={entryId}
        version={version}
        changedFields={changedSinceFields}
        open={compareOpen}
        onOpenChange={setCompareOpen}
      />
      {isOldest && (
        <p className="px-4 pb-2 text-xs font-medium text-muted-foreground">
          {version.isCreation
            ? t("entries.detail.historyCreated")
            : t("entries.detail.historyEarliestKept")}
        </p>
      )}
      {showChanged && (
        <p className="px-4 pb-2 text-xs text-muted-foreground">
          {t("entries.detail.historyChanged", {
            fields: formatChangedFields(version.changedFields, fieldLabels),
          })}
        </p>
      )}
      <HistorySecretRow
        label={t("entries.detail.password")}
        revealLabel={t("entries.detail.revealPassword")}
        hideLabel={t("entries.detail.hidePassword")}
        fetchValue={() =>
          entriesApi.getHistoryPassword(
            dbId,
            entryId,
            version.index,
            version.fingerprint
          )
        }
      />
      {version.protectedFields.map((key) => (
        <HistorySecretRow
          key={key}
          label={key}
          revealLabel={t("entries.detail.revealField", { field: key })}
          hideLabel={t("entries.detail.hideField", { field: key })}
          fetchValue={async () => {
            const field = await entriesApi.getHistoryProtectedField(
              dbId,
              entryId,
              version.index,
              version.fingerprint,
              key
            );
            return field.value;
          }}
        />
      ))}
    </li>
  );
}

/**
 * One masked secret of a historical version (its password or a protected custom
 * field), revealed only on the explicit eye-toggle. Mirrors the live Entry's
 * reveal-on-demand rule: nothing is fetched until the user asks, and `fetchValue`
 * carries the version's index + fingerprint guard (ADR-0008). Hiding drops the
 * value from state so it doesn't linger.
 */
function HistorySecretRow({
  label,
  revealLabel,
  hideLabel,
  fetchValue,
}: Readonly<{
  label: string;
  revealLabel: string;
  hideLabel: string;
  fetchValue: () => Promise<string>;
}>) {
  const [value, setValue] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const isVisible = value !== null;

  const reveal = useCallback(async () => {
    setIsLoading(true);
    try {
      setValue(await fetchValue());
    } finally {
      setIsLoading(false);
    }
  }, [fetchValue]);

  const hide = useCallback(() => setValue(null), []);

  let displayValue: ReactNode;
  if (isLoading) {
    displayValue = <Loader2 className="inline h-3 w-3 animate-spin" />;
  } else if (isVisible) {
    displayValue = value;
  } else {
    displayValue = "••••••••";
  }

  return (
    <div className="flex min-w-0 items-center justify-between gap-2 px-4 pb-2">
      <small className="shrink-0 text-xs font-medium text-muted-foreground">
        {label}
      </small>
      <div className="flex w-0 min-w-0 flex-1 items-center justify-end gap-2">
        <span className="min-w-0 truncate text-right text-xs text-muted-foreground">
          {displayValue}
        </span>
        <Button
          variant="outline"
          size="icon-xs"
          aria-label={isVisible ? hideLabel : revealLabel}
          onClick={isVisible ? hide : reveal}
          disabled={isLoading}
        >
          {isVisible ? (
            <EyeOff className="h-3 w-3" />
          ) : (
            <Eye className="h-3 w-3" />
          )}
        </Button>
      </div>
    </div>
  );
}

// Mirror EntryItemDetails' metadata formatting so a version's timestamp reads
// the same as the live Entry's "modified" line.
function formatHistoryDate(value: string): string {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return new Intl.DateTimeFormat(undefined, {
    year: "numeric",
    month: "short",
    day: "numeric",
    hour: "numeric",
    minute: "2-digit",
    second: "2-digit",
  }).format(date);
}
