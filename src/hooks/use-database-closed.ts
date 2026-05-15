// SPDX-License-Identifier: MIT

import { listen } from "@tauri-apps/api/event";
import { useEffect } from "react";
import { useNavigate } from "@tanstack/react-router";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { useDatabaseTabs } from "@/stores/database-tabs";

/**
 * Payload emitted by the backend when the open-Vault map drops an entry it
 * could not safely keep — today this fires after a successful restore. The
 * `reason` discriminator lets us tailor the post-close behaviour (e.g.
 * which toast to show) without inspecting the path.
 */
interface DatabaseClosedPayload {
  path: string;
  reason: "restore" | (string & {});
}

/**
 * Listens for `database-closed` events from the backend and routes the user
 * back to the unlock screen for the affected Vault. The in-memory tab is
 * locked rather than removed so the user lands on the same Vault they were
 * looking at — restore is meant to recover, not to lose context.
 */
export function useDatabaseClosed() {
  const navigate = useNavigate();
  const { t } = useTranslation();
  const lockTab = useDatabaseTabs((state) => state.lockTab);

  useEffect(() => {
    const unlistenPromise = listen<DatabaseClosedPayload>(
      "database-closed",
      (event) => {
        const { path, reason } = event.payload;
        const tabs = useDatabaseTabs.getState().tabs;
        const activeTabId = useDatabaseTabs.getState().activeTabId;
        const matched = tabs.find(
          (tab) =>
            tab.dbId === path || tab.info?.path === path || tab.path === path
        );
        if (matched) {
          lockTab(matched.id);
          if (matched.id === activeTabId) {
            void navigate({ to: "/unlock", search: { path } });
          }
        }
        if (reason === "restore") {
          toast.success(t("settings.backups.list.restore.success"));
        }
      }
    );
    return () => {
      void unlistenPromise.then((unlisten) => unlisten());
    };
  }, [lockTab, navigate, t]);
}
