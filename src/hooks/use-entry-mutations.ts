// SPDX-License-Identifier: MIT

import { useMutation, useQueryClient } from "@tanstack/react-query";
import { queryKeys } from "@/lib/query-keys";
import { entries } from "@/lib/tauri";
import type { CreateEntryData, Entry, UpdateEntryData } from "@/lib/types";

interface CreateEntryParams {
  dbId: string;
  groupId: string;
  data: CreateEntryData;
}

interface UpdateEntryParams {
  dbId: string;
  id: string;
  data: UpdateEntryData;
}

interface DeleteEntryParams {
  dbId: string;
  id: string;
}

/**
 * Hook providing mutations for entry operations.
 */
export function useEntryMutations(dbId: string | null) {
  const queryClient = useQueryClient();

  const invalidateAfterEntryMutation = () => {
    if (dbId) {
      void queryClient.invalidateQueries({
        predicate: (query) =>
          query.queryKey[0] === queryKeys.entries.all[0] &&
          query.queryKey[1] === dbId,
      });
      void queryClient.invalidateQueries({
        queryKey: queryKeys.groups.entryCounts(dbId),
      });
    }
  };

  const createEntry = useMutation<Entry, Error, CreateEntryParams>({
    mutationFn: ({ dbId, groupId, data }) =>
      entries.create(dbId, groupId, data),
    onSuccess: invalidateAfterEntryMutation,
  });

  const updateEntry = useMutation<Entry, Error, UpdateEntryParams>({
    mutationFn: ({ dbId, id, data }) => entries.update(dbId, id, data),
    onSuccess: invalidateAfterEntryMutation,
  });

  const deleteEntry = useMutation<void, Error, DeleteEntryParams>({
    mutationFn: ({ dbId, id }) => entries.delete(dbId, id),
    onSuccess: invalidateAfterEntryMutation,
  });

  return {
    createEntry,
    updateEntry,
    deleteEntry,
  };
}
