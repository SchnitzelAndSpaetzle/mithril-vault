// SPDX-License-Identifier: MIT

import { useMutation, useQueryClient } from "@tanstack/react-query";
import { queryKeys } from "@/lib/query-keys";
import { database, tags } from "@/lib/tauri";

interface RenameTagParams {
  dbId: string;
  oldName: string;
  newName: string;
}

interface DeleteTagParams {
  dbId: string;
  tagName: string;
}

/**
 * Hook providing mutations for bulk tag operations.
 */
export function useTagMutations(dbId: string | null) {
  const queryClient = useQueryClient();

  const invalidateAfterTagMutation = () => {
    if (dbId) {
      void queryClient.invalidateQueries({
        predicate: (query) =>
          query.queryKey[0] === queryKeys.entries.all[0] &&
          query.queryKey[1] === dbId,
      });
    }
  };

  const renameTag = useMutation<number, Error, RenameTagParams>({
    mutationFn: ({ dbId, oldName, newName }) =>
      tags.rename(dbId, oldName, newName),
    onSuccess: (_data, variables) => {
      void database.save(variables.dbId);
      invalidateAfterTagMutation();
    },
  });

  const deleteTag = useMutation<number, Error, DeleteTagParams>({
    mutationFn: ({ dbId, tagName }) => tags.delete(dbId, tagName),
    onSuccess: (_data, variables) => {
      void database.save(variables.dbId);
      invalidateAfterTagMutation();
    },
  });

  return { renameTag, deleteTag };
}
