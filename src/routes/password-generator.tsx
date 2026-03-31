import { createFileRoute } from "@tanstack/react-router";
import { SiteSettingsHeader } from "@/components/layout/site-settings-header.tsx";
import { SidebarInset, SidebarProvider } from "@/components/ui/sidebar.tsx";
import { AppSettingsSidebar } from "@/components/layout/app-settings-sidebar.tsx";
import { PasswordGeneratorPage } from "@/components/generator/PasswordGeneratorPage.tsx";

export const Route = createFileRoute("/password-generator")({
  component: RouteComponent,
});

function RouteComponent() {
  return (
    <SidebarProvider>
      <AppSettingsSidebar />
      <SidebarInset>
        <SiteSettingsHeader />
        <PasswordGeneratorPage />
      </SidebarInset>
    </SidebarProvider>
  );
}
