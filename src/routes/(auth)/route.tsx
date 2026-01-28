import {
  createFileRoute,
  Link,
  Outlet,
  useCanGoBack,
  useRouter,
} from "@tanstack/react-router";
import { ArrowBigLeft, GalleryVerticalEnd, Settings } from "lucide-react";
import { Button } from "@/components/ui/button.tsx";

export const Route = createFileRoute("/(auth)")({
  component: AuthRouteComponent,
});

function AuthRouteComponent() {
  const router = useRouter();
  const canGoBack = useCanGoBack();

  return (
    <div className="relative" data-slot="content" aria-hidden="true">
      <div className="bg-muted flex min-h-svh flex-col items-center justify-center gap-6 p-6 md:p-10">
        <div className="flex w-full max-w-sm flex-col gap-6">
          <div className="flex items-center gap-2 self-center font-medium">
            <div className="bg-primary text-primary-foreground flex size-6 items-center justify-center rounded-md">
              <GalleryVerticalEnd className="size-4" />
            </div>
            MithrilVault
          </div>
          <Outlet />
        </div>
      </div>

      <div className="absolute top-5 left-5 z-10">
        <div
          className="flex justify-center gap-2"
          data-slot="actions"
          aria-hidden="true"
        >
          <Button variant="outline" type="button" size="icon-sm" asChild>
            <Link to="/settings">
              <Settings />
            </Link>
          </Button>
          {canGoBack && (
            <Button
              variant="outline"
              size="sm"
              onClick={() => router.history.back()}
            >
              <ArrowBigLeft />
              Back
            </Button>
          )}
        </div>
      </div>
    </div>
  );
}
