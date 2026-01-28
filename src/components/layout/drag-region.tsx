import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
} from "@/components/ui/resizable";
import EntryList from "@/components/entries/EntryList.tsx";
import EntryItemDetails from "@/components/entries/EntryItemDetails.tsx";
import { SearchForm } from "@/components/search-form.tsx";
import { SidebarTrigger } from "@/components/ui/sidebar.tsx";
import { Separator } from "@/components/ui/separator.tsx";
import {
  ArrowDownAZ,
  Dices,
  EllipsisVertical,
  Pencil,
  Plus,
  Share,
  Trash,
} from "lucide-react";
import { Button } from "@/components/ui/button.tsx";

export default function DragRegion() {
  return (
    <ResizablePanelGroup orientation="horizontal" className="h-full w-full">
      {/* Panel 1 - Entry List */}
      <ResizablePanel defaultSize={40} minSize={250}>
        <div className="flex h-14 shrink-0 items-center gap-2 border-b">
          <div className="flex flex-1 items-center gap-2 px-3">
            <SidebarTrigger />
            <Separator
              orientation="vertical"
              className="data-[orientation=vertical]:h-6 mr-2"
            />
            <div className="flex flex-col">
              <p className="text-sm">All</p>
              <small className="text-muted-foreground text-xs">124 Items</small>
            </div>
          </div>
        </div>
        <div className="flex items-center gap-2 p-2">
          <SearchForm className="w-full" />
          <Button variant="outline" size="icon-sm" className="">
            <ArrowDownAZ />
          </Button>
        </div>

        <div className="h-full w-full overflow-auto scrollbar-hide">
          <EntryList />
        </div>
      </ResizablePanel>

      {/* Resizable Handle with grip icon */}
      <ResizableHandle withHandle />

      {/* Panel 2 - Content Area */}
      <ResizablePanel defaultSize={75} minSize={360}>
        <div className="flex h-14 shrink-0 items-center gap-2 border-b">
          <div className="flex justify-between w-full">
            <div className="flex items-center gap-2 px-3">
              <Button variant="ghost" size="icon-sm" aria-label="add entry">
                <Plus />
              </Button>
              <Button variant="ghost" size="icon-sm" aria-label="edit entry">
                <Pencil />
              </Button>
              <Button variant="ghost" size="icon-sm" aria-label="delete entry">
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
        <div className="h-full w-full overflow-auto scrollbar-hide">
          <div className="p-4">
            {/* Example content to demonstrate scrolling */}
            <div className="space-y-2">
              <EntryItemDetails />
            </div>
          </div>
        </div>
      </ResizablePanel>
    </ResizablePanelGroup>
  );
}
