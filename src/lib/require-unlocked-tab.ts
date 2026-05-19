import { redirect } from "@tanstack/react-router";

import { useDatabaseTabs } from "@/stores/database-tabs";

/**
 * Shared `beforeLoad` guard for routes parameterised by `$dbId`.
 *
 * Looks up the tab in the active-tab store, redirects to `/` when the
 * id is unknown, redirects to `/unlock` when the tab is `unlocking` or
 * `locked`, and otherwise marks the tab active and returns its id so
 * the route can stash it in the route context.
 *
 * Throws via `redirect(...)` rather than returning; matches how
 * TanStack Router expects `beforeLoad` to bail out.
 */
export function requireUnlockedTab(dbId: string): { tabId: string } {
  const state = useDatabaseTabs.getState();
  const tab = state.tabs.find(
    (item) => item.dbId === dbId || item.path === dbId
  );

  if (!tab) {
    throw redirect({ to: "/" });
  }

  if (tab.state === "unlocking" || tab.state === "locked") {
    throw redirect({
      to: "/unlock",
      search: tab.path ? { path: tab.path } : {},
    });
  }

  state.setActiveTab(tab.id);
  return { tabId: tab.id };
}
