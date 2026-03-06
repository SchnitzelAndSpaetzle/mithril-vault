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
import { getKeepassIcon } from "@/lib/keepass-icons";

const DEFAULT_FOLDER_ICON = 48;

interface GroupTreeCreateSubgroupDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  parentName: string;
  onCreateSubgroup: (name: string, iconId: number) => void;
  isPending: boolean;
}

export function GroupTreeCreateSubgroupDialog({
  open,
  onOpenChange,
  parentName,
  onCreateSubgroup,
  isPending,
}: GroupTreeCreateSubgroupDialogProps) {
  const { t } = useTranslation();
  const [groupName, setGroupName] = useState("");
  const [iconId, setIconId] = useState(DEFAULT_FOLDER_ICON);

  const handleSubmit = () => {
    const trimmed = groupName.trim();
    if (!trimmed) return;

    onCreateSubgroup(trimmed, iconId);
    setGroupName("");
    setIconId(DEFAULT_FOLDER_ICON);
    onOpenChange(false);
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{t("groups.createTitle")}</DialogTitle>
          <DialogDescription>
            {t("groups.createDescription", { parentName })}
          </DialogDescription>
        </DialogHeader>
        <div className="grid gap-4 py-4">
          <div className="grid gap-2">
            <Label htmlFor="new-group-name">{t("common.name")}</Label>
            <div className="flex gap-2">
              <IconPickerPopover selectedIconId={iconId} onSelect={setIconId}>
                <Button
                  variant="outline"
                  size="icon"
                  className="shrink-0"
                  type="button"
                >
                  {createElement(getKeepassIcon(iconId), {
                    className: "h-4 w-4",
                  })}
                </Button>
              </IconPickerPopover>
              <Input
                id="new-group-name"
                value={groupName}
                onChange={(e) => setGroupName(e.target.value)}
                placeholder={t("groups.enterGroupName")}
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
          <Button
            onClick={handleSubmit}
            disabled={!groupName.trim() || isPending}
          >
            {isPending ? t("groups.creating") : t("common.create")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
