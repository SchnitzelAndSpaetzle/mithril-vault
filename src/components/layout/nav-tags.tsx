import {
  SidebarGroup,
  SidebarMenu,
  SidebarMenuBadge,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarMenuSub,
} from "@/components/ui/sidebar.tsx";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible.tsx";
import { Tags } from "lucide-react";
import { NavTagItem } from "@/components/layout/nav-tag-item.tsx";
import { NavTagsRenameDialog } from "@/components/layout/nav-tags-rename-dialog.tsx";
import { NavTagsDeleteDialog } from "@/components/layout/nav-tags-delete-dialog.tsx";
import { useNavTagsController } from "@/hooks/use-nav-tags-controller";

interface NavTagsProps {
  dbId: string;
}

export default function NavTags({ dbId }: NavTagsProps) {
  const {
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
    isRenamePending,
    isDeletePending,
  } = useNavTagsController(dbId);

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
                    <NavTagItem
                      key={tag}
                      tag={tag}
                      isActive={activeTag === tag}
                      onSelect={handleTagClick}
                      onRename={openRenameDialog}
                      onDelete={openDeleteDialog}
                    />
                  ))}
                </SidebarMenuSub>
              </CollapsibleContent>
            </SidebarMenuItem>
          </Collapsible>
        </SidebarMenu>
      </SidebarGroup>

      <NavTagsRenameDialog
        key={`${targetTag}-${renameDialogOpen ? "open" : "closed"}`}
        open={renameDialogOpen}
        onOpenChange={setRenameDialogOpen}
        targetTag={targetTag}
        onConfirm={handleRename}
        isPending={isRenamePending}
      />

      <NavTagsDeleteDialog
        open={deleteDialogOpen}
        onOpenChange={setDeleteDialogOpen}
        targetTag={targetTag}
        onConfirm={handleDelete}
        isPending={isDeletePending}
      />
    </>
  );
}
