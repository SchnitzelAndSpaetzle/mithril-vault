// SPDX-License-Identifier: MIT

import { ChevronDown, ChevronRight } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import type { Entry } from "@/lib/types";

interface ReusedGroupRowProps {
  /// Member Entry ids. Always size ≥ 2 — the backend elides singleton
  /// groups before serializing, so the row's existence already means
  /// "two or more Entries share this password".
  entryIds: string[];
  /// Live Entry list used to look up titles. A stale list (member id
  /// not present) falls back to the id string so the row stays
  /// readable while a re-fetch completes.
  entries: Entry[];
  /// Parent navigates to the Entry detail view. Kept as a callback so
  /// the row stays decoupled from the router and the tab store and
  /// is testable in isolation.
  onOpenEntry: (entryId: string) => void;
}

export function ReusedGroupRow({
  entryIds,
  entries,
  onOpenEntry,
}: Readonly<ReusedGroupRowProps>) {
  const { t } = useTranslation();
  const [expanded, setExpanded] = useState(false);
  const entryById = new Map(entries.map((e) => [e.id, e]));

  return (
    <li className="flex flex-col gap-2 px-4 py-3">
      <button
        type="button"
        className="flex items-center justify-between gap-3 text-left"
        onClick={() => setExpanded((prev) => !prev)}
        aria-expanded={expanded}
        aria-label={
          expanded
            ? t("passwordHealth.reused.collapse")
            : t("passwordHealth.reused.expand")
        }
      >
        <div className="flex flex-col gap-0.5 min-w-0">
          <span className="font-medium">
            {t("passwordHealth.findings.password.reused")}
          </span>
          <span className="text-xs text-muted-foreground">
            {t("passwordHealth.reused.memberCount", { count: entryIds.length })}
          </span>
        </div>
        {expanded ? (
          <ChevronDown className="size-4 text-muted-foreground" />
        ) : (
          <ChevronRight className="size-4 text-muted-foreground" />
        )}
      </button>

      {expanded && (
        <ul className="flex flex-col divide-y rounded-md border bg-background">
          {entryIds.map((memberId) => {
            const memberEntry = entryById.get(memberId);
            return (
              <li
                key={memberId}
                className="flex items-center justify-between gap-3 px-3 py-2"
              >
                <span className="truncate text-sm">
                  {memberEntry?.title ?? memberId}
                </span>
                <Button
                  size="sm"
                  variant="outline"
                  onClick={() => onOpenEntry(memberId)}
                >
                  {t("passwordHealth.actions.openEntry")}
                </Button>
              </li>
            );
          })}
        </ul>
      )}
    </li>
  );
}
