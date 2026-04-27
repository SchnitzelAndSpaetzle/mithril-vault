// SPDX-License-Identifier: MIT

import { listen } from "@tauri-apps/api/event";
import { useEffect, useRef } from "react";
import { useNavigate } from "@tanstack/react-router";
import { database } from "@/lib/tauri";
import { useDatabaseTabs } from "@/stores/database-tabs";

const ACTIVITY_EVENTS = [
  "mousemove",
  "keydown",
  "click",
  "scroll",
  "touchstart",
] as const;

const THROTTLE_MS = 30_000;

export function useAutoLock() {
  const lastReportRef = useRef(0);
  const navigate = useNavigate();
  const lockTab = useDatabaseTabs((state) => state.lockTab);
  const getActiveTabId = () => useDatabaseTabs.getState().activeTabId;
  const getTabs = () => useDatabaseTabs.getState().tabs;

  useEffect(() => {
    const reportActivity = () => {
      const now = Date.now();
      if (now - lastReportRef.current >= THROTTLE_MS) {
        lastReportRef.current = now;
        void database.reportActivity();
      }
    };

    // Report initial activity on mount
    reportActivity();

    const handleActivity = () => {
      reportActivity();
    };

    for (const event of ACTIVITY_EVENTS) {
      window.addEventListener(event, handleActivity, { passive: true });
    }

    // Listen for backend lock event
    const unlistenPromise = listen<string[]>("database-locked", (event) => {
      const lockedPaths = event.payload;
      const tabs = getTabs();
      const activeTabId = getActiveTabId();

      let activeTabLocked = false;

      for (const tab of tabs) {
        if (tab.dbId && lockedPaths.includes(tab.dbId)) {
          lockTab(tab.id);
          if (tab.id === activeTabId) {
            activeTabLocked = true;
          }
        }
      }

      if (activeTabLocked) {
        void navigate({ to: "/unlock" });
      }
    });

    return () => {
      for (const event of ACTIVITY_EVENTS) {
        window.removeEventListener(event, handleActivity);
      }
      void unlistenPromise.then((unlisten) => unlisten());
    };
  }, [lockTab, navigate]);
}
