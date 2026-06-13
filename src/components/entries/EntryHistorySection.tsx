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
            {versions.map((version, index) => (
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
              </li>
            ))}
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
