import { useEffect, useRef, useState } from "react";
import { standardSchemaResolver } from "@hookform/resolvers/standard-schema";
import { ask } from "@tauri-apps/plugin-dialog";
import { type Resolver, useForm } from "react-hook-form";
import { toast } from "sonner";
import { useEntryMutations } from "@/hooks/use-entry-mutations";
import { useTags } from "@/hooks/use-tags";
import { entryFormSchema, type EntryFormValues } from "@/lib/formTypes";
import { entries as entriesApi } from "@/lib/tauri";
import type { Entry } from "@/lib/types";

interface UseEntryEditFormOptions {
  entry: Entry | null | undefined;
  dbId: string;
  groupId: string;
  onSave: (entry: Entry) => void;
  onCancel: () => void;
  onDirtyChange?: ((isDirty: boolean) => void) | undefined;
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

function toCustomFieldPayload(values: EntryFormValues) {
  const customFields: Record<string, string> = {};
  const protectedCustomFields: Record<string, string> = {};

  for (const cf of values.customFields) {
    if (cf.isProtected) {
      protectedCustomFields[cf.key] = cf.value;
    } else {
      customFields[cf.key] = cf.value;
    }
  }

  return { customFields, protectedCustomFields };
}

export function useEntryEditForm({
  entry,
  dbId,
  groupId,
  onSave,
  onCancel,
  onDirtyChange,
}: UseEntryEditFormOptions) {
  const isEditMode = Boolean(entry);
  const entryId = entry?.id ?? null;
  const entryRef = useRef<Entry | null | undefined>(entry);
  const [isLoadingSecrets, setIsLoadingSecrets] = useState(isEditMode);
  const [secretLoadError, setSecretLoadError] = useState<string | null>(null);
  const [secretReloadToken, setSecretReloadToken] = useState(0);

  const { createEntry, updateEntry } = useEntryMutations(dbId);
  const { data: availableTags } = useTags(dbId);

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
    setSecretLoadError(null);
    setIsLoadingSecrets(Boolean(entryId));
  }, [dbId, entryId, form]);

  useEffect(() => {
    onDirtyChange?.(form.formState.isDirty);
  }, [form.formState.isDirty, onDirtyChange]);

  useEffect(() => {
    const currentEntry = entry;
    if (!currentEntry) return;
    let cancelled = false;

    async function loadSecrets(entryForLoad: Entry) {
      try {
        const [password, ...protectedValues] = await Promise.all([
          entriesApi.getPassword(dbId, entryForLoad.id),
          ...entryForLoad.customFieldMeta
            .filter((m) => m.isProtected)
            .map((m) =>
              entriesApi
                .getProtectedCustomField(dbId, entryForLoad.id, m.key)
                .then((r) => ({ key: m.key, value: r.value }))
            ),
        ]);

        if (cancelled) return;

        form.setValue("password", password, { shouldDirty: false });
        setSecretLoadError(null);

        for (const pv of protectedValues) {
          const fieldIndex = entryForLoad.customFieldMeta.findIndex(
            (m) => m.key === pv.key
          );
          if (fieldIndex !== -1) {
            form.setValue(`customFields.${fieldIndex}.value`, pv.value, {
              shouldDirty: false,
            });
          }
        }
      } catch (error) {
        if (!cancelled) {
          const message =
            error instanceof Error ? error.message : String(error);
          setSecretLoadError(`Failed to load protected values: ${message}`);
          toast.error("Failed to load protected values for this entry.");
        }
      } finally {
        if (!cancelled) {
          setIsLoadingSecrets(false);
        }
      }
    }

    void loadSecrets(currentEntry);

    return () => {
      cancelled = true;
    };
  }, [entry, entry?.id, dbId, form, secretReloadToken]);

  async function onSubmit(values: EntryFormValues) {
    if (isEditMode && secretLoadError) {
      toast.error("Retry loading protected values before saving.");
      return;
    }

    const { customFields, protectedCustomFields } =
      toCustomFieldPayload(values);

    try {
      if (isEditMode && entry) {
        const result = await updateEntry.mutateAsync({
          dbId,
          id: entry.id,
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
        return;
      }

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
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      toast.error(
        `Failed to ${isEditMode ? "update" : "create"} entry: ${message}`
      );
    }
  }

  async function handleCancel() {
    if (form.formState.isDirty) {
      const confirmed = await ask(
        "You have unsaved changes. Are you sure you want to discard them?",
        { title: "Unsaved Changes", kind: "warning" }
      );
      if (!confirmed) {
        return;
      }
    }

    onCancel();
  }

  function retrySecretLoad() {
    setSecretLoadError(null);
    setIsLoadingSecrets(true);
    setSecretReloadToken((prev) => prev + 1);
  }

  function setGeneratedPassword(password: string) {
    form.setValue("password", password, { shouldDirty: true });
  }

  const isPending = createEntry.isPending || updateEntry.isPending;
  const isSubmitDisabled =
    isPending || (isEditMode && Boolean(secretLoadError));

  return {
    form,
    entryId,
    isEditMode,
    isLoadingSecrets,
    secretLoadError,
    isPending,
    isSubmitDisabled,
    availableTags,
    watchedPassword: form.watch("password"),
    watchedUsername: form.watch("username"),
    onSubmit,
    handleCancel,
    retrySecretLoad,
    setGeneratedPassword,
  };
}
