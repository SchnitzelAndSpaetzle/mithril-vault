import { createFileRoute, redirect } from "@tanstack/react-router";
import { UnlockView } from "@/views/UnlockView.tsx";
import { z } from "zod/v4";
import { settings } from "@/lib/tauri";
import { useDatabaseTabs } from "@/stores/database-tabs";

const unlockSearchSchema = z.object({
  path: z.string().optional(),
});

export const Route = createFileRoute("/(auth)/unlock")({
  validateSearch: unlockSearchSchema,
  loaderDeps: ({ search }) => ({ path: search.path }),
  loader: async ({ deps }) => {
    const path = deps.path;
    let initialKeyfile = "";
    let rememberKeyfile = false;
    let isLocked = false;

    if (path) {
      const state = useDatabaseTabs.getState();
      const existing = state.tabs.find(
        (tab) => tab.path === path || tab.info?.path === path
      );

      if (existing?.state === "open") {
        state.setActiveTab(existing.id);
        throw redirect({
          to: "/dashboard/index/$dbId",
          params: { dbId: existing.dbId ?? existing.path ?? path },
        });
      }

      if (existing?.state === "locked") {
        isLocked = true;
        state.setActiveTab(existing.id);
        state.updateTabState(existing.id, { state: "unlocking" });
      } else {
        const tabId = existing?.id ?? state.addTab(path);
        state.setActiveTab(tabId);
        state.updateTabState(tabId, { path, state: "unlocking" });
      }

      try {
        const savedKeyfile = await settings.getKeyfileForDatabase(path);
        if (savedKeyfile) {
          initialKeyfile = savedKeyfile;
          rememberKeyfile = true;
        }
      } catch {
        // Ignore errors - just don't pre-populate
      }
    }

    return {
      initialPath: path,
      initialKeyfile,
      rememberKeyfile,
      isLocked,
    };
  },
  component: RouteComponent,
});

function RouteComponent() {
  const { initialPath, initialKeyfile, rememberKeyfile, isLocked } =
    Route.useLoaderData();
  return (
    <UnlockView
      initialPath={initialPath}
      initialKeyfile={initialKeyfile}
      rememberKeyfile={rememberKeyfile}
      isLocked={isLocked}
    />
  );
}
