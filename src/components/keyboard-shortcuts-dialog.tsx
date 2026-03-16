// SPDX-License-Identifier: MIT

import { useCallback, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { useShortcut } from "@/hooks/use-shortcut";
import {
  formatShortcut,
  SHORTCUT_GROUPS,
  type ShortcutDef,
  SHORTCUTS,
} from "@/lib/shortcuts";

type ShortcutI18nKey =
  | "shortcuts.search"
  | "shortcuts.newEntry"
  | "shortcuts.save"
  | "shortcuts.lockDatabase"
  | "shortcuts.settings"
  | "shortcuts.toggleSidebar"
  | "shortcuts.shortcutsHelp"
  | "shortcuts.copyUsername"
  | "shortcuts.copyPassword"
  | "shortcuts.openUrl"
  | "shortcuts.editEntry"
  | "shortcuts.deleteEntry"
  | "shortcuts.navigateDown"
  | "shortcuts.navigateUp"
  | "shortcuts.firstEntry"
  | "shortcuts.lastEntry"
  | "shortcuts.activateEntry";

export function KeyboardShortcutsDialog() {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);

  useShortcut(
    SHORTCUTS.shortcutsHelp,
    useCallback(() => setOpen(true), []),
    true
  );

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogContent className="max-w-lg max-h-[85vh] flex flex-col">
        <DialogHeader>
          <DialogTitle>{t("shortcuts.title")}</DialogTitle>
          <DialogDescription className="sr-only">
            {t("shortcuts.title")}
          </DialogDescription>
        </DialogHeader>
        <div className="space-y-6 overflow-y-auto pr-1">
          <ShortcutGroup
            title={t("shortcuts.globalGroup")}
            shortcuts={SHORTCUT_GROUPS.global}
          />
          <ShortcutGroup
            title={t("shortcuts.entryGroup")}
            shortcuts={SHORTCUT_GROUPS.entry}
          />
          <ShortcutGroup
            title={t("shortcuts.listGroup")}
            shortcuts={SHORTCUT_GROUPS.list}
          />
        </div>
      </DialogContent>
    </Dialog>
  );
}

function ShortcutGroup({
  title,
  shortcuts,
}: {
  title: string;
  shortcuts: readonly ShortcutDef[];
}) {
  const { t } = useTranslation();

  return (
    <div>
      <h4 className="text-sm font-medium text-muted-foreground mb-2">
        {title}
      </h4>
      <div className="space-y-1">
        {shortcuts.map((def) => (
          <div
            key={def.id}
            className="flex items-center justify-between rounded-md px-2 py-1.5"
          >
            <span className="text-sm">{t(def.i18nKey as ShortcutI18nKey)}</span>
            <kbd className="inline-flex h-6 items-center rounded border bg-muted px-2 text-xs font-medium text-muted-foreground">
              {formatShortcut(def)}
            </kbd>
          </div>
        ))}
      </div>
    </div>
  );
}
