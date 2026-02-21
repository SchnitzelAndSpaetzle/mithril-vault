// SPDX-License-Identifier: MIT

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
          <DialogTitle>Delete Group</DialogTitle>
          <DialogDescription>
            Are you sure you want to delete &ldquo;{group.name}&rdquo;?
            {totalEntries > 0 && (
              <>
                {" "}
                This group contains{" "}
                <strong>
                  {totalEntries} {totalEntries === 1 ? "entry" : "entries"}
                </strong>
                .
              </>
            )}{" "}
            This will move the group and all its contents to the Recycle Bin.
          </DialogDescription>
        </DialogHeader>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button
            variant="destructive"
            onClick={handleConfirm}
            disabled={isPending}
          >
            {isPending ? "Deleting..." : "Delete"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
