// SPDX-License-Identifier: MIT

import { useEffect } from "react";
import { ChevronDown, Database, Loader2, Lock, Settings } from "lucide-react";
import { useNavigate } from "@tanstack/react-router";

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
import { Skeleton } from "@/components/ui/skeleton.tsx";
import { useDatabaseInfo } from "@/hooks/use-database-info.ts";
import { useRecentDatabases } from "@/hooks/use-recent-databases.ts";
import { database } from "@/lib/tauri.ts";

/**
 * Extracts the filename from a full file path.
 */
function getFilename(path: string): string {
  const parts = path.split(/[/\\]/);
  return parts[parts.length - 1] || path;
}

export function DatabaseSwitcher() {
  const navigate = useNavigate();
  const { databaseInfo, isLoading: isLoadingDb } = useDatabaseInfo();
  const { recentDatabases, isLoading: isLoadingRecent } = useRecentDatabases();

  // Redirect to home if no database is open (after loading completes)
  useEffect(() => {
    if (!isLoadingDb && !databaseInfo) {
      void navigate({ to: "/" });
    }
  }, [isLoadingDb, databaseInfo, navigate]);

  const handleLock = async () => {
    try {
      await database.close();
      void navigate({ to: "/" });
    } catch (error) {
      console.error("Failed to close database:", error);
    }
  };

  const handleSelectDatabase = (path: string) => {
    void navigate({ to: "/unlock", search: { path } });
  };

  // Filter out the currently open database from a recent list
  const otherDatabases = recentDatabases.filter(
    (db) => db.path !== databaseInfo?.path
  );

  // Show loading state
  if (isLoadingDb) {
    return (
      <SidebarMenu>
        <SidebarMenuItem className="flex items-center gap-2">
          <div className="flex grow items-center gap-2 px-1.5">
            <Skeleton className="size-5 rounded-md" />
            <Skeleton className="h-4 w-24" />
          </div>
        </SidebarMenuItem>
      </SidebarMenu>
    );
  }

  // If no database is open, don't render (redirect will happen)
  if (!databaseInfo) {
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
                  {databaseInfo.name || getFilename(databaseInfo.path)}
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
                Recent Databases
              </DropdownMenuLabel>
              {isLoadingRecent ? (
                <div className="flex items-center justify-center p-2">
                  <Loader2 className="size-4 animate-spin" />
                </div>
              ) : otherDatabases.length === 0 ? (
                <DropdownMenuItem disabled className="text-muted-foreground">
                  No other databases
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
                    <span className="truncate">{getFilename(db.path)}</span>
                  </DropdownMenuItem>
                ))
              )}
              <DropdownMenuSeparator />
              <DropdownMenuItem
                onClick={() => void navigate({ to: "/" })}
                className="gap-2 p-2"
              >
                <div className="bg-background flex size-6 items-center justify-center rounded-md border">
                  <Database className="size-4" />
                </div>
                <div className="text-muted-foreground font-medium">
                  Open another database
                </div>
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </div>

        <div className="flex items-center gap-2" data-collapsible="icon">
          {/*TODO: database settings page*/}
          <Button
            size="icon"
            className="size-8 group-data-[collapsible=icon]:opacity-0"
            variant="ghost"
          >
            <Settings />
            <span className="sr-only">Settings</span>
          </Button>
          <Button
            size="icon"
            className="size-8 group-data-[collapsible=icon]:opacity-0"
            variant="ghost"
            onClick={() => void handleLock()}
          >
            <Lock />
            <span className="sr-only">Lock Database</span>
          </Button>
        </div>
      </SidebarMenuItem>
    </SidebarMenu>
  );
}
