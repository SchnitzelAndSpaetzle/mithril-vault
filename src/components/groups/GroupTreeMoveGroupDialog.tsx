// SPDX-License-Identifier: MIT

import { useState } from "react";
import { useTranslation } from "react-i18next";
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
  const { t } = useTranslation();
  const [targetParentId, setTargetParentId] = useState("");
  const { data: allGroups } = useGroups(open ? dbId : null);

  const handleOpenChange = (nextOpen: boolean) => {
    if (!nextOpen) {
      setTargetParentId("");
    }
    onOpenChange(nextOpen);
  };

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
    handleOpenChange(false);
  };

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{t("groups.moveTitle")}</DialogTitle>
          <DialogDescription>
            {t("groups.moveDescription", { groupName: group.name })}
          </DialogDescription>
        </DialogHeader>
        <div className="grid gap-4 py-4">
          <div className="grid gap-2">
            <Label htmlFor="move-target">{t("groups.destination")}</Label>
            <Select value={targetParentId} onValueChange={setTargetParentId}>
              <SelectTrigger className="w-full" id="move-target">
                <SelectValue placeholder={t("groups.selectGroup")} />
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
          <Button variant="outline" onClick={() => handleOpenChange(false)}>
            {t("common.cancel")}
          </Button>
          <Button
            onClick={handleSubmit}
            disabled={!targetParentId || isPending}
          >
            {isPending ? t("groups.moving") : t("common.move")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
