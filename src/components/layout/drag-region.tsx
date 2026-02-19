import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
} from "@/components/ui/resizable";
import EntryList from "@/components/entries/EntryList.tsx";
import EntryItemDetails from "@/components/entries/EntryItemDetails.tsx";
import { EntryItemDetailsEmpty } from "@/components/entries/EntryItemDetailsEmpty.tsx";
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
import { useActiveDatabase } from "@/hooks/use-active-database";

export default function DragRegion() {
  const { groupName, entryCount } = useEntryListHeader();
  const { tab, dbId } = useActiveDatabase();
  const selectedEntryId = tab?.selectedEntryId ?? null;

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
                <Button variant="ghost" size="icon-sm" aria-label="add entry">
                  <Plus />
                </Button>
                <Button variant="ghost" size="icon-sm" aria-label="edit entry">
                  <Pencil />
                </Button>
                <Button
                  variant="ghost"
                  size="icon-sm"
                  aria-label="delete entry"
                >
                  <Trash />
                </Button>
                <Separator
                  orientation="vertical"
                  className="data-[orientation=vertical]:h-6"
                />
                <Button variant="ghost" size="icon-sm" aria-label="edit entry">
                  <Share />
                </Button>
              </div>
              <div className="flex items-center gap-2 px-3">
                <Button variant="ghost" size="icon-sm" aria-label="add entry">
                  <Dices />
                </Button>
                <Button variant="ghost" size="icon-sm" aria-label="edit entry">
                  <EllipsisVertical />
                </Button>
              </div>
            </div>
          </div>
          <div className="min-h-0 flex-1 overflow-auto scrollbar-hide">
            <div className="flex flex-col gap-4 p-4 pb-0 md:pb-20">
              {selectedEntryId && dbId ? (
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
