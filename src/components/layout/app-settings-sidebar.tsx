import * as React from "react";
import { Link, useLocation } from "@tanstack/react-router";
import {
  Cog,
  Database,
  Globe,
  Palette,
  Shield,
  SlidersHorizontal,
  WandSparkles,
} from "lucide-react";
import {
  Sidebar,
  SidebarContent,
  SidebarGroup,
  SidebarGroupLabel,
  SidebarHeader,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarRail,
} from "@/components/ui/sidebar";

const sections = [
  { id: "general", title: "General", icon: SlidersHorizontal },
  { id: "security", title: "Security", icon: Shield },
  { id: "appearance", title: "Appearance", icon: Palette },
  { id: "browser", title: "Browser", icon: Globe },
  { id: "advanced", title: "Advanced", icon: WandSparkles },
  { id: "database", title: "Database", icon: Database },
] as const;

export function AppSettingsSidebar({
  ...props
}: React.ComponentProps<typeof Sidebar>) {
  const location = useLocation();

  return (
    <Sidebar {...props}>
      <SidebarHeader>
        <SidebarMenu>
          <SidebarMenuItem>
            <SidebarMenuButton size="lg" asChild>
              <Link to="/settings">
                <div className="bg-sidebar-primary text-sidebar-primary-foreground flex aspect-square size-8 items-center justify-center rounded-lg">
                  <Cog className="size-4" />
                </div>
                <div className="flex flex-col gap-0.5 leading-none">
                  <span className="font-medium">Settings</span>
                  <span className="text-xs text-muted-foreground">
                    Preferences
                  </span>
                </div>
              </Link>
            </SidebarMenuButton>
          </SidebarMenuItem>
        </SidebarMenu>
      </SidebarHeader>

      <SidebarContent>
        <SidebarGroup>
          <SidebarGroupLabel>Sections</SidebarGroupLabel>
          <SidebarMenu>
            {sections.map((section) => (
              <SidebarMenuItem key={section.id}>
                <SidebarMenuButton asChild>
                  <a
                    href={`#${section.id}`}
                    className="font-medium"
                    aria-current={
                      location.pathname === "/settings" &&
                      location.hash === `#${section.id}`
                        ? "page"
                        : undefined
                    }
                  >
                    <section.icon className="size-4" />
                    <span>{section.title}</span>
                  </a>
                </SidebarMenuButton>
              </SidebarMenuItem>
            ))}
          </SidebarMenu>
        </SidebarGroup>
      </SidebarContent>
      <SidebarRail />
    </Sidebar>
  );
}
