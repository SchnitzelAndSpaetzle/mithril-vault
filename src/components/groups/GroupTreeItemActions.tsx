// SPDX-License-Identifier: MIT

import { useState } from "react";
import { useTranslation } from "react-i18next";
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
import { SidebarMenuAction } from "@/components/ui/sidebar";
import type { Group } from "@/lib/types";
import { GroupTreeCreateSubgroupDialog } from "./GroupTreeCreateSubgroupDialog";
import { GroupTreeEditGroupDialog } from "./GroupTreeEditGroupDialog";
import { GroupTreeDeleteGroupDialog } from "./GroupTreeDeleteGroupDialog";
import { GroupTreeMoveGroupDialog } from "./GroupTreeMoveGroupDialog";

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
  const { t } = useTranslation();
  const [createDialogOpen, setCreateDialogOpen] = useState(false);
  const [editDialogOpen, setEditDialogOpen] = useState(false);
  const [deleteDialogOpen, setDeleteDialogOpen] = useState(false);
  const [moveDialogOpen, setMoveDialogOpen] = useState(false);

  return (
    <>
      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <SidebarMenuAction showOnHover>
            <MoreHorizontal />
            <span className="sr-only">{t("groups.actions")}</span>
          </SidebarMenuAction>
        </DropdownMenuTrigger>
        <DropdownMenuContent side="right" align="start">
          <DropdownMenuItem onSelect={() => setCreateDialogOpen(true)}>
            <FolderPlus className="mr-2 h-4 w-4" />
            {t("groups.newSubgroup")}
          </DropdownMenuItem>
          {!isRoot && (
            <>
              <DropdownMenuItem onSelect={() => setEditDialogOpen(true)}>
                <Pencil className="mr-2 h-4 w-4" />
                {t("common.edit")}
              </DropdownMenuItem>
              <DropdownMenuItem onSelect={() => setMoveDialogOpen(true)}>
                <FolderInput className="mr-2 h-4 w-4" />
                {t("groups.moveTo")}
              </DropdownMenuItem>
              <DropdownMenuSeparator />
              <DropdownMenuItem
                onSelect={() => setDeleteDialogOpen(true)}
                className="text-destructive focus:text-destructive"
              >
                <Trash2 className="mr-2 h-4 w-4" />
                {t("common.delete")}
              </DropdownMenuItem>
            </>
          )}
        </DropdownMenuContent>
      </DropdownMenu>

      <GroupTreeCreateSubgroupDialog
        open={createDialogOpen}
        onOpenChange={setCreateDialogOpen}
        parentName={group.name}
        onCreateSubgroup={onCreateSubgroup}
        isPending={isCreatePending}
      />
      <GroupTreeEditGroupDialog
        key={`${group.id}-${editDialogOpen ? "open" : "closed"}`}
        open={editDialogOpen}
        onOpenChange={setEditDialogOpen}
        group={group}
        onUpdateGroup={onUpdateGroup}
        isPending={isUpdatePending}
      />
      <GroupTreeDeleteGroupDialog
        open={deleteDialogOpen}
        onOpenChange={setDeleteDialogOpen}
        group={group}
        dbId={dbId}
        onDelete={onDelete}
        isPending={isDeletePending}
      />
      <GroupTreeMoveGroupDialog
        open={moveDialogOpen}
        onOpenChange={setMoveDialogOpen}
        group={group}
        dbId={dbId}
        onMove={onMove}
        isPending={isMovePending}
      />
    </>
  );
}
