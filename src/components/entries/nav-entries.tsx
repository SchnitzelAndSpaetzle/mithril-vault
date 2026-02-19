import { SidebarTrigger } from "@/components/ui/sidebar.tsx";
import { Separator } from "@/components/ui/separator.tsx";
import type { ReactNode } from "react";

interface NavEntriesProps {
  children?: ReactNode;
  actions?: ReactNode;
}
export default function NavEntries({ children, actions }: NavEntriesProps) {
  return (
    <div className="flex h-14 shrink-0 items-center gap-2 border-b">
      <div className="flex flex-1 items-center gap-2 px-3">
        <SidebarTrigger />
        <Separator
          orientation="vertical"
          className="data-[orientation=vertical]:h-6 mr-2"
        />
        {children}
      </div>
      {actions && <div className="flex items-center gap-1 px-3">{actions}</div>}
    </div>
  );
}
