import type { ComponentProps } from "react";
import { useTranslation } from "react-i18next";
import {
  AlarmClockMinus,
  MessageCircleQuestion,
  Search,
  Settings2,
  ShieldIcon,
} from "lucide-react";
import { useNavigate } from "@tanstack/react-router";
import { NavMain } from "@/components/layout/nav-main.tsx";
import { NavSecondary } from "@/components/nav-secondary.tsx";
import { DatabaseSwitcher } from "@/components/layout/database-switcher.tsx";
import {
  Sidebar,
  SidebarContent,
  SidebarGroup,
  SidebarGroupContent,
  SidebarGroupLabel,
  SidebarHeader,
  SidebarRail,
} from "@/components/ui/sidebar.tsx";
import { Separator } from "@/components/ui/separator.tsx";
import { GroupTree } from "@/components/groups/GroupTree";
import { useActiveDatabase } from "@/hooks/use-active-database";
import { usePasswordHealthSummary } from "@/hooks/use-password-health";
import NavTags from "@/components/layout/nav-tags.tsx";

export function AppSidebar({ ...props }: ComponentProps<typeof Sidebar>) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { dbId } = useActiveDatabase();
  const { totalUnhealthy, highestSeverity } = usePasswordHealthSummary(
    dbId ?? null
  );

  if (!dbId) {
    return null;
  }

  const securityBadge =
    totalUnhealthy > 0 ? (
      <span
        className={
          highestSeverity === "critical"
            ? "text-red-600 dark:text-red-500"
            : "text-amber-600 dark:text-amber-500"
        }
      >
        {totalUnhealthy}
      </span>
    ) : undefined;

  const navMain = [
    {
      title: t("sidebar.allEntries"),
      icon: Search,
      onSelect: () =>
        void navigate({
          to: "/dashboard/index/$dbId",
          params: { dbId },
        }),
    },
    {
      title: t("sidebar.expired"),
      icon: AlarmClockMinus,
      disabled: true,
    },
    {
      title: t("sidebar.security"),
      icon: ShieldIcon,
      badge: securityBadge,
      onSelect: () =>
        void navigate({
          to: "/dashboard/security/$dbId",
          params: { dbId },
        }),
    },
  ];

  const navSecondary = [
    {
      title: t("sidebar.settings"),
      icon: Settings2,
      onSelect: () =>
        void navigate({
          to: "/settings",
        }),
    },
    {
      title: t("sidebar.help"),
      icon: MessageCircleQuestion,
      disabled: true,
    },
  ];

  return (
    <Sidebar className="border-r-0" {...props}>
      <SidebarHeader className="flex h-14 shrink-0 items-center gap-2 border-b pt-3">
        <DatabaseSwitcher />
      </SidebarHeader>
      <SidebarContent>
        <NavMain items={navMain} />
        <Separator />
        <NavTags dbId={dbId} />
        <SidebarGroup>
          <SidebarGroupLabel>{t("sidebar.groups")}</SidebarGroupLabel>
          <SidebarGroupContent>
            <GroupTree dbId={dbId} />
          </SidebarGroupContent>
        </SidebarGroup>
        <NavSecondary items={navSecondary} className="border-t mt-auto" />
      </SidebarContent>
      <SidebarRail />
    </Sidebar>
  );
}
