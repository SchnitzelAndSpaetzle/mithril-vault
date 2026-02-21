// SPDX-License-Identifier: MIT

import { useState } from "react";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import { flattenGroups, getDescendantIds } from "@/lib/group-utils";
import { useGroups } from "@/hooks/use-groups";
import type { Group } from "@/lib/types";

interface GroupTreeMoveGroupDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  group: Group;
  dbId: string;
  onMove: (targetParentId: string) => void;
  isPending: boolean;
}

export function GroupTreeMoveGroupDialog({
  open,
  onOpenChange,
  group,
  dbId,
  onMove,
  isPending,
}: GroupTreeMoveGroupDialogProps) {
  const [targetParentId, setTargetParentId] = useState("");
  const { data: allGroups } = useGroups(open ? dbId : null);

  const excludedIds = new Set([group.id, ...getDescendantIds(group)]);
  const moveTargets = allGroups
    ? flattenGroups(allGroups).filter(
        (candidate) => !excludedIds.has(candidate.id)
      )
    : [];

  const handleSubmit = () => {
    if (!targetParentId) {
      return;
    }
    onMove(targetParentId);
    setTargetParentId("");
    onOpenChange(false);
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Move Group</DialogTitle>
          <DialogDescription>
            Select a new parent group for &ldquo;{group.name}&rdquo;.
          </DialogDescription>
        </DialogHeader>
        <div className="grid gap-4 py-4">
          <div className="grid gap-2">
            <Label htmlFor="move-target">Destination</Label>
            <Select value={targetParentId} onValueChange={setTargetParentId}>
              <SelectTrigger className="w-full" id="move-target">
                <SelectValue placeholder="Select a group" />
              </SelectTrigger>
              <SelectContent>
                {moveTargets.map((group) => (
                  <SelectItem key={group.id} value={group.id}>
                    <span style={{ paddingLeft: `${group.depth * 12}px` }}>
                      {group.name}
                    </span>
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button
            onClick={handleSubmit}
            disabled={!targetParentId || isPending}
          >
            {isPending ? "Moving..." : "Move"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
