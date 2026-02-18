import EntryList from "@/components/entries/EntryList.tsx";
import NavEntries from "@/components/entries/nav-entries.tsx";
import { SearchForm } from "@/components/search-form.tsx";
import { Button } from "@/components/ui/button.tsx";
import { Plus } from "lucide-react";
import SortDropdown from "@/components/entries/sort-dropdown";
import { useEntryListHeader } from "@/hooks/use-entry-list-header";

export default function MobileContentArea() {
  const { groupName, entryCount } = useEntryListHeader();

  return (
    <div className="h-full w-full min-w-0 overflow-auto overflow-x-hidden scrollbar-hide">
      <NavEntries>
        <div className="flex flex-col">
          <p className="text-sm">{groupName}</p>
          <small className="text-muted-foreground text-xs">
            {entryCount} {entryCount === 1 ? "Item" : "Items"}
          </small>
        </div>
      </NavEntries>
      <EntryList />
      <div className="sticky bottom-0 z-10">
        <div className="flex items-center gap-2 p-4 border-t backdrop-blur-2xl">
          <Button variant="outline" size="icon-sm" className="">
            <Plus />
          </Button>
          <SearchForm className="w-full" />
          <SortDropdown />
        </div>
      </div>
    </div>
  );
}
