// SPDX-License-Identifier: MIT

import { createElement, useState } from "react";
import {
  FolderInput,
  FolderPlus,
  MoreHorizontal,
  Pencil,
  Trash2,
} from "lucide-react";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
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
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { SidebarMenuAction } from "@/components/ui/sidebar";
import { IconPickerPopover } from "@/components/entries/IconPickerPopover";
import { getKeepassIcon, parseGroupIconId } from "@/lib/keepass-icons";
import {
  flattenGroups,
  getDescendantIds,
  sumGroupEntryCounts,
} from "@/lib/group-utils";
import { useGroups } from "@/hooks/use-groups";
import { useGroupEntryCounts } from "@/hooks/use-group-entry-counts";
import type { Group } from "@/lib/types";

const DEFAULT_FOLDER_ICON = 48;

interface GroupTreeItemActionsProps {
  group: Group;
  dbId: string;
  isRoot: boolean;
  onCreateSubgroup: (name: string, iconId: number) => void;
  onUpdateGroup: (data: { name?: string; icon?: string }) => void;
  onDelete: () => void;
  onMove: (targetParentId: string) => void;
  isCreatePending: boolean;
  isUpdatePending: boolean;
  isDeletePending: boolean;
  isMovePending: boolean;
}

export function GroupTreeItemActions({
  group,
  dbId,
  isRoot,
  onCreateSubgroup,
  onUpdateGroup,
  onDelete,
  onMove,
  isCreatePending,
  isUpdatePending,
  isDeletePending,
  isMovePending,
}: GroupTreeItemActionsProps) {
  const [createDialogOpen, setCreateDialogOpen] = useState(false);
  const [editDialogOpen, setEditDialogOpen] = useState(false);
  const [deleteDialogOpen, setDeleteDialogOpen] = useState(false);
  const [moveDialogOpen, setMoveDialogOpen] = useState(false);

  // Create subgroup state
  const [newGroupName, setNewGroupName] = useState("");
  const [newGroupIcon, setNewGroupIcon] = useState(DEFAULT_FOLDER_ICON);

  // Edit group state
  const [editedName, setEditedName] = useState(group.name);
  const [editedIcon, setEditedIcon] = useState(
    parseGroupIconId(group.icon) ?? DEFAULT_FOLDER_ICON
  );

  // Move group state
  const [targetParentId, setTargetParentId] = useState("");

  // Data for delete warning and move dialog
  const { data: entryCounts } = useGroupEntryCounts(
    deleteDialogOpen ? dbId : null
  );
  const { data: allGroups } = useGroups(moveDialogOpen ? dbId : null);

  const totalEntries =
    entryCounts && deleteDialogOpen
      ? sumGroupEntryCounts(group, entryCounts)
      : 0;

  // Build filtered group list for move dialog (exclude self and descendants)
  const excludedIds = new Set([group.id, ...getDescendantIds(group)]);
  const moveTargets = allGroups
    ? flattenGroups(allGroups).filter((g) => !excludedIds.has(g.id))
    : [];

  const handleCreateSubmit = () => {
    if (newGroupName.trim()) {
      onCreateSubgroup(newGroupName.trim(), newGroupIcon);
      setNewGroupName("");
      setNewGroupIcon(DEFAULT_FOLDER_ICON);
      setCreateDialogOpen(false);
    }
  };

  const handleEditSubmit = () => {
    const trimmedName = editedName.trim();
    if (!trimmedName) return;

    const nameChanged = trimmedName !== group.name;
    const currentIcon = parseGroupIconId(group.icon) ?? DEFAULT_FOLDER_ICON;
    const iconChanged = editedIcon !== currentIcon;

    if (!nameChanged && !iconChanged) return;

    const data: { name?: string; icon?: string } = {};
    if (nameChanged) data.name = trimmedName;
    if (iconChanged) data.icon = String(editedIcon);

    onUpdateGroup(data);
    setEditDialogOpen(false);
  };

  const handleDeleteConfirm = () => {
    onDelete();
    setDeleteDialogOpen(false);
  };

  const handleMoveSubmit = () => {
    if (targetParentId) {
      onMove(targetParentId);
      setTargetParentId("");
      setMoveDialogOpen(false);
    }
  };

  const handleEditDialogOpen = (open: boolean) => {
    if (open) {
      setEditedName(group.name);
      setEditedIcon(parseGroupIconId(group.icon) ?? DEFAULT_FOLDER_ICON);
    }
    setEditDialogOpen(open);
  };

  return (
    <>
      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <SidebarMenuAction showOnHover>
            <MoreHorizontal />
            <span className="sr-only">Group actions</span>
          </SidebarMenuAction>
        </DropdownMenuTrigger>
        <DropdownMenuContent side="right" align="start">
          <DropdownMenuItem onSelect={() => setCreateDialogOpen(true)}>
            <FolderPlus className="mr-2 h-4 w-4" />
            New Subgroup
          </DropdownMenuItem>
          {!isRoot && (
            <>
              <DropdownMenuItem onSelect={() => handleEditDialogOpen(true)}>
                <Pencil className="mr-2 h-4 w-4" />
                Edit
              </DropdownMenuItem>
              <DropdownMenuItem onSelect={() => setMoveDialogOpen(true)}>
                <FolderInput className="mr-2 h-4 w-4" />
                Move to...
              </DropdownMenuItem>
              <DropdownMenuSeparator />
              <DropdownMenuItem
                onSelect={() => setDeleteDialogOpen(true)}
                className="text-destructive focus:text-destructive"
              >
                <Trash2 className="mr-2 h-4 w-4" />
                Delete
              </DropdownMenuItem>
            </>
          )}
        </DropdownMenuContent>
      </DropdownMenu>

      {/* Create Subgroup Dialog */}
      <Dialog open={createDialogOpen} onOpenChange={setCreateDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Create New Group</DialogTitle>
            <DialogDescription>
              Create a new subgroup inside &ldquo;{group.name}&rdquo;.
            </DialogDescription>
          </DialogHeader>
          <div className="grid gap-4 py-4">
            <div className="grid gap-2">
              <Label htmlFor="new-group-name">Name</Label>
              <div className="flex gap-2">
                <IconPickerPopover
                  selectedIconId={newGroupIcon}
                  onSelect={setNewGroupIcon}
                >
                  <Button
                    variant="outline"
                    size="icon"
                    className="shrink-0"
                    type="button"
                  >
                    {createElement(getKeepassIcon(newGroupIcon), {
                      className: "h-4 w-4",
                    })}
                  </Button>
                </IconPickerPopover>
                <Input
                  id="new-group-name"
                  value={newGroupName}
                  onChange={(e) => setNewGroupName(e.target.value)}
                  placeholder="Enter group name"
                  onKeyDown={(e) => {
                    if (e.key === "Enter") {
                      handleCreateSubmit();
                    }
                  }}
                />
              </div>
            </div>
          </div>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => setCreateDialogOpen(false)}
            >
              Cancel
            </Button>
            <Button
              onClick={handleCreateSubmit}
              disabled={!newGroupName.trim() || isCreatePending}
            >
              {isCreatePending ? "Creating..." : "Create"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Edit Group Dialog */}
      <Dialog open={editDialogOpen} onOpenChange={handleEditDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Edit Group</DialogTitle>
            <DialogDescription>
              Change the name or icon for &ldquo;{group.name}&rdquo;.
            </DialogDescription>
          </DialogHeader>
          <div className="grid gap-4 py-4">
            <div className="grid gap-2">
              <Label htmlFor="edit-group-name">Name</Label>
              <div className="flex gap-2">
                <IconPickerPopover
                  selectedIconId={editedIcon}
                  onSelect={setEditedIcon}
                >
                  <Button
                    variant="outline"
                    size="icon"
                    className="shrink-0"
                    type="button"
                  >
                    {createElement(getKeepassIcon(editedIcon), {
                      className: "h-4 w-4",
                    })}
                  </Button>
                </IconPickerPopover>
                <Input
                  id="edit-group-name"
                  value={editedName}
                  onChange={(e) => setEditedName(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") {
                      handleEditSubmit();
                    }
                  }}
                />
              </div>
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setEditDialogOpen(false)}>
              Cancel
            </Button>
            <Button
              onClick={handleEditSubmit}
              disabled={
                !editedName.trim() ||
                (editedName.trim() === group.name &&
                  editedIcon ===
                    (parseGroupIconId(group.icon) ?? DEFAULT_FOLDER_ICON)) ||
                isUpdatePending
              }
            >
              {isUpdatePending ? "Saving..." : "Save"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Delete Group Dialog */}
      <Dialog open={deleteDialogOpen} onOpenChange={setDeleteDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Delete Group</DialogTitle>
            <DialogDescription>
              Are you sure you want to delete &ldquo;{group.name}&rdquo;?
              {totalEntries > 0 && (
                <>
                  {" "}
                  This group contains{" "}
                  <strong>
                    {totalEntries} {totalEntries === 1 ? "entry" : "entries"}
                  </strong>
                  .
                </>
              )}{" "}
              This will move the group and all its contents to the Recycle Bin.
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
              onClick={handleDeleteConfirm}
              disabled={isDeletePending}
            >
              {isDeletePending ? "Deleting..." : "Delete"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Move Group Dialog */}
      <Dialog open={moveDialogOpen} onOpenChange={setMoveDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Move Group</DialogTitle>
            <DialogDescription>
              Select a new parent group for &ldquo;{group.name}&rdquo;.
            </DialogDescription>
          </DialogHeader>
          <div className="grid gap-4 py-4">
            <div className="grid gap-2">
              <Label htmlFor="move-target">Destination</Label>
              <Select value={targetParentId} onValueChange={setTargetParentId}>
                <SelectTrigger className="w-full" id="move-target">
                  <SelectValue placeholder="Select a group" />
                </SelectTrigger>
                <SelectContent>
                  {moveTargets.map((g) => (
                    <SelectItem key={g.id} value={g.id}>
                      <span style={{ paddingLeft: `${g.depth * 12}px` }}>
                        {g.name}
                      </span>
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setMoveDialogOpen(false)}>
              Cancel
            </Button>
            <Button
              onClick={handleMoveSubmit}
              disabled={!targetParentId || isMovePending}
            >
              {isMovePending ? "Moving..." : "Move"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}
