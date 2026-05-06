// SPDX-License-Identifier: MIT

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { renderHook, act } from "@testing-library/react";
import { useAutoLock } from "../use-auto-lock";

const mockReportActivity = vi.fn().mockResolvedValue(undefined);
const mockListen = vi.fn();
const mockNavigate = vi.fn();
const mockLockTab = vi.fn();
let mockTabs: unknown[] = [];
let mockActiveTabId: string | null = null;

vi.mock("@/lib/tauri", () => ({
  database: {
    reportActivity: (...args: unknown[]) => mockReportActivity(...args),
  },
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: (...args: unknown[]) => mockListen(...args),
}));

vi.mock("@tanstack/react-router", () => ({
  useNavigate: () => mockNavigate,
}));

vi.mock("@/stores/database-tabs", () => {
  const store = Object.assign(
    (selector: (state: unknown) => unknown) =>
      selector({
        lockTab: mockLockTab,
        tabs: mockTabs,
        activeTabId: mockActiveTabId,
      }),
    {
      getState: () => ({ tabs: mockTabs, activeTabId: mockActiveTabId }),
    }
  );
  return { useDatabaseTabs: store };
});

describe("useAutoLock", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    mockReportActivity.mockClear();
    mockNavigate.mockClear();
    mockLockTab.mockClear();
    mockListen.mockReturnValue(Promise.resolve(vi.fn()));
    mockTabs = [];
    mockActiveTabId = null;
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("reports activity on mount", () => {
    renderHook(() => useAutoLock());
    expect(mockReportActivity).toHaveBeenCalledTimes(1);
  });

  it("registers event listeners for activity", () => {
    const addSpy = vi.spyOn(window, "addEventListener");
    renderHook(() => useAutoLock());

    const registeredEvents = addSpy.mock.calls.map((call) => call[0]);
    expect(registeredEvents).toContain("mousemove");
    expect(registeredEvents).toContain("keydown");
    expect(registeredEvents).toContain("click");
    expect(registeredEvents).toContain("scroll");
    expect(registeredEvents).toContain("touchstart");

    addSpy.mockRestore();
  });

  it("throttles activity reports to once per 30 seconds", () => {
    renderHook(() => useAutoLock());
    expect(mockReportActivity).toHaveBeenCalledTimes(1);

    // Simulate activity events
    act(() => {
      window.dispatchEvent(new Event("mousemove"));
      window.dispatchEvent(new Event("keydown"));
      window.dispatchEvent(new Event("click"));
    });

    // Still only 1 call due to throttle
    expect(mockReportActivity).toHaveBeenCalledTimes(1);

    // Advance past throttle window
    act(() => {
      vi.advanceTimersByTime(30_000);
    });

    act(() => {
      window.dispatchEvent(new Event("mousemove"));
    });

    expect(mockReportActivity).toHaveBeenCalledTimes(2);
  });

  it("listens for database-locked event", () => {
    renderHook(() => useAutoLock());
    expect(mockListen).toHaveBeenCalledWith(
      "database-locked",
      expect.any(Function)
    );
  });

  it("cleans up event listeners on unmount", () => {
    const removeSpy = vi.spyOn(window, "removeEventListener");
    const { unmount } = renderHook(() => useAutoLock());

    unmount();

    const removedEvents = removeSpy.mock.calls.map((call) => call[0]);
    expect(removedEvents).toContain("mousemove");
    expect(removedEvents).toContain("keydown");
    expect(removedEvents).toContain("click");
    expect(removedEvents).toContain("scroll");
    expect(removedEvents).toContain("touchstart");

    removeSpy.mockRestore();
  });

  it("redirects to unlock with active database path when active tab is locked", () => {
    let onDatabaseLocked: ((event: { payload: string[] }) => void) | undefined;
    mockListen.mockImplementation((_event, callback) => {
      onDatabaseLocked = callback as (event: { payload: string[] }) => void;
      return Promise.resolve(vi.fn());
    });

    mockTabs = [
      {
        id: "tab-1",
        dbId: "/tmp/test.kdbx",
        path: "/tmp/test.kdbx",
      },
    ];
    mockActiveTabId = "tab-1";

    renderHook(() => useAutoLock());

    act(() => {
      onDatabaseLocked?.({ payload: ["/tmp/test.kdbx"] });
    });

    expect(mockLockTab).toHaveBeenCalledWith("tab-1");
    expect(mockNavigate).toHaveBeenCalledWith({
      to: "/unlock",
      search: { path: "/tmp/test.kdbx" },
    });
  });
});
