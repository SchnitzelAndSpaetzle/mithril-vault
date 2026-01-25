import { useEffect, useRef, useState } from "react";
import { Separator } from "@/components/ui/separator.tsx";
import { GripVertical } from "lucide-react";
import EntryList from "@/components/entries/EntryList.tsx";

const MIN_PANEL1_WIDTH = 250;
const MIN_PANEL2_WIDTH = 300;
const SEPARATOR_WIDTH = 16;

export default function DragRegion() {
  const [panel1Width, setPanel1Width] = useState(300);
  const [isResizing, setIsResizing] = useState(false);
  const [isHovering, setIsHovering] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);

  const handleMouseDown = (event: React.MouseEvent) => {
    setIsResizing(true);
    event.preventDefault();
  };

  const handleMouseUp = () => {
    setIsResizing(false);
  };

  const handleMouseMove = (event: React.MouseEvent) => {
    if (!isResizing || !containerRef.current) return;

    const containerRect = containerRef.current.getBoundingClientRect();
    const containerWidth = containerRect.width;
    const newPanel1Width = event.clientX - containerRect.left;

    // Calculate max width ensuring panel 2 stays at least MIN_PANEL2_WIDTH
    const maxPanel1Width = containerWidth - MIN_PANEL2_WIDTH - SEPARATOR_WIDTH;

    // Constrain panel 1 width between min and max
    const constrainedWidth = Math.max(
      MIN_PANEL1_WIDTH,
      Math.min(newPanel1Width, maxPanel1Width)
    );

    setPanel1Width(constrainedWidth);
  };

  // Prevent text selection during drag
  useEffect(() => {
    if (isResizing) {
      document.body.style.cursor = "col-resize";
      document.body.style.userSelect = "none";
    } else {
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
    }

    return () => {
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
    };
  }, [isResizing]);

  // Global mouse up listener
  useEffect(() => {
    const handleGlobalMouseUp = () => {
      setIsResizing(false);
    };

    if (isResizing) {
      window.addEventListener("mouseup", handleGlobalMouseUp);
    }

    return () => {
      window.removeEventListener("mouseup", handleGlobalMouseUp);
    };
  }, [isResizing]);

  return (
    <div
      ref={containerRef}
      className="flex w-full h-full"
      onMouseMove={handleMouseMove}
      onMouseUp={handleMouseUp}
    >
      {/* Panel 1 with hidden scrollbar */}
      <div style={{ width: panel1Width }} className="h-full">
        <div className="h-full w-full overflow-auto scrollbar-hide">
          <div className="p-4">
            <h2 className="text-lg font-semibold mb-2">Panel 1</h2>
            <p className="text-sm text-muted-foreground">
              Width: {panel1Width}px
            </p>
            <p className="text-sm text-muted-foreground">
              Min: {MIN_PANEL1_WIDTH}px
            </p>
            {/* Example content to demonstrate scrolling */}
            <div className="mt-4">
              <EntryList />
            </div>
          </div>
        </div>
      </div>

      {/* Separator with hover effect and drag icon */}
      <div
        className="relative flex items-center justify-center"
        style={{ width: SEPARATOR_WIDTH }}
        onMouseEnter={() => setIsHovering(true)}
        onMouseLeave={() => setIsHovering(false)}
      >
        <Separator
          className={`cursor-col-resize transition-colors ${
            isHovering || isResizing ? "bg-primary/50" : "bg-border"
          }`}
          orientation="vertical"
          onMouseDown={handleMouseDown}
        />
        <div
          className={`absolute inset-0 flex items-center justify-center pointer-events-none transition-opacity ${
            isHovering || isResizing ? "opacity-100" : "opacity-0"
          }`}
        >
          <GripVertical className="h-4 w-4 text-primary" />
        </div>
      </div>

      {/* Panel 2 with hidden scrollbar */}
      <div className="flex-1 h-full">
        <div className="h-full w-full overflow-auto scrollbar-hide">
          <div className="p-4">
            <h2 className="text-lg font-semibold mb-2">Panel 2</h2>
            <p className="text-sm text-muted-foreground">
              Min: {MIN_PANEL2_WIDTH}px
            </p>
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
      </div>
    </div>
  );
}
