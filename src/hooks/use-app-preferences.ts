// SPDX-License-Identifier: MIT

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { queryKeys } from "@/lib/query-keys";
import { settings, windowProtection } from "@/lib/tauri";
import type { AppPreferences } from "@/lib/types";

export function useAppPreferences() {
  const queryClient = useQueryClient();

  const preferencesQuery = useQuery<AppPreferences, Error>({
    queryKey: queryKeys.settings.preferences(),
    queryFn: () => settings.getPreferences(),
    staleTime: 30_000,
  });

  const updateMutation = useMutation<void, Error, AppPreferences>({
    mutationFn: async (nextPreferences) => {
      const previous = queryClient.getQueryData<AppPreferences>(
        queryKeys.settings.preferences()
      );
      await settings.updatePreferences(nextPreferences);
      const previousValue = previous?.security.preventScreenCapture;
      const nextValue = nextPreferences.security.preventScreenCapture;
      if (previousValue !== nextValue) {
        await windowProtection.setProtected(nextValue);
      }
    },
    onSuccess: (_data, nextPreferences) => {
      queryClient.setQueryData(
        queryKeys.settings.preferences(),
        nextPreferences
      );
      void queryClient.invalidateQueries({
        queryKey: queryKeys.settings.preferences(),
      });
    },
  });

  const resetMutation = useMutation<AppPreferences, Error, void>({
    mutationFn: async () => {
      const previous = queryClient.getQueryData<AppPreferences>(
        queryKeys.settings.preferences()
      );
      const nextPreferences = await settings.resetPreferences();
      const previousValue = previous?.security.preventScreenCapture;
      const nextValue = nextPreferences.security.preventScreenCapture;
      if (previousValue !== nextValue) {
        await windowProtection.setProtected(nextValue);
      }
      return nextPreferences;
    },
    onSuccess: (nextPreferences) => {
      queryClient.setQueryData(
        queryKeys.settings.preferences(),
        nextPreferences
      );
    },
  });

  return {
    preferences: preferencesQuery.data ?? null,
    isLoading: preferencesQuery.isLoading,
    error: preferencesQuery.error ?? null,
    refetch: preferencesQuery.refetch,
    updatePreferences: updateMutation.mutateAsync,
    isUpdating: updateMutation.isPending,
    resetPreferences: resetMutation.mutateAsync,
    isResetting: resetMutation.isPending,
  };
}
