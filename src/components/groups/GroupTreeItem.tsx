// SPDX-License-Identifier: MIT

import { ChevronRight, Folder, FolderOpen } from "lucide-react";
import { useNavigate, useSearch } from "@tanstack/react-router";
import { toast } from "sonner";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible";
import {
  SidebarMenuAction,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarMenuSub,
} from "@/components/ui/sidebar";
import { GroupTreeItemActions } from "./GroupTreeItemActions";
import type { CustomIconMap, Group } from "@/lib/types";
import { useDatabaseTabs } from "@/stores/database-tabs";
import { useGroupMutations } from "@/hooks/use-group-mutations";
import {
  isFolderIcon,
  KEEPASS_ICON_MAP,
  parseGroupIconId,
} from "@/lib/keepass-icons";

function GroupIcon({
  iconStr,
  customIconUuid,
  customIcons,
  isExpanded,
}: {
  iconStr: string | null;
  customIconUuid: string | null;
  customIcons: CustomIconMap;
  isExpanded: boolean;
}) {
  const customIcon = customIconUuid ? customIcons[customIconUuid] : null;
  if (customIcon) {
    return (
      <img
        src={`data:image/png;base64,${customIcon}`}
        alt=""
        aria-hidden="true"
        className="h-4 w-4"
      />
    );
  }
  const iconId = parseGroupIconId(iconStr);
  if (isFolderIcon(iconId)) {
    const FolderIcon = isExpanded ? FolderOpen : Folder;
    return <FolderIcon className="h-4 w-4" />;
  }
  const ResolvedIcon = KEEPASS_ICON_MAP[iconId!] ?? Folder;
  return <ResolvedIcon className="h-4 w-4" />;
}

interface GroupTreeItemProps {
  group: Group;
  dbId: string;
  customIcons: CustomIconMap;
  depth?: number;
}

export function GroupTreeItem({
  group,
  dbId,
  customIcons,
  depth = 0,
}: GroupTreeItemProps) {
  const navigate = useNavigate();
  const search = useSearch({ strict: false });
  const { createGroup, updateGroup, deleteGroup, moveGroup } =
    useGroupMutations(dbId);

  const tab = useDatabaseTabs((state) =>
    state.tabs.find((t) => t.dbId === dbId || t.path === dbId)
  );
  const updateTabState = useDatabaseTabs((state) => state.updateTabState);

  const isSelected = search.groupId === group.id;
  const expandedIds = tab?.expandedGroupIds ?? [];
  const isExpanded = expandedIds.includes(group.id);
  const hasChildren = group.children.length > 0;
  const isRoot = !group.parentId;

  const handleSelect = () => {
    if (tab) {
      updateTabState(tab.id, { selectedGroupId: group.id });
    }
    void navigate({
      to: "/dashboard/index/$dbId",
      params: { dbId },
      search: { groupId: group.id },
    });
  };

  const handleToggleExpand = () => {
    if (!tab) return;

    const newExpandedIds = isExpanded
      ? expandedIds.filter((id) => id !== group.id)
      : [...expandedIds, group.id];

    updateTabState(tab.id, { expandedGroupIds: newExpandedIds });
  };

  const handleCreateSubgroup = (name: string, iconId: number) => {
    createGroup.mutate(
      { dbId, parentId: group.id, name },
      {
        onSuccess: (newGroup) => {
          if (iconId !== 48) {
            updateGroup.mutate(
              {
                dbId,
                id: newGroup.id,
                data: { icon: String(iconId) },
              },
              {
                onSuccess: () => {
                  toast.success(`Group "${name}" created`);
                },
                onError: (error) => {
                  toast.error(
                    `Group "${name}" created, but failed to set icon: ${error.message}`
                  );
                },
              }
            );
            return;
          }
          toast.success(`Group "${name}" created`);
        },
        onError: (error) => {
          toast.error(`Failed to create group: ${error.message}`);
        },
      }
    );
  };

  const handleUpdateGroup = (data: { name?: string; icon?: string }) => {
    updateGroup.mutate(
      { dbId, id: group.id, data },
      {
        onSuccess: () => {
          toast.success("Group updated");
        },
        onError: (error) => {
          toast.error(`Failed to update group: ${error.message}`);
        },
      }
    );
  };

  const handleDelete = () => {
    deleteGroup.mutate(
      { dbId, id: group.id },
      {
        onSuccess: () => {
          toast.success(`Group "${group.name}" moved to Recycle Bin`);
        },
        onError: (error) => {
          toast.error(`Failed to delete group: ${error.message}`);
        },
      }
    );
  };

  const handleMove = (targetParentId: string) => {
    moveGroup.mutate(
      { dbId, id: group.id, targetParentId },
      {
        onSuccess: () => {
          toast.success(`Group "${group.name}" moved`);
        },
        onError: (error) => {
          toast.error(`Failed to move group: ${error.message}`);
        },
      }
    );
  };

  const actionsProps = {
    group,
    dbId,
    isRoot,
    onCreateSubgroup: handleCreateSubgroup,
    onUpdateGroup: handleUpdateGroup,
    onDelete: handleDelete,
    onMove: handleMove,
    isCreatePending: createGroup.isPending,
    isUpdatePending: updateGroup.isPending,
    isDeletePending: deleteGroup.isPending,
    isMovePending: moveGroup.isPending,
  };

  if (hasChildren) {
    return (
      <Collapsible open={isExpanded} onOpenChange={handleToggleExpand} asChild>
        <SidebarMenuItem>
          <SidebarMenuButton
            onClick={handleSelect}
            isActive={isSelected}
            tooltip={group.name}
          >
            <GroupIcon
              iconStr={group.icon}
              customIconUuid={group.customIconUuid}
              customIcons={customIcons}
              isExpanded={isExpanded}
            />
            <span className="truncate">{group.name}</span>
          </SidebarMenuButton>
          <CollapsibleTrigger asChild>
            <SidebarMenuAction
              className="bg-sidebar-accent text-sidebar-accent-foreground left-2 data-[state=open]:rotate-90"
              showOnHover
              aria-label={isExpanded ? "Collapse" : "Expand"}
            >
              <ChevronRight className="h-3 w-3" />
            </SidebarMenuAction>
          </CollapsibleTrigger>
          <GroupTreeItemActions {...actionsProps} />
          <CollapsibleContent>
            <SidebarMenuSub>
              {group.children.map((child) => (
                <GroupTreeItem
                  key={child.id}
                  group={child}
                  dbId={dbId}
                  customIcons={customIcons}
                  depth={depth + 1}
                />
              ))}
            </SidebarMenuSub>
          </CollapsibleContent>
        </SidebarMenuItem>
      </Collapsible>
    );
  }

  return (
    <SidebarMenuItem>
      <SidebarMenuButton
        onClick={handleSelect}
        isActive={isSelected}
        tooltip={group.name}
      >
        <GroupIcon
          iconStr={group.icon}
          customIconUuid={group.customIconUuid}
          customIcons={customIcons}
          isExpanded={false}
        />
        <span className="truncate">{group.name}</span>
      </SidebarMenuButton>
      <GroupTreeItemActions {...actionsProps} />
    </SidebarMenuItem>
  );
}
