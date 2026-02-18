// SPDX-License-Identifier: MIT

import { useCallback, useEffect, useState } from "react";
import { keepPreviousData, useQuery } from "@tanstack/react-query";
import { queryKeys } from "@/lib/query-keys";
import { entries } from "@/lib/tauri";

export function useEntryDetail(entryId: string | null, dbId: string | null) {
  const [password, setPassword] = useState<string | null>(null);
  const [isPasswordLoading, setIsPasswordLoading] = useState(false);

  const {
    data: entry,
    isLoading,
    isError,
  } = useQuery({
    queryKey: queryKeys.entries.detail(dbId ?? "none", entryId ?? "none"),
    queryFn: () => entries.get(dbId!, entryId!),
    enabled: Boolean(dbId) && Boolean(entryId),
    staleTime: 30_000,
    placeholderData: keepPreviousData,
  });

  // Clear password when entry changes (security requirement)
  useEffect(() => {
    setPassword(null);
  }, [entryId]);

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
    revealPassword,
    hidePassword,
  };
}
