// SPDX-License-Identifier: MIT

import { useTranslation } from "react-i18next";
import { ChevronDown, Database, Loader2, Lock, Settings } from "lucide-react";
import { useNavigate } from "@tanstack/react-router";
import { open } from "@tauri-apps/plugin-dialog";

import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu.tsx";
import {
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
} from "@/components/ui/sidebar.tsx";
import { Button } from "@/components/ui/button.tsx";
import { useActiveDatabase } from "@/hooks/use-active-database";
import { useRecentDatabases } from "@/hooks/use-recent-databases.ts";
import { database } from "@/lib/tauri.ts";
import {
  type DatabaseTabsState,
  useDatabaseTabs,
} from "@/stores/database-tabs";

/**
 * Extracts the filename from a full file path.
 */
function getFilename(path: string | undefined, fallback: string): string {
  if (!path) return fallback;
  const parts = path.split(/[/\\]/);
  return parts[parts.length - 1] || path;
}

export function DatabaseSwitcher() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { tab, dbId } = useActiveDatabase();
  const removeTab = useDatabaseTabs(
    (state: DatabaseTabsState) => state.removeTab
  );
  const { recentDatabases, isLoading: isLoadingRecent } = useRecentDatabases();

  const handleLock = async () => {
    if (!tab?.id || !dbId) {
      return;
    }

    try {
      await database.close(dbId);
      removeTab(tab.id);
      void navigate({ to: "/" });
    } catch (error) {
      console.error("Failed to close database:", error);
    }
  };

  const handleSelectDatabase = (path: string) => {
    void navigate({ to: "/unlock", search: { path } });
  };

  const handleOpenAnotherDatabase = async () => {
    try {
      const file = await open({
        title: t("databaseSwitcher.openDatabase"),
        filters: [{ name: "KeePass Database", extensions: ["kdbx"] }],
      });

      if (!file) {
        return;
      }
      void navigate({ to: "/unlock", search: { path: file as string } });
    } catch {
      // User cancelled or error - ignore
    }
  };

  // Filter out the currently open database from a recent list
  const otherDatabases = recentDatabases.filter((db) => db.path !== tab?.path);

  // If no database is open, don't render
  if (!tab) {
    return null;
  }

  return (
    <SidebarMenu>
      <SidebarMenuItem className="flex items-center gap-2">
        <div className="flex grow max-w-40">
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <SidebarMenuButton className="w-fit px-1.5">
                <div className="bg-sidebar-primary text-sidebar-primary-foreground flex aspect-square size-5 items-center justify-center rounded-md">
                  <Database className="size-3" />
                </div>
                <span className="truncate font-medium">
                  {tab.info?.name ||
                    getFilename(tab.path, t("databaseSwitcher.newDatabase"))}
                </span>
                <ChevronDown className="opacity-50" />
              </SidebarMenuButton>
            </DropdownMenuTrigger>
            <DropdownMenuContent
              className="w-64 rounded-lg"
              align="start"
              side="bottom"
              sideOffset={4}
            >
              <DropdownMenuLabel className="text-muted-foreground text-xs">
                {t("databaseSwitcher.recentDatabases")}
              </DropdownMenuLabel>
              {isLoadingRecent ? (
                <div className="flex items-center justify-center p-2">
                  <Loader2 className="size-4 animate-spin" />
                </div>
              ) : otherDatabases.length === 0 ? (
                <DropdownMenuItem disabled className="text-muted-foreground">
                  {t("databaseSwitcher.noOtherDatabases")}
                </DropdownMenuItem>
              ) : (
                otherDatabases.map((db) => (
                  <DropdownMenuItem
                    key={db.path}
                    onClick={() => handleSelectDatabase(db.path)}
                    className="gap-2 p-2"
                  >
                    <div className="flex size-6 items-center justify-center rounded-xs border">
                      <Database className="size-4 shrink-0" />
                    </div>
                    <span className="truncate">
                      {getFilename(db.path, t("databaseSwitcher.newDatabase"))}
                    </span>
                  </DropdownMenuItem>
                ))
              )}
              <DropdownMenuSeparator />
              <DropdownMenuItem
                onClick={() => void handleOpenAnotherDatabase()}
                className="gap-2 p-2"
              >
                <div className="bg-background flex size-6 items-center justify-center rounded-md border">
                  <Database className="size-4" />
                </div>
                <div className="text-muted-foreground font-medium">
                  {t("databaseSwitcher.openAnother")}
                </div>
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </div>

        <div className="flex items-center gap-2" data-collapsible="icon">
          <Button
            size="icon"
            className="size-8 group-data-[collapsible=icon]:opacity-0"
            variant="ghost"
            onClick={() => void navigate({ to: "/settings" })}
          >
            <Settings />
            <span className="sr-only">{t("databaseSwitcher.settings")}</span>
          </Button>
          <Button
            size="icon"
            className="size-8 group-data-[collapsible=icon]:opacity-0"
            variant="ghost"
            onClick={() => void handleLock()}
          >
            <Lock />
            <span className="sr-only">
              {t("databaseSwitcher.lockDatabase")}
            </span>
          </Button>
        </div>
      </SidebarMenuItem>
    </SidebarMenu>
  );
}
