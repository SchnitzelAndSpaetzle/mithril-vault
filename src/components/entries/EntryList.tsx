import EntryListItem from "@/components/entries/EntryListItem.tsx";
import { ItemGroup, ItemSeparator } from "@/components/ui/item.tsx";

export default function EntryList() {
  return (
    <ItemGroup>
      {Array.from({ length: 50 }, (_, i) => (
        <div key={i}>
          <EntryListItem
            username="blablablablabla@test.com"
            title={`Website ${i}`}
            id=""
            groupId=""
            tags={[]}
            customFields={{}}
            customFieldMeta={[]}
            createdAt=""
            modifiedAt=""
            accessedAt=""
          />
          <ItemSeparator />
        </div>
      ))}
    </ItemGroup>
  );
}
