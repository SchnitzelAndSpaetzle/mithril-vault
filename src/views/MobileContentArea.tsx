import EntryList from "@/components/entries/EntryList.tsx";
import NavEntries from "@/components/entries/nav-entries.tsx";
import { SearchForm } from "@/components/search-form.tsx";
import { Button } from "@/components/ui/button.tsx";
import { ArrowDownAZ, Plus } from "lucide-react";

export default function MobileContentArea() {
  return (
    <div className="h-full w-full overflow-auto scrollbar-hide">
      <NavEntries>
        <div className="flex flex-col">
          <p className="text-sm">All</p>
          <small className="text-muted-foreground text-xs">124 Items</small>
        </div>
      </NavEntries>
      <EntryList />
      <div className="sticky bottom-0 z-10">
        <div className="flex items-center gap-2 p-4 border-t backdrop-blur-2xl">
          <Button variant="outline" size="icon-sm" className="">
            <Plus />
          </Button>
          <SearchForm className="w-full" />
          <Button variant="outline" size="icon-sm" className="">
            <ArrowDownAZ />
          </Button>
        </div>
      </div>
    </div>
  );
}
