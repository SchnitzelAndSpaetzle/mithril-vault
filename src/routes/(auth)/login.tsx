import { createFileRoute } from "@tanstack/react-router";
import { LoginForm } from "@/components/security/unlock-database-form/login-form.tsx";

export const Route = createFileRoute("/(auth)/login")({
  component: RouteComponent,
});

function RouteComponent() {
  return <LoginForm />;
}
