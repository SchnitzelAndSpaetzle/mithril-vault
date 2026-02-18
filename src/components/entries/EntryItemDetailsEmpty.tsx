import {
  Empty,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from "@/components/ui/empty";
import { KeyIcon } from "lucide-react";

export function EntryItemDetailsEmpty() {
  return (
    <Empty>
      <EmptyHeader>
        <EmptyMedia variant="icon">
          <KeyIcon />
        </EmptyMedia>
        <EmptyTitle>No Entry Selected</EmptyTitle>
        <EmptyDescription>
          Select an entry from the list to view its details.
        </EmptyDescription>
      </EmptyHeader>
    </Empty>
  );
}
