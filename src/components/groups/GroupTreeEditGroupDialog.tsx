// SPDX-License-Identifier: MIT

import { createElement, useState } from "react";
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
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { IconPickerPopover } from "@/components/entries/IconPickerPopover";
import { getKeepassIcon, parseGroupIconId } from "@/lib/keepass-icons";
import type { Group } from "@/lib/types";

const DEFAULT_FOLDER_ICON = 48;

interface GroupTreeEditGroupDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  group: Group;
  onUpdateGroup: (data: { name?: string; icon?: string }) => void;
  isPending: boolean;
}

export function GroupTreeEditGroupDialog({
  open,
  onOpenChange,
  group,
  onUpdateGroup,
  isPending,
}: GroupTreeEditGroupDialogProps) {
  const { t } = useTranslation();
  const [editedName, setEditedName] = useState(group.name);
  const [editedIcon, setEditedIcon] = useState(
    parseGroupIconId(group.icon) ?? DEFAULT_FOLDER_ICON
  );

  const originalIconId = parseGroupIconId(group.icon) ?? DEFAULT_FOLDER_ICON;

  const handleSubmit = () => {
    const trimmedName = editedName.trim();
    if (!trimmedName) return;

    const nameChanged = trimmedName !== group.name;
    const iconChanged = editedIcon !== originalIconId;

    if (!nameChanged && !iconChanged) return;

    const data: { name?: string; icon?: string } = {};
    if (nameChanged) data.name = trimmedName;
    if (iconChanged) data.icon = String(editedIcon);

    onUpdateGroup(data);
    onOpenChange(false);
  };

  const saveDisabled =
    !editedName.trim() ||
    (editedName.trim() === group.name && editedIcon === originalIconId) ||
    isPending;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{t("groups.editTitle")}</DialogTitle>
          <DialogDescription>
            {t("groups.editDescription", { groupName: group.name })}
          </DialogDescription>
        </DialogHeader>
        <div className="grid gap-4 py-4">
          <div className="grid gap-2">
            <Label htmlFor="edit-group-name">{t("common.name")}</Label>
            <div className="flex gap-2">
              <IconPickerPopover
                selectedIconId={editedIcon}
                onSelect={setEditedIcon}
              >
                <Button
                  variant="outline"
                  size="icon"
                  className="shrink-0"
                  type="button"
                >
                  {createElement(getKeepassIcon(editedIcon), {
                    className: "h-4 w-4",
                  })}
                </Button>
              </IconPickerPopover>
              <Input
                id="edit-group-name"
                value={editedName}
                onChange={(e) => setEditedName(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") {
                    handleSubmit();
                  }
                }}
              />
            </div>
          </div>
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            {t("common.cancel")}
          </Button>
          <Button onClick={handleSubmit} disabled={saveDisabled}>
            {isPending ? t("groups.saving") : t("common.save")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
