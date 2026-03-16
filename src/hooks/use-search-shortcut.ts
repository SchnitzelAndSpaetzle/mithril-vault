// SPDX-License-Identifier: MIT

import { useEffect } from "react";
import { isInputTarget, matchesShortcut, SHORTCUTS } from "@/lib/shortcuts";

export function useSearchShortcut(callback: () => void, enabled: boolean) {
  useEffect(() => {
    if (!enabled) return;

    function handleKeyDown(e: KeyboardEvent) {
      if (matchesShortcut(e, SHORTCUTS.search)) {
        e.preventDefault();
        callback();
        return;
      }

      if (e.key === "/") {
        if (isInputTarget(e)) return;
        e.preventDefault();
        callback();
      }
    }

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [callback, enabled]);
}
