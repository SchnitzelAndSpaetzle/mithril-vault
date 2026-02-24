import { useState } from "react";
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
  const [newTagName, setNewTagName] = useState(targetTag);
  const handleConfirm = () => onConfirm(newTagName);

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Rename Tag</DialogTitle>
          <DialogDescription>
            This will rename &quot;{targetTag}&quot; across all entries.
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
          placeholder="New tag name"
          autoFocus
        />
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button
            onClick={handleConfirm}
            disabled={
              !newTagName.trim() || newTagName.trim() === targetTag || isPending
            }
          >
            {isPending ? "Renaming..." : "Rename"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
