import {
  createFileRoute,
  useCanGoBack,
  useRouter,
} from "@tanstack/react-router";
import { ArrowBigLeft } from "lucide-react";
import NavEntries from "@/components/entries/nav-entries.tsx";
import { EntryEditForm } from "@/components/entries/EntryEditForm.tsx";
import { Button } from "@/components/ui/button.tsx";
import { useActiveDatabase } from "@/hooks/use-active-database";

export const Route = createFileRoute("/dashboard/entry/new")({
  component: EntryNewMobileComponent,
});

function EntryNewMobileComponent() {
  const router = useRouter();
  const canGoBack = useCanGoBack();
  const { tab, dbId } = useActiveDatabase();
  const groupId = tab?.selectedGroupId ?? tab?.info?.rootGroupId ?? "";

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
        {dbId ? (
          <EntryEditForm
            dbId={dbId}
            groupId={groupId}
            onSave={() => router.history.back()}
            onCancel={() => router.history.back()}
          />
        ) : null}
      </div>
    </div>
  );
}
