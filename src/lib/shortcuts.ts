// SPDX-License-Identifier: MIT

const isMac =
  typeof navigator !== "undefined" &&
  (navigator.platform?.startsWith("Mac") ||
    navigator.userAgent.includes("Mac"));

export type ShortcutScope = "global" | "entry" | "list";

export interface ShortcutDef {
  id: string;
  key: string;
  aliases?: readonly string[];
  ctrlOrMeta: boolean;
  shift?: boolean;
  scope: ShortcutScope;
  i18nKey: string;
}

export const SHORTCUTS = {
  search: {
    id: "search",
    key: "k",
    ctrlOrMeta: true,
    scope: "global",
    i18nKey: "shortcuts.search",
  },
  newEntry: {
    id: "newEntry",
    key: "n",
    ctrlOrMeta: true,
    scope: "global",
    i18nKey: "shortcuts.newEntry",
  },
  save: {
    id: "save",
    key: "s",
    ctrlOrMeta: true,
    scope: "global",
    i18nKey: "shortcuts.save",
  },
  lockDatabase: {
    id: "lockDatabase",
    key: "l",
    ctrlOrMeta: true,
    scope: "global",
    i18nKey: "shortcuts.lockDatabase",
  },
  settings: {
    id: "settings",
    key: ",",
    ctrlOrMeta: true,
    scope: "global",
    i18nKey: "shortcuts.settings",
  },
  toggleSidebar: {
    id: "toggleSidebar",
    key: "b",
    ctrlOrMeta: true,
    scope: "global",
    i18nKey: "shortcuts.toggleSidebar",
  },
  shortcutsHelp: {
    id: "shortcutsHelp",
    key: "/",
    ctrlOrMeta: true,
    scope: "global",
    i18nKey: "shortcuts.shortcutsHelp",
  },
  copyUsername: {
    id: "copyUsername",
    key: "u",
    ctrlOrMeta: true,
    shift: true,
    scope: "entry",
    i18nKey: "shortcuts.copyUsername",
  },
  copyPassword: {
    id: "copyPassword",
    key: "c",
    ctrlOrMeta: true,
    shift: true,
    scope: "entry",
    i18nKey: "shortcuts.copyPassword",
  },
  openUrl: {
    id: "openUrl",
    key: "o",
    ctrlOrMeta: true,
    shift: true,
    scope: "entry",
    i18nKey: "shortcuts.openUrl",
  },
  editEntry: {
    id: "editEntry",
    key: "e",
    ctrlOrMeta: true,
    scope: "entry",
    i18nKey: "shortcuts.editEntry",
  },
  deleteEntry: {
    id: "deleteEntry",
    key: "Delete",
    aliases: ["Backspace"],
    ctrlOrMeta: false,
    scope: "entry",
    i18nKey: "shortcuts.deleteEntry",
  },
} as const satisfies Record<string, ShortcutDef>;

export type ShortcutId = keyof typeof SHORTCUTS;

export function matchesShortcut(e: KeyboardEvent, def: ShortcutDef): boolean {
  if (def.ctrlOrMeta && !(e.ctrlKey || e.metaKey)) return false;
  if (!def.ctrlOrMeta && (e.ctrlKey || e.metaKey)) return false;
  if (def.shift && !e.shiftKey) return false;
  if (!def.shift && e.shiftKey) return false;
  const eventKey = e.key.toLowerCase();
  if (eventKey === def.key.toLowerCase()) return true;
  return (def.aliases ?? []).some((alias) => eventKey === alias.toLowerCase());
}

export function formatShortcut(def: ShortcutDef): string {
  const parts: string[] = [];
  if (def.ctrlOrMeta) {
    parts.push(isMac ? "\u2318" : "Ctrl");
  }
  if (def.shift) {
    parts.push(isMac ? "\u21E7" : "Shift");
  }
  const keyLabel = KEY_LABELS[def.key] ?? def.key.toUpperCase();
  parts.push(keyLabel);
  return isMac ? parts.join("") : parts.join("+");
}

const KEY_LABELS: Record<string, string> = {
  ",": ",",
  "/": "?",
  Delete: isMac ? "\u232B" : "Del",
  ArrowDown: "↓",
  ArrowUp: "↑",
  Home: isMac ? "↖" : "Home",
  End: isMac ? "↘" : "End",
  Enter: "↵",
};

export function isInputTarget(e: KeyboardEvent): boolean {
  const tag = (e.target as HTMLElement)?.tagName;
  if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT") return true;
  if ((e.target as HTMLElement)?.isContentEditable) return true;
  return false;
}

export const LIST_NAV_SHORTCUTS: readonly ShortcutDef[] = [
  {
    id: "navigateDown",
    key: "ArrowDown",
    ctrlOrMeta: false,
    scope: "list",
    i18nKey: "shortcuts.navigateDown",
  },
  {
    id: "navigateUp",
    key: "ArrowUp",
    ctrlOrMeta: false,
    scope: "list",
    i18nKey: "shortcuts.navigateUp",
  },
  {
    id: "firstEntry",
    key: "Home",
    ctrlOrMeta: false,
    scope: "list",
    i18nKey: "shortcuts.firstEntry",
  },
  {
    id: "lastEntry",
    key: "End",
    ctrlOrMeta: false,
    scope: "list",
    i18nKey: "shortcuts.lastEntry",
  },
  {
    id: "activateEntry",
    key: "Enter",
    ctrlOrMeta: false,
    scope: "list",
    i18nKey: "shortcuts.activateEntry",
  },
];

export const SHORTCUT_GROUPS = {
  global: [
    SHORTCUTS.search,
    SHORTCUTS.newEntry,
    SHORTCUTS.save,
    SHORTCUTS.lockDatabase,
    SHORTCUTS.settings,
    SHORTCUTS.toggleSidebar,
    SHORTCUTS.shortcutsHelp,
  ],
  entry: [
    SHORTCUTS.copyUsername,
    SHORTCUTS.copyPassword,
    SHORTCUTS.openUrl,
    SHORTCUTS.editEntry,
    SHORTCUTS.deleteEntry,
  ],
  list: LIST_NAV_SHORTCUTS,
} as const;
