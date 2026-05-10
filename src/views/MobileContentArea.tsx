import { useCallback } from "react";
import { useTranslation } from "react-i18next";
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
import { useShortcut } from "@/hooks/use-shortcut";
import { useEntryListHeader } from "@/hooks/use-entry-list-header";
import { useActiveDatabase } from "@/hooks/use-active-database";
import { useAppPreferences } from "@/hooks/use-app-preferences";
import { useDatabaseTabs } from "@/stores/database-tabs";
import { useNavigate, useSearch } from "@tanstack/react-router";
import { toast } from "sonner";
import { SHORTCUTS } from "@/lib/shortcuts";
import { clipboard, database } from "@/lib/tauri";

const MOBILE_SEARCH_INPUT_ID = "mobile-global-search-input";

export default function MobileContentArea() {
  const { t } = useTranslation();
  const { groupName, entryCount, activeTag } = useEntryListHeader();
  const navigate = useNavigate();
  const { tab, dbId } = useActiveDatabase();
  const updateTabInfo = useDatabaseTabs((s) => s.updateTabInfo);
  const { preferences } = useAppPreferences();
  const clearClipboardOnLock = Boolean(
    preferences?.security.clearClipboardOnLock
  );

  const search = useSearch({ strict: false });
  const searchGroupId = (search.groupId as string | undefined) ?? null;
  const searchTag = (search.tag as string | undefined) ?? null;

  const searchState = useSearchEntries(dbId, searchGroupId, searchTag);

  const openCreateEntry = useCallback(
    () => void navigate({ to: "/dashboard/entry/new" }),
    [navigate]
  );
  useCreateEntryShortcut(openCreateEntry, true);

  const focusSearchInput = useCallback(() => {
    const input = document.getElementById(MOBILE_SEARCH_INPUT_ID);
    if (input instanceof HTMLInputElement) {
      input.focus();
    }
  }, []);
  useSearchShortcut(focusSearchInput, Boolean(dbId));

  const handleSearchEscape = useCallback(() => {
    searchState.clearSearch();
  }, [searchState]);

  useShortcut(
    SHORTCUTS.save,
    useCallback(() => {
      if (!dbId) return;
      void database.save(dbId).then(() => {
        toast.success(t("shortcuts.toast.saved"));
      });
    }, [dbId, t]),
    Boolean(dbId)
  );

  useShortcut(
    SHORTCUTS.lockDatabase,
    useCallback(() => {
      if (!tab?.id || !dbId) return;
      void (async () => {
        try {
          if (clearClipboardOnLock) {
            try {
              await clipboard.clear();
            } catch (error) {
              console.error("Failed to clear clipboard before lock:", error);
            }
          }
          const info = await database.lock(dbId);
          updateTabInfo(tab.id, info);
          void navigate({ to: "/unlock", search: { path: dbId } });
        } catch {
          // lock failed silently
        }
      })();
    }, [tab, dbId, updateTabInfo, navigate, clearClipboardOnLock]),
    Boolean(dbId)
  );

  useShortcut(
    SHORTCUTS.settings,
    useCallback(() => {
      void navigate({ to: "/settings" });
    }, [navigate]),
    Boolean(dbId)
  );

  return (
    <div className="flex h-full w-full min-w-0 flex-col">
      <NavEntries>
        <div className="flex flex-col">
          {searchState.isSearchActive ? (
            <>
              <p className="text-sm">
                {activeTag
                  ? t("entries.search.searchInTag", { tag: activeTag })
                  : groupName !== "All"
                    ? t("entries.search.searchInGroup", { group: groupName })
                    : t("entries.search.searchResults")}
              </p>
              <small className="text-muted-foreground text-xs">
                {t("entries.search.match", {
                  count: searchState.results.length,
                })}
              </small>
            </>
          ) : (
            <>
              <p className="text-sm">{groupName}</p>
              <small className="text-muted-foreground text-xs">
                {t("entries.count", { count: entryCount })}
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
            inputId={MOBILE_SEARCH_INPUT_ID}
            autoFocus
          />
          {!searchState.isSearchActive && <SortDropdown />}
        </div>
      </div>
    </div>
  );
}
