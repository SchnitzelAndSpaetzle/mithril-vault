// SPDX-License-Identifier: MIT

import { beforeEach, describe, expect, it, vi } from "vitest";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderHook, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { useAppPreferences } from "@/hooks/use-app-preferences";
import { settings, windowProtection } from "@/lib/tauri";
import type { AppPreferences } from "@/lib/types";

vi.mock("@/lib/tauri", () => ({
  settings: {
    getPreferences: vi.fn(),
    updatePreferences: vi.fn(),
    resetPreferences: vi.fn(),
  },
  windowProtection: {
    setProtected: vi.fn(),
    isSupported: vi.fn(),
  },
}));

function makePreferences(): AppPreferences {
  return {
    general: {
      language: "en",
      startupBehavior: "showUnlockScreen",
      defaultDatabasePath: null,
    },
    security: {
      autoLockTimeout: 300,
      clipboardClearTimeout: 30,
      clearClipboardOnLock: true,
      showClipboardCountdown: false,
      showPasswordByDefault: false,
      minimizeToTray: true,
      startMinimized: false,
      preventScreenCapture: true,
      autoDownloadFavicons: false,
      allowThirdPartyFaviconFallbacks: false,
    },
    appearance: {
      theme: "system",
      colorPreset: "default",
      fontSize: 14,
      entryListColumns: {
        username: true,
        url: true,
        modifiedAt: true,
        tags: true,
      },
    },
    browserIntegration: {
      enabled: false,
      allowedSites: [],
    },
    advanced: {
      debugMode: false,
      dataLocation: "/tmp/mithril-vault",
    },
    backups: {
      enabled: true,
    },
  };
}

function createWrapper() {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });

  const QueryWrapper = ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  );

  QueryWrapper.displayName = "QueryWrapper";

  return QueryWrapper;
}

describe("useAppPreferences", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("loads preferences", async () => {
    const preferences = makePreferences();
    vi.mocked(settings.getPreferences).mockResolvedValue(preferences);

    const { result } = renderHook(() => useAppPreferences(), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(result.current.preferences).toEqual(preferences);
    });
    expect(settings.getPreferences).toHaveBeenCalledTimes(1);
  });

  it("updates preferences", async () => {
    const preferences = makePreferences();
    vi.mocked(settings.getPreferences).mockResolvedValue(preferences);
    vi.mocked(settings.updatePreferences).mockResolvedValue(undefined);

    const { result } = renderHook(() => useAppPreferences(), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(result.current.preferences).not.toBeNull();
    });

    await result.current.updatePreferences(preferences);

    expect(settings.updatePreferences).toHaveBeenCalledWith(preferences);
    expect(windowProtection.setProtected).not.toHaveBeenCalled();
  });

  it("invokes window protection when preventScreenCapture changes", async () => {
    const preferences = makePreferences();
    vi.mocked(settings.getPreferences).mockResolvedValue(preferences);
    vi.mocked(settings.updatePreferences).mockResolvedValue(undefined);
    vi.mocked(windowProtection.setProtected).mockResolvedValue(undefined);

    const { result } = renderHook(() => useAppPreferences(), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(result.current.preferences).not.toBeNull();
    });

    const next: AppPreferences = {
      ...preferences,
      security: { ...preferences.security, preventScreenCapture: false },
    };
    await result.current.updatePreferences(next);

    expect(windowProtection.setProtected).toHaveBeenCalledWith(false);
  });

  it("keeps update successful when runtime window protection apply fails", async () => {
    const preferences = makePreferences();
    vi.mocked(settings.getPreferences).mockResolvedValue(preferences);
    vi.mocked(settings.updatePreferences).mockResolvedValue(undefined);
    vi.mocked(windowProtection.setProtected).mockRejectedValue(
      new Error("window protection failed")
    );
    const consoleWarn = vi
      .spyOn(console, "warn")
      .mockImplementation(() => undefined);

    const { result } = renderHook(() => useAppPreferences(), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(result.current.preferences).not.toBeNull();
    });

    const next: AppPreferences = {
      ...preferences,
      security: { ...preferences.security, preventScreenCapture: false },
    };

    await expect(
      result.current.updatePreferences(next)
    ).resolves.toBeUndefined();

    expect(settings.updatePreferences).toHaveBeenCalledWith(next);
    expect(windowProtection.setProtected).toHaveBeenCalledWith(false);
    expect(consoleWarn).toHaveBeenCalledWith(
      "Failed to apply window content protection:",
      expect.any(Error)
    );

    consoleWarn.mockRestore();
  });

  it("resets preferences", async () => {
    const preferences = makePreferences();
    const resetPreferences = {
      ...preferences,
      appearance: {
        ...preferences.appearance,
        theme: "light" as const,
      },
    };

    vi.mocked(settings.getPreferences).mockResolvedValue(preferences);
    vi.mocked(settings.resetPreferences).mockResolvedValue(resetPreferences);

    const { result } = renderHook(() => useAppPreferences(), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(result.current.preferences).toEqual(preferences);
    });

    const reset = await result.current.resetPreferences();

    expect(settings.resetPreferences).toHaveBeenCalledTimes(1);
    expect(reset).toEqual(resetPreferences);
  });

  it("keeps reset successful when runtime window protection apply fails", async () => {
    const preferences = makePreferences();
    const resetPreferences = {
      ...preferences,
      security: { ...preferences.security, preventScreenCapture: false },
    };
    vi.mocked(settings.getPreferences).mockResolvedValue(preferences);
    vi.mocked(settings.resetPreferences).mockResolvedValue(resetPreferences);
    vi.mocked(windowProtection.setProtected).mockRejectedValue(
      new Error("window protection failed")
    );
    const consoleWarn = vi
      .spyOn(console, "warn")
      .mockImplementation(() => undefined);

    const { result } = renderHook(() => useAppPreferences(), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(result.current.preferences).toEqual(preferences);
    });

    await expect(result.current.resetPreferences()).resolves.toEqual(
      resetPreferences
    );

    expect(windowProtection.setProtected).toHaveBeenCalledWith(false);
    expect(consoleWarn).toHaveBeenCalledWith(
      "Failed to apply window content protection:",
      expect.any(Error)
    );

    consoleWarn.mockRestore();
  });
});
