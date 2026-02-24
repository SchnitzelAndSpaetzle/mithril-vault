// SPDX-License-Identifier: MIT

import { useEffect } from "react";

/**
 * Registers Ctrl/Cmd+K and "/" keyboard shortcuts to focus the search input.
 * "/" only triggers when no input/textarea is focused.
 */
export function useSearchShortcut(callback: () => void, enabled: boolean) {
  useEffect(() => {
    if (!enabled) return;

    function handleKeyDown(e: KeyboardEvent) {
      if ((e.ctrlKey || e.metaKey) && e.key === "k") {
        e.preventDefault();
        callback();
        return;
      }

      if (e.key === "/") {
        const tag = (e.target as HTMLElement)?.tagName;
        if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT") return;
        if ((e.target as HTMLElement)?.isContentEditable) return;
        e.preventDefault();
        callback();
      }
    }

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [callback, enabled]);
}
