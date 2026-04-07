// SPDX-License-Identifier: MIT

import { type ReactNode, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { PasswordGenerator } from "@/components/generator/PasswordGenerator";

interface PasswordGeneratorDialogProps {
  onUsePassword: (password: string) => void;
  children: ReactNode;
}

export function PasswordGeneratorDialog({
  onUsePassword,
  children,
}: PasswordGeneratorDialogProps) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);

  function handleUse(value: string) {
    onUsePassword(value);
    setOpen(false);
  }

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger asChild>{children}</DialogTrigger>
      <DialogContent className="max-w-lg max-h-[80vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle>{t("passwordGenerator.title")}</DialogTitle>
        </DialogHeader>
        <PasswordGenerator onUsePassword={handleUse} />
      </DialogContent>
    </Dialog>
  );
}
