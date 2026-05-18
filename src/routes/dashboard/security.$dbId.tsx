import { createFileRoute } from "@tanstack/react-router";

import { PasswordHealthReportView } from "@/components/security/PasswordHealthReportView";
import { requireUnlockedTab } from "@/lib/require-unlocked-tab";

export const Route = createFileRoute("/dashboard/security/$dbId")({
  beforeLoad: ({ params }) => requireUnlockedTab(params.dbId),
  component: SecurityRoutePage,
});

function SecurityRoutePage() {
  const { dbId } = Route.useParams();
  return <PasswordHealthReportView dbId={dbId} />;
}
