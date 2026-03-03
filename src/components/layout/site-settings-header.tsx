import { ArrowLeft } from "lucide-react";
import { useCanGoBack, useNavigate, useRouter } from "@tanstack/react-router";
import { Button } from "@/components/ui/button";
import { Separator } from "@/components/ui/separator";
import { SidebarTrigger } from "@/components/ui/sidebar";
import {
  Breadcrumb,
  BreadcrumbItem,
  BreadcrumbList,
  BreadcrumbPage,
  BreadcrumbSeparator,
} from "@/components/ui/breadcrumb.tsx";
import { useActiveDatabase } from "@/hooks/use-active-database";

function getDatabaseLabel(path?: string): string {
  if (!path) {
    return "No database";
  }

  const parts = path.split(/[/\\]/);
  return parts[parts.length - 1] ?? path;
}

export function SiteSettingsHeader() {
  const { tab, dbId } = useActiveDatabase();
  const router = useRouter();
  const canGoBack = useCanGoBack();
  const navigate = useNavigate();

  const handleBack = () => {
    if (canGoBack) {
      router.history.back();
      return;
    }

    if (dbId) {
      void navigate({
        to: "/dashboard/index/$dbId",
        params: { dbId },
      });
      return;
    }

    void navigate({ to: "/" });
  };

  return (
    <header className="flex h-16 shrink-0 items-center justify-between gap-2 border-b px-3">
      <div className="flex items-center gap-2 px-3">
        <SidebarTrigger />
        <Separator orientation="vertical" className="mr-2 h-4" />
        <Breadcrumb>
          <BreadcrumbList>
            <BreadcrumbItem>
              <BreadcrumbPage>Settings</BreadcrumbPage>
            </BreadcrumbItem>
            <BreadcrumbSeparator className="hidden md:block" />
            <BreadcrumbItem className="hidden md:block">
              <BreadcrumbPage>
                {tab?.info?.name ?? getDatabaseLabel(tab?.path)}
              </BreadcrumbPage>
            </BreadcrumbItem>
          </BreadcrumbList>
        </Breadcrumb>
      </div>
      <Button variant="outline" size="sm" type="button" onClick={handleBack}>
        <ArrowLeft className="size-4" />
        Back
      </Button>
    </header>
  );
}
