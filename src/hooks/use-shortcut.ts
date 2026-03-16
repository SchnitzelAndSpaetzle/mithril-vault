// SPDX-License-Identifier: MIT

import { useEffect } from "react";
import {
  isInputTarget,
  matchesShortcut,
  type ShortcutDef,
} from "@/lib/shortcuts";

export function useShortcut(
  def: ShortcutDef,
  callback: () => void,
  enabled: boolean
) {
  useEffect(() => {
    if (!enabled) return;

    function handleKeyDown(e: KeyboardEvent) {
      if (!def.ctrlOrMeta && isInputTarget(e)) return;
      if (!matchesShortcut(e, def)) return;
      e.preventDefault();
      callback();
    }

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [def, callback, enabled]);
}
