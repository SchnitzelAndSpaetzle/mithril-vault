import { createFileRoute, redirect } from "@tanstack/react-router";

import { PasswordHealthReportView } from "@/components/security/PasswordHealthReportView";
import { useDatabaseTabs } from "@/stores/database-tabs";

export const Route = createFileRoute("/dashboard/security/$dbId")({
  beforeLoad: ({ params }) => {
    const state = useDatabaseTabs.getState();
    const tab = state.tabs.find(
      (item) => item.dbId === params.dbId || item.path === params.dbId
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
  },
  component: SecurityRoutePage,
});

function SecurityRoutePage() {
  const { dbId } = Route.useParams();
  return <PasswordHealthReportView dbId={dbId} />;
}
