import { useState } from "react";
import { useNavigate, useSearch } from "@tanstack/react-router";
import {
  SidebarGroup,
  SidebarMenu,
  SidebarMenuBadge,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarMenuSub,
  SidebarMenuSubItem,
} from "@/components/ui/sidebar.tsx";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible.tsx";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { EllipsisVertical, Pencil, Tags, Trash2 } from "lucide-react";
import { useTags } from "@/hooks/use-tags";
import { useTagMutations } from "@/hooks/use-tag-mutations";
import { toast } from "sonner";

interface NavTagsProps {
  dbId: string;
}

export default function NavTags({ dbId }: NavTagsProps) {
  const { data: tags } = useTags(dbId);
  const search = useSearch({ strict: false });
  const navigate = useNavigate();
  const { renameTag, deleteTag } = useTagMutations(dbId);

  const activeTag = (search.tag as string | undefined) ?? null;

  const [renameDialogOpen, setRenameDialogOpen] = useState(false);
  const [deleteDialogOpen, setDeleteDialogOpen] = useState(false);
  const [targetTag, setTargetTag] = useState("");
  const [newTagName, setNewTagName] = useState("");

  const tagList = tags ?? [];

  const handleTagClick = (tag: string) => {
    void navigate({
      to: "/dashboard/index/$dbId",
      params: { dbId },
      search: (prev: Record<string, unknown>) => {
        const { groupId: _, tag: __, ...rest } = prev;
        return { ...rest, tag };
      },
    });
  };

  const openRenameDialog = (tag: string) => {
    setTargetTag(tag);
    setNewTagName(tag);
    setRenameDialogOpen(true);
  };

  const openDeleteDialog = (tag: string) => {
    setTargetTag(tag);
    setDeleteDialogOpen(true);
  };

  const handleRename = () => {
    const trimmed = newTagName.trim();
    if (!trimmed || trimmed === targetTag) {
      setRenameDialogOpen(false);
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
          toast.error(`Failed to delete tag: ${error.message}`);
        },
      }
    );
  };

  return (
    <>
      <SidebarGroup className="group-data-[collapsible=icon]:hidden">
        <SidebarMenu>
          <Collapsible defaultOpen className="group/collapsible">
            <SidebarMenuItem>
              <CollapsibleTrigger asChild>
                <div>
                  <SidebarMenuButton>
                    <Tags />
                    Tags
                  </SidebarMenuButton>
                  <SidebarMenuBadge>{tagList.length}</SidebarMenuBadge>
                </div>
              </CollapsibleTrigger>
              <CollapsibleContent>
                <SidebarMenuSub>
                  {tagList.map((tag) => (
                    <SidebarMenuSubItem key={tag}>
                      <SidebarMenuButton
                        isActive={activeTag === tag}
                        onClick={() => handleTagClick(tag)}
                        className="cursor-pointer"
                      >
                        {tag}
                      </SidebarMenuButton>
                      <DropdownMenu>
                        <DropdownMenuTrigger asChild>
                          <button className="absolute right-1 top-1/2 -translate-y-1/2 rounded-sm p-0.5 opacity-0 hover:bg-sidebar-accent group-hover/menu-sub-item:opacity-100 focus-visible:opacity-100">
                            <EllipsisVertical className="size-4" />
                          </button>
                        </DropdownMenuTrigger>
                        <DropdownMenuContent side="right" align="start">
                          <DropdownMenuItem
                            onClick={() => openRenameDialog(tag)}
                          >
                            <Pencil className="mr-2 size-4" />
                            Rename
                          </DropdownMenuItem>
                          <DropdownMenuItem
                            onClick={() => openDeleteDialog(tag)}
                            className="text-destructive focus:text-destructive"
                          >
                            <Trash2 className="mr-2 size-4" />
                            Delete
                          </DropdownMenuItem>
                        </DropdownMenuContent>
                      </DropdownMenu>
                    </SidebarMenuSubItem>
                  ))}
                </SidebarMenuSub>
              </CollapsibleContent>
            </SidebarMenuItem>
          </Collapsible>
        </SidebarMenu>
      </SidebarGroup>

      <Dialog open={renameDialogOpen} onOpenChange={setRenameDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Rename Tag</DialogTitle>
            <DialogDescription>
              This will rename &quot;{targetTag}&quot; across all entries.
            </DialogDescription>
          </DialogHeader>
          <Input
            value={newTagName}
            onChange={(e) => setNewTagName(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") handleRename();
            }}
            placeholder="New tag name"
            autoFocus
          />
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => setRenameDialogOpen(false)}
            >
              Cancel
            </Button>
            <Button
              onClick={handleRename}
              disabled={
                !newTagName.trim() ||
                newTagName.trim() === targetTag ||
                renameTag.isPending
              }
            >
              {renameTag.isPending ? "Renaming..." : "Rename"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog open={deleteDialogOpen} onOpenChange={setDeleteDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Delete Tag</DialogTitle>
            <DialogDescription>
              Are you sure you want to remove &quot;{targetTag}&quot; from all
              entries? This cannot be undone.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => setDeleteDialogOpen(false)}
            >
              Cancel
            </Button>
            <Button
              variant="destructive"
              onClick={handleDelete}
              disabled={deleteTag.isPending}
            >
              {deleteTag.isPending ? "Deleting..." : "Delete"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}
