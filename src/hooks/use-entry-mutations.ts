// SPDX-License-Identifier: MIT

import { useMutation, useQueryClient } from "@tanstack/react-query";
import { queryKeys } from "@/lib/query-keys";
import { database, entries } from "@/lib/tauri";
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

interface MoveEntryParams {
  dbId: string;
  id: string;
  targetGroupId: string;
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
    onSuccess: (_data, variables) => {
      void database.save(variables.dbId);
      invalidateAfterEntryMutation();
    },
  });

  const updateEntry = useMutation<Entry, Error, UpdateEntryParams>({
    mutationFn: ({ dbId, id, data }) => entries.update(dbId, id, data),
    onSuccess: (_data, variables) => {
      void database.save(variables.dbId);
      invalidateAfterEntryMutation();
    },
  });

  const moveEntry = useMutation<Entry, Error, MoveEntryParams>({
    mutationFn: ({ dbId, id, targetGroupId }) =>
      entries.move(dbId, id, targetGroupId),
    onSuccess: (_data, variables) => {
      void database.save(variables.dbId);
      invalidateAfterEntryMutation();
    },
  });

  const deleteEntry = useMutation<void, Error, DeleteEntryParams>({
    mutationFn: ({ dbId, id }) => entries.delete(dbId, id),
    onSuccess: (_data, variables) => {
      void database.save(variables.dbId);
      invalidateAfterEntryMutation();
    },
  });

  return {
    createEntry,
    updateEntry,
    moveEntry,
    deleteEntry,
  };
}
