// SPDX-License-Identifier: MIT

import { afterEach, describe, expect, it, vi } from "vitest";
import { fireEvent, renderHook } from "@testing-library/react";
import { useShortcut } from "../use-shortcut";
import type { ShortcutDef } from "@/lib/shortcuts";

const ctrlK: ShortcutDef = {
  id: "test",
  key: "k",
  ctrlOrMeta: true,
  scope: "global",
  i18nKey: "test",
};

const shiftCtrlU: ShortcutDef = {
  id: "testShift",
  key: "u",
  ctrlOrMeta: true,
  shift: true,
  scope: "entry",
  i18nKey: "test",
};

const deleteKey: ShortcutDef = {
  id: "testDelete",
  key: "Delete",
  ctrlOrMeta: false,
  scope: "entry",
  i18nKey: "test",
};

describe("useShortcut", () => {
  afterEach(() => {
    vi.clearAllMocks();
  });

  it("triggers callback on matching key combo", () => {
    const callback = vi.fn();
    renderHook(() => useShortcut(ctrlK, callback, true));

    fireEvent.keyDown(window, { key: "k", ctrlKey: true });
    expect(callback).toHaveBeenCalledTimes(1);
  });

  it("triggers on metaKey (macOS)", () => {
    const callback = vi.fn();
    renderHook(() => useShortcut(ctrlK, callback, true));

    fireEvent.keyDown(window, { key: "k", metaKey: true });
    expect(callback).toHaveBeenCalledTimes(1);
  });

  it("does not trigger without modifier when ctrlOrMeta is true", () => {
    const callback = vi.fn();
    renderHook(() => useShortcut(ctrlK, callback, true));

    fireEvent.keyDown(window, { key: "k" });
    expect(callback).not.toHaveBeenCalled();
  });

  it("does not trigger when disabled", () => {
    const callback = vi.fn();
    renderHook(() => useShortcut(ctrlK, callback, false));

    fireEvent.keyDown(window, { key: "k", ctrlKey: true });
    expect(callback).not.toHaveBeenCalled();
  });

  it("removes listeners on unmount", () => {
    const callback = vi.fn();
    const { unmount } = renderHook(() => useShortcut(ctrlK, callback, true));

    unmount();
    fireEvent.keyDown(window, { key: "k", ctrlKey: true });
    expect(callback).not.toHaveBeenCalled();
  });

  it("supports shift modifier shortcuts", () => {
    const callback = vi.fn();
    renderHook(() => useShortcut(shiftCtrlU, callback, true));

    fireEvent.keyDown(window, { key: "u", ctrlKey: true, shiftKey: true });
    expect(callback).toHaveBeenCalledTimes(1);
  });

  it("does not trigger shift shortcut without shift", () => {
    const callback = vi.fn();
    renderHook(() => useShortcut(shiftCtrlU, callback, true));

    fireEvent.keyDown(window, { key: "u", ctrlKey: true });
    expect(callback).not.toHaveBeenCalled();
  });

  it("supports non-modifier shortcuts like Delete", () => {
    const callback = vi.fn();
    renderHook(() => useShortcut(deleteKey, callback, true));

    fireEvent.keyDown(window, { key: "Delete" });
    expect(callback).toHaveBeenCalledTimes(1);
  });

  it("skips non-modifier shortcuts when input is focused", () => {
    const callback = vi.fn();
    renderHook(() => useShortcut(deleteKey, callback, true));

    const input = document.createElement("input");
    document.body.append(input);

    fireEvent.keyDown(input, { key: "Delete" });
    expect(callback).not.toHaveBeenCalled();

    input.remove();
  });

  it("still fires modifier shortcuts when input is focused", () => {
    const callback = vi.fn();
    renderHook(() => useShortcut(ctrlK, callback, true));

    const input = document.createElement("input");
    document.body.append(input);

    fireEvent.keyDown(input, { key: "k", ctrlKey: true });
    expect(callback).toHaveBeenCalledTimes(1);

    input.remove();
  });
});
