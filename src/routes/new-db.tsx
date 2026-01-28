import { createFileRoute } from "@tanstack/react-router";

export const Route = createFileRoute("/new-db")({
  component: RouteComponent,
});

function RouteComponent() {
  return <div>Hello new-db!</div>;
}
