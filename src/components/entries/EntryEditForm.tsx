import { createElement, useEffect, useState } from "react";
import { Controller, type Resolver, useForm } from "react-hook-form";
import { standardSchemaResolver } from "@hookform/resolvers/standard-schema";
import { ask } from "@tauri-apps/plugin-dialog";
import { Dices, Eye, EyeClosed, Loader2 } from "lucide-react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import {
  Field,
  FieldError,
  FieldGroup,
  FieldLabel,
} from "@/components/ui/field";
import {
  InputGroup,
  InputGroupAddon,
  InputGroupButton,
  InputGroupInput,
} from "@/components/ui/input-group";
import { PasswordStrengthIndicator } from "@/components/database/create-wizard/PasswordStrengthIndicator";
import { TagInput } from "@/components/entries/TagInput";
import { IconPickerPopover } from "@/components/entries/IconPickerPopover";
import { PasswordGeneratorPopover } from "@/components/entries/PasswordGeneratorPopover";
import { CustomFieldsEditor } from "@/components/entries/CustomFieldsEditor";
import { useEntryMutations } from "@/hooks/use-entry-mutations";
import { useTags } from "@/hooks/use-tags";
import { entries as entriesApi } from "@/lib/tauri";
import { getKeepassIcon } from "@/lib/keepass-icons";
import { entryFormSchema, type EntryFormValues } from "@/lib/formTypes";
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
}

export function EntryEditForm({
  entry,
  dbId,
  groupId,
  onSave,
  onCancel,
}: EntryEditFormProps) {
  const isEditMode = !!entry;
  const [showPassword, setShowPassword] = useState(false);
  const [isLoadingSecrets, setIsLoadingSecrets] = useState(isEditMode);

  const { createEntry, updateEntry } = useEntryMutations(dbId);
  const { data: availableTags } = useTags(dbId);

  const form = useForm<EntryFormValues>({
    resolver: standardSchemaResolver(
      entryFormSchema
    ) as Resolver<EntryFormValues>,
    defaultValues: entry
      ? {
          title: entry.title,
          username: entry.username,
          password: "",
          url: entry.url ?? "",
          notes: entry.notes ?? "",
          iconId: entry.iconId ?? 0,
          tags: [...entry.tags],
          customFields: entry.customFieldMeta.map((meta) => ({
            key: meta.key,
            value: meta.isProtected ? "" : (entry.customFields[meta.key] ?? ""),
            isProtected: meta.isProtected,
          })),
        }
      : {
          title: "",
          username: "",
          password: "",
          url: "",
          notes: "",
          iconId: 0,
          tags: [],
          customFields: [],
        },
  });

  // Fetch password and protected custom fields for edit mode
  useEffect(() => {
    if (!entry) return;

    async function loadSecrets() {
      try {
        const [password, ...protectedValues] = await Promise.all([
          entriesApi.getPassword(dbId, entry!.id),
          ...entry!.customFieldMeta
            .filter((m) => m.isProtected)
            .map((m) =>
              entriesApi
                .getProtectedCustomField(dbId, entry!.id, m.key)
                .then((r) => ({ key: m.key, value: r.value }))
            ),
        ]);

        form.setValue("password", password, { shouldDirty: false });

        for (const pv of protectedValues) {
          const fieldIndex = entry!.customFieldMeta.findIndex(
            (m) => m.key === pv.key
          );
          if (fieldIndex !== -1) {
            form.setValue(`customFields.${fieldIndex}.value`, pv.value, {
              shouldDirty: false,
            });
          }
        }
      } finally {
        setIsLoadingSecrets(false);
      }
    }

    void loadSecrets();
    // eslint-disable-next-line react-hooks/exhaustive-deps -- Only re-run when entry ID changes, not on every entry object update
  }, [entry?.id, dbId, form]);

  async function onSubmit(values: EntryFormValues) {
    const customFields: Record<string, string> = {};
    const protectedCustomFields: Record<string, string> = {};

    for (const cf of values.customFields) {
      if (cf.isProtected) {
        protectedCustomFields[cf.key] = cf.value;
      } else {
        customFields[cf.key] = cf.value;
      }
    }

    try {
      if (isEditMode) {
        const result = await updateEntry.mutateAsync({
          dbId,
          id: entry!.id,
          data: {
            title: values.title,
            username: values.username,
            password: values.password,
            url: values.url || undefined,
            notes: values.notes || undefined,
            iconId: values.iconId,
            tags: values.tags,
            customFields,
            protectedCustomFields,
          },
        });
        toast.success("Entry updated");
        onSave(result);
      } else {
        const result = await createEntry.mutateAsync({
          dbId,
          groupId,
          data: {
            title: values.title,
            username: values.username,
            password: values.password,
            url: values.url || undefined,
            notes: values.notes || undefined,
            iconId: values.iconId,
            tags: values.tags,
            customFields,
            protectedCustomFields,
          },
        });
        toast.success("Entry created");
        onSave(result);
      }
    } catch (error) {
      toast.error(
        `Failed to ${isEditMode ? "update" : "create"} entry: ${String(error)}`
      );
    }
  }

  async function handleCancel() {
    if (form.formState.isDirty) {
      const confirmed = await ask(
        "You have unsaved changes. Are you sure you want to discard them?",
        { title: "Unsaved Changes", kind: "warning" }
      );
      if (!confirmed) return;
    }
    onCancel();
  }

  const isPending = createEntry.isPending || updateEntry.isPending;
  const watchedPassword = form.watch("password");

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
        {/* Icon + Title */}
        <Field>
          <FieldLabel htmlFor="title">Title</FieldLabel>
          <div className="flex items-center gap-2">
            <Controller
              name="iconId"
              control={form.control}
              render={({ field }) => (
                <IconPickerPopover
                  selectedIconId={field.value}
                  onSelect={field.onChange}
                >
                  <Button
                    type="button"
                    variant="outline"
                    size="icon-sm"
                    aria-label="Choose icon"
                  >
                    {createElement(getKeepassIcon(field.value), {
                      className: "size-4",
                    })}
                  </Button>
                </IconPickerPopover>
              )}
            />
            <Controller
              name="title"
              control={form.control}
              render={({ field, fieldState }) => (
                <div className="flex-1">
                  <Input
                    {...field}
                    id="title"
                    aria-invalid={fieldState.invalid}
                    placeholder="Entry title"
                    disabled={isPending}
                  />
                  {fieldState.error && (
                    <FieldError>{fieldState.error.message}</FieldError>
                  )}
                </div>
              )}
            />
          </div>
        </Field>

        {/* Username */}
        <Field>
          <FieldLabel htmlFor="username">Username</FieldLabel>
          <Controller
            name="username"
            control={form.control}
            render={({ field }) => (
              <Input
                {...field}
                id="username"
                placeholder="Username or email"
                autoComplete="username"
                disabled={isPending}
              />
            )}
          />
        </Field>

        {/* Password */}
        <Field>
          <FieldLabel htmlFor="password">Password</FieldLabel>
          <Controller
            name="password"
            control={form.control}
            render={({ field, fieldState }) => (
              <>
                <InputGroup>
                  <InputGroupInput
                    {...field}
                    id="password"
                    aria-invalid={fieldState.invalid}
                    type={showPassword ? "text" : "password"}
                    placeholder="Enter password..."
                    autoComplete="new-password"
                    disabled={isPending}
                  />
                  <InputGroupAddon align="inline-end">
                    <InputGroupButton
                      variant="ghost"
                      size="icon-xs"
                      type="button"
                      aria-label={
                        showPassword ? "Hide password" : "Show password"
                      }
                      onClick={() => setShowPassword((prev) => !prev)}
                      disabled={isPending}
                    >
                      {showPassword ? <Eye /> : <EyeClosed />}
                    </InputGroupButton>
                    <PasswordGeneratorPopover
                      onUsePassword={(pw) => {
                        form.setValue("password", pw, { shouldDirty: true });
                      }}
                    >
                      <InputGroupButton
                        variant="ghost"
                        size="icon-xs"
                        type="button"
                        aria-label="Generate password"
                        disabled={isPending}
                      >
                        <Dices />
                      </InputGroupButton>
                    </PasswordGeneratorPopover>
                  </InputGroupAddon>
                </InputGroup>
                <PasswordStrengthIndicator password={watchedPassword} />
                {fieldState.error && (
                  <FieldError>{fieldState.error.message}</FieldError>
                )}
              </>
            )}
          />
        </Field>

        {/* URL */}
        <Field>
          <FieldLabel htmlFor="url">URL</FieldLabel>
          <Controller
            name="url"
            control={form.control}
            render={({ field, fieldState }) => (
              <>
                <Input
                  {...field}
                  id="url"
                  aria-invalid={fieldState.invalid}
                  placeholder="https://example.com"
                  disabled={isPending}
                />
                {fieldState.error && (
                  <FieldError>{fieldState.error.message}</FieldError>
                )}
              </>
            )}
          />
        </Field>

        {/* Tags */}
        <Field>
          <FieldLabel>Tags</FieldLabel>
          <Controller
            name="tags"
            control={form.control}
            render={({ field }) => (
              <TagInput
                value={field.value}
                onChange={field.onChange}
                disabled={isPending}
                suggestions={availableTags ?? []}
              />
            )}
          />
        </Field>

        {/* Notes */}
        <Field>
          <FieldLabel htmlFor="notes">Notes</FieldLabel>
          <Controller
            name="notes"
            control={form.control}
            render={({ field }) => (
              <Textarea
                {...field}
                id="notes"
                placeholder="Additional notes..."
                disabled={isPending}
                rows={4}
              />
            )}
          />
        </Field>

        {/* Custom Fields */}
        <CustomFieldsEditor control={form.control} disabled={isPending} />

        {/* Actions */}
        <div className="flex items-center gap-2 pt-2">
          <Button type="submit" disabled={isPending}>
            {isPending && <Loader2 className="mr-2 size-4 animate-spin" />}
            {isEditMode ? "Save Changes" : "Create Entry"}
          </Button>
          <Button
            type="button"
            variant="outline"
            onClick={handleCancel}
            disabled={isPending}
          >
            Cancel
          </Button>
        </div>
      </FieldGroup>
    </form>
  );
}
