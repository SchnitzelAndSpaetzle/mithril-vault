import { createFileRoute } from "@tanstack/react-router";

export const Route = createFileRoute("/dashboard/entry/edit")({
  component: RouteComponent,
});

function RouteComponent() {
  return <div>Hello &#34;/entry/edit&#34;!</div>;
}
