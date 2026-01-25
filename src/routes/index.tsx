import { createFileRoute } from "@tanstack/react-router";
import {
  SidebarInset,
  SidebarProvider,
  SidebarTrigger,
} from "@/components/ui/sidebar.tsx";
import { AppSidebar } from "@/components/app-sidebar.tsx";
import { NavActions } from "@/components/nav-actions.tsx";
import { SearchForm } from "@/components/search-form.tsx";
import { Separator } from "@/components/ui/separator.tsx";
import DesktopContentArea from "@/views/DesktopContentArea.tsx";

export const Route = createFileRoute("/")({
  component: Index,
});

function Index() {
  return (
    <SidebarProvider>
      <AppSidebar />
      <SidebarInset className="flex flex-col h-screen overflow-hidden">
        <header className="flex h-14 shrink-0 items-center gap-2 border-b">
          <div className="flex flex-1 items-center gap-2 px-3">
            <SidebarTrigger />
          </div>
          <Separator
            orientation="vertical"
            className="data-[orientation=vertical]:h-6"
          />
          <SearchForm className="w-full px-3" />
          <Separator
            orientation="vertical"
            className="data-[orientation=vertical]:h-6"
          />
          <div className="ml-auto px-3">
            <NavActions />
          </div>
        </header>
        <div className="flex-1 overflow-hidden">
          <DesktopContentArea />
        </div>
      </SidebarInset>
    </SidebarProvider>
  );
}
