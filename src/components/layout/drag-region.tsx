import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
} from "@/components/ui/resizable";
import EntryList from "@/components/entries/EntryList.tsx";

export default function DragRegion() {
  return (
    <ResizablePanelGroup orientation="horizontal" className="h-full w-full">
      {/* Panel 1 - Entry List */}
      <ResizablePanel defaultSize={40} minSize={250}>
        <div className="h-full w-full overflow-auto scrollbar-hide">
          <div className="p-4">
            <h2 className="text-lg font-semibold mb-2">Panel 1</h2>
            <div className="mt-4">
              <EntryList />
            </div>
          </div>
        </div>
      </ResizablePanel>

      {/* Resizable Handle with grip icon */}
      <ResizableHandle withHandle />

      {/* Panel 2 - Content Area */}
      <ResizablePanel defaultSize={75} minSize={350}>
        <div className="h-full w-full overflow-auto scrollbar-hide">
          <div className="p-4">
            <h2 className="text-lg font-semibold mb-2">Panel 2</h2>
            {/* Example content to demonstrate scrolling */}
            <div className="mt-4 space-y-2">
              {Array.from({ length: 50 }, (_, i) => (
                <div key={i} className="p-2 bg-muted rounded">
                  Item {i + 1}
                </div>
              ))}
            </div>
          </div>
        </div>
      </ResizablePanel>
    </ResizablePanelGroup>
  );
}
