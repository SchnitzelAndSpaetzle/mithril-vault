// SPDX-License-Identifier: MIT

import { useEffect, useRef, useState, type RefObject } from "react";
import { getCurrentWebview } from "@tauri-apps/api/webview";

interface PhysicalPosition {
  x: number;
  y: number;
}

interface UseAttachmentDropArgs {
  /**
   * Whether drops should be acted on. The native drag-drop event is
   * window-global, so the listener is always registered, but it no-ops unless
   * enabled — desktop only, an Entry selected, and not mid-transition.
   */
  enabled: boolean;
  /** The Entry detail region a drop must land on to count (scoping). */
  panelRef: RefObject<HTMLElement | null>;
  /** Invoked once per drop that lands inside the panel while enabled. */
  onDrop: () => void;
}

/**
 * Wires the native `tauri://drag-drop` window event to a single Entry's detail
 * panel. The event is window-global, so this hook scopes it two ways: it acts
 * only while `enabled` (desktop, an Entry selected, not transitioning) and only
 * when the drop position falls inside `panelRef`. It never reads the dropped
 * paths — those are captured in Rust (ADR-0004); `onDrop` triggers the commit
 * command that drains the backend buffer. The returned `isDragOver` flag drives
 * the drop-zone highlight while a drag hovers the panel.
 */
export function useAttachmentDrop({
  enabled,
  panelRef,
  onDrop,
}: UseAttachmentDropArgs): { isDragOver: boolean } {
  const [isDragOver, setIsDragOver] = useState(false);

  // Keep the latest enabled/onDrop in refs so the listener is registered once
  // (on mount) yet always reads current values — re-registering the native
  // listener on every render would drop in-flight drag state.
  const enabledRef = useRef(enabled);
  const onDropRef = useRef(onDrop);
  useEffect(() => {
    enabledRef.current = enabled;
    onDropRef.current = onDrop;
  }, [enabled, onDrop]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let disposed = false;

    const register = getCurrentWebview().onDragDropEvent((event) => {
      const payload = event.payload as
        | { type: "enter" | "over"; position: PhysicalPosition }
        | { type: "drop"; position: PhysicalPosition }
        | { type: "leave" };

      if (!enabledRef.current) {
        setIsDragOver(false);
        return;
      }

      if (payload.type === "leave") {
        setIsDragOver(false);
        return;
      }

      const inside = isInsidePanel(panelRef.current, payload.position);

      if (payload.type === "drop") {
        setIsDragOver(false);
        if (inside) onDropRef.current();
        return;
      }

      // enter / over: highlight only while hovering the panel.
      setIsDragOver(inside);
    });

    void register.then((fn) => {
      if (disposed) {
        fn();
      } else {
        unlisten = fn;
      }
    });

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [panelRef]);

  return { isDragOver };
}

/**
 * Hit-tests a physical drop position against an element's box. The native event
 * reports physical pixels (window-relative); `getBoundingClientRect` is in CSS
 * pixels (viewport-relative), so we scale by the device pixel ratio. On the
 * frameless desktop window the webview fills the viewport, so the two origins
 * align.
 */
function isInsidePanel(
  el: HTMLElement | null,
  position: PhysicalPosition
): boolean {
  if (!el) return false;
  const rect = el.getBoundingClientRect();
  const dpr = window.devicePixelRatio || 1;
  const x = position.x / dpr;
  const y = position.y / dpr;
  return x >= rect.left && x <= rect.right && y >= rect.top && y <= rect.bottom;
}
