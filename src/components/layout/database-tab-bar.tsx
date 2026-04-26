// SPDX-License-Identifier: MIT

import { useNavigate, useRouterState } from "@tanstack/react-router";
import { open } from "@tauri-apps/plugin-dialog";
import { Plus, X } from "lucide-react";

import { Button } from "@/components/ui/button";
import { useAppPreferences } from "@/hooks/use-app-preferences";
import { clipboard, database } from "@/lib/tauri";
import { cn } from "@/lib/utils";
import {
  type DatabaseTab,
  type DatabaseTabsState,
  useDatabaseTabs,
} from "@/stores/database-tabs";

function getFilename(path?: string): string {
  if (!path) return "New Database";
  const parts = path.split(/[/\\]/);
  return parts[parts.length - 1] || path;
}

export function DatabaseTabBar() {
  const navigate = useNavigate();
  const { preferences } = useAppPreferences();
  const tabs = useDatabaseTabs(
    (state: DatabaseTabsState) => state.tabs
  ) as DatabaseTab[];
  const activeTabId = useDatabaseTabs(
    (state: DatabaseTabsState) => state.activeTabId
  );
  const activeDbId = useRouterState({
    select: (state) => {
      for (let i = state.matches.length - 1; i >= 0; i -= 1) {
        const params = state.matches[i]?.params as
          | Record<string, string>
          | undefined;
        if (params && typeof params["dbId"] === "string") {
          return params["dbId"];
        }
      }

      return null;
    },
  });
  const addTab = useDatabaseTabs((state: DatabaseTabsState) => state.addTab);
  const removeTab = useDatabaseTabs(
    (state: DatabaseTabsState) => state.removeTab
  );
  const setActiveTab = useDatabaseTabs(
    (state: DatabaseTabsState) => state.setActiveTab
  );

  const handleAddTab = async () => {
    try {
      const file = await open({
        title: "Open Database",
        filters: [{ name: "KeePass Database", extensions: ["kdbx"] }],
      });

      if (!file) {
        return;
      }

      const id = addTab(file as string);
      setActiveTab(id);
      await navigate({ to: "/unlock", search: { path: file as string } });
    } catch {
      // User cancelled or error - ignore
    }
  };

  const handleCloseTab = async (tabId: string) => {
    const tab = tabs.find((item: DatabaseTab) => item.id === tabId);

    if (tab?.state === "open" && tab.path) {
      try {
        if (preferences?.security.clearClipboardOnLock) {
          try {
            await clipboard.clear();
          } catch (error) {
            console.error("Failed to clear clipboard before lock:", error);
          }
        }
        await database.close(tab.path);
      } catch (error) {
        console.error("Failed to close database:", error);
      }
    }

    removeTab(tabId);

    const nextState = useDatabaseTabs.getState() as DatabaseTabsState;
    const nextOpenTab = nextState.tabs.find(
      (item) => item.state === "open" && (item.dbId ?? item.path)
    );

    if (nextOpenTab) {
      const nextDbId = nextOpenTab.dbId ?? nextOpenTab.path;
      void navigate({
        to: "/dashboard/index/$dbId",
        params: { dbId: nextDbId as string },
      });
      return;
    }

    const nextUnlockingTab = nextState.tabs.find(
      (item) => item.state === "unlocking"
    );

    if (!nextUnlockingTab) {
      void navigate({ to: "/" });
      return;
    }

    void navigate({
      to: "/unlock",
      search: nextUnlockingTab.path ? { path: nextUnlockingTab.path } : {},
    });
  };

  if (tabs.length < 2) {
    return null;
  }

  return (
    <div className="border-b bg-background/80 backdrop-blur h-12">
      <div className="flex h-full items-center gap-2 overflow-x-auto px-3">
        {tabs.map((tab: DatabaseTab) => {
          const isActive =
            (activeDbId && tab.dbId === activeDbId) ||
            (!activeDbId && tab.id === activeTabId);
          const tabLabel = tab.info?.name || getFilename(tab.path);
          const tabStatus = tab.state === "unlocking" ? "Unlock" : "Open";
          const tabClasses = cn(
            "group flex items-center gap-2 rounded-full border px-3 py-1 text-sm transition",
            isActive
              ? "border-foreground/20 bg-muted text-foreground"
              : "border-transparent text-muted-foreground hover:border-border hover:text-foreground"
          );

          const tabDbId = tab.dbId ?? tab.path;

          if (tab.state === "open" && tabDbId) {
            return (
              <button
                key={tab.id}
                type="button"
                className={tabClasses}
                onClick={() => {
                  setActiveTab(tab.id);
                  void navigate({
                    to: "/dashboard/index/$dbId",
                    params: { dbId: tabDbId as string },
                  });
                }}
              >
                <span className="max-w-36 truncate">{tabLabel}</span>
                <span className="text-xs text-muted-foreground">
                  {tabStatus}
                </span>
                <span className="sr-only">Close tab</span>
                <span
                  role="button"
                  tabIndex={0}
                  className="rounded-full p-1 text-muted-foreground transition hover:text-foreground"
                  onClick={(event) => {
                    event.preventDefault();
                    event.stopPropagation();
                    void handleCloseTab(tab.id);
                  }}
                  onKeyDown={(event) => {
                    if (event.key === "Enter" || event.key === " ") {
                      event.preventDefault();
                      event.stopPropagation();
                      void handleCloseTab(tab.id);
                    }
                  }}
                >
                  <X className="size-3" />
                </span>
              </button>
            );
          }

          return (
            <button
              key={tab.id}
              type="button"
              onClick={() => {
                setActiveTab(tab.id);
                void navigate({
                  to: "/unlock",
                  search: tab.path ? { path: tab.path } : {},
                });
              }}
              className={tabClasses}
            >
              <span className="max-w-36 truncate">{tabLabel}</span>
              <span className="text-xs text-muted-foreground">{tabStatus}</span>
              <span className="sr-only">Close tab</span>
              <span
                role="button"
                tabIndex={0}
                className="rounded-full p-1 text-muted-foreground transition hover:text-foreground"
                onClick={(event) => {
                  event.preventDefault();
                  event.stopPropagation();
                  void handleCloseTab(tab.id);
                }}
                onKeyDown={(event) => {
                  if (event.key === "Enter" || event.key === " ") {
                    event.preventDefault();
                    event.stopPropagation();
                    void handleCloseTab(tab.id);
                  }
                }}
              >
                <X className="size-3" />
              </span>
            </button>
          );
        })}

        <Button
          type="button"
          variant="ghost"
          size="icon"
          className="shrink-0"
          onClick={() => void handleAddTab()}
        >
          <Plus className="size-4" />
          <span className="sr-only">Open another database</span>
        </Button>
      </div>
    </div>
  );
}
