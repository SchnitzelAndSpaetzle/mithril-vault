// SPDX-License-Identifier: MIT

import { useQuery } from "@tanstack/react-query";
import { queryKeys } from "@/lib/query-keys";
import { groups } from "@/lib/tauri";
import type { GroupEntryCounts } from "@/lib/types";

/**
 * Hook to fetch entry counts per group.
 */
export function useGroupEntryCounts(dbId: string | null) {
  return useQuery<GroupEntryCounts, Error>({
    queryKey: queryKeys.groups.entryCounts(dbId ?? "none"),
    queryFn: () => (dbId ? groups.getEntryCounts(dbId) : Promise.resolve({})),
    enabled: Boolean(dbId),
    staleTime: 30_000,
  });
}
