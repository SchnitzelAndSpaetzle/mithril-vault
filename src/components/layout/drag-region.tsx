import { useState } from "react";
import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
} from "@/components/ui/resizable";
import EntryList from "@/components/entries/EntryList.tsx";
import EntryItemDetails from "@/components/entries/EntryItemDetails.tsx";
import { EntryItemDetailsEmpty } from "@/components/entries/EntryItemDetailsEmpty.tsx";
import { EntryEditForm } from "@/components/entries/EntryEditForm.tsx";
import { SearchForm } from "@/components/search-form.tsx";
import { SidebarTrigger } from "@/components/ui/sidebar.tsx";
import { Separator } from "@/components/ui/separator.tsx";
import {
  Dices,
  EllipsisVertical,
  Pencil,
  Plus,
  Share,
  Trash,
} from "lucide-react";
import { Button } from "@/components/ui/button.tsx";
import SortDropdown from "@/components/entries/sort-dropdown";
import { useEntryListHeader } from "@/hooks/use-entry-list-header";
import { useEntryDetail } from "@/hooks/use-entry-detail";
import { useActiveDatabase } from "@/hooks/use-active-database";
import { useDatabaseTabs } from "@/stores/database-tabs";
import type { Entry } from "@/lib/types";

type EditMode = "view" | "edit" | "create";

export default function DragRegion() {
  const { groupName, entryCount } = useEntryListHeader();
  const { tab, dbId } = useActiveDatabase();
  const selectedEntryId = tab?.selectedEntryId ?? null;
  const [editMode, setEditMode] = useState<EditMode>("view");

  const selectedGroupId = tab?.selectedGroupId ?? null;
  const rootGroupId = tab?.info?.rootGroupId ?? "";
  const groupId = selectedGroupId ?? rootGroupId;

  const { entry: editEntry } = useEntryDetail(
    editMode === "edit" && selectedEntryId ? selectedEntryId : "",
    editMode === "edit" && dbId ? dbId : ""
  );

  const updateTabState = useDatabaseTabs((s) => s.updateTabState);

  function handleSave(saved: Entry) {
    setEditMode("view");
    if (tab) {
      updateTabState(tab.id, { selectedEntryId: saved.id });
    }
  }

  function handleCancel() {
    setEditMode("view");
  }

  const isEditing = editMode !== "view";

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
                <p className="text-sm">{groupName}</p>
                <small className="text-muted-foreground text-xs">
                  {entryCount} {entryCount === 1 ? "Item" : "Items"}
                </small>
              </div>
            </div>
          </div>
          <div className="flex items-center gap-2 p-2">
            <SearchForm className="w-full" />
            <SortDropdown />
          </div>

          <div className="flex min-h-0 min-w-0 flex-1 flex-col">
            <EntryList />
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
                <Button
                  variant="ghost"
                  size="icon-sm"
                  aria-label="add entry"
                  disabled={!dbId || isEditing}
                  onClick={() => setEditMode("create")}
                >
                  <Plus />
                </Button>
                <Button
                  variant="ghost"
                  size="icon-sm"
                  aria-label="edit entry"
                  disabled={!selectedEntryId || isEditing}
                  onClick={() => setEditMode("edit")}
                >
                  <Pencil />
                </Button>
                <Button
                  variant="ghost"
                  size="icon-sm"
                  aria-label="delete entry"
                  disabled={!selectedEntryId || isEditing}
                >
                  <Trash />
                </Button>
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
                <EntryEditForm
                  entry={editEntry ?? null}
                  dbId={dbId}
                  groupId={groupId}
                  onSave={handleSave}
                  onCancel={handleCancel}
                />
              ) : editMode === "create" && dbId ? (
                <EntryEditForm
                  dbId={dbId}
                  groupId={groupId}
                  onSave={handleSave}
                  onCancel={handleCancel}
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
