// SPDX-License-Identifier: MIT

import {
  queryOptions,
  useQuery,
  type UseQueryResult,
} from "@tanstack/react-query";
import { queryKeys } from "@/lib/query-keys";
import { settings } from "@/lib/tauri";
import type { RecentDatabase } from "@/lib/types";

interface UseRecentDatabasesResult {
  recentDatabases: RecentDatabase[];
  isLoading: boolean;
  error: Error | null;
  refetch: UseQueryResult<RecentDatabase[], Error>["refetch"];
}

export const recentDatabasesQueryOptions = queryOptions<
  RecentDatabase[],
  Error
>({
  queryKey: queryKeys.settings.recentDatabases(),
  queryFn: async () => {
    const appSettings = await settings.get();
    return appSettings.recentDatabases;
  },
  retry: false,
  refetchOnWindowFocus: true,
});

/**
 * Hook to fetch the list of recent databases from settings.
 */
export function useRecentDatabases(): UseRecentDatabasesResult {
  const { data, isLoading, error, refetch } = useQuery(
    recentDatabasesQueryOptions
  );

  return {
    recentDatabases: data ?? [],
    isLoading,
    error: error ?? null,
    refetch,
  };
}
