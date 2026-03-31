// SPDX-License-Identifier: MIT

import { SHORTCUTS } from "@/lib/shortcuts";
import { useShortcut } from "@/hooks/use-shortcut";

export function useCreateEntryShortcut(callback: () => void, enabled: boolean) {
  useShortcut(SHORTCUTS.newEntry, callback, enabled);
}
