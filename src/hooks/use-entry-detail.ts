// SPDX-License-Identifier: MIT

import { useCallback, useState } from "react";
import { keepPreviousData, useQuery } from "@tanstack/react-query";
import { queryKeys } from "@/lib/query-keys";
import { entries } from "@/lib/tauri";

export function useEntryDetail(entryId: string | null, dbId: string | null) {
  const [password, setPassword] = useState<string | null>(null);
  const [isPasswordLoading, setIsPasswordLoading] = useState(false);

  // Clear password when entry changes (security requirement).
  // Done during render via the "store previous prop in state" pattern so we
  // don't trigger an extra render via useEffect.
  const [prevEntryId, setPrevEntryId] = useState(entryId);
  if (prevEntryId !== entryId) {
    setPrevEntryId(entryId);
    setPassword(null);
  }

  const {
    data: entry,
    isLoading,
    isError,
    isPlaceholderData,
  } = useQuery({
    queryKey: queryKeys.entries.detail(dbId ?? "none", entryId ?? "none"),
    queryFn: () => entries.get(dbId!, entryId!),
    enabled: Boolean(dbId) && Boolean(entryId),
    staleTime: 30_000,
    placeholderData: keepPreviousData,
  });

  const isTransitioning = Boolean(
    entryId && entry && (isPlaceholderData || entry.id !== entryId)
  );

  const revealPassword = useCallback(async () => {
    if (!dbId || !entryId) return;
    setIsPasswordLoading(true);
    try {
      const pw = await entries.getPassword(dbId, entryId);
      setPassword(pw);
    } finally {
      setIsPasswordLoading(false);
    }
  }, [dbId, entryId]);

  const hidePassword = useCallback(() => {
    setPassword(null);
  }, []);

  return {
    entry: entry ?? null,
    isLoading,
    isError,
    password,
    isPasswordVisible: password !== null,
    isPasswordLoading,
    isTransitioning,
    revealPassword,
    hidePassword,
  };
}
