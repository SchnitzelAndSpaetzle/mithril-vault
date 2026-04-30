// SPDX-License-Identifier: MIT

import { useQuery } from "@tanstack/react-query";
import { useAppPreferences } from "@/hooks/use-app-preferences";
import { windowProtection } from "@/lib/tauri";

export interface WindowProtectionState {
  enabled: boolean;
  isSupported: boolean;
}

export function useWindowProtection(): WindowProtectionState {
  const { preferences } = useAppPreferences();
  const supportedQuery = useQuery<boolean, Error>({
    queryKey: ["windowProtection", "supported"],
    queryFn: () => windowProtection.isSupported(),
    staleTime: Infinity,
  });

  return {
    enabled: preferences?.security.preventScreenCapture ?? false,
    isSupported: supportedQuery.data ?? false,
  };
}
