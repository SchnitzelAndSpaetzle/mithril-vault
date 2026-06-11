import { createFileRoute } from "@tanstack/react-router";
import { useIsMobile } from "@/hooks/use-mobile";
import MobileContentArea from "@/views/MobileContentArea.tsx";
import DesktopContentArea from "@/views/DesktopContentArea.tsx";
import { database, KeepassIdSchema } from "@/lib/tauri";
import { requireUnlockedTab } from "@/lib/require-unlocked-tab";
import { z } from "zod/v4";
import { EntrySortFieldSchema, SortOrderSchema } from "@/lib/types";

const DashboardSearchSchema = z.object({
  groupId: KeepassIdSchema.optional(),
  sortBy: EntrySortFieldSchema.optional().default("title"),
  sortOrder: SortOrderSchema.optional().default("asc"),
  tag: z.string().optional(),
  hasAttachments: z.boolean().optional(),
});

export const Route = createFileRoute("/dashboard/index/$dbId")({
  beforeLoad: ({ params }) => requireUnlockedTab(params.dbId),
  loaderDeps: ({ search }) => ({ groupId: search.groupId ?? null }),
  loader: async ({ params }) => {
    const info = await database.getInfo(params.dbId);
    return { info };
  },
  component: DashboardIndex,
  validateSearch: DashboardSearchSchema,
  pendingComponent: Loading,
  errorComponent: ({ error }) => {
    return <div>{error.message}</div>;
  },
});

function DashboardIndex() {
  const isMobile = useIsMobile();
  Route.useLoaderData();

  return (
    <div className="flex min-h-0 flex-1 overflow-hidden">
      {isMobile ? <MobileContentArea /> : <DesktopContentArea />}
    </div>
  );
}

function Loading() {
  // TODO: create loading component
  return <div>Loading...</div>;
}
