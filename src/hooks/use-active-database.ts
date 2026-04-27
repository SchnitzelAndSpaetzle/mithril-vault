// SPDX-License-Identifier: MIT

import { useRouterState } from "@tanstack/react-router";
import {
  type DatabaseTabsState,
  useDatabaseTabs,
} from "@/stores/database-tabs";

export function useActiveDatabase() {
  const routeDbId = useRouterState({
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

  const tab = useDatabaseTabs((state: DatabaseTabsState) => {
    if (routeDbId) {
      return (
        state.tabs.find(
          (item) => item.dbId === routeDbId || item.path === routeDbId
        ) ?? null
      );
    }

    return state.tabs.find((item) => item.id === state.activeTabId) ?? null;
  });

  return {
    tab,
    dbId: routeDbId ?? tab?.path ?? null,
    isUnlocking: tab?.state === "unlocking",
    isLocked: tab?.state === "locked",
  };
}
