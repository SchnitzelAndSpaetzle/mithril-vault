// SPDX-License-Identifier: MIT

import { useQuery } from "@tanstack/react-query";
import { queryKeys } from "@/lib/query-keys";
import { entries } from "@/lib/tauri";
import type { Entry } from "@/lib/types";

function collectTags(entryList: Entry[]) {
  const tags = new Set<string>();
  const splitPattern = /[;,]/;

  const addTagValue = (value: string) => {
    const parts = value.split(splitPattern);
    for (const part of parts) {
      const normalized = part.trim();
      if (normalized.length > 0) {
        tags.add(normalized);
      }
    }
  };

  for (const entry of entryList) {
    for (const tag of entry.tags) {
      addTagValue(tag);
    }

    const customTags = entry.customFields["Tags"] ?? entry.customFields["tags"];
    if (customTags) {
      addTagValue(customTags);
    }
  }

  return Array.from(tags).sort((a, b) =>
    a.localeCompare(b, undefined, { sensitivity: "base" })
  );
}

/**
 * Hook to fetch unique tags for a database.
 */
export function useTags(dbId: string | null) {
  return useQuery<Entry[], Error, string[]>({
    queryKey: queryKeys.entries.tags(dbId ?? "none"),
    queryFn: () => (dbId ? entries.list(dbId) : Promise.resolve([])),
    select: collectTags,
    enabled: Boolean(dbId),
    staleTime: 30_000,
  });
}
