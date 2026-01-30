import { createFileRoute } from "@tanstack/react-router";
import { UnlockView } from "@/views/UnlockView.tsx";
import { z } from "zod/v4";

const unlockSearchSchema = z.object({
  path: z.string().optional(),
});

export const Route = createFileRoute("/(auth)/unlock")({
  validateSearch: unlockSearchSchema,
  component: RouteComponent,
});

function RouteComponent() {
  const { path } = Route.useSearch();
  return <UnlockView initialPath={path} />;
}
