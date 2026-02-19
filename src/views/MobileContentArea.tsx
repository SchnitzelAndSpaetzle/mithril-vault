import EntryList from "@/components/entries/EntryList.tsx";
import NavEntries from "@/components/entries/nav-entries.tsx";
import { SearchForm } from "@/components/search-form.tsx";
import { Button } from "@/components/ui/button.tsx";
import { Plus } from "lucide-react";
import SortDropdown from "@/components/entries/sort-dropdown";
import { useEntryListHeader } from "@/hooks/use-entry-list-header";
import { useNavigate } from "@tanstack/react-router";

export default function MobileContentArea() {
  const { groupName, entryCount } = useEntryListHeader();
  const navigate = useNavigate();

  return (
    <div className="flex h-full w-full min-w-0 flex-col">
      <NavEntries>
        <div className="flex flex-col">
          <p className="text-sm">{groupName}</p>
          <small className="text-muted-foreground text-xs">
            {entryCount} {entryCount === 1 ? "Item" : "Items"}
          </small>
        </div>
      </NavEntries>
      <div className="flex min-h-0 flex-1 flex-col">
        <EntryList />
      </div>
      <div className="shrink-0 border-t backdrop-blur-2xl">
        <div className="flex items-center gap-2 p-4">
          <Button
            variant="outline"
            size="icon-sm"
            onClick={() => void navigate({ to: "/dashboard/entry/new" })}
          >
            <Plus />
          </Button>
          <SearchForm className="w-full" />
          <SortDropdown />
        </div>
      </div>
    </div>
  );
}
