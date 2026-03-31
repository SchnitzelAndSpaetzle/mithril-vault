import * as React from "react";
import { useTranslation } from "react-i18next";
import { Link, useLocation } from "@tanstack/react-router";
import {
  Cog,
  Database,
  Globe,
  Keyboard,
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

const sectionDefs = [
  {
    id: "general",
    titleKey: "settings.general.title",
    icon: SlidersHorizontal,
  },
  { id: "security", titleKey: "settings.security.title", icon: Shield },
  { id: "appearance", titleKey: "settings.appearance.title", icon: Palette },
  { id: "browser", titleKey: "settings.browser.title", icon: Globe },
  { id: "shortcuts", titleKey: "settings.shortcuts.title", icon: Keyboard },
  { id: "advanced", titleKey: "settings.advanced.title", icon: WandSparkles },
  { id: "database", titleKey: "settings.database.title", icon: Database },
] as const;

export function AppSettingsSidebar({
  ...props
}: React.ComponentProps<typeof Sidebar>) {
  const { t } = useTranslation();
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
                  <span className="font-medium">{t("settings.title")}</span>
                  <span className="text-xs text-muted-foreground">
                    {t("settings.sidebar.preferences")}
                  </span>
                </div>
              </Link>
            </SidebarMenuButton>
          </SidebarMenuItem>
        </SidebarMenu>
      </SidebarHeader>

      <SidebarContent>
        <SidebarGroup>
          <SidebarGroupLabel>
            {t("settings.sidebar.sections")}
          </SidebarGroupLabel>
          <SidebarMenu>
            {sectionDefs.map((section) => (
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
                    <span>{t(section.titleKey)}</span>
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
