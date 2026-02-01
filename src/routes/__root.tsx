import { createRootRoute, Outlet } from "@tanstack/react-router";
import type { CSSProperties } from "react";
import App from "@/App.tsx";
import { DatabaseTabBar } from "@/components/layout/database-tab-bar";
import { useDatabaseTabs } from "@/stores/database-tabs";

export const Route = createRootRoute({
  component: RootRouteComponent,
});

function RootRouteComponent() {
  const hasMultipleTabs = useDatabaseTabs((state) => state.tabs.length > 1);
  const style = {
    "--app-top-offset": hasMultipleTabs ? "48px" : "0px",
  } as CSSProperties;

  return (
    <App>
      <div className="flex h-svh flex-col overflow-hidden" style={style}>
        <DatabaseTabBar />
        <div className="min-h-0 flex-1">
          <Outlet />
        </div>
      </div>
      {/*<TanStackRouterDevtools />*/}
    </App>
  );
}
