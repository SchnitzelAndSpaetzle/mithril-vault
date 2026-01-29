// SPDX-License-Identifier: MIT

import { useEffect, useState } from "react";
import { database } from "@/lib/tauri";
import type { DatabaseInfo } from "@/lib/types";

interface UseDatabaseInfoResult {
  databaseInfo: DatabaseInfo | null;
  isLoading: boolean;
  error: Error | null;
  refetch: () => Promise<void>;
}

/**
 * Hook to fetch the currently open database info.
 * Returns null if no database is open.
 */
export function useDatabaseInfo(): UseDatabaseInfoResult {
  const [databaseInfo, setDatabaseInfo] = useState<DatabaseInfo | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<Error | null>(null);

  const fetchInfo = async () => {
    setIsLoading(true);
    setError(null);
    try {
      const info = await database.getInfo();
      setDatabaseInfo(info);
    } catch (err) {
      setError(err instanceof Error ? err : new Error(String(err)));
      setDatabaseInfo(null);
    } finally {
      setIsLoading(false);
    }
  };

  useEffect(() => {
    void fetchInfo();
  }, []);

  return {
    databaseInfo,
    isLoading,
    error,
    refetch: fetchInfo,
  };
}
