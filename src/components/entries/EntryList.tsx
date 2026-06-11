import { useTranslation } from "react-i18next";
import EntryListItem from "@/components/entries/EntryListItem";
import { ItemSeparator } from "@/components/ui/item";
import { useActiveDatabase } from "@/hooks/use-active-database";
import { useCustomIcons } from "@/hooks/use-custom-icons";
import { useEntries } from "@/hooks/use-entries";
import { usePasswordHealthReport } from "@/hooks/use-password-health";
import { findingsForEntry } from "@/lib/password-health";
import { useEntryListInteraction } from "@/hooks/use-entry-list-interaction";
import { useSortedEntries } from "@/hooks/use-sorted-entries";
import { useSearch } from "@tanstack/react-router";
import { ScrollArea } from "@/components/ui/scroll-area";
import { useMemo, useRef } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import type { EntrySortField, SortOrder } from "@/lib/types";
import { filterEntries } from "@/lib/entry-filters";

const EMPTY_ICONS = {};
const ESTIMATED_ITEM_HEIGHT = 65;

interface EntryListProps {
  onEntrySelect?: (id: string) => Promise<void> | void;
}

export default function EntryList({ onEntrySelect }: Readonly<EntryListProps>) {
  const { t } = useTranslation();
  const { dbId } = useActiveDatabase();
  const search = useSearch({ strict: false });
  const { data: customIcons } = useCustomIcons(dbId);
  const { data: healthReport } = usePasswordHealthReport(dbId ?? null);
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

  // Pre-index reuse-group sizes per Entry id so the per-row tooltip
  // can stitch in "Reused (N entries)" without iterating the
  // `reuseGroups` array for every row in the virtualized list.
  const reusedGroupSizeByEntryId = useMemo(() => {
    const map = new Map<string, number>();
    if (!healthReport) return map;
    for (const group of healthReport.reuseGroups) {
      for (const id of group.entryIds) {
        map.set(id, group.entryIds.length);
      }
    }
    return map;
  }, [healthReport]);

  const tagFilter = search.tag;
  const attachmentsFilter = search.hasAttachments === true;
  // Filters stack on top of the group-scoped entries from `useEntries`.
  const displayEntries = useMemo(
    () =>
      filterEntries(sortedEntries, {
        tag: tagFilter,
        hasAttachments: attachmentsFilter,
      }),
    [sortedEntries, tagFilter, attachmentsFilter]
  );

  // eslint-disable-next-line react-hooks/incompatible-library -- virtualizer is not passed to memoized components
  const virtualizer = useVirtualizer({
    count: displayEntries.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => ESTIMATED_ITEM_HEIGHT,
    measureElement: (el) => el.getBoundingClientRect().height,
    overscan: 10,
  });

  const { selectedEntryId, handleItemClick, onKeyDown } =
    useEntryListInteraction({
      entries: displayEntries,
      onEntrySelect,
      virtualizer,
    });

  if (!dbId) {
    return (
      <div className="px-3 py-2 text-sm text-muted-foreground">
        {t("entries.openDatabase")}
      </div>
    );
  }

  if (isLoading) {
    return (
      <div className="px-3 py-2 text-sm text-muted-foreground">
        {t("entries.loading")}
      </div>
    );
  }

  if (isError) {
    return (
      <div className="px-3 py-2 text-sm text-destructive">
        {t("entries.loadError", { error: error.message })}
      </div>
    );
  }

  if (displayEntries.length === 0) {
    let noEntriesMessage;

    if (tagFilter) {
      noEntriesMessage = t("entries.noEntriesWithTag", { tag: tagFilter });
    } else if (attachmentsFilter) {
      noEntriesMessage = t("entries.noEntriesWithAttachments");
    } else {
      noEntriesMessage = t("entries.noEntries");
    }

    return (
      <div className="px-3 py-2 text-sm text-muted-foreground">
        {noEntriesMessage}
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
          const entry = displayEntries[virtualItem.index];
          if (!entry) return null;
          return (
            <div
              key={entry.id}
              ref={virtualizer.measureElement}
              data-index={virtualItem.index}
              data-entry-id={entry.id}
              className="absolute left-0 w-full"
              style={{
                transform: `translateY(${virtualItem.start}px)`,
              }}
            >
              <EntryListItem
                {...entry}
                customIcons={customIcons ?? EMPTY_ICONS}
                isSelected={entry.id === selectedEntryId}
                onClick={handleItemClick}
                findings={findingsForEntry(healthReport, entry.id)}
                reusedGroupSize={reusedGroupSizeByEntryId.get(entry.id)}
              />
              <ItemSeparator />
            </div>
          );
        })}
      </div>
    </ScrollArea>
  );
}
