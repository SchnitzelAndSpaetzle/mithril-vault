// SPDX-License-Identifier: MIT

import { SidebarMenuSkeleton } from "@/components/ui/sidebar";

export function GroupTreeSkeleton() {
  return (
    <div className="flex flex-col gap-1">
      <SidebarMenuSkeleton showIcon />
      <SidebarMenuSkeleton showIcon />
      <SidebarMenuSkeleton showIcon />
    </div>
  );
}
