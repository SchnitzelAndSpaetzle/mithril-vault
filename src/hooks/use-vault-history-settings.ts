// SPDX-License-Identifier: MIT

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { queryKeys } from "@/lib/query-keys";
import { database } from "@/lib/tauri";
import { saveWithErrorToast } from "@/lib/save-with-error-toast";
import type { VaultHistorySettings } from "@/lib/types";

/**
 * Reads and writes the per-Vault Entry-History retention
 * (`Meta.history_max_items`) — the writable vault-meta surface (#326). The
 * update persists to disk via {@link saveWithErrorToast} so the limit travels
 * with the file, then invalidates the cached read.
 */
export function useVaultHistorySettings(dbId: string | null) {
  const queryClient = useQueryClient();
  const { t } = useTranslation();

  const query = useQuery<VaultHistorySettings | null, Error>({
    queryKey: queryKeys.database.historySettings(dbId ?? "none"),
    queryFn: () =>
      dbId ? database.getHistorySettings(dbId) : Promise.resolve(null),
    enabled: Boolean(dbId),
    staleTime: 30_000,
  });

  const mutation = useMutation<void, Error, number | null>({
    mutationFn: async (maxItems) => {
      if (!dbId) throw new Error("no database open");
      await database.updateHistorySettings(dbId, maxItems);
      await saveWithErrorToast(dbId, t);
    },
    onSuccess: () => {
      void queryClient.invalidateQueries({
        queryKey: queryKeys.database.historySettings(dbId ?? "none"),
      });
    },
  });

  return {
    settings: query.data ?? null,
    isLoading: query.isLoading,
    error: query.error,
    update: mutation.mutate,
    isUpdating: mutation.isPending,
  };
}
