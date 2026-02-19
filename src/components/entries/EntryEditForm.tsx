import { createElement, useEffect, useMemo, useRef, useState } from "react";
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
import { useEntries } from "@/hooks/use-entries";
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
  onDirtyChange?: (isDirty: boolean) => void;
}

function getEntryFormDefaults(entry?: Entry | null): EntryFormValues {
  if (!entry) {
    return {
      title: "",
      username: "",
      password: "",
      url: "",
      notes: "",
      iconId: 0,
      tags: [],
      customFields: [],
    };
  }

  return {
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
  };
}

export function EntryEditForm({
  entry,
  dbId,
  groupId,
  onSave,
  onCancel,
  onDirtyChange,
}: EntryEditFormProps) {
  const isEditMode = !!entry;
  const [showPassword, setShowPassword] = useState(false);
  const [isLoadingSecrets, setIsLoadingSecrets] = useState(isEditMode);
  const [isUsernameFocused, setIsUsernameFocused] = useState(false);
  const [activeUsernameSuggestionIndex, setActiveUsernameSuggestionIndex] =
    useState(-1);
  const entryRef = useRef<Entry | null | undefined>(entry);
  const entryId = entry?.id ?? null;

  const { createEntry, updateEntry } = useEntryMutations(dbId);
  const { data: availableTags } = useTags(dbId);
  const { data: allEntries } = useEntries(dbId);
  const usernameSuggestionsAll = useMemo(() => {
    const usernames = new Set<string>();
    for (const existingEntry of allEntries ?? []) {
      const normalizedUsername = existingEntry.username.trim();
      if (normalizedUsername.length > 0) {
        usernames.add(normalizedUsername);
      }
    }
    return Array.from(usernames).sort((a, b) =>
      a.localeCompare(b, undefined, { sensitivity: "base" })
    );
  }, [allEntries]);

  const form = useForm<EntryFormValues>({
    resolver: standardSchemaResolver(
      entryFormSchema
    ) as Resolver<EntryFormValues>,
    defaultValues: getEntryFormDefaults(entry),
  });

  useEffect(() => {
    entryRef.current = entry;
  }, [entry]);

  useEffect(() => {
    const currentEntry = entryRef.current ?? null;
    form.reset(getEntryFormDefaults(currentEntry));
    setShowPassword(false);
    setIsUsernameFocused(false);
    setActiveUsernameSuggestionIndex(-1);
    setIsLoadingSecrets(Boolean(entryId));
  }, [dbId, entryId, form]);

  useEffect(() => {
    onDirtyChange?.(form.formState.isDirty);
  }, [form.formState.isDirty, onDirtyChange]);

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
  const watchedUsername = form.watch("username");
  const normalizedUsernameInput = watchedUsername.trim().toLowerCase();
  const usernameSuggestions = useMemo(() => {
    const selectedUsername = watchedUsername.trim().toLowerCase();
    return usernameSuggestionsAll.filter((username) => {
      const normalized = username.toLowerCase();
      if (normalized === selectedUsername) {
        return false;
      }
      return (
        normalizedUsernameInput.length > 0 &&
        normalized.includes(normalizedUsernameInput)
      );
    });
  }, [normalizedUsernameInput, usernameSuggestionsAll, watchedUsername]);
  const showUsernameSuggestions =
    isUsernameFocused && !isPending && usernameSuggestions.length > 0;

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
              <div className="relative">
                <Input
                  {...field}
                  id="username"
                  placeholder="Username or email"
                  autoComplete="username"
                  disabled={isPending}
                  onFocus={() => {
                    setIsUsernameFocused(true);
                    setActiveUsernameSuggestionIndex(-1);
                  }}
                  onBlur={(event) => {
                    field.onBlur();
                    if (
                      event.relatedTarget instanceof HTMLElement &&
                      event.relatedTarget.dataset["usernameSuggestion"] ===
                        "true"
                    ) {
                      return;
                    }
                    setIsUsernameFocused(false);
                    setActiveUsernameSuggestionIndex(-1);
                  }}
                  onKeyDown={(event) => {
                    if (event.key === "ArrowDown" && showUsernameSuggestions) {
                      event.preventDefault();
                      setActiveUsernameSuggestionIndex((prev) =>
                        prev < usernameSuggestions.length - 1 ? prev + 1 : 0
                      );
                      return;
                    }

                    if (event.key === "ArrowUp" && showUsernameSuggestions) {
                      event.preventDefault();
                      setActiveUsernameSuggestionIndex((prev) =>
                        prev > 0 ? prev - 1 : usernameSuggestions.length - 1
                      );
                      return;
                    }

                    if (event.key === "Enter" && showUsernameSuggestions) {
                      const selectedSuggestion =
                        activeUsernameSuggestionIndex >= 0
                          ? usernameSuggestions[activeUsernameSuggestionIndex]
                          : usernameSuggestions[0];
                      if (selectedSuggestion) {
                        event.preventDefault();
                        field.onChange(selectedSuggestion);
                        setIsUsernameFocused(false);
                        setActiveUsernameSuggestionIndex(-1);
                      }
                    }
                  }}
                />

                {showUsernameSuggestions && (
                  <div
                    role="listbox"
                    className="bg-popover text-popover-foreground absolute z-50 mt-1 max-h-44 w-full overflow-y-auto rounded-md border shadow-md"
                  >
                    {usernameSuggestions.map((username, index) => (
                      <button
                        key={username}
                        type="button"
                        role="option"
                        data-username-suggestion="true"
                        aria-selected={index === activeUsernameSuggestionIndex}
                        className="hover:bg-accent hover:text-accent-foreground data-[active=true]:bg-accent data-[active=true]:text-accent-foreground block w-full px-3 py-2 text-left text-sm"
                        data-active={index === activeUsernameSuggestionIndex}
                        onMouseDown={(event) => event.preventDefault()}
                        onClick={() => {
                          field.onChange(username);
                          setIsUsernameFocused(false);
                          setActiveUsernameSuggestionIndex(-1);
                        }}
                      >
                        {username}
                      </button>
                    ))}
                  </div>
                )}
              </div>
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
