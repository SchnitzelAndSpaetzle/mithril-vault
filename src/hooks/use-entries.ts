// SPDX-License-Identifier: MIT

import { useQuery } from "@tanstack/react-query";
import { entries } from "@/lib/tauri";
import { queryKeys } from "@/lib/query-keys";
import type { Entry } from "@/lib/types";
import { z } from "zod/v4";

const KeepassIdSchema = z.guid();

function normalizeGroupId(groupId?: string | null): string | null {
  if (!groupId) {
    return null;
  }

  const trimmed = groupId.trim();
  if (!trimmed || trimmed === "null" || trimmed === "undefined") {
    return null;
  }

  return KeepassIdSchema.safeParse(trimmed).success ? trimmed : null;
}

/**
 * Hook to fetch entries for a database (optionally filtered by group).
 */
export function useEntries(dbId: string | null, groupId?: string | null) {
  const normalizedGroupId = normalizeGroupId(groupId);

  return useQuery<Entry[], Error>({
    queryKey: queryKeys.entries.list(dbId ?? "none", normalizedGroupId),
    queryFn: () =>
      dbId
        ? entries.list(dbId, normalizedGroupId ?? undefined)
        : Promise.resolve([]),
    enabled: Boolean(dbId),
    staleTime: 30_000,
  });
}
