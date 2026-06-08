import { Loader2 } from "lucide-react";
import { CustomFieldsEditor } from "@/components/entries/custom-fields-editor";
import { FieldGroup } from "@/components/ui/field";
import {
  EntryExpiryField,
  EntryFormActions,
  EntryGroupField,
  EntryNotesField,
  EntryPasswordField,
  EntryTagsField,
  EntryTitleField,
  EntryUrlField,
  EntryUsernameField,
} from "@/components/entries/entry-edit-form";
import { useCustomIcons } from "@/hooks/use-custom-icons";
import { useEntryEditForm } from "@/hooks/use-entry-edit-form";
import { useUsernameSuggestions } from "@/hooks/use-username-suggestions";
import type { Entry } from "@/lib/types";

interface EntryEditFormProps {
  /** Existing entry to edit. When undefined, form is in "create" mode. */
  entry?: Entry | null;
  /** Database ID */
  dbId: string;
  /** Target group ID for new entries */
  groupId: string;
  /** Called after successful save */
  onSave: (entry: Entry) => void;
  /** Called when user cancels (after unsaved changes check) */
  onCancel: () => void;
  onDirtyChange?: (isDirty: boolean) => void;
}

export function EntryEditForm({
  entry,
  dbId,
  groupId,
  onSave,
  onCancel,
  onDirtyChange,
}: EntryEditFormProps) {
  const {
    form,
    isEditMode,
    isLoadingSecrets,
    secretLoadError,
    isPending,
    isSubmitDisabled,
    availableTags,
    watchedPassword,
    watchedUsername,
    onSubmit,
    handleCancel,
    saveAndCreateAnother,
    retrySecretLoad,
    setGeneratedPassword,
    isFetchingFavicon,
    isClearingCustomIcon,
    hasCustomIcon,
    canFetchFavicon,
    fetchFaviconFromUrl,
    clearCustomIcon,
  } = useEntryEditForm({
    entry,
    dbId,
    groupId,
    onSave,
    onCancel,
    onDirtyChange,
  });
  const { data: customIcons } = useCustomIcons(dbId);
  const handleIconChange = (iconId: number) => {
    form.setValue("iconId", iconId, { shouldDirty: true });
    form.setValue("customIconUuid", null, { shouldDirty: true });
  };
  const handleCustomIconChange = (iconUuid: string) => {
    form.setValue("customIconUuid", iconUuid, { shouldDirty: true });
  };
  const {
    usernameSuggestions,
    activeUsernameSuggestionIndex,
    showUsernameSuggestions,
    handleFocus,
    handleBlur,
    handleKeyDown,
    applySuggestion,
  } = useUsernameSuggestions({
    dbId,
    watchedUsername,
    isPending,
  });

  if (isLoadingSecrets) {
    return (
      <div className="flex items-center justify-center py-12">
        <Loader2 className="size-6 animate-spin text-muted-foreground" />
      </div>
    );
  }

  return (
    <form onSubmit={form.handleSubmit(onSubmit)}>
      <FieldGroup>
        <EntryGroupField
          control={form.control}
          dbId={dbId}
          isPending={isPending}
        />
        <EntryTitleField
          control={form.control}
          isPending={isPending}
          autoFocus={!isEditMode}
          isEditMode={isEditMode}
          hasCustomIcon={hasCustomIcon}
          canFetchFavicon={canFetchFavicon}
          isFetchingFavicon={isFetchingFavicon}
          isClearingCustomIcon={isClearingCustomIcon}
          customIcons={customIcons ?? {}}
          onIconChange={handleIconChange}
          onCustomIconChange={handleCustomIconChange}
          onFetchFavicon={fetchFaviconFromUrl}
          onClearCustomIcon={clearCustomIcon}
        />
        <EntryUsernameField
          control={form.control}
          isPending={isPending}
          usernameSuggestions={usernameSuggestions}
          activeUsernameSuggestionIndex={activeUsernameSuggestionIndex}
          showUsernameSuggestions={showUsernameSuggestions}
          onFocus={handleFocus}
          onBlur={handleBlur}
          onKeyDown={handleKeyDown}
          onSelectSuggestion={applySuggestion}
        />
        <EntryPasswordField
          key={entry?.id ?? "new"}
          control={form.control}
          isPending={isPending}
          watchedPassword={watchedPassword}
          onUseGeneratedPassword={setGeneratedPassword}
        />
        <EntryExpiryField control={form.control} isPending={isPending} />
        <EntryUrlField control={form.control} isPending={isPending} />
        <EntryTagsField
          control={form.control}
          isPending={isPending}
          availableTags={availableTags}
        />
        <EntryNotesField control={form.control} isPending={isPending} />

        <CustomFieldsEditor control={form.control} disabled={isPending} />

        <EntryFormActions
          isPending={isPending}
          isSubmitDisabled={isSubmitDisabled}
          isEditMode={isEditMode}
          secretLoadError={secretLoadError}
          onCancel={handleCancel}
          onRetrySecretLoad={retrySecretLoad}
          onSaveAndNew={!isEditMode ? saveAndCreateAnother : undefined}
        />
      </FieldGroup>
    </form>
  );
}
