// SPDX-License-Identifier: MIT

import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button.tsx";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip.tsx";
import { Pencil, Plus, Trash } from "lucide-react";
import { formatShortcut, SHORTCUTS } from "@/lib/shortcuts";

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
        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              variant="ghost"
              size="icon-sm"
              aria-label={t("entries.newEntry")}
              disabled={disableNew}
              onClick={onNew}
            >
              <Plus />
            </Button>
          </TooltipTrigger>
          <TooltipContent side="bottom">
            {t("entries.newEntry")} ({formatShortcut(SHORTCUTS.newEntry)})
          </TooltipContent>
        </Tooltip>
      )}
      {onEdit && (
        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              variant="ghost"
              size="icon-sm"
              aria-label={t("entries.editEntry")}
              disabled={disableEdit}
              onClick={onEdit}
            >
              <Pencil />
            </Button>
          </TooltipTrigger>
          <TooltipContent side="bottom">
            {t("entries.editEntry")} ({formatShortcut(SHORTCUTS.editEntry)})
          </TooltipContent>
        </Tooltip>
      )}
      {onDelete && (
        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              variant="ghost"
              size="icon-sm"
              aria-label={t("entries.deleteEntry")}
              disabled={disableDelete}
              onClick={onDelete}
            >
              <Trash />
            </Button>
          </TooltipTrigger>
          <TooltipContent side="bottom">
            {t("entries.deleteEntry")} ({formatShortcut(SHORTCUTS.deleteEntry)})
          </TooltipContent>
        </Tooltip>
      )}
    </>
  );
}
