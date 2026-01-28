import { createFileRoute } from "@tanstack/react-router";

export const Route = createFileRoute("/(auth)/import-file")({
  component: RouteComponent,
});

function RouteComponent() {
  return <div>Hello /(auth)/import!</div>;
}
