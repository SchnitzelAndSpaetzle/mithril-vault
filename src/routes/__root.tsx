import { createRootRoute, Outlet } from "@tanstack/react-router";
import App from "@/App.tsx";

export const Route = createRootRoute({
  component: () => (
    <App>
      <Outlet />
      {/*<TanStackRouterDevtools />*/}
    </App>
  ),
});
