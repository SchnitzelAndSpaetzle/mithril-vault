// SPDX-License-Identifier: MIT

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { act, renderHook } from "@testing-library/react";
import { useCopyToClipboard } from "../use-copy-to-clipboard";

const writeText = vi.fn();

describe("useCopyToClipboard", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    writeText.mockReset();
    writeText.mockResolvedValue(undefined);

    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: { writeText },
    });
  });

  afterEach(() => {
    vi.runOnlyPendingTimers();
    vi.useRealTimers();
  });

  it("copies text and resets copied state after timeout", async () => {
    const { result } = renderHook(() => useCopyToClipboard());

    await act(async () => {
      await result.current.copy("hello");
    });

    expect(writeText).toHaveBeenCalledWith("hello");
    expect(result.current.isCopied).toBe(true);

    act(() => {
      vi.advanceTimersByTime(2000);
    });

    expect(result.current.isCopied).toBe(false);
  });

  it("restarts timeout when copied multiple times quickly", async () => {
    const { result } = renderHook(() => useCopyToClipboard());

    await act(async () => {
      await result.current.copy("first");
    });

    act(() => {
      vi.advanceTimersByTime(1500);
    });

    await act(async () => {
      await result.current.copy("second");
    });

    act(() => {
      vi.advanceTimersByTime(600);
    });
    expect(result.current.isCopied).toBe(true);

    act(() => {
      vi.advanceTimersByTime(1400);
    });
    expect(result.current.isCopied).toBe(false);

    expect(writeText).toHaveBeenNthCalledWith(1, "first");
    expect(writeText).toHaveBeenNthCalledWith(2, "second");
  });

  it("propagates clipboard write errors and keeps copied state false", async () => {
    writeText.mockRejectedValueOnce(new Error("clipboard denied"));
    const { result } = renderHook(() => useCopyToClipboard());

    await expect(result.current.copy("secret")).rejects.toThrow(
      "clipboard denied"
    );
    expect(result.current.isCopied).toBe(false);
  });
});
