// SPDX-License-Identifier: MIT

import { describe, expect, it } from "vitest";
import {
  formatShortcut,
  isInputTarget,
  matchesShortcut,
  SHORTCUT_GROUPS,
  SHORTCUTS,
} from "../shortcuts";

describe("matchesShortcut", () => {
  it("matches ctrl+key combo", () => {
    const e = new KeyboardEvent("keydown", { key: "k", ctrlKey: true });
    expect(matchesShortcut(e, SHORTCUTS.search)).toBe(true);
  });

  it("matches meta+key combo", () => {
    const e = new KeyboardEvent("keydown", { key: "k", metaKey: true });
    expect(matchesShortcut(e, SHORTCUTS.search)).toBe(true);
  });

  it("rejects when modifier missing", () => {
    const e = new KeyboardEvent("keydown", { key: "k" });
    expect(matchesShortcut(e, SHORTCUTS.search)).toBe(false);
  });

  it("rejects wrong key", () => {
    const e = new KeyboardEvent("keydown", { key: "j", ctrlKey: true });
    expect(matchesShortcut(e, SHORTCUTS.search)).toBe(false);
  });

  it("matches shift+ctrl combo", () => {
    const e = new KeyboardEvent("keydown", {
      key: "c",
      ctrlKey: true,
      shiftKey: true,
    });
    expect(matchesShortcut(e, SHORTCUTS.copyPassword)).toBe(true);
  });

  it("rejects shift combo without shift pressed", () => {
    const e = new KeyboardEvent("keydown", { key: "c", ctrlKey: true });
    expect(matchesShortcut(e, SHORTCUTS.copyPassword)).toBe(false);
  });

  it("matches non-modifier shortcut", () => {
    const e = new KeyboardEvent("keydown", { key: "Delete" });
    expect(matchesShortcut(e, SHORTCUTS.deleteEntry)).toBe(true);
  });

  it("rejects non-modifier shortcut if modifier is pressed", () => {
    const e = new KeyboardEvent("keydown", { key: "Delete", ctrlKey: true });
    expect(matchesShortcut(e, SHORTCUTS.deleteEntry)).toBe(false);
  });

  it("is case-insensitive for key matching", () => {
    const e = new KeyboardEvent("keydown", { key: "K", ctrlKey: true });
    expect(matchesShortcut(e, SHORTCUTS.search)).toBe(true);
  });
});

describe("formatShortcut", () => {
  it("formats a ctrl+key shortcut", () => {
    const formatted = formatShortcut(SHORTCUTS.search);
    expect(formatted).toMatch(/K/i);
    expect(formatted).toMatch(/Ctrl|\u2318/);
  });

  it("formats a shift+ctrl shortcut", () => {
    const formatted = formatShortcut(SHORTCUTS.copyPassword);
    expect(formatted).toMatch(/C/i);
    expect(formatted).toMatch(/Shift|\u21E7/);
  });

  it("formats delete key", () => {
    const formatted = formatShortcut(SHORTCUTS.deleteEntry);
    expect(formatted).toMatch(/Del|\u232B/);
  });
});

describe("isInputTarget", () => {
  it("returns true for INPUT elements", () => {
    const input = document.createElement("input");
    const e = new KeyboardEvent("keydown", { bubbles: true });
    Object.defineProperty(e, "target", { value: input });
    expect(isInputTarget(e)).toBe(true);
  });

  it("returns true for TEXTAREA elements", () => {
    const textarea = document.createElement("textarea");
    const e = new KeyboardEvent("keydown", { bubbles: true });
    Object.defineProperty(e, "target", { value: textarea });
    expect(isInputTarget(e)).toBe(true);
  });

  it("returns true for contentEditable elements", () => {
    const e = new KeyboardEvent("keydown", { bubbles: true });
    Object.defineProperty(e, "target", {
      value: { tagName: "DIV", isContentEditable: true },
    });
    expect(isInputTarget(e)).toBe(true);
  });

  it("returns false for regular elements", () => {
    const div = document.createElement("div");
    const e = new KeyboardEvent("keydown", { bubbles: true });
    Object.defineProperty(e, "target", { value: div });
    expect(isInputTarget(e)).toBe(false);
  });
});

describe("SHORTCUTS", () => {
  it("has unique ids for all shortcuts", () => {
    const ids = Object.values(SHORTCUTS).map((s) => s.id);
    expect(new Set(ids).size).toBe(ids.length);
  });

  it("has unique key combos", () => {
    const combos = Object.values(SHORTCUTS).map(
      (s) => `${s.ctrlOrMeta}-${"shift" in s ? s.shift : false}-${s.key}`
    );
    expect(new Set(combos).size).toBe(combos.length);
  });
});

describe("SHORTCUT_GROUPS", () => {
  it("global group contains all global shortcuts", () => {
    const globalShortcuts = Object.values(SHORTCUTS).filter(
      (s) => s.scope === "global"
    );
    expect(SHORTCUT_GROUPS.global).toHaveLength(globalShortcuts.length);
  });

  it("entry group contains all entry shortcuts", () => {
    const entryShortcuts = Object.values(SHORTCUTS).filter(
      (s) => s.scope === "entry"
    );
    expect(SHORTCUT_GROUPS.entry).toHaveLength(entryShortcuts.length);
  });
});
