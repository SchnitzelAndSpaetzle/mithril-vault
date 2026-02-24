import { useCallback, useRef, useState } from "react";
import { useNavigate, useSearch } from "@tanstack/react-router";
import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
} from "@/components/ui/resizable";
import { useCreateEntryShortcut } from "@/hooks/use-create-entry-shortcut";
import { useSearchEntries } from "@/hooks/use-search-entries";
import { useSearchShortcut } from "@/hooks/use-search-shortcut";
import EntryList from "@/components/entries/EntryList.tsx";
import EntryItemDetails from "@/components/entries/EntryItemDetails.tsx";
import { EntryItemDetailsEmpty } from "@/components/entries/EntryItemDetailsEmpty.tsx";
import { EntryEditForm } from "@/components/entries/EntryEditForm.tsx";
import { EntryActions } from "@/components/entries/EntryActions.tsx";
import { SearchForm } from "@/components/search-form.tsx";
import SearchResultsList from "@/components/search/SearchResultsList.tsx";
import { SidebarTrigger } from "@/components/ui/sidebar.tsx";
import { Separator } from "@/components/ui/separator.tsx";
import { Dices, EllipsisVertical, Loader2, Share, X } from "lucide-react";
import { Button } from "@/components/ui/button.tsx";
import SortDropdown from "@/components/entries/sort-dropdown";
import { useEntryListHeader } from "@/hooks/use-entry-list-header";
import { useEntryDetail } from "@/hooks/use-entry-detail";
import { useActiveDatabase } from "@/hooks/use-active-database";
import { useEntryMutations } from "@/hooks/use-entry-mutations";
import { useDatabaseTabs } from "@/stores/database-tabs";
import { ask } from "@tauri-apps/plugin-dialog";
import { toast } from "sonner";
import { Badge } from "@/components/ui/badge";
import type { Entry } from "@/lib/types";

type EditMode = "view" | "edit" | "create";

export default function DragRegion() {
  const { groupName, entryCount, activeTag } = useEntryListHeader();
  const navigate = useNavigate();
  const { tab, dbId } = useActiveDatabase();
  const selectedEntryId = tab?.selectedEntryId ?? null;
  const [editMode, setEditMode] = useState<EditMode>("view");
  const [hasUnsavedChanges, setHasUnsavedChanges] = useState(false);

  const selectedGroupId = tab?.selectedGroupId ?? null;
  const rootGroupId = tab?.info?.rootGroupId ?? "";
  const groupId = selectedGroupId ?? rootGroupId;

  const search = useSearch({ strict: false });
  const searchGroupId = (search.groupId as string | undefined) ?? null;
  const searchTag = (search.tag as string | undefined) ?? null;

  const searchState = useSearchEntries(dbId, searchGroupId, searchTag);
  const searchInputRef = useRef<HTMLInputElement>(null);

  const {
    entry: editEntry,
    isLoading: isEditEntryLoading,
    isTransitioning: isEditEntryTransitioning,
  } = useEntryDetail(
    editMode === "edit" && selectedEntryId ? selectedEntryId : "",
    editMode === "edit" && dbId ? dbId : ""
  );

  const updateTabState = useDatabaseTabs((s) => s.updateTabState);
  const { deleteEntry } = useEntryMutations(dbId ?? null);

  const isEditing = editMode !== "view";

  function handleSave(saved: Entry) {
    setEditMode("view");
    setHasUnsavedChanges(false);
    if (tab) {
      updateTabState(tab.id, { selectedEntryId: saved.id });
    }
  }

  function handleCancel() {
    setEditMode("view");
    setHasUnsavedChanges(false);
  }

  const handleEntrySelect = async (id: string) => {
    if (!tab) {
      return;
    }

    if (id === selectedEntryId) {
      return;
    }

    if (isEditing && hasUnsavedChanges) {
      const confirmed = await ask(
        "You have unsaved changes. Do you want to discard them and open another entry?",
        { title: "Unsaved Changes", kind: "warning" }
      );
      if (!confirmed) {
        return;
      }
    }

    if (isEditing) {
      setEditMode("view");
      setHasUnsavedChanges(false);
    }

    updateTabState(tab.id, { selectedEntryId: id });
  };

  const handleDelete = async () => {
    if (!dbId || !selectedEntryId) return;

    const confirmed = await ask(
      "Are you sure you want to delete this entry? This action cannot be undone.",
      { title: "Delete Entry", kind: "warning" }
    );

    if (!confirmed) return;

    deleteEntry.mutate(
      { dbId, id: selectedEntryId },
      {
        onSuccess: () => {
          if (tab) {
            updateTabState(tab.id, { selectedEntryId: null });
          }
          toast.success("Entry deleted.");
        },
        onError: (error) => {
          toast.error(`Failed to delete entry: ${error.message}`);
        },
      }
    );
  };

  const openCreateMode = useCallback(() => setEditMode("create"), []);
  useCreateEntryShortcut(openCreateMode, Boolean(dbId) && !isEditing);

  const focusSearchInput = useCallback(() => {
    searchInputRef.current?.focus();
  }, []);
  useSearchShortcut(focusSearchInput, Boolean(dbId) && !isEditing);

  const handleSearchEscape = useCallback(() => {
    searchState.clearSearch();
    searchInputRef.current?.blur();
  }, [searchState]);

  return (
    <ResizablePanelGroup
      orientation="horizontal"
      className="h-full min-h-0 w-full"
    >
      {/* Panel 1 - Entry List */}
      <ResizablePanel defaultSize={40} minSize={250} className="min-w-0">
        <div className="flex h-full min-h-0 min-w-0 flex-col">
          <div className="flex h-14 shrink-0 items-center gap-2 border-b">
            <div className="flex flex-1 items-center gap-2 px-3">
              <SidebarTrigger />
              <Separator
                orientation="vertical"
                className="data-[orientation=vertical]:h-6 mr-2"
              />
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
                    <div className="flex items-center gap-2">
                      <p className="text-sm">{groupName}</p>
                      {activeTag && (
                        <Badge
                          asChild
                          variant="secondary"
                          className="gap-1 text-xs"
                        >
                          <button
                            type="button"
                            className="cursor-pointer"
                            onClick={() => {
                              if (!dbId) {
                                return;
                              }
                              void navigate({
                                to: "/dashboard/index/$dbId",
                                params: { dbId },
                                search: (prev) => {
                                  const { tag: _tag, ...rest } = prev;
                                  return rest;
                                },
                              });
                            }}
                            aria-label={`Clear tag filter ${activeTag}`}
                          >
                            {activeTag}
                            <X className="size-3" />
                          </button>
                        </Badge>
                      )}
                    </div>
                    <small className="text-muted-foreground text-xs">
                      {entryCount} {entryCount === 1 ? "Item" : "Items"}
                    </small>
                  </>
                )}
              </div>
            </div>
          </div>
          <div className="flex items-center gap-2 p-2">
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

          <div className="flex min-h-0 min-w-0 flex-1 flex-col">
            {searchState.isSearchActive ? (
              <SearchResultsList
                results={searchState.results}
                query={searchState.query}
                onEntrySelect={handleEntrySelect}
              />
            ) : (
              <EntryList onEntrySelect={handleEntrySelect} />
            )}
          </div>
        </div>
      </ResizablePanel>

      {/* Resizable Handle with grip icon */}
      <ResizableHandle withHandle />

      {/* Panel 2 - Content Area */}
      <ResizablePanel defaultSize={75} minSize={360}>
        <div className="flex h-full min-h-0 flex-col">
          <div className="flex h-14 shrink-0 items-center gap-2 border-b">
            <div className="flex justify-between w-full">
              <div className="flex items-center gap-2 px-3">
                <EntryActions
                  onNew={() => setEditMode("create")}
                  onEdit={() => setEditMode("edit")}
                  onDelete={() => void handleDelete()}
                  disableNew={!dbId || isEditing}
                  disableEdit={!selectedEntryId || isEditing}
                  disableDelete={!selectedEntryId || isEditing}
                />
                <Separator
                  orientation="vertical"
                  className="data-[orientation=vertical]:h-6"
                />
                <Button
                  variant="ghost"
                  size="icon-sm"
                  aria-label="share entry"
                  disabled={isEditing}
                >
                  <Share />
                </Button>
              </div>
              <div className="flex items-center gap-2 px-3">
                <Button
                  variant="ghost"
                  size="icon-sm"
                  aria-label="password generator"
                  disabled={isEditing}
                >
                  <Dices />
                </Button>
                <Button
                  variant="ghost"
                  size="icon-sm"
                  aria-label="more options"
                  disabled={isEditing}
                >
                  <EllipsisVertical />
                </Button>
              </div>
            </div>
          </div>
          <div className="min-h-0 flex-1 overflow-auto scrollbar-hide">
            <div className="flex flex-col gap-4 p-4 pb-0 md:pb-20">
              {editMode === "edit" && selectedEntryId && dbId ? (
                isEditEntryLoading || isEditEntryTransitioning ? (
                  <div className="flex items-center justify-center py-12">
                    <Loader2 className="size-6 animate-spin text-muted-foreground" />
                  </div>
                ) : editEntry ? (
                  <EntryEditForm
                    entry={editEntry}
                    dbId={dbId}
                    groupId={groupId}
                    onSave={handleSave}
                    onCancel={handleCancel}
                    onDirtyChange={setHasUnsavedChanges}
                  />
                ) : (
                  <EntryItemDetailsEmpty />
                )
              ) : editMode === "create" && dbId ? (
                <EntryEditForm
                  dbId={dbId}
                  groupId={groupId}
                  onSave={handleSave}
                  onCancel={handleCancel}
                  onDirtyChange={setHasUnsavedChanges}
                />
              ) : selectedEntryId && dbId ? (
                <EntryItemDetails entryId={selectedEntryId} dbId={dbId} />
              ) : (
                <EntryItemDetailsEmpty />
              )}
            </div>
          </div>
        </div>
      </ResizablePanel>
    </ResizablePanelGroup>
  );
}
