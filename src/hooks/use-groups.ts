// SPDX-License-Identifier: MIT

import { useQuery } from "@tanstack/react-query";
import { queryKeys } from "@/lib/query-keys";
import { groups } from "@/lib/tauri";
import type { Group } from "@/lib/types";

/**
 * Hook to fetch groups for a database.
 * Returns the full group tree.
 */
export function useGroups(dbId: string | null) {
  return useQuery<Group[], Error>({
    queryKey: queryKeys.groups.list(dbId ?? "none"),
    queryFn: () => (dbId ? groups.list(dbId) : Promise.resolve([])),
    enabled: Boolean(dbId),
    staleTime: 30_000,
  });
}
