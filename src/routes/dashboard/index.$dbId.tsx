import { createFileRoute, redirect } from "@tanstack/react-router";
import { useIsMobile } from "@/hooks/use-mobile.ts";
import MobileContentArea from "@/views/MobileContentArea.tsx";
import DesktopContentArea from "@/views/DesktopContentArea.tsx";
import { database } from "@/lib/tauri";
import { useDatabaseTabs } from "@/stores/database-tabs";

export const Route = createFileRoute("/dashboard/index/$dbId")({
  beforeLoad: ({ params }) => {
    const state = useDatabaseTabs.getState();
    const tab = state.tabs.find(
      (item) => item.dbId === params.dbId || item.path === params.dbId
    );

    if (!tab) {
      throw redirect({ to: "/" });
    }

    if (tab.state === "unlocking") {
      throw redirect({
        to: "/unlock",
        search: tab.path ? { path: tab.path } : {},
      });
    }

    state.setActiveTab(tab.id);

    return { tabId: tab.id };
  },
  loader: async ({ params }) => {
    const info = await database.getInfo(params.dbId);
    return { info };
  },
  component: DashboardIndex,
  pendingComponent: Loading,
});

function DashboardIndex() {
  const isMobile = useIsMobile();
  Route.useLoaderData();

  return (
    <div className="flex min-h-0 flex-1 overflow-hidden">
      {isMobile ? <MobileContentArea /> : <DesktopContentArea />}
    </div>
  );
}

function Loading() {
  // TODO: create loading component
  return <div>Loading...</div>;
}
