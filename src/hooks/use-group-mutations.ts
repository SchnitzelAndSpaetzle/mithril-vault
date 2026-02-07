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

  const invalidateGroups = () => {
    if (dbId) {
      void queryClient.invalidateQueries({
        queryKey: queryKeys.groups.list(dbId),
      });
      void queryClient.invalidateQueries({
        queryKey: queryKeys.groups.entryCounts(dbId),
      });
    }
  };

  const createGroup = useMutation<Group, Error, CreateGroupParams>({
    mutationFn: ({ dbId, parentId, name }) =>
      groups.create(dbId, parentId, name),
    onSuccess: invalidateGroups,
  });

  const renameGroup = useMutation<Group, Error, RenameGroupParams>({
    mutationFn: ({ dbId, id, name }) => groups.rename(dbId, id, name),
    onSuccess: invalidateGroups,
  });

  const deleteGroup = useMutation<void, Error, DeleteGroupParams>({
    mutationFn: ({ dbId, id }) => groups.delete(dbId, id),
    onSuccess: invalidateGroups,
  });

  const moveGroup = useMutation<Group, Error, MoveGroupParams>({
    mutationFn: ({ dbId, id, targetParentId }) =>
      groups.move(dbId, id, targetParentId),
    onSuccess: invalidateGroups,
  });

  return {
    createGroup,
    renameGroup,
    deleteGroup,
    moveGroup,
  };
}
