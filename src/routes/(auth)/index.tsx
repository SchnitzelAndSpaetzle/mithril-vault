import { createFileRoute } from "@tanstack/react-router";
import OpenOrCreateDatabase from "@/components/security/unlock-database-form/open-or-create-database.tsx";
import { queryClient } from "@/router";
import { recentDatabasesQueryOptions } from "@/hooks/use-recent-databases";

export const Route = createFileRoute("/(auth)/")({
  loader: () => queryClient.ensureQueryData(recentDatabasesQueryOptions),
  component: Index,
});

function Index() {
  const recentDatabases = Route.useLoaderData();
  return <OpenOrCreateDatabase recentDatabases={recentDatabases} />;
}
