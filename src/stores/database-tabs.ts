// SPDX-License-Identifier: MIT

import { create } from "zustand";
import type { DatabaseInfo } from "@/lib/types";

export type DatabaseTabState = "unlocking" | "open" | "locked";

export interface DatabaseTab {
  id: string;
  path?: string;
  dbId?: string;
  info: DatabaseInfo | null;
  state: DatabaseTabState;
  selectedGroupId: string | null;
  selectedEntryId: string | null;
  expandedGroupIds: string[];
}

export interface DatabaseTabsState {
  tabs: DatabaseTab[];
  activeTabId: string | null;
  addTab: (path: string) => string;
  removeTab: (id: string) => void;
  setActiveTab: (id: string) => void;
  updateTabInfo: (id: string, info: DatabaseInfo) => void;
  updateTabState: (id: string, updates: Partial<DatabaseTab>) => void;
  lockTab: (id: string) => void;
}

function createTabId(): string {
  return crypto.randomUUID();
}

function normalizePath(path: string): string {
  return path.trim();
}

type SetState = (
  partial:
    | DatabaseTabsState
    | Partial<DatabaseTabsState>
    | ((
        state: DatabaseTabsState
      ) => DatabaseTabsState | Partial<DatabaseTabsState>)
) => void;

type GetState = () => DatabaseTabsState;

export const useDatabaseTabs = create<DatabaseTabsState>(
  (set: SetState, get: GetState) => ({
    tabs: [],
    activeTabId: null,
    addTab: (path: string) => {
      const normalizedPath = normalizePath(path);
      const existing = get().tabs.find(
        (tab) =>
          tab.path === normalizedPath || tab.info?.path === normalizedPath
      );

      if (existing) {
        set({ activeTabId: existing.id });
        return existing.id;
      }

      const id = createTabId();
      const newTab: DatabaseTab = {
        id,
        path: normalizedPath,
        info: null,
        state: "unlocking",
        selectedGroupId: null,
        selectedEntryId: null,
        expandedGroupIds: [],
      };

      set((state: DatabaseTabsState) => ({
        tabs: [...state.tabs, newTab],
        activeTabId: id,
      }));

      return id;
    },
    removeTab: (id: string) =>
      set((state: DatabaseTabsState) => {
        const tabs = state.tabs.filter((tab) => tab.id !== id);
        let activeTabId = state.activeTabId;

        if (state.activeTabId === id) {
          activeTabId = tabs[tabs.length - 1]?.id ?? null;
        }

        return { tabs, activeTabId };
      }),
    setActiveTab: (id: string) =>
      set((state: DatabaseTabsState) =>
        state.tabs.some((tab) => tab.id === id) ? { activeTabId: id } : state
      ),
    updateTabInfo: (id: string, info: DatabaseInfo) =>
      set((state: DatabaseTabsState) => ({
        tabs: state.tabs.map((tab) =>
          tab.id === id
            ? {
                ...tab,
                info,
                path: info.path,
                dbId: info.path,
                state: info.isLocked ? "locked" : "open",
              }
            : tab
        ),
      })),
    updateTabState: (id: string, updates: Partial<DatabaseTab>) =>
      set((state: DatabaseTabsState) => ({
        tabs: state.tabs.map((tab) =>
          tab.id === id
            ? {
                ...tab,
                ...updates,
              }
            : tab
        ),
      })),
    lockTab: (id: string) =>
      set((state: DatabaseTabsState) => ({
        tabs: state.tabs.map((tab) =>
          tab.id === id
            ? {
                ...tab,
                state: "locked" as const,
                selectedEntryId: null,
                selectedGroupId: null,
              }
            : tab
        ),
      })),
  })
);
