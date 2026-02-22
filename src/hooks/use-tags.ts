// SPDX-License-Identifier: MIT

import { useQuery } from "@tanstack/react-query";
import { queryKeys } from "@/lib/query-keys";
import { entries } from "@/lib/tauri";
import type { Entry } from "@/lib/types";
import { collectUniqueTags } from "@/lib/tag-utils";

/**
 * Hook to fetch unique tags for a database.
 */
export function useTags(dbId: string | null) {
  return useQuery<Entry[], Error, string[]>({
    queryKey: queryKeys.entries.tags(dbId ?? "none"),
    queryFn: () => (dbId ? entries.list(dbId) : Promise.resolve([])),
    select: collectUniqueTags,
    enabled: Boolean(dbId),
    staleTime: 30_000,
  });
}
