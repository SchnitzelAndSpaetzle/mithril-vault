import { useCallback, useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { type UseFormReturn, useWatch } from "react-hook-form";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { useAppPreferences } from "@/hooks/use-app-preferences";
import { queryKeys } from "@/lib/query-keys";
import { database, entries as entriesApi } from "@/lib/tauri";
import type { EntryFormValues } from "@/lib/formTypes";

interface UseEntryFormIconActionsOptions {
  entryId: string | null;
  dbId: string;
  form: UseFormReturn<EntryFormValues>;
}

/**
 * Owns the custom-icon and favicon side effects of the entry edit form:
 * manual favicon refetch, custom-icon clear, the submit-time icon-assignment
 * helper, and the auto-favicon-on-save hook. Surfaces the derived
 * `hasCustomIcon` and `canFetchFavicon` flags used by the title field.
 */
export function useEntryFormIconActions({
  entryId,
  dbId,
  form,
}: UseEntryFormIconActionsOptions) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const { preferences } = useAppPreferences();

  const [isFetchingFavicon, setIsFetchingFavicon] = useState(false);
  const [isClearingCustomIcon, setIsClearingCustomIcon] = useState(false);

  const watchedCustomIconUuid =
    useWatch({ control: form.control, name: "customIconUuid" }) ?? null;
  const watchedUrl = useWatch({ control: form.control, name: "url" }) ?? "";
  const isUrlDirty = Boolean(form.formState.dirtyFields.url);

  const refreshFaviconQueries = useCallback(
    async (targetEntryId: string) => {
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
    },
    [dbId, queryClient]
  );

  const applyCustomIconChange = useCallback(
    async (targetEntryId: string, nextUuid: string | null) => {
      try {
        const changed = nextUuid
          ? await entriesApi.setCustomIcon(dbId, targetEntryId, nextUuid)
          : await entriesApi.clearCustomIcon(dbId, targetEntryId);
        if (changed) {
          await database.save(dbId);
          await refreshFaviconQueries(targetEntryId);
        }
      } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        toast.error(
          t("entries.toast.customIconAssignFailed", { error: message })
        );
      }
    },
    [dbId, refreshFaviconQueries, t]
  );

  const fetchFaviconFromUrl = useCallback(async () => {
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
  }, [dbId, entryId, form, refreshFaviconQueries, t]);

  const clearCustomIcon = useCallback(async () => {
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
  }, [dbId, entryId, form, refreshFaviconQueries, t]);

  const maybeAutoFetchFavicon = useCallback(
    async (targetEntryId: string, urlValue: string) => {
      if (!preferences?.security.autoDownloadFavicons) return;
      if (!urlValue.trim()) return;
      try {
        const outcome = await entriesApi.fetchFavicon(
          dbId,
          targetEntryId,
          false
        );
        if (outcome === "updated") {
          await database.save(dbId);
          await refreshFaviconQueries(targetEntryId);
        }
      } catch {
        // Keep save flow non-blocking for favicon fetch failures.
      }
    },
    [dbId, preferences?.security.autoDownloadFavicons, refreshFaviconQueries]
  );

  return {
    isFetchingFavicon,
    isClearingCustomIcon,
    hasCustomIcon: Boolean(watchedCustomIconUuid),
    canFetchFavicon: Boolean(entryId && watchedUrl.trim() && !isUrlDirty),
    fetchFaviconFromUrl,
    clearCustomIcon,
    applyCustomIconChange,
    maybeAutoFetchFavicon,
  };
}
