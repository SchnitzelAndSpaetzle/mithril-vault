// SPDX-License-Identifier: MIT

import { listen } from "@tauri-apps/api/event";
import { useEffect } from "react";
import { useNavigate } from "@tanstack/react-router";
import { useTranslation } from "react-i18next";
import { useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { queryKeys } from "@/lib/query-keys";
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
 * Domain-scoped query key prefixes that include the dbId in their second
 * position. Listing them here lets us evict every cached fact about a Vault
 * in one place when the on-disk bytes have been swapped under us.
 */
const DOMAIN_PREFIXES = [
  queryKeys.database.all,
  queryKeys.entries.all,
  queryKeys.groups.all,
  queryKeys.backups.all,
] as const;

/**
 * Listens for `database-closed` events from the backend and routes the user
 * back to the unlock screen for the affected Vault. The in-memory tab is
 * locked rather than removed so the user lands on the same Vault they were
 * looking at — restore is meant to recover, not to lose context.
 *
 * Also evicts every React Query cache scoped to the closed Vault's path.
 * Without this, a user who restores a backup and immediately unlocks could
 * briefly see pre-restore entries/groups/icons rendered from cached data
 * that React Query still considers fresh (staleTime: 30–60s on most
 * queries). `removeQueries` is used rather than `invalidateQueries` so the
 * stale data is gone synchronously — there is no "background refetch with
 * old data still visible" window for a Vault whose on-disk bytes were just
 * replaced.
 */
export function useDatabaseClosed() {
  const navigate = useNavigate();
  const { t } = useTranslation();
  const queryClient = useQueryClient();
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
        for (const prefix of DOMAIN_PREFIXES) {
          queryClient.removeQueries({ queryKey: [...prefix, path] });
        }
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
  }, [lockTab, navigate, queryClient, t]);
}
