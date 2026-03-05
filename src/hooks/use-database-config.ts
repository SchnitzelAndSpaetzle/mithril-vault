// SPDX-License-Identifier: MIT

import { useQuery } from "@tanstack/react-query";
import { queryKeys } from "@/lib/query-keys";
import { database } from "@/lib/tauri";
import type { DatabaseConfig } from "@/lib/types";

export function useDatabaseConfig(dbId: string | null) {
  return useQuery<DatabaseConfig | null, Error>({
    queryKey: queryKeys.database.config(dbId ?? "none"),
    queryFn: () => (dbId ? database.getConfig(dbId) : Promise.resolve(null)),
    enabled: Boolean(dbId),
    staleTime: 30_000,
  });
}
