// SPDX-License-Identifier: MIT

import { useAppPreferences } from "@/hooks/use-app-preferences";

const FALLBACK_TIMEOUT_SECONDS = 30;

export function useClipboardTimeout(): number {
  const { preferences } = useAppPreferences();

  return (
    preferences?.security.clipboardClearTimeout ?? FALLBACK_TIMEOUT_SECONDS
  );
}
