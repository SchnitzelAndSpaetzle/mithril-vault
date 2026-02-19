import { Loader2 } from "lucide-react";
import { Button } from "@/components/ui/button";

interface EntryFormActionsProps {
  isPending: boolean;
  isSubmitDisabled: boolean;
  isEditMode: boolean;
  secretLoadError: string | null;
  onCancel: () => void;
  onRetrySecretLoad: () => void;
}

export function EntryFormActions({
  isPending,
  isSubmitDisabled,
  isEditMode,
  secretLoadError,
  onCancel,
  onRetrySecretLoad,
}: EntryFormActionsProps) {
  return (
    <>
      <div className="flex items-center gap-2 pt-2">
        <Button type="submit" disabled={isSubmitDisabled}>
          {isPending && <Loader2 className="mr-2 size-4 animate-spin" />}
          {isEditMode ? "Save Changes" : "Create Entry"}
        </Button>
        <Button
          type="button"
          variant="outline"
          onClick={onCancel}
          disabled={isPending}
        >
          Cancel
        </Button>
      </div>
      {secretLoadError && (
        <div className="flex items-center justify-between gap-2 rounded-md border border-destructive/40 bg-destructive/10 p-2 text-sm text-destructive">
          <span>Protected values could not be loaded.</span>
          <Button
            type="button"
            size="sm"
            variant="outline"
            onClick={onRetrySecretLoad}
            disabled={isPending}
          >
            Retry
          </Button>
        </div>
      )}
    </>
  );
}
