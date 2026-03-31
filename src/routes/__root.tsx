import { createRootRoute, Outlet } from "@tanstack/react-router";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { type CSSProperties, useEffect } from "react";
import App from "@/App.tsx";
import { DatabaseTabBar } from "@/components/layout/database-tab-bar";
import { KeyboardShortcutsDialog } from "@/components/keyboard-shortcuts-dialog";
import { useActiveDatabase } from "@/hooks/use-active-database";
import { useDatabaseTabs } from "@/stores/database-tabs";

export const Route = createRootRoute({
  component: RootRouteComponent,
});

function RootRouteComponent() {
  const hasMultipleTabs = useDatabaseTabs((state) => state.tabs.length > 1);
  const { tab, isUnlocking } = useActiveDatabase();
  const style = {
    "--app-top-offset": hasMultipleTabs ? "48px" : "0px",
  } as CSSProperties;

  useEffect(() => {
    const title = formatWindowTitle(tab, isUnlocking);
    void getCurrentWindow().setTitle(title);
  }, [tab, isUnlocking]);

  return (
    <App>
      <div className="flex h-svh flex-col overflow-hidden" style={style}>
        <DatabaseTabBar />
        <div className="min-h-0 flex-1">
          <Outlet />
        </div>
      </div>
      <KeyboardShortcutsDialog />
      {/*<TanStackRouterDevtools />*/}
    </App>
  );
}

function formatWindowTitle(
  tab: ReturnType<typeof useActiveDatabase>["tab"],
  isUnlocking: boolean
): string {
  const appTitle = "MithrilVault";

  if (!tab) {
    return appTitle;
  }

  const dbLabel = tab.info?.name ?? getFilename(tab.path);
  if (!dbLabel) {
    return appTitle;
  }

  const isLocked =
    isUnlocking || tab.state === "unlocking" || tab.info?.isLocked;
  const status = isLocked ? "locked" : "unlocked";

  return `${dbLabel} [${status}] - ${appTitle}`;
}

function getFilename(path?: string): string | null {
  if (!path) {
    return null;
  }

  const parts = path.split(/[/\\]/);
  return parts[parts.length - 1] || path;
}
