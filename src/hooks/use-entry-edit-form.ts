import { useEffect, useRef, useState } from "react";
import { standardSchemaResolver } from "@hookform/resolvers/standard-schema";
import { useQueryClient } from "@tanstack/react-query";
import { ask } from "@tauri-apps/plugin-dialog";
import { type Resolver, useForm, useWatch } from "react-hook-form";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { useAppPreferences } from "@/hooks/use-app-preferences";
import { useEntryMutations } from "@/hooks/use-entry-mutations";
import { useTags } from "@/hooks/use-tags";
import { PASSWORD_GENERATOR_DEFAULTS } from "@/lib/constants";
import { entryFormSchema, type EntryFormValues } from "@/lib/formTypes";
import { queryKeys } from "@/lib/query-keys";
import { database, entries as entriesApi, generator } from "@/lib/tauri";
import type { Entry } from "@/lib/types";

interface UseEntryEditFormOptions {
  entry: Entry | null | undefined;
  dbId: string;
  groupId: string;
  onSave: (entry: Entry) => void;
  onCancel: () => void;
  onDirtyChange?: ((isDirty: boolean) => void) | undefined;
}

function getEntryFormDefaults(
  entry?: Entry | null,
  defaultGroupId?: string
): EntryFormValues {
  if (!entry) {
    return {
      title: "",
      username: "",
      password: "",
      url: "",
      notes: "",
      iconId: 0,
      customIconUuid: null,
      tags: [],
      customFields: [],
      groupId: defaultGroupId,
    };
  }

  return {
    title: entry.title,
    username: entry.username,
    password: "",
    url: entry.url ?? "",
    notes: entry.notes ?? "",
    iconId: entry.iconId ?? 0,
    customIconUuid: entry.customIconUuid ?? null,
    tags: [...entry.tags],
    customFields: entry.customFieldMeta.map((meta) => ({
      key: meta.key,
      value: meta.isProtected ? "" : (entry.customFields[meta.key] ?? ""),
      isProtected: meta.isProtected,
    })),
    groupId: entry.groupId,
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
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const isEditMode = Boolean(entry);
  const entryId = entry?.id ?? null;
  const entryRef = useRef<Entry | null | undefined>(entry);
  const groupIdRef = useRef(groupId);
  const [isLoadingSecrets, setIsLoadingSecrets] = useState(isEditMode);
  const [secretLoadError, setSecretLoadError] = useState<string | null>(null);
  const [secretReloadToken, setSecretReloadToken] = useState(0);
  const [isFetchingFavicon, setIsFetchingFavicon] = useState(false);
  const [isClearingCustomIcon, setIsClearingCustomIcon] = useState(false);

  const { createEntry, updateEntry, moveEntry } = useEntryMutations(dbId);
  const { data: availableTags } = useTags(dbId);
  const { preferences } = useAppPreferences();

  const form = useForm<EntryFormValues>({
    resolver: standardSchemaResolver(
      entryFormSchema
    ) as Resolver<EntryFormValues>,
    defaultValues: getEntryFormDefaults(entry, groupId),
  });

  useEffect(() => {
    entryRef.current = entry;
  }, [entry]);

  useEffect(() => {
    groupIdRef.current = groupId;
  }, [groupId]);

  // Reset state when the active entry changes. React-internal state (error,
  // loading flag) is reset during render via the "store previous prop in
  // state" pattern; the RHF form.reset stays in an effect since it's an
  // external system update.
  const [prevFormKey, setPrevFormKey] = useState({ dbId, entryId });
  if (prevFormKey.dbId !== dbId || prevFormKey.entryId !== entryId) {
    setPrevFormKey({ dbId, entryId });
    setSecretLoadError(null);
    setIsLoadingSecrets(Boolean(entryId));
  }

  useEffect(() => {
    const currentEntry = entryRef.current ?? null;
    form.reset(getEntryFormDefaults(currentEntry, groupIdRef.current));
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
          setSecretLoadError(message);
          toast.error(t("entries.toast.secretLoadFailed"));
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
  }, [entry, dbId, form, secretReloadToken, t]);

  // Auto-generate password when creating a new entry
  useEffect(() => {
    if (!isEditMode) {
      void generateNewPassword();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isEditMode]);

  async function onSubmit(values: EntryFormValues) {
    if (isEditMode && secretLoadError) {
      toast.error(t("entries.toast.secretRetryRequired"));
      return;
    }

    const { customFields, protectedCustomFields } =
      toCustomFieldPayload(values);

    try {
      if (isEditMode && entry) {
        let result = await updateEntry.mutateAsync({
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

        if (form.formState.dirtyFields.customIconUuid) {
          const nextUuid = values.customIconUuid;
          const customIconChanged = nextUuid
            ? await entriesApi.setCustomIcon(dbId, entry.id, nextUuid)
            : await entriesApi.clearCustomIcon(dbId, entry.id);
          if (customIconChanged) {
            await database.save(dbId);
            await refreshFaviconQueries(entry.id);
          }
        }

        if (values.groupId && values.groupId !== entry.groupId) {
          result = await moveEntry.mutateAsync({
            dbId,
            id: entry.id,
            targetGroupId: values.groupId,
          });
        }

        toast.success(t("entries.toast.updated"));
        onSave(result);
        void maybeAutoFetchFavicon(result.id, values.url);
        return;
      }

      const result = await createEntry.mutateAsync({
        dbId,
        groupId: values.groupId ?? groupId,
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
      toast.success(t("entries.toast.created"));
      onSave(result);
      void maybeAutoFetchFavicon(result.id, values.url);
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      toast.error(
        isEditMode
          ? t("entries.toast.updateFailed", { error: message })
          : t("entries.toast.createFailed", { error: message })
      );
    }
  }

  async function handleCancel() {
    if (form.formState.isDirty) {
      const confirmed = await ask(t("entries.unsavedChanges.message"), {
        title: t("entries.unsavedChanges.title"),
        kind: "warning",
      });
      if (!confirmed) {
        return;
      }
    }

    onCancel();
  }

  async function generateNewPassword() {
    try {
      const { password } = await generator.generate(
        PASSWORD_GENERATOR_DEFAULTS
      );
      form.setValue("password", password, { shouldDirty: false });
    } catch {
      // User can still type or use the generator popover
    }
  }

  async function saveAndCreateAnother() {
    const valid = await form.trigger();
    if (!valid) return;

    const values = form.getValues();
    const { customFields, protectedCustomFields } =
      toCustomFieldPayload(values);

    try {
      const result = await createEntry.mutateAsync({
        dbId,
        groupId: values.groupId ?? groupId,
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
      toast.success(t("entries.toast.created"));
      form.reset(getEntryFormDefaults(null, values.groupId ?? groupId));
      void generateNewPassword();
      void maybeAutoFetchFavicon(result.id, values.url);
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      toast.error(t("entries.toast.createFailed", { error: message }));
    }
  }

  function retrySecretLoad() {
    setSecretLoadError(null);
    setIsLoadingSecrets(true);
    setSecretReloadToken((prev) => prev + 1);
  }

  function setGeneratedPassword(password: string) {
    form.setValue("password", password, { shouldDirty: true });
  }

  async function fetchFaviconFromUrl() {
    if (!entryId) return;
    setIsFetchingFavicon(true);
    try {
      const outcome = await entriesApi.fetchFavicon(dbId, entryId, true);
      if (outcome === "updated") {
        await database.save(dbId);
        const refreshed = await entriesApi.get(dbId, entryId);
        form.setValue("customIconUuid", refreshed.customIconUuid ?? null, {
          shouldDirty: false,
        });
        toast.success(t("entries.toast.faviconUpdated"));
        await refreshFaviconQueries(entryId);
      } else if (outcome === "unchanged") {
        toast.success(t("entries.toast.faviconAlreadyUpToDate"));
      } else {
        toast.error(t("entries.toast.faviconNotFound"));
      }
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      toast.error(t("entries.toast.faviconUpdateFailed", { error: message }));
    } finally {
      setIsFetchingFavicon(false);
    }
  }

  async function clearCustomIcon() {
    if (!entryId) return;
    setIsClearingCustomIcon(true);
    try {
      const changed = await entriesApi.clearCustomIcon(dbId, entryId);
      if (changed) {
        await database.save(dbId);
        form.setValue("customIconUuid", null, { shouldDirty: false });
        toast.success(t("entries.toast.customIconCleared"));
        await refreshFaviconQueries(entryId);
      }
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      toast.error(t("entries.toast.customIconClearFailed", { error: message }));
    } finally {
      setIsClearingCustomIcon(false);
    }
  }

  async function maybeAutoFetchFavicon(
    targetEntryId: string,
    urlValue: string
  ) {
    if (!preferences?.security.autoDownloadFavicons) return;
    if (!urlValue.trim()) return;
    try {
      const outcome = await entriesApi.fetchFavicon(dbId, targetEntryId, false);
      if (outcome === "updated") {
        await database.save(dbId);
        await refreshFaviconQueries(targetEntryId);
      }
    } catch {
      // Keep save flow non-blocking for favicon fetch failures.
    }
  }

  async function refreshFaviconQueries(targetEntryId: string) {
    await Promise.all([
      queryClient.invalidateQueries({
        queryKey: queryKeys.database.customIcons(dbId),
      }),
      queryClient.invalidateQueries({
        queryKey: queryKeys.entries.detail(dbId, targetEntryId),
      }),
      queryClient.invalidateQueries({
        predicate: (query) =>
          query.queryKey[0] === queryKeys.entries.all[0] &&
          query.queryKey[1] === dbId,
      }),
    ]);
  }

  const isPending =
    createEntry.isPending || updateEntry.isPending || moveEntry.isPending;
  const isSubmitDisabled =
    isPending || (isEditMode && Boolean(secretLoadError));
  const watchedPassword =
    useWatch({ control: form.control, name: "password" }) ?? "";
  const watchedUsername =
    useWatch({ control: form.control, name: "username" }) ?? "";
  const watchedUrl = useWatch({ control: form.control, name: "url" }) ?? "";
  const isUrlDirty = Boolean(form.formState.dirtyFields.url);

  return {
    form,
    entryId,
    isEditMode,
    isLoadingSecrets,
    secretLoadError,
    isPending,
    isSubmitDisabled,
    availableTags,
    watchedPassword,
    watchedUsername,
    watchedUrl,
    onSubmit,
    handleCancel,
    saveAndCreateAnother,
    retrySecretLoad,
    setGeneratedPassword,
    isFetchingFavicon,
    isClearingCustomIcon,
    hasCustomIcon: Boolean(entry?.customIconUuid),
    canFetchFavicon: Boolean(entryId && watchedUrl.trim() && !isUrlDirty),
    fetchFaviconFromUrl,
    clearCustomIcon,
  };
}
