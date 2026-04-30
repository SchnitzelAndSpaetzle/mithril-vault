// SPDX-License-Identifier: MIT

import { beforeEach, describe, expect, it, vi } from "vitest";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderHook, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { useWindowProtection } from "@/hooks/use-window-protection";
import { windowProtection } from "@/lib/tauri";

vi.mock("@/lib/tauri", () => ({
  windowProtection: {
    isSupported: vi.fn(),
  },
}));

const mockUseAppPreferences = vi.fn();

vi.mock("@/hooks/use-app-preferences", () => ({
  useAppPreferences: () => mockUseAppPreferences(),
}));

function createWrapper() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  const Wrapper = ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  );
  Wrapper.displayName = "Wrapper";
  return Wrapper;
}

describe("useWindowProtection", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("reports enabled from preferences and supported from backend", async () => {
    mockUseAppPreferences.mockReturnValue({
      preferences: { security: { preventScreenCapture: true } },
    });
    vi.mocked(windowProtection.isSupported).mockResolvedValue(true);

    const { result } = renderHook(() => useWindowProtection(), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(result.current.isSupported).toBe(true);
    });
    expect(result.current.enabled).toBe(true);
  });

  it("reports enabled false when preferences are null", () => {
    mockUseAppPreferences.mockReturnValue({ preferences: null });
    vi.mocked(windowProtection.isSupported).mockResolvedValue(false);

    const { result } = renderHook(() => useWindowProtection(), {
      wrapper: createWrapper(),
    });

    expect(result.current.enabled).toBe(false);
    expect(result.current.isSupported).toBe(false);
  });

  it("reports unsupported when backend returns false", async () => {
    mockUseAppPreferences.mockReturnValue({
      preferences: { security: { preventScreenCapture: false } },
    });
    vi.mocked(windowProtection.isSupported).mockResolvedValue(false);

    const { result } = renderHook(() => useWindowProtection(), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(windowProtection.isSupported).toHaveBeenCalled();
    });
    expect(result.current.enabled).toBe(false);
    expect(result.current.isSupported).toBe(false);
  });
});
