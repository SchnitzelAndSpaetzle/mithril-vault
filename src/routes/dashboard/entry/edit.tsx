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
import { useEntryDetail } from "@/hooks/use-entry-detail";

export const Route = createFileRoute("/dashboard/entry/edit")({
  component: EntryEditMobileComponent,
});

function EntryEditMobileComponent() {
  const router = useRouter();
  const canGoBack = useCanGoBack();
  const { tab, dbId } = useActiveDatabase();
  const entryId = tab?.selectedEntryId ?? "";
  const groupId = tab?.selectedGroupId ?? tab?.info?.rootGroupId ?? "";

  const { entry } = useEntryDetail(
    entryId && dbId ? entryId : "",
    entryId && dbId ? dbId : ""
  );

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
        {dbId && entry ? (
          <EntryEditForm
            entry={entry}
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
