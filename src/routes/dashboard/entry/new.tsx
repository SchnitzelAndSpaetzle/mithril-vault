import { createFileRoute } from "@tanstack/react-router";

export const Route = createFileRoute("/dashboard/entry/new")({
  component: RouteComponent,
});

function RouteComponent() {
  return <div>Hello &#34;/entry/new&#34;!</div>;
}
