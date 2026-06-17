// SPDX-License-Identifier: MIT

import { useCallback, useState, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { useQuery } from "@tanstack/react-query";
import { ChevronDown, Eye, EyeOff, History, Loader2 } from "lucide-react";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible";
import { Button } from "@/components/ui/button";
import { Separator } from "@/components/ui/separator.tsx";
import { entries as entriesApi } from "@/lib/tauri";
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

  const { data: versions = [] } = useQuery({
    queryKey: queryKeys.entries.history(dbId, entryId),
    queryFn: () => entriesApi.listHistory(dbId, entryId),
  });

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
            {versions.map((version, index) => {
              // The oldest version (last, newest-first) carries an origin
              // label: "Created" when it's the original snapshot, otherwise
              // "Earliest kept version". Its changed line is still shown —
              // changedFields is diffed against the next-newer version, so it's
              // accurate even when the original predecessor was pruned.
              const isOldest = index === versions.length - 1;
              const showChanged = version.changedFields.length > 0;
              return (
                <li key={`${version.index}:${version.modifiedAt}`}>
                  {index > 0 && <Separator />}
                  <div className="flex min-w-0 items-center justify-between gap-3 px-4 py-2">
                    <span className="min-w-0 flex-1 truncate text-sm font-medium">
                      {version.title}
                    </span>
                    <span className="shrink-0 text-sm text-muted-foreground">
                      {formatHistoryDate(version.modifiedAt)}
                    </span>
                  </div>
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
                        fields: formatChangedFields(
                          version.changedFields,
                          fieldLabels
                        ),
                      })}
                    </p>
                  )}
                  {/* Per-version secret reveal, mirroring the live Entry: the
                      value is fetched only on the explicit action, addressed by
                      this version's index and guarded by its fingerprint. */}
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
                      revealLabel={t("entries.detail.revealField", {
                        field: key,
                      })}
                      hideLabel={t("entries.detail.hideField", { field: key })}
                      fetchValue={() =>
                        entriesApi
                          .getHistoryProtectedField(
                            dbId,
                            entryId,
                            version.index,
                            version.fingerprint,
                            key
                          )
                          .then((field) => field.value)
                      }
                    />
                  ))}
                </li>
              );
            })}
          </ul>
        )}
      </CollapsibleContent>
    </Collapsible>
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
