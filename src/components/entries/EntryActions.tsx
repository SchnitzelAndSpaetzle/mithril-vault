// SPDX-License-Identifier: MIT

import { useTranslation } from "react-i18next";
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
  const { t } = useTranslation();

  return (
    <>
      {onNew && (
        <Button
          variant="ghost"
          size="icon-sm"
          aria-label={t("entries.newEntry")}
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
          aria-label={t("entries.editEntry")}
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
          aria-label={t("entries.deleteEntry")}
          disabled={disableDelete}
          onClick={onDelete}
        >
          <Trash />
        </Button>
      )}
    </>
  );
}
