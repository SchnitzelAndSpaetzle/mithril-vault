// SPDX-License-Identifier: MIT

import { type ShortcutDef, formatShortcut } from "@/lib/shortcuts";

interface ShortcutBadgeProps {
  shortcut: ShortcutDef;
  className?: string;
}

export function ShortcutBadge({ shortcut, className }: ShortcutBadgeProps) {
  return (
    <kbd
      className={`ml-auto text-xs tracking-widest text-muted-foreground ${className ?? ""}`}
    >
      {formatShortcut(shortcut)}
    </kbd>
  );
}
