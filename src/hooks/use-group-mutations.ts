// SPDX-License-Identifier: MIT

import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { queryKeys } from "@/lib/query-keys";
import { groups } from "@/lib/tauri";
import { saveWithErrorToast } from "@/lib/save-with-error-toast";
import type { Group } from "@/lib/types";

interface CreateGroupParams {
  dbId: string;
  parentId: string;
  name: string;
}

interface RenameGroupParams {
  dbId: string;
  id: string;
  name: string;
}

interface DeleteGroupParams {
  dbId: string;
  id: string;
}

interface UpdateGroupParams {
  dbId: string;
  id: string;
  data: { name?: string; icon?: string };
}

interface MoveGroupParams {
  dbId: string;
  id: string;
  targetParentId?: string;
}

/**
 * Hook providing mutations for group operations.
 */
export function useGroupMutations(dbId: string | null) {
  const queryClient = useQueryClient();
  const { t } = useTranslation();

  const invalidateAfterGroupMutation = () => {
    if (dbId) {
      void queryClient.invalidateQueries({
        queryKey: queryKeys.groups.list(dbId),
      });
      void queryClient.invalidateQueries({
        queryKey: queryKeys.groups.entryCounts(dbId),
      });
      void queryClient.invalidateQueries({
        queryKey: queryKeys.groups.recycleBinId(dbId),
      });
      void queryClient.invalidateQueries({
        predicate: (query) =>
          query.queryKey[0] === queryKeys.entries.all[0] &&
          query.queryKey[1] === dbId,
      });
    }
  };

  const createGroup = useMutation<Group, Error, CreateGroupParams>({
    mutationFn: async ({ dbId, parentId, name }) => {
      const group = await groups.create(dbId, parentId, name);
      await saveWithErrorToast(dbId, t);
      return group;
    },
    onSettled: () => invalidateAfterGroupMutation(),
  });

  const updateGroup = useMutation<Group, Error, UpdateGroupParams>({
    mutationFn: async ({ dbId, id, data }) => {
      const group = await groups.update(dbId, id, data);
      await saveWithErrorToast(dbId, t);
      return group;
    },
    onSettled: () => invalidateAfterGroupMutation(),
  });

  const renameGroup = useMutation<Group, Error, RenameGroupParams>({
    mutationFn: async ({ dbId, id, name }) => {
      const group = await groups.rename(dbId, id, name);
      await saveWithErrorToast(dbId, t);
      return group;
    },
    onSettled: () => invalidateAfterGroupMutation(),
  });

  const deleteGroup = useMutation<void, Error, DeleteGroupParams>({
    mutationFn: async ({ dbId, id }) => {
      await groups.delete(dbId, id, true);
      await saveWithErrorToast(dbId, t);
    },
    onSettled: () => invalidateAfterGroupMutation(),
  });

  const moveGroup = useMutation<Group, Error, MoveGroupParams>({
    mutationFn: async ({ dbId, id, targetParentId }) => {
      const group = await groups.move(dbId, id, targetParentId);
      await saveWithErrorToast(dbId, t);
      return group;
    },
    onSettled: () => invalidateAfterGroupMutation(),
  });

  return {
    createGroup,
    updateGroup,
    renameGroup,
    deleteGroup,
    moveGroup,
  };
}
