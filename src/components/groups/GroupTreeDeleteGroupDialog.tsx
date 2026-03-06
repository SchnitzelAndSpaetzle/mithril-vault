// SPDX-License-Identifier: MIT

import { useTranslation } from "react-i18next";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { useGroupEntryCounts } from "@/hooks/use-group-entry-counts";
import { sumGroupEntryCounts } from "@/lib/group-utils";
import type { Group } from "@/lib/types";

interface GroupTreeDeleteGroupDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  group: Group;
  dbId: string;
  onDelete: () => void;
  isPending: boolean;
}

export function GroupTreeDeleteGroupDialog({
  open,
  onOpenChange,
  group,
  dbId,
  onDelete,
  isPending,
}: GroupTreeDeleteGroupDialogProps) {
  const { t } = useTranslation();
  const { data: entryCounts } = useGroupEntryCounts(open ? dbId : null);
  const totalEntries = entryCounts
    ? sumGroupEntryCounts(group, entryCounts)
    : 0;

  const handleConfirm = () => {
    onDelete();
    onOpenChange(false);
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{t("groups.deleteTitle")}</DialogTitle>
          <DialogDescription>
            {t("groups.deleteDescription", { groupName: group.name })}
            {totalEntries > 0 && (
              <> {t("groups.deleteEntryCount", { count: totalEntries })}</>
            )}{" "}
            {t("groups.deleteRecycleBin")}
          </DialogDescription>
        </DialogHeader>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            {t("common.cancel")}
          </Button>
          <Button
            variant="destructive"
            onClick={handleConfirm}
            disabled={isPending}
          >
            {isPending ? t("groups.deleting") : t("common.delete")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
