// SPDX-License-Identifier: MIT

import { useCallback } from "react";
import { useNavigate } from "@tanstack/react-router";
import { useIsMobile } from "@/hooks/use-mobile";
import { useActiveDatabase } from "@/hooks/use-active-database";
import { useEntryListKeyboard } from "@/hooks/use-entry-list-keyboard";
import { useDatabaseTabs } from "@/stores/database-tabs";
import type { Virtualizer } from "@tanstack/react-virtual";
import type { Entry } from "@/lib/types";

interface UseEntryListInteractionOptions {
  entries: Entry[];
  onEntrySelect?: ((id: string) => Promise<void> | void) | undefined;
  virtualizer: Virtualizer<HTMLDivElement, Element>;
}

export function useEntryListInteraction({
  entries,
  onEntrySelect,
  virtualizer,
}: UseEntryListInteractionOptions) {
  const { tab } = useActiveDatabase();
  const isMobile = useIsMobile();
  const navigate = useNavigate();
  const updateTabState = useDatabaseTabs((s) => s.updateTabState);

  const selectedEntryId = tab?.selectedEntryId ?? null;

  const handleEntryClick = useCallback(
    async (id: string) => {
      if (onEntrySelect) {
        await onEntrySelect(id);
        return;
      }

      if (tab) {
        updateTabState(tab.id, { selectedEntryId: id });
      }
    },
    [onEntrySelect, tab, updateTabState]
  );

  const handleEntryActivate = useCallback(
    (id: string) => {
      if (isMobile) {
        void navigate({ to: "/dashboard/entry/$id", params: { id } });
      }
    },
    [isMobile, navigate]
  );

  const handleItemClick = useCallback(
    (id: string) => {
      void (async () => {
        await handleEntryClick(id);
        handleEntryActivate(id);
      })();
    },
    [handleEntryClick, handleEntryActivate]
  );

  const scrollToIndex = useCallback(
    (index: number) => {
      virtualizer.scrollToIndex(index, { align: "auto" });
    },
    [virtualizer]
  );

  const { onKeyDown } = useEntryListKeyboard({
    entries,
    selectedEntryId,
    onSelect: (id) => {
      void handleEntryClick(id);
    },
    onActivate: handleEntryActivate,
    scrollToIndex,
  });

  return { selectedEntryId, handleItemClick, onKeyDown };
}
