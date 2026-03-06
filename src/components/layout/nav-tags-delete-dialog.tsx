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

interface NavTagsDeleteDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  targetTag: string;
  onConfirm: () => void;
  isPending: boolean;
}

export function NavTagsDeleteDialog({
  open,
  onOpenChange,
  targetTag,
  onConfirm,
  isPending,
}: NavTagsDeleteDialogProps) {
  const { t } = useTranslation();

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{t("tags.deleteTitle")}</DialogTitle>
          <DialogDescription>
            {t("tags.deleteDescription", { tag: targetTag })}
          </DialogDescription>
        </DialogHeader>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            {t("common.cancel")}
          </Button>
          <Button
            variant="destructive"
            onClick={onConfirm}
            disabled={isPending}
          >
            {isPending ? t("tags.deleting") : t("common.delete")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
