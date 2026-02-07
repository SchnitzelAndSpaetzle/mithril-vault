import { type LucideIcon } from "lucide-react";

import {
  SidebarGroup,
  SidebarMenu,
  SidebarMenuBadge,
  SidebarMenuButton,
  SidebarMenuItem,
} from "@/components/ui/sidebar.tsx";
import React from "react";

export function NavMain({
  items,
}: {
  items: {
    title: string;
    icon: LucideIcon;
    isActive?: boolean;
    badge?: React.ReactNode;
    onSelect?: () => void;
    disabled?: boolean;
  }[];
}) {
  return (
    <SidebarGroup className="group-data-[collapsible=icon]:hidden">
      <SidebarMenu>
        {items.map((item) => (
          <SidebarMenuItem key={item.title}>
            <SidebarMenuButton
              isActive={item.isActive ?? false}
              onClick={item.onSelect}
              disabled={item.disabled}
              aria-disabled={item.disabled}
            >
              <item.icon />
              <span>{item.title}</span>
            </SidebarMenuButton>
            {item.badge && <SidebarMenuBadge>{item.badge}</SidebarMenuBadge>}
          </SidebarMenuItem>
        ))}
      </SidebarMenu>
    </SidebarGroup>
  );
}
