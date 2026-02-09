import EntryListItem from "@/components/entries/EntryListItem.tsx";
import { ItemGroup, ItemSeparator } from "@/components/ui/item.tsx";
import { useActiveDatabase } from "@/hooks/use-active-database";
import { useCustomIcons } from "@/hooks/use-custom-icons";
import { useEntries } from "@/hooks/use-entries";
import { useSearch } from "@tanstack/react-router";

export default function EntryList() {
  const { dbId } = useActiveDatabase();
  const search = useSearch({ from: "/dashboard/index/$dbId" });
  const { data: customIcons } = useCustomIcons(dbId);
  const {
    data: entries,
    isLoading,
    isError,
    error,
  } = useEntries(dbId, search.groupId);

  if (!dbId) {
    return (
      <div className="px-3 py-2 text-sm text-muted-foreground">
        Open a database to view entries.
      </div>
    );
  }

  if (isLoading) {
    return (
      <div className="px-3 py-2 text-sm text-muted-foreground">
        Loading entries...
      </div>
    );
  }

  if (isError) {
    return (
      <div className="px-3 py-2 text-sm text-destructive">
        Failed to load entries: {error.message}
      </div>
    );
  }

  if (!entries || entries.length === 0) {
    return (
      <div className="px-3 py-2 text-sm text-muted-foreground">
        No entries found.
      </div>
    );
  }

  return (
    <ItemGroup>
      {entries.map((entry) => (
        <div key={entry.id}>
          <EntryListItem {...entry} customIcons={customIcons ?? {}} />
          <ItemSeparator />
        </div>
      ))}
    </ItemGroup>
  );
}
