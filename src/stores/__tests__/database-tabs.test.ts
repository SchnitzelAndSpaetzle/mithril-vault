// SPDX-License-Identifier: MIT

import { afterEach, describe, expect, it } from "vitest";
import { act } from "@testing-library/react";
import { useDatabaseTabs } from "../database-tabs";
import type { DatabaseInfo } from "@/lib/types";

function makeDatabaseInfo(overrides?: Partial<DatabaseInfo>): DatabaseInfo {
  return {
    name: "Test DB",
    path: "/test/path.kdbx",
    isModified: false,
    isLocked: false,
    rootGroupId: "root-uuid",
    version: "KDBX 4.0",
    ...overrides,
  };
}

describe("useDatabaseTabs", () => {
  afterEach(() => {
    // Reset store state between tests
    act(() => {
      const state = useDatabaseTabs.getState();
      for (const tab of state.tabs) {
        state.removeTab(tab.id);
      }
    });
  });

  describe("lockTab", () => {
    it("sets tab state to locked and clears selections", () => {
      let tabId: string;
      act(() => {
        tabId = useDatabaseTabs.getState().addTab("/test/db.kdbx");
        useDatabaseTabs.getState().updateTabState(tabId, {
          selectedGroupId: "group-1",
          selectedEntryId: "entry-1",
        });
      });

      act(() => {
        useDatabaseTabs.getState().lockTab(tabId!);
      });

      const tab = useDatabaseTabs.getState().tabs.find((t) => t.id === tabId!);
      expect(tab?.state).toBe("locked");
      expect(tab?.selectedGroupId).toBeNull();
      expect(tab?.selectedEntryId).toBeNull();
    });

    it("does not affect other tabs", () => {
      let tabId1: string;
      let tabId2: string;
      act(() => {
        tabId1 = useDatabaseTabs.getState().addTab("/test/db1.kdbx");
        tabId2 = useDatabaseTabs.getState().addTab("/test/db2.kdbx");
        useDatabaseTabs.getState().updateTabInfo(tabId1!, makeDatabaseInfo());
        useDatabaseTabs.getState().updateTabInfo(tabId2!, makeDatabaseInfo());
      });

      act(() => {
        useDatabaseTabs.getState().lockTab(tabId1!);
      });

      const tab1 = useDatabaseTabs
        .getState()
        .tabs.find((t) => t.id === tabId1!);
      const tab2 = useDatabaseTabs
        .getState()
        .tabs.find((t) => t.id === tabId2!);
      expect(tab1?.state).toBe("locked");
      expect(tab2?.state).toBe("open");
    });
  });

  describe("updateTabInfo", () => {
    it("derives state as locked when info.isLocked is true", () => {
      let tabId: string;
      act(() => {
        tabId = useDatabaseTabs.getState().addTab("/test/db.kdbx");
      });

      act(() => {
        useDatabaseTabs
          .getState()
          .updateTabInfo(tabId!, makeDatabaseInfo({ isLocked: true }));
      });

      const tab = useDatabaseTabs.getState().tabs.find((t) => t.id === tabId!);
      expect(tab?.state).toBe("locked");
    });

    it("derives state as open when info.isLocked is false", () => {
      let tabId: string;
      act(() => {
        tabId = useDatabaseTabs.getState().addTab("/test/db.kdbx");
      });

      act(() => {
        useDatabaseTabs
          .getState()
          .updateTabInfo(tabId!, makeDatabaseInfo({ isLocked: false }));
      });

      const tab = useDatabaseTabs.getState().tabs.find((t) => t.id === tabId!);
      expect(tab?.state).toBe("open");
    });
  });
});
