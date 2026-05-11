// SPDX-License-Identifier: MIT

import { useMemo, useRef } from "react";
import { useTranslation } from "react-i18next";
import { useVirtualizer } from "@tanstack/react-virtual";
import { ScrollArea } from "@/components/ui/scroll-area";
import { ItemSeparator } from "@/components/ui/item";
import { useActiveDatabase } from "@/hooks/use-active-database";
import { useGroups } from "@/hooks/use-groups";
import { useCustomIcons } from "@/hooks/use-custom-icons";
import { useEntryListInteraction } from "@/hooks/use-entry-list-interaction";
import { buildGroupPathMap, type SearchResult } from "@/lib/search-utils";
import SearchResultItem from "@/components/search/SearchResultItem";
import { SearchX } from "lucide-react";

const EMPTY_ICONS = {};
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
  const { t } = useTranslation();
  const { dbId } = useActiveDatabase();
  const { data: groups } = useGroups(dbId);
  const { data: customIcons } = useCustomIcons(dbId);
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

  const { selectedEntryId, handleItemClick, onKeyDown } =
    useEntryListInteraction({
      entries,
      onEntrySelect,
      virtualizer,
    });

  if (results.length === 0) {
    return (
      <div className="flex flex-col items-center gap-2 px-3 py-8 text-sm text-muted-foreground">
        <SearchX className="size-8 opacity-50" />
        <p>{t("entries.search.noResults")}</p>
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
