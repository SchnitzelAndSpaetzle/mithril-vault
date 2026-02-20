import { useEffect } from "react";

/**
 * Registers a Ctrl/Cmd+N keyboard shortcut to trigger entry creation.
 */
export function useCreateEntryShortcut(callback: () => void, enabled: boolean) {
  useEffect(() => {
    if (!enabled) return;

    function handleKeyDown(e: KeyboardEvent) {
      if ((e.ctrlKey || e.metaKey) && e.key === "n") {
        e.preventDefault();
        callback();
      }
    }

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [callback, enabled]);
}
