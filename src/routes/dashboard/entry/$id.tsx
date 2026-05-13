import {
  createFileRoute,
  useCanGoBack,
  useNavigate,
  useRouter,
} from "@tanstack/react-router";
import NavEntries from "@/components/entries/nav-entries.tsx";
import EntryItemDetails from "@/components/entries/EntryItemDetails.tsx";
import { EntryActions } from "@/components/entries/EntryActions.tsx";
import { Button } from "@/components/ui/button.tsx";
import { ArrowBigLeft } from "lucide-react";
import { useActiveDatabase } from "@/hooks/use-active-database";
import { useEntryMutations } from "@/hooks/use-entry-mutations";
import { useDatabaseTabs } from "@/stores/database-tabs";
import { ask } from "@tauri-apps/plugin-dialog";
import { toast } from "sonner";
import { SaveError } from "@/lib/save-with-error-toast";

export const Route = createFileRoute("/dashboard/entry/$id")({
  component: EntryMobileComponent,
});

function EntryMobileComponent() {
  const router = useRouter();
  const navigate = useNavigate();
  const canGoBack = useCanGoBack();
  const { id } = Route.useParams();
  const { dbId, tab } = useActiveDatabase();
  const { deleteEntry } = useEntryMutations(dbId ?? null);
  const updateTabState = useDatabaseTabs((s) => s.updateTabState);

  const handleEdit = () => {
    void navigate({ to: "/dashboard/entry/edit" });
  };

  const handleNew = () => {
    void navigate({ to: "/dashboard/entry/new" });
  };

  const handleDelete = async () => {
    if (!dbId) return;

    const confirmed = await ask(
      "Are you sure you want to delete this entry? This action cannot be undone.",
      { title: "Delete Entry", kind: "warning" }
    );

    if (!confirmed) return;

    deleteEntry.mutate(
      { dbId, id },
      {
        onSuccess: () => {
          if (tab) {
            updateTabState(tab.id, { selectedEntryId: null });
          }
          toast.success("Entry deleted.");
          router.history.back();
        },
        onError: (error) => {
          // saveWithErrorToast already surfaced a save/backup error toast.
          if (error instanceof SaveError) return;
          toast.error(`Failed to delete entry: ${error.message}`);
        },
      }
    );
  };

  return (
    <div className="overflow-auto">
      <NavEntries
        actions={
          <EntryActions
            onNew={handleNew}
            onEdit={handleEdit}
            onDelete={() => void handleDelete()}
          />
        }
      >
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
