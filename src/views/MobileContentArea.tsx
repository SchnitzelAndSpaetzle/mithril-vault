import { useCallback, useRef } from "react";
import EntryList from "@/components/entries/EntryList.tsx";
import NavEntries from "@/components/entries/nav-entries.tsx";
import { SearchForm } from "@/components/search-form.tsx";
import SearchResultsList from "@/components/search/SearchResultsList.tsx";
import { Button } from "@/components/ui/button.tsx";
import { Plus } from "lucide-react";
import SortDropdown from "@/components/entries/sort-dropdown";
import { useCreateEntryShortcut } from "@/hooks/use-create-entry-shortcut";
import { useSearchEntries } from "@/hooks/use-search-entries";
import { useSearchShortcut } from "@/hooks/use-search-shortcut";
import { useEntryListHeader } from "@/hooks/use-entry-list-header";
import { useActiveDatabase } from "@/hooks/use-active-database";
import { useNavigate, useSearch } from "@tanstack/react-router";

export default function MobileContentArea() {
  const { groupName, entryCount, activeTag } = useEntryListHeader();
  const navigate = useNavigate();
  const { dbId } = useActiveDatabase();

  const search = useSearch({ strict: false });
  const searchGroupId = (search.groupId as string | undefined) ?? null;
  const searchTag = (search.tag as string | undefined) ?? null;

  const searchState = useSearchEntries(dbId, searchGroupId, searchTag);
  const searchInputRef = useRef<HTMLInputElement>(null);

  const openCreateEntry = useCallback(
    () => void navigate({ to: "/dashboard/entry/new" }),
    [navigate]
  );
  useCreateEntryShortcut(openCreateEntry, true);

  const focusSearchInput = useCallback(() => {
    searchInputRef.current?.focus();
  }, []);
  useSearchShortcut(focusSearchInput, Boolean(dbId));

  const handleSearchEscape = useCallback(() => {
    searchState.clearSearch();
    searchInputRef.current?.blur();
  }, [searchState]);

  return (
    <div className="flex h-full w-full min-w-0 flex-col">
      <NavEntries>
        <div className="flex flex-col">
          {searchState.isSearchActive ? (
            <>
              <p className="text-sm">
                {activeTag
                  ? `Search in #${activeTag}`
                  : groupName !== "All"
                    ? `Search in ${groupName}`
                    : "Search Results"}
              </p>
              <small className="text-muted-foreground text-xs">
                {searchState.results.length}{" "}
                {searchState.results.length === 1 ? "match" : "matches"}
              </small>
            </>
          ) : (
            <>
              <p className="text-sm">{groupName}</p>
              <small className="text-muted-foreground text-xs">
                {entryCount} {entryCount === 1 ? "Item" : "Items"}
              </small>
            </>
          )}
        </div>
      </NavEntries>
      <div className="flex min-h-0 flex-1 flex-col">
        {searchState.isSearchActive ? (
          <SearchResultsList
            results={searchState.results}
            query={searchState.query}
          />
        ) : (
          <EntryList />
        )}
      </div>
      <div className="shrink-0 border-t backdrop-blur-2xl">
        <div className="flex items-center gap-2 p-4">
          <Button
            variant="outline"
            size="icon-sm"
            onClick={() => void navigate({ to: "/dashboard/entry/new" })}
          >
            <Plus />
          </Button>
          <SearchForm
            className="w-full"
            query={searchState.query}
            onQueryChange={searchState.setQuery}
            onClear={searchState.clearSearch}
            onEscape={handleSearchEscape}
            inputRef={searchInputRef}
            autoFocus
          />
          {!searchState.isSearchActive && <SortDropdown />}
        </div>
      </div>
    </div>
  );
}
