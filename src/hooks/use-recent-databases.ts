// SPDX-License-Identifier: MIT

import { useEffect, useState } from "react";
import { settings } from "@/lib/tauri";
import type { RecentDatabase } from "@/lib/types";

interface UseRecentDatabasesResult {
  recentDatabases: RecentDatabase[];
  isLoading: boolean;
  error: Error | null;
  refetch: () => Promise<void>;
}

/**
 * Hook to fetch the list of recent databases from settings.
 */
export function useRecentDatabases(): UseRecentDatabasesResult {
  const [recentDatabases, setRecentDatabases] = useState<RecentDatabase[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<Error | null>(null);

  const fetchRecentDatabases = async () => {
    setIsLoading(true);
    setError(null);
    try {
      const appSettings = await settings.get();
      setRecentDatabases(appSettings.recentDatabases);
    } catch (err) {
      setError(err instanceof Error ? err : new Error(String(err)));
      setRecentDatabases([]);
    } finally {
      setIsLoading(false);
    }
  };

  useEffect(() => {
    void fetchRecentDatabases();
  }, []);

  return {
    recentDatabases,
    isLoading,
    error,
    refetch: fetchRecentDatabases,
  };
}
