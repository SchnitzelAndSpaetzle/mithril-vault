// SPDX-License-Identifier: MIT

import { afterEach, describe, expect, it, vi } from "vitest";
import { fireEvent, renderHook } from "@testing-library/react";
import { useSearchShortcut } from "../use-search-shortcut";

describe("useSearchShortcut", () => {
  afterEach(() => {
    vi.clearAllMocks();
  });

  it("triggers callback on Ctrl+K and Cmd+K", () => {
    const callback = vi.fn();
    renderHook(() => useSearchShortcut(callback, true));

    fireEvent.keyDown(window, { key: "k", ctrlKey: true });
    fireEvent.keyDown(window, { key: "k", metaKey: true });

    expect(callback).toHaveBeenCalledTimes(2);
  });

  it("triggers callback on slash outside form controls", () => {
    const callback = vi.fn();
    renderHook(() => useSearchShortcut(callback, true));

    fireEvent.keyDown(window, { key: "/" });

    expect(callback).toHaveBeenCalledTimes(1);
  });

  it("does not trigger on slash when input-like targets are focused", () => {
    const callback = vi.fn();
    renderHook(() => useSearchShortcut(callback, true));

    const input = document.createElement("input");
    const textarea = document.createElement("textarea");
    const select = document.createElement("select");

    document.body.append(input, textarea, select);

    fireEvent.keyDown(input, { key: "/" });
    fireEvent.keyDown(textarea, { key: "/" });
    fireEvent.keyDown(select, { key: "/" });

    expect(callback).not.toHaveBeenCalled();
    input.remove();
    textarea.remove();
    select.remove();
  });

  it("does not trigger on slash for content-editable targets", () => {
    const callback = vi.fn();
    renderHook(() => useSearchShortcut(callback, true));

    const event = new KeyboardEvent("keydown", {
      key: "/",
      bubbles: true,
      cancelable: true,
    });

    Object.defineProperty(event, "target", {
      value: { tagName: "DIV", isContentEditable: true },
      configurable: true,
    });

    window.dispatchEvent(event);
    expect(callback).not.toHaveBeenCalled();
  });

  it("does nothing when shortcut handling is disabled", () => {
    const callback = vi.fn();
    renderHook(() => useSearchShortcut(callback, false));

    fireEvent.keyDown(window, { key: "k", ctrlKey: true });
    fireEvent.keyDown(window, { key: "/" });

    expect(callback).not.toHaveBeenCalled();
  });

  it("removes listeners on unmount", () => {
    const callback = vi.fn();
    const { unmount } = renderHook(() => useSearchShortcut(callback, true));

    unmount();
    fireEvent.keyDown(window, { key: "k", ctrlKey: true });

    expect(callback).not.toHaveBeenCalled();
  });
});
