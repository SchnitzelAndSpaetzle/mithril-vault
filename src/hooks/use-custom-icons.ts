// SPDX-License-Identifier: MIT

import { useQuery } from "@tanstack/react-query";
import { queryKeys } from "@/lib/query-keys";
import { database } from "@/lib/tauri";
import type { CustomIconMap } from "@/lib/types";

/**
 * Hook to fetch custom icon data for a database.
 */
export function useCustomIcons(dbId: string | null) {
  return useQuery<CustomIconMap, Error>({
    queryKey: queryKeys.database.customIcons(dbId ?? "none"),
    queryFn: () => (dbId ? database.getCustomIcons(dbId) : Promise.resolve({})),
    enabled: Boolean(dbId),
    staleTime: 60_000,
  });
}
