import { createRootRoute, Outlet } from "@tanstack/react-router";
import App from "@/App.tsx";
import { DatabaseTabBar } from "@/components/layout/database-tab-bar";
import { TanStackRouterDevtools } from "@tanstack/react-router-devtools";

export const Route = createRootRoute({
  component: () => (
    <App>
      <DatabaseTabBar />
      <Outlet />
      <TanStackRouterDevtools />
    </App>
  ),
});
