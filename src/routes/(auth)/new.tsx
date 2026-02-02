import { createFileRoute } from "@tanstack/react-router";
import { CreateDatabaseView } from "@/views/CreateDatabaseView";

export const Route = createFileRoute("/(auth)/new")({
  component: CreateDatabaseView,
});
