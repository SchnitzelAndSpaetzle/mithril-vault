import { useState } from "react";
import { useNavigate, useSearch } from "@tanstack/react-router";
import { toast } from "sonner";
import { useTags } from "@/hooks/use-tags";
import { useTagMutations } from "@/hooks/use-tag-mutations";
import { SaveError } from "@/lib/save-with-error-toast";

interface UseNavTagsControllerResult {
  tagList: string[];
  activeTag: string | null;
  renameDialogOpen: boolean;
  deleteDialogOpen: boolean;
  targetTag: string;
  setRenameDialogOpen: (open: boolean) => void;
  setDeleteDialogOpen: (open: boolean) => void;
  handleTagClick: (tag: string) => void;
  openRenameDialog: (tag: string) => void;
  openDeleteDialog: (tag: string) => void;
  handleRename: (newName: string) => void;
  handleDelete: () => void;
  isRenamePending: boolean;
  isDeletePending: boolean;
}

export function useNavTagsController(dbId: string): UseNavTagsControllerResult {
  const { data: tags } = useTags(dbId);
  const search = useSearch({ strict: false });
  const navigate = useNavigate();
  const { renameTag, deleteTag } = useTagMutations(dbId);

  const activeTag = (search.tag as string | undefined) ?? null;
  const tagList = tags ?? [];

  const [renameDialogOpen, setRenameDialogOpen] = useState(false);
  const [deleteDialogOpen, setDeleteDialogOpen] = useState(false);
  const [targetTag, setTargetTag] = useState("");

  const handleTagClick = (tag: string) => {
    void navigate({
      to: "/dashboard/index/$dbId",
      params: { dbId },
      search: (prev: Record<string, unknown>) => {
        const { groupId: _groupId, tag: _tag, ...rest } = prev;
        return { ...rest, tag };
      },
    });
  };

  const openRenameDialog = (tag: string) => {
    setTargetTag(tag);
    setRenameDialogOpen(true);
  };

  const openDeleteDialog = (tag: string) => {
    setTargetTag(tag);
    setDeleteDialogOpen(true);
  };

  const handleRename = (newName: string) => {
    const trimmed = newName.trim();
    if (!trimmed || trimmed === targetTag) {
      return;
    }

    renameTag.mutate(
      { dbId, oldName: targetTag, newName: trimmed },
      {
        onSuccess: (count) => {
          toast.success(
            `Renamed tag in ${count} ${count === 1 ? "entry" : "entries"}.`
          );
          setRenameDialogOpen(false);
          if (activeTag === targetTag) {
            void navigate({
              to: "/dashboard/index/$dbId",
              params: { dbId },
              search: (prev) => ({
                ...prev,
                tag: trimmed,
              }),
            });
          }
        },
        onError: (error) => {
          if (error instanceof SaveError) return;
          toast.error(`Failed to rename tag: ${error.message}`);
        },
      }
    );
  };

  const handleDelete = () => {
    deleteTag.mutate(
      { dbId, tagName: targetTag },
      {
        onSuccess: (count) => {
          toast.success(
            `Removed tag from ${count} ${count === 1 ? "entry" : "entries"}.`
          );
          setDeleteDialogOpen(false);
          if (activeTag === targetTag) {
            void navigate({
              to: "/dashboard/index/$dbId",
              params: { dbId },
              search: (prev) => {
                const { tag: _tag, ...rest } = prev;
                return rest;
              },
            });
          }
        },
        onError: (error) => {
          if (error instanceof SaveError) return;
          toast.error(`Failed to delete tag: ${error.message}`);
        },
      }
    );
  };

  return {
    tagList,
    activeTag,
    renameDialogOpen,
    deleteDialogOpen,
    targetTag,
    setRenameDialogOpen,
    setDeleteDialogOpen,
    handleTagClick,
    openRenameDialog,
    openDeleteDialog,
    handleRename,
    handleDelete,
    isRenamePending: renameTag.isPending,
    isDeletePending: deleteTag.isPending,
  };
}
