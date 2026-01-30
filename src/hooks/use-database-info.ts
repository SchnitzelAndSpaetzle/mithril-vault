// SPDX-License-Identifier: MIT

import { useQuery, type UseQueryResult } from "@tanstack/react-query";
import { queryKeys } from "@/lib/query-keys";
import { database } from "@/lib/tauri";
import type { DatabaseInfo } from "@/lib/types";

interface UseDatabaseInfoResult {
  databaseInfo: DatabaseInfo | null;
  isLoading: boolean;
  error: Error | null;
  refetch: UseQueryResult<DatabaseInfo | null, Error>["refetch"];
}

/**
 * Hook to fetch the currently open database info.
 * Returns null if no database is open.
 */
export function useDatabaseInfo(): UseDatabaseInfoResult {
  const { data, isLoading, error, refetch } = useQuery<
    DatabaseInfo | null,
    Error
  >({
    queryKey: queryKeys.database.info(),
    queryFn: () => database.getInfo(),
    retry: false,
    refetchOnWindowFocus: true,
  });

  return {
    databaseInfo: data ?? null,
    isLoading,
    error: error ?? null,
    refetch,
  };
}
