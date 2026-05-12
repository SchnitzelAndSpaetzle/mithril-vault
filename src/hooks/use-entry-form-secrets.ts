import { useCallback, useEffect, useState } from "react";
import type { UseFormReturn } from "react-hook-form";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { entries as entriesApi } from "@/lib/tauri";
import type { EntryFormValues } from "@/lib/formTypes";
import type { Entry } from "@/lib/types";

interface UseEntryFormSecretsOptions {
  entry: Entry | null | undefined;
  dbId: string;
  form: UseFormReturn<EntryFormValues>;
}

/**
 * Lazy-loads the password and protected custom field values for an entry,
 * writing them into the form without marking fields dirty. Resets loading
 * state when the active (dbId, entryId) pair changes, and exposes a manual
 * retry path for failed loads.
 */
export function useEntryFormSecrets({
  entry,
  dbId,
  form,
}: UseEntryFormSecretsOptions) {
  const { t } = useTranslation();
  const isEditMode = Boolean(entry);
  const entryId = entry?.id ?? null;

  const [isLoadingSecrets, setIsLoadingSecrets] = useState(isEditMode);
  const [secretLoadError, setSecretLoadError] = useState<string | null>(null);
  const [secretReloadToken, setSecretReloadToken] = useState(0);

  // Reset secret-loading state when the active entry changes. Done during
  // render via the "store previous prop in state" pattern so we don't trigger
  // an extra render via useEffect.
  const [prevKey, setPrevKey] = useState({ dbId, entryId });
  if (prevKey.dbId !== dbId || prevKey.entryId !== entryId) {
    setPrevKey({ dbId, entryId });
    setSecretLoadError(null);
    setIsLoadingSecrets(Boolean(entryId));
  }

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

  const retrySecretLoad = useCallback(() => {
    setSecretLoadError(null);
    setIsLoadingSecrets(true);
    setSecretReloadToken((prev) => prev + 1);
  }, []);

  return { isLoadingSecrets, secretLoadError, retrySecretLoad };
}
