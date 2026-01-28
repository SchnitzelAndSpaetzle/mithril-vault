import { createFileRoute } from "@tanstack/react-router";
import { useIsMobile } from "@/hooks/use-mobile.ts";
import MobileContentArea from "@/views/MobileContentArea.tsx";
import DesktopContentArea from "@/views/DesktopContentArea.tsx";

export const Route = createFileRoute("/dashboard/")({
  component: DashboardIndex,
});

function DashboardIndex() {
  const isMobile = useIsMobile();

  return (
    <div className="flex-1 overflow-hidden">
      {isMobile ? <MobileContentArea /> : <DesktopContentArea />}
    </div>
  );
}
