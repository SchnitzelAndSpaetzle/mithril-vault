import { useTranslation } from "react-i18next";
import { Loader2 } from "lucide-react";
import { Button } from "@/components/ui/button";

interface EntryFormActionsProps {
  isPending: boolean;
  isSubmitDisabled: boolean;
  isEditMode: boolean;
  secretLoadError: string | null;
  onCancel: () => void;
  onRetrySecretLoad: () => void;
  onSaveAndNew?: (() => void) | undefined;
}

export function EntryFormActions({
  isPending,
  isSubmitDisabled,
  isEditMode,
  secretLoadError,
  onCancel,
  onRetrySecretLoad,
  onSaveAndNew,
}: EntryFormActionsProps) {
  const { t } = useTranslation();

  return (
    <>
      <div className="flex items-center gap-2 pt-2">
        <Button type="submit" disabled={isSubmitDisabled}>
          {isPending && <Loader2 className="mr-2 size-4 animate-spin" />}
          {isEditMode
            ? t("entries.form.saveChanges")
            : t("entries.form.createEntry")}
        </Button>
        {!isEditMode && onSaveAndNew && (
          <Button
            type="button"
            variant="outline"
            disabled={isSubmitDisabled}
            onClick={onSaveAndNew}
          >
            {isPending && <Loader2 className="mr-2 size-4 animate-spin" />}
            {t("entries.form.saveAndNew")}
          </Button>
        )}
        <Button
          type="button"
          variant="outline"
          onClick={onCancel}
          disabled={isPending}
        >
          {t("common.cancel")}
        </Button>
      </div>
      {secretLoadError && (
        <div className="flex items-center justify-between gap-2 rounded-md border border-destructive/40 bg-destructive/10 p-2 text-sm text-destructive">
          <span>{t("entries.form.protectedValuesError")}</span>
          <Button
            type="button"
            size="sm"
            variant="outline"
            onClick={onRetrySecretLoad}
            disabled={isPending}
          >
            {t("common.retry")}
          </Button>
        </div>
      )}
    </>
  );
}
