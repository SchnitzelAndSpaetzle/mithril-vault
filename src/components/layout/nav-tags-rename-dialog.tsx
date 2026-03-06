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
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";

interface NavTagsRenameDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  targetTag: string;
  onConfirm: (newTagName: string) => void;
  isPending: boolean;
}

export function NavTagsRenameDialog({
  open,
  onOpenChange,
  targetTag,
  onConfirm,
  isPending,
}: NavTagsRenameDialogProps) {
  const { t } = useTranslation();
  const [newTagName, setNewTagName] = useState(targetTag);
  const handleConfirm = () => onConfirm(newTagName);

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{t("tags.renameTitle")}</DialogTitle>
          <DialogDescription>
            {t("tags.renameDescription", { tag: targetTag })}
          </DialogDescription>
        </DialogHeader>
        <Input
          value={newTagName}
          onChange={(e) => setNewTagName(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") {
              handleConfirm();
            }
          }}
          placeholder={t("tags.newTagPlaceholder")}
          autoFocus
        />
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            {t("common.cancel")}
          </Button>
          <Button
            onClick={handleConfirm}
            disabled={
              !newTagName.trim() || newTagName.trim() === targetTag || isPending
            }
          >
            {isPending ? t("tags.renaming") : t("common.rename")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
