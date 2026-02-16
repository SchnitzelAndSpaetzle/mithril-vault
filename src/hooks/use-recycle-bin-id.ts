// SPDX-License-Identifier: MIT

import { useQuery } from "@tanstack/react-query";
import { queryKeys } from "@/lib/query-keys";
import { groups } from "@/lib/tauri";

/**
 * Hook to fetch the recycle bin group ID if it exists.
 */
export function useRecycleBinId(dbId: string | null) {
  return useQuery<string | null, Error>({
    queryKey: queryKeys.groups.recycleBinId(dbId ?? "none"),
    queryFn: () =>
      dbId ? groups.getRecycleBinId(dbId) : Promise.resolve(null),
    enabled: Boolean(dbId),
    staleTime: 60_000,
  });
}
