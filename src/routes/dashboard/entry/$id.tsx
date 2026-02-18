import {
  createFileRoute,
  useCanGoBack,
  useRouter,
} from "@tanstack/react-router";
import NavEntries from "@/components/entries/nav-entries.tsx";
import EntryItemDetails from "@/components/entries/EntryItemDetails.tsx";
import { Button } from "@/components/ui/button.tsx";
import { ArrowBigLeft } from "lucide-react";
import { useActiveDatabase } from "@/hooks/use-active-database";

export const Route = createFileRoute("/dashboard/entry/$id")({
  component: EntryMobileComponent,
});

function EntryMobileComponent() {
  const router = useRouter();
  const canGoBack = useCanGoBack();
  const { id } = Route.useParams();
  const { dbId } = useActiveDatabase();

  return (
    <div className="overflow-auto">
      <NavEntries>
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
      </NavEntries>
      <div className="flex flex-col gap-4 p-4">
        {dbId ? <EntryItemDetails entryId={id} dbId={dbId} /> : null}
      </div>
    </div>
  );
}
