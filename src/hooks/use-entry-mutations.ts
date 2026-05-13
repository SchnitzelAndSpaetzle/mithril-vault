// SPDX-License-Identifier: MIT

import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { queryKeys } from "@/lib/query-keys";
import { entries } from "@/lib/tauri";
import { saveWithErrorToast } from "@/lib/save-with-error-toast";
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
  const { t } = useTranslation();

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
    mutationFn: async ({ dbId, groupId, data }) => {
      const entry = await entries.create(dbId, groupId, data);
      await saveWithErrorToast(dbId, t);
      return entry;
    },
    onSettled: () => invalidateAfterEntryMutation(),
  });

  const updateEntry = useMutation<Entry, Error, UpdateEntryParams>({
    mutationFn: async ({ dbId, id, data }) => {
      const entry = await entries.update(dbId, id, data);
      await saveWithErrorToast(dbId, t);
      return entry;
    },
    onSettled: () => invalidateAfterEntryMutation(),
  });

  const moveEntry = useMutation<Entry, Error, MoveEntryParams>({
    mutationFn: async ({ dbId, id, targetGroupId }) => {
      const entry = await entries.move(dbId, id, targetGroupId);
      await saveWithErrorToast(dbId, t);
      return entry;
    },
    onSettled: () => invalidateAfterEntryMutation(),
  });

  const deleteEntry = useMutation<void, Error, DeleteEntryParams>({
    mutationFn: async ({ dbId, id }) => {
      await entries.delete(dbId, id);
      await saveWithErrorToast(dbId, t);
    },
    onSettled: () => invalidateAfterEntryMutation(),
  });

  return {
    createEntry,
    updateEntry,
    moveEntry,
    deleteEntry,
  };
}
