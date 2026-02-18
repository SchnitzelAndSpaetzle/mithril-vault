import EntryListItem from "@/components/entries/EntryListItem";
import { ItemSeparator } from "@/components/ui/item";
import { useActiveDatabase } from "@/hooks/use-active-database";
import { useCustomIcons } from "@/hooks/use-custom-icons";
import { useEntries } from "@/hooks/use-entries";
import { useEntryListKeyboard } from "@/hooks/use-entry-list-keyboard";
import { useSortedEntries } from "@/hooks/use-sorted-entries";
import { useDatabaseTabs } from "@/stores/database-tabs";
import { useNavigate, useSearch } from "@tanstack/react-router";
import { ScrollArea } from "@/components/ui/scroll-area";
import { useIsMobile } from "@/hooks/use-mobile";
import { useCallback, useRef } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import type { EntrySortField, SortOrder } from "@/lib/types";

const EMPTY_ICONS: Record<string, string> = {};
const ITEM_HEIGHT = 57;

export default function EntryList() {
  const { dbId, tab } = useActiveDatabase();
  const search = useSearch({ from: "/dashboard/index/$dbId" });
  const { data: customIcons } = useCustomIcons(dbId);
  const isMobile = useIsMobile();
  const navigate = useNavigate();
  const updateTabState = useDatabaseTabs((s) => s.updateTabState);
  const scrollRef = useRef<HTMLDivElement | null>(null);

  const sortBy: EntrySortField = search.sortBy ?? "title";
  const sortOrder: SortOrder = search.sortOrder ?? "asc";

  const {
    data: entries,
    isLoading,
    isError,
    error,
  } = useEntries(dbId, search.groupId);

  const sortedEntries = useSortedEntries(entries, sortBy, sortOrder);

  const virtualizer = useVirtualizer({
    count: sortedEntries.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => ITEM_HEIGHT,
    overscan: 10,
  });

  const selectedEntryId = tab?.selectedEntryId ?? null;

  const handleEntryClick = useCallback(
    (id: string) => {
      if (tab) {
        updateTabState(tab.id, { selectedEntryId: id });
      }
    },
    [tab, updateTabState]
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
      handleEntryClick(id);
      handleEntryActivate(id);
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
    entries: sortedEntries,
    selectedEntryId,
    onSelect: handleEntryClick,
    onActivate: handleEntryActivate,
    scrollToIndex,
  });

  if (!dbId) {
    return (
      <div className="px-3 py-2 text-sm text-muted-foreground">
        Open a database to view entries.
      </div>
    );
  }

  if (isLoading) {
    return (
      <div className="px-3 py-2 text-sm text-muted-foreground">
        Loading entries...
      </div>
    );
  }

  if (isError) {
    return (
      <div className="px-3 py-2 text-sm text-destructive">
        Failed to load entries: {error.message}
      </div>
    );
  }

  if (sortedEntries.length === 0) {
    return (
      <div className="px-3 py-2 text-sm text-muted-foreground">
        No entries found.
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
          const entry = sortedEntries[virtualItem.index];
          if (!entry) return null;
          return (
            <div
              key={entry.id}
              data-entry-id={entry.id}
              className="absolute left-0 w-full"
              style={{
                height: `${virtualItem.size}px`,
                transform: `translateY(${virtualItem.start}px)`,
              }}
            >
              <EntryListItem
                {...entry}
                customIcons={customIcons ?? EMPTY_ICONS}
                isSelected={entry.id === selectedEntryId}
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
