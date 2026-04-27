// SPDX-License-Identifier: MIT

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { renderHook, act } from "@testing-library/react";
import { useAutoLock } from "../use-auto-lock";

const mockReportActivity = vi.fn().mockResolvedValue(undefined);
const mockListen = vi.fn();
const mockNavigate = vi.fn();

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
  const lockTab = vi.fn();
  const store = Object.assign(
    (selector: (state: unknown) => unknown) =>
      selector({ lockTab, tabs: [], activeTabId: null }),
    {
      getState: () => ({ tabs: [], activeTabId: null }),
    }
  );
  return { useDatabaseTabs: store };
});

describe("useAutoLock", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    mockReportActivity.mockClear();
    mockNavigate.mockClear();
    mockListen.mockReturnValue(Promise.resolve(vi.fn()));
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
});
