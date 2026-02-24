// SPDX-License-Identifier: MIT

import { useCallback, useMemo, useRef } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { useNavigate } from "@tanstack/react-router";
import { ScrollArea } from "@/components/ui/scroll-area";
import { ItemSeparator } from "@/components/ui/item";
import { useActiveDatabase } from "@/hooks/use-active-database";
import { useGroups } from "@/hooks/use-groups";
import { useCustomIcons } from "@/hooks/use-custom-icons";
import { useEntryListKeyboard } from "@/hooks/use-entry-list-keyboard";
import { useIsMobile } from "@/hooks/use-mobile";
import { useDatabaseTabs } from "@/stores/database-tabs";
import { buildGroupPathMap, type SearchResult } from "@/lib/search-utils";
import SearchResultItem from "@/components/search/SearchResultItem";
import { SearchX } from "lucide-react";

const EMPTY_ICONS: Record<string, string> = {};
const ESTIMATED_ITEM_HEIGHT = 75;

interface SearchResultsListProps {
  results: SearchResult[];
  query: string;
  onEntrySelect?: (id: string) => Promise<void> | void;
}

export default function SearchResultsList({
  results,
  query,
  onEntrySelect,
}: SearchResultsListProps) {
  const { dbId, tab } = useActiveDatabase();
  const { data: groups } = useGroups(dbId);
  const { data: customIcons } = useCustomIcons(dbId);
  const isMobile = useIsMobile();
  const navigate = useNavigate();
  const updateTabState = useDatabaseTabs((s) => s.updateTabState);
  const scrollRef = useRef<HTMLDivElement | null>(null);

  const groupPathMap = useMemo(() => buildGroupPathMap(groups ?? []), [groups]);

  const entries = useMemo(() => results.map((r) => r.entry), [results]);

  // eslint-disable-next-line react-hooks/incompatible-library -- virtualizer is not passed to memoized components
  const virtualizer = useVirtualizer({
    count: results.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => ESTIMATED_ITEM_HEIGHT,
    measureElement: (el) => el.getBoundingClientRect().height,
    overscan: 10,
  });

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

  if (results.length === 0) {
    return (
      <div className="flex flex-col items-center gap-2 px-3 py-8 text-sm text-muted-foreground">
        <SearchX className="size-8 opacity-50" />
        <p>No entries match your search.</p>
      </div>
    );
  }

  return (
    <ScrollArea
      viewportRef={scrollRef}
      className="min-h-0 flex-1 overflow-hidden focus:outline-none"
      role="listbox"
      tabIndex={0}
      onKeyDown={onKeyDown}
    >
      <div
        className="relative w-full"
        style={{ height: `${virtualizer.getTotalSize()}px` }}
      >
        {virtualizer.getVirtualItems().map((virtualItem) => {
          const result = results[virtualItem.index];
          if (!result) return null;
          return (
            <div
              key={result.entry.id}
              ref={virtualizer.measureElement}
              data-index={virtualItem.index}
              data-entry-id={result.entry.id}
              className="absolute left-0 w-full"
              style={{
                transform: `translateY(${virtualItem.start}px)`,
              }}
            >
              <SearchResultItem
                result={result}
                query={query}
                groupPath={groupPathMap.get(result.entry.groupId) ?? ""}
                customIcons={customIcons ?? EMPTY_ICONS}
                isSelected={result.entry.id === selectedEntryId}
                onClick={handleItemClick}
              />
              <ItemSeparator />
            </div>
          );
        })}
      </div>
    </ScrollArea>
  );
}
