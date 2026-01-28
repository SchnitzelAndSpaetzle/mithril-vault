import {
  SidebarGroup,
  SidebarMenu,
  SidebarMenuAction,
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
import { Plus, Tags } from "lucide-react";

interface NavTagsProps {
  tags: string[];
}

export default function NavTags({ tags }: NavTagsProps) {
  return (
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
                <SidebarMenuBadge>{tags.length}</SidebarMenuBadge>
              </div>
            </CollapsibleTrigger>
            <CollapsibleContent>
              <SidebarMenuSub>
                {tags.map((tag) => (
                  <SidebarMenuSubItem key={tag}>
                    <SidebarMenuButton asChild>
                      <a href="#">{tag}</a>
                    </SidebarMenuButton>
                    <SidebarMenuAction showOnHover>
                      <Plus />
                    </SidebarMenuAction>
                  </SidebarMenuSubItem>
                ))}
              </SidebarMenuSub>
            </CollapsibleContent>
          </SidebarMenuItem>
        </Collapsible>
      </SidebarMenu>
    </SidebarGroup>
  );
}
