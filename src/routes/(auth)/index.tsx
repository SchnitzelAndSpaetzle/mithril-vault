import { createFileRoute } from "@tanstack/react-router";
import OpenOrCreateDatabase from "@/components/security/unlock-database-form/open-or-create-database.tsx";

export const Route = createFileRoute("/(auth)/")({
  component: Index,
});

function Index() {
  return <OpenOrCreateDatabase />;
}
