// SPDX-License-Identifier: MIT

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { act, renderHook } from "@testing-library/react";
import { useClipboardCountdown } from "../use-clipboard-countdown";
import { toast } from "sonner";

vi.mock("sonner", () => ({
  toast: { success: vi.fn(), dismiss: vi.fn() },
}));

vi.mock("@/hooks/use-app-preferences", () => ({
  useAppPreferences: vi.fn(() => ({
    preferences: {
      security: {
        showClipboardCountdown: true,
      },
    },
  })),
}));

describe("useClipboardCountdown", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.runOnlyPendingTimers();
    vi.useRealTimers();
  });

  it("shows toast with countdown when started", () => {
    const { result } = renderHook(() => useClipboardCountdown());
    act(() => {
      result.current(10);
    });
    expect(toast.success).toHaveBeenCalledWith(
      "clipboard.countdown",
      expect.objectContaining({ id: "clipboard-countdown", duration: Infinity })
    );
  });

  it("updates toast each second", () => {
    const { result } = renderHook(() => useClipboardCountdown());
    act(() => {
      result.current(3);
    });
    expect(toast.success).toHaveBeenCalledTimes(1);

    act(() => {
      vi.advanceTimersByTime(1000);
    });
    expect(toast.success).toHaveBeenCalledTimes(2);

    act(() => {
      vi.advanceTimersByTime(1000);
    });
    expect(toast.success).toHaveBeenCalledTimes(3);
  });

  it("dismisses toast when countdown reaches zero", () => {
    const { result } = renderHook(() => useClipboardCountdown());
    act(() => {
      result.current(2);
    });

    act(() => {
      vi.advanceTimersByTime(2000);
    });
    expect(toast.dismiss).toHaveBeenCalledWith("clipboard-countdown");
  });

  it("restarts countdown when called again", () => {
    const { result } = renderHook(() => useClipboardCountdown());
    act(() => {
      result.current(5);
    });
    act(() => {
      vi.advanceTimersByTime(2000);
    });

    vi.clearAllMocks();
    act(() => {
      result.current(3);
    });
    expect(toast.success).toHaveBeenCalledTimes(1);
  });

  it("uses a single active countdown across multiple hook instances", () => {
    const { result: first } = renderHook(() => useClipboardCountdown());
    const { result: second } = renderHook(() => useClipboardCountdown());

    act(() => {
      first.current(5);
    });
    expect(toast.success).toHaveBeenCalledTimes(1);

    act(() => {
      vi.advanceTimersByTime(1000);
    });
    expect(toast.success).toHaveBeenCalledTimes(2);

    act(() => {
      second.current(3);
    });
    expect(toast.success).toHaveBeenCalledTimes(3);

    act(() => {
      vi.advanceTimersByTime(1000);
    });
    expect(toast.success).toHaveBeenCalledTimes(4);
  });

  it("does nothing when showClipboardCountdown is disabled", async () => {
    const { useAppPreferences } = await import("@/hooks/use-app-preferences");
    vi.mocked(useAppPreferences).mockReturnValue({
      preferences: {
        security: { showClipboardCountdown: false },
      },
    } as ReturnType<typeof useAppPreferences>);

    const { result } = renderHook(() => useClipboardCountdown());
    act(() => {
      result.current(10);
    });
    expect(toast.success).not.toHaveBeenCalled();
  });
});
