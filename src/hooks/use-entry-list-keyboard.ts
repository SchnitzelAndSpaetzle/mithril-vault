// SPDX-License-Identifier: MIT

import type React from "react";
import { useCallback } from "react";
import type { Entry } from "@/lib/types";

interface UseEntryListKeyboardOptions {
  entries: Entry[];
  selectedEntryId: string | null;
  onSelect: (id: string) => void;
  onActivate: (id: string) => void;
  scrollToIndex: (index: number) => void;
}

export function useEntryListKeyboard({
  entries,
  selectedEntryId,
  onSelect,
  onActivate,
  scrollToIndex,
}: UseEntryListKeyboardOptions) {
  const onKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (entries.length === 0) return;

      const currentIndex = selectedEntryId
        ? entries.findIndex((entry) => entry.id === selectedEntryId)
        : -1;

      let nextIndex: number;

      switch (e.key) {
        case "ArrowDown":
          e.preventDefault();
          nextIndex =
            currentIndex < entries.length - 1 ? currentIndex + 1 : currentIndex;
          break;
        case "ArrowUp":
          e.preventDefault();
          nextIndex = currentIndex > 0 ? currentIndex - 1 : 0;
          break;
        case "Home":
          e.preventDefault();
          nextIndex = 0;
          break;
        case "End":
          e.preventDefault();
          nextIndex = entries.length - 1;
          break;
        case "Enter":
          if (selectedEntryId && currentIndex !== -1) {
            e.preventDefault();
            onActivate(selectedEntryId);
          }
          return;
        default:
          return;
      }

      const nextEntry = entries[nextIndex];
      if (nextEntry) {
        onSelect(nextEntry.id);
        scrollToIndex(nextIndex);
      }
    },
    [entries, selectedEntryId, onSelect, onActivate, scrollToIndex]
  );

  return { onKeyDown };
}
