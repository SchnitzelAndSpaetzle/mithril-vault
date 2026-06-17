// SPDX-License-Identifier: MIT

import { useState } from "react";
import { useTranslation } from "react-i18next";
import { useQuery } from "@tanstack/react-query";
import { ChevronDown, History } from "lucide-react";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible";
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
                </li>
              );
            })}
          </ul>
        )}
      </CollapsibleContent>
    </Collapsible>
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
