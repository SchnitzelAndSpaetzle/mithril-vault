import type { ReactNode } from "react";
import { SidebarInset, SidebarProvider } from "@/components/ui/sidebar.tsx";
import { AppSidebar } from "@/components/layout/app-sidebar.tsx";

interface LayoutDbSidebar {
  children?: ReactNode;
}

export default function LayoutDbSidebar({ children }: LayoutDbSidebar) {
  return (
    <SidebarProvider>
      <AppSidebar />
      <SidebarInset className="flex flex-col h-screen overflow-hidden">
        {children}
      </SidebarInset>
    </SidebarProvider>
  );
}
