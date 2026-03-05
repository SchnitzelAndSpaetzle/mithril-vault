import { createFileRoute } from "@tanstack/react-router";
import { SidebarInset, SidebarProvider } from "@/components/ui/sidebar.tsx";
import { AppSettingsSidebar } from "@/components/layout/app-settings-sidebar.tsx";
import { SiteSettingsHeader } from "@/components/layout/site-settings-header.tsx";
import { SettingsView } from "@/views/SettingsView";

export const Route = createFileRoute("/settings/")({
  component: RouteComponent,
});

function RouteComponent() {
  return (
    <SidebarProvider className="h-full min-h-0">
      <AppSettingsSidebar />
      <SidebarInset className="flex min-h-0 flex-1 flex-col overflow-hidden">
        <SiteSettingsHeader />
        <div className="min-h-0 flex-1 overflow-y-auto">
          <SettingsView />
        </div>
      </SidebarInset>
    </SidebarProvider>
  );
}
