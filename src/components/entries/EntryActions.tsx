// SPDX-License-Identifier: MIT

import { Button } from "@/components/ui/button.tsx";
import { Pencil, Plus, Trash } from "lucide-react";

interface EntryActionsProps {
  onNew?: () => void;
  onEdit?: () => void;
  onDelete?: () => void;
  disableNew?: boolean;
  disableEdit?: boolean;
  disableDelete?: boolean;
}

export function EntryActions({
  onNew,
  onEdit,
  onDelete,
  disableNew,
  disableEdit,
  disableDelete,
}: EntryActionsProps) {
  return (
    <>
      {onNew && (
        <Button
          variant="ghost"
          size="icon-sm"
          aria-label="new entry"
          disabled={disableNew}
          onClick={onNew}
        >
          <Plus />
        </Button>
      )}
      {onEdit && (
        <Button
          variant="ghost"
          size="icon-sm"
          aria-label="edit entry"
          disabled={disableEdit}
          onClick={onEdit}
        >
          <Pencil />
        </Button>
      )}
      {onDelete && (
        <Button
          variant="ghost"
          size="icon-sm"
          aria-label="delete entry"
          disabled={disableDelete}
          onClick={onDelete}
        >
          <Trash />
        </Button>
      )}
    </>
  );
}
