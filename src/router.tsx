// src/router.tsx
import { QueryClient } from "@tanstack/react-query";
import { createRouter } from "@tanstack/react-router";
// import { setupRouterSsrQueryIntegration } from "@tanstack/react-router-ssr-query";
import { routeTree } from "./routeTree.gen";

export const queryClient = new QueryClient();

export const router = createRouter({
  routeTree,
  // optionally expose the QueryClient via router context
  context: { queryClient },
  scrollRestoration: true,
  defaultPreload: "intent",
});

// TODO: check fix on ssr tanstack and come back to resolve this
// setupRouterSsrQueryIntegration({
// router,
// queryClient,
// optional:
// handleRedirects: true,
// wrapQueryClient: true,
// });

export function getRouter() {
  return router;
}
