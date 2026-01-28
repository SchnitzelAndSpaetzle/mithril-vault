import { type ReactNode } from "react";

interface ActionsMobileEntriesProps {
  children?: ReactNode;
}
export default function ActionsMobileEntries({
  children,
}: ActionsMobileEntriesProps) {
  return (
    <div className="sticky bottom-0 z-10">
      <div className="flex items-center gap-2 p-4 border-t backdrop-blur-2xl">
        {children}
      </div>
    </div>
  );
}
