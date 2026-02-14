// SPDX-License-Identifier: MIT

import { useMutation, useQueryClient } from "@tanstack/react-query";
import { queryKeys } from "@/lib/query-keys";
import { groups } from "@/lib/tauri";
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
    mutationFn: ({ dbId, parentId, name }) =>
      groups.create(dbId, parentId, name),
    onSuccess: invalidateAfterGroupMutation,
  });

  const renameGroup = useMutation<Group, Error, RenameGroupParams>({
    mutationFn: ({ dbId, id, name }) => groups.rename(dbId, id, name),
    onSuccess: invalidateAfterGroupMutation,
  });

  const deleteGroup = useMutation<void, Error, DeleteGroupParams>({
    mutationFn: ({ dbId, id }) => groups.delete(dbId, id, true),
    onSuccess: invalidateAfterGroupMutation,
  });

  const moveGroup = useMutation<Group, Error, MoveGroupParams>({
    mutationFn: ({ dbId, id, targetParentId }) =>
      groups.move(dbId, id, targetParentId),
    onSuccess: invalidateAfterGroupMutation,
  });

  return {
    createGroup,
    renameGroup,
    deleteGroup,
    moveGroup,
  };
}
