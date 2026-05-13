// SPDX-License-Identifier: MIT

import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { queryKeys } from "@/lib/query-keys";
import { tags } from "@/lib/tauri";
import { saveWithErrorToast } from "@/lib/save-with-error-toast";

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
  const { t } = useTranslation();

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
    mutationFn: async ({ dbId, oldName, newName }) => {
      const count = await tags.rename(dbId, oldName, newName);
      await saveWithErrorToast(dbId, t);
      return count;
    },
    onSettled: () => invalidateAfterTagMutation(),
  });

  const deleteTag = useMutation<number, Error, DeleteTagParams>({
    mutationFn: async ({ dbId, tagName }) => {
      const count = await tags.delete(dbId, tagName);
      await saveWithErrorToast(dbId, t);
      return count;
    },
    onSettled: () => invalidateAfterTagMutation(),
  });

  return { renameTag, deleteTag };
}
