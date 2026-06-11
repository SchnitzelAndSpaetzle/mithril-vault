// SPDX-License-Identifier: MIT

import { beforeEach, describe, expect, it, vi } from "vitest";
import { act, renderHook } from "@testing-library/react";
import { type RefObject } from "react";

import { useAttachmentDrop } from "../use-attachment-drop";

// Capture the handler the hook registers for the native drag-drop event so the
// tests can fire synthetic enter/over/leave/drop events at it. This overrides
// the inert default mock in the global test setup.
const dragDrop = vi.hoisted(() => ({
  handler: null as ((event: { payload: unknown }) => void) | null,
}));
vi.mock("@tauri-apps/api/webview", () => ({
  getCurrentWebview: () => ({
    onDragDropEvent: (handler: (event: { payload: unknown }) => void) => {
      dragDrop.handler = handler;
      return Promise.resolve(() => {
        dragDrop.handler = null;
      });
    },
  }),
}));

// A 100x100 panel anchored at the origin: a position at (50,50) lands inside,
// (500,500) outside.
function makePanelRef(): RefObject<HTMLElement | null> {
  const el = document.createElement("div");
  vi.spyOn(el, "getBoundingClientRect").mockReturnValue({
    left: 0,
    top: 0,
    right: 100,
    bottom: 100,
    width: 100,
    height: 100,
    x: 0,
    y: 0,
    toJSON: () => ({}),
  });
  return { current: el };
}

function fire(payload: unknown) {
  act(() => {
    dragDrop.handler?.({ payload });
  });
}

const INSIDE = { x: 50, y: 50 };
const OUTSIDE = { x: 500, y: 500 };

describe("useAttachmentDrop", () => {
  beforeEach(() => {
    dragDrop.handler = null;
    window.devicePixelRatio = 1;
  });

  it("highlights while a drag hovers inside the panel and clears on leave", () => {
    const panelRef = makePanelRef();
    const { result } = renderHook(() =>
      useAttachmentDrop({ enabled: true, panelRef, onDrop: vi.fn() })
    );

    fire({ type: "over", position: INSIDE });
    expect(result.current.isDragOver).toBe(true);

    fire({ type: "leave" });
    expect(result.current.isDragOver).toBe(false);
  });

  it("does not highlight when the drag hovers outside the panel", () => {
    const panelRef = makePanelRef();
    const { result } = renderHook(() =>
      useAttachmentDrop({ enabled: true, panelRef, onDrop: vi.fn() })
    );

    fire({ type: "enter", position: OUTSIDE });
    expect(result.current.isDragOver).toBe(false);
  });

  it("invokes onDrop for a drop inside the panel and clears the highlight", () => {
    const panelRef = makePanelRef();
    const onDrop = vi.fn();
    const { result } = renderHook(() =>
      useAttachmentDrop({ enabled: true, panelRef, onDrop })
    );

    fire({ type: "over", position: INSIDE });
    fire({ type: "drop", position: INSIDE });

    expect(onDrop).toHaveBeenCalledTimes(1);
    expect(result.current.isDragOver).toBe(false);
  });

  it("ignores a drop that lands outside the panel", () => {
    const panelRef = makePanelRef();
    const onDrop = vi.fn();
    renderHook(() => useAttachmentDrop({ enabled: true, panelRef, onDrop }));

    fire({ type: "drop", position: OUTSIDE });

    expect(onDrop).not.toHaveBeenCalled();
  });

  it("no-ops entirely while disabled", () => {
    const panelRef = makePanelRef();
    const onDrop = vi.fn();
    const { result } = renderHook(() =>
      useAttachmentDrop({ enabled: false, panelRef, onDrop })
    );

    fire({ type: "over", position: INSIDE });
    fire({ type: "drop", position: INSIDE });

    expect(result.current.isDragOver).toBe(false);
    expect(onDrop).not.toHaveBeenCalled();
  });

  it("treats a missing panel element as outside", () => {
    const onDrop = vi.fn();
    const panelRef: RefObject<HTMLElement | null> = { current: null };
    const { result } = renderHook(() =>
      useAttachmentDrop({ enabled: true, panelRef, onDrop })
    );

    fire({ type: "drop", position: INSIDE });

    expect(onDrop).not.toHaveBeenCalled();
    expect(result.current.isDragOver).toBe(false);
  });

  it("scales the physical drop position by the device pixel ratio", () => {
    // On a 2x display the native event reports physical pixels: a CSS-(50,50)
    // hit inside the 100x100 panel arrives as physical (100,100).
    window.devicePixelRatio = 2;
    const panelRef = makePanelRef();
    const onDrop = vi.fn();
    renderHook(() => useAttachmentDrop({ enabled: true, panelRef, onDrop }));

    fire({ type: "drop", position: { x: 100, y: 100 } });
    expect(onDrop).toHaveBeenCalledTimes(1);

    // The same physical point would fall outside without the scaling.
    fire({ type: "drop", position: { x: 300, y: 300 } });
    expect(onDrop).toHaveBeenCalledTimes(1);
  });
});
