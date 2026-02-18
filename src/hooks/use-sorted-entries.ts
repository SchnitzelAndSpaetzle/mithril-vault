// SPDX-License-Identifier: MIT

import { useMemo } from "react";
import type { Entry, EntrySortField, SortOrder } from "@/lib/types";

function compareStrings(a: string, b: string): number {
  return a.localeCompare(b, undefined, { sensitivity: "base" });
}

function compareDates(a: string, b: string): number {
  return new Date(a).getTime() - new Date(b).getTime();
}

export function sortEntries(
  entries: Entry[],
  sortBy: EntrySortField,
  sortOrder: SortOrder
): Entry[] {
  const sorted = [...entries].sort((a, b) => {
    let result: number;

    switch (sortBy) {
      case "title":
        result = compareStrings(a.title, b.title);
        break;
      case "username":
        result = compareStrings(a.username, b.username);
        break;
      case "url":
        result = compareStrings(a.url ?? "", b.url ?? "");
        break;
      case "modifiedAt":
        result = compareDates(a.modifiedAt, b.modifiedAt);
        break;
      case "createdAt":
        result = compareDates(a.createdAt, b.createdAt);
        break;
    }

    return sortOrder === "desc" ? -result : result;
  });

  return sorted;
}

export function useSortedEntries(
  entries: Entry[] | undefined,
  sortBy: EntrySortField,
  sortOrder: SortOrder
): Entry[] {
  return useMemo(() => {
    if (!entries || entries.length === 0) return [];
    return sortEntries(entries, sortBy, sortOrder);
  }, [entries, sortBy, sortOrder]);
}
