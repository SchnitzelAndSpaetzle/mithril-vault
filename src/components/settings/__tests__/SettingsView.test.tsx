// SPDX-License-Identifier: MIT

import { beforeEach, describe, expect, it, vi } from "vitest";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import {
  fireEvent,
  render as rtlRender,
  screen,
  waitFor,
} from "@testing-library/react";
import type { ReactElement } from "react";
import { SettingsView } from "@/views/SettingsView";
import type { AppPreferences } from "@/lib/types";

// SettingsView's BackupsListSection uses react-query; wrap every render in a
// fresh provider so the unmocked hook doesn't blow up. The wrapped rerender
// also preserves the provider — RTL's default rerender replaces children at
// the root, which would otherwise drop the QueryClientProvider.
function render(ui: ReactElement) {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false, gcTime: 0 },
      mutations: { retry: false },
    },
  });
  const result = rtlRender(
    <QueryClientProvider client={queryClient}>{ui}</QueryClientProvider>
  );
  return {
    ...result,
    rerender: (next: ReactElement) =>
      result.rerender(
        <QueryClientProvider client={queryClient}>{next}</QueryClientProvider>
      ),
  };
}

vi.mock("@/lib/tauri", () => ({
  backups: {
    list: vi.fn().mockResolvedValue([]),
    delete: vi.fn().mockResolvedValue(undefined),
  },
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => undefined),
}));

const mockUseAppPreferences = vi.fn();
const mockUseTheme = vi.fn();
const mockUseActiveDatabase = vi.fn();
const mockUseDatabaseConfig = vi.fn();
const toastSuccess = vi.fn();
const toastError = vi.fn();

vi.mock("@/hooks/use-app-preferences", () => ({
  useAppPreferences: () => mockUseAppPreferences(),
}));

vi.mock("@/hooks/use-theme", () => ({
  useTheme: () => mockUseTheme(),
}));

vi.mock("@/hooks/use-active-database", () => ({
  useActiveDatabase: () => mockUseActiveDatabase(),
}));

vi.mock("@/hooks/use-database-config", () => ({
  useDatabaseConfig: () => mockUseDatabaseConfig(),
}));

vi.mock("@/hooks/use-window-protection", () => ({
  useWindowProtection: () => ({ enabled: true, isSupported: true }),
}));

vi.mock("sonner", () => ({
  toast: {
    success: (message: string) => toastSuccess(message),
    error: (message: string) => toastError(message),
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
      allowedSites: ["example.com"],
    },
    advanced: {
      debugMode: false,
      dataLocation: "/mock/mithril-vault",
    },
    backups: {
      enabled: true,
      maxVersions: 10,
      onOpen: false,
    },
    audit: {
      enabled: true,
      retentionDays: 90,
    },
    attachments: { softWarnBytes: 5_000_000, hardCapBytes: 25_000_000 },
  };
}

describe("SettingsView", () => {
  beforeEach(() => {
    vi.clearAllMocks();

    mockUseTheme.mockReturnValue({
      theme: "system",
      setTheme: vi.fn(),
      setThemePreview: vi.fn(),
      colorPreset: "default",
      setColorPreset: vi.fn(),
      setColorPresetPreview: vi.fn(),
    });

    mockUseActiveDatabase.mockReturnValue({
      dbId: null,
      tab: null,
      isUnlocking: false,
    });

    mockUseDatabaseConfig.mockReturnValue({
      data: null,
      isLoading: false,
      error: null,
    });

    mockUseAppPreferences.mockReturnValue({
      preferences: makePreferences(),
      isLoading: false,
      error: null,
      updatePreferences: vi.fn().mockResolvedValue(undefined),
      isUpdating: false,
      resetPreferences: vi.fn().mockResolvedValue(makePreferences()),
      isResetting: false,
    });
  });

  it("renders settings sections", () => {
    render(<SettingsView />);

    expect(
      screen.getByRole("heading", { name: "settings.title" })
    ).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { name: "settings.general.title" })
    ).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { name: "settings.security.title" })
    ).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { name: "settings.appearance.title" })
    ).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { name: "settings.browser.title" })
    ).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { name: "settings.advanced.title" })
    ).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { name: "settings.backups.title" })
    ).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { name: "settings.database.title" })
    ).toBeInTheDocument();
  });

  it("persists backups.enabled toggle", async () => {
    const updatePreferences = vi.fn().mockResolvedValue(undefined);
    mockUseAppPreferences.mockReturnValue({
      preferences: makePreferences(),
      isLoading: false,
      error: null,
      updatePreferences,
      isUpdating: false,
      resetPreferences: vi.fn().mockResolvedValue(makePreferences()),
      isResetting: false,
    });

    render(<SettingsView />);

    const toggle = screen.getByLabelText("settings.backups.enabled.label");
    fireEvent.click(toggle);

    fireEvent.click(
      screen.getByRole("button", { name: "settings.saveChanges" })
    );

    await waitFor(() => {
      expect(updatePreferences).toHaveBeenCalledTimes(1);
    });

    expect(updatePreferences).toHaveBeenCalledWith(
      expect.objectContaining({
        backups: expect.objectContaining({ enabled: false }),
      })
    );
  });

  it("renders loading state", () => {
    mockUseAppPreferences.mockReturnValue({
      preferences: null,
      isLoading: true,
      error: null,
      updatePreferences: vi.fn().mockResolvedValue(undefined),
      isUpdating: false,
      resetPreferences: vi.fn().mockResolvedValue(makePreferences()),
      isResetting: false,
    });

    render(<SettingsView />);
    expect(screen.getByText("common.loading")).toBeInTheDocument();
  });

  it("renders top-level load error state", () => {
    mockUseAppPreferences.mockReturnValue({
      preferences: null,
      isLoading: false,
      error: new Error("boom"),
      updatePreferences: vi.fn().mockResolvedValue(undefined),
      isUpdating: false,
      resetPreferences: vi.fn().mockResolvedValue(makePreferences()),
      isResetting: false,
    });

    render(<SettingsView />);
    expect(screen.getByText("errors.failedToLoadSettings")).toBeInTheDocument();
  });

  it("saves updated browser allowed sites", async () => {
    const updatePreferences = vi.fn().mockResolvedValue(undefined);
    mockUseAppPreferences.mockReturnValue({
      preferences: makePreferences(),
      isLoading: false,
      error: null,
      updatePreferences,
      isUpdating: false,
      resetPreferences: vi.fn().mockResolvedValue(makePreferences()),
      isResetting: false,
    });

    render(<SettingsView />);

    const allowedSites = screen.getByLabelText("settings.browser.allowedSites");
    fireEvent.change(allowedSites, {
      target: { value: "example.com\nfoo.bar\n  \nsub.domain" },
    });

    fireEvent.click(
      screen.getByRole("button", { name: "settings.saveChanges" })
    );

    await waitFor(() => {
      expect(updatePreferences).toHaveBeenCalledTimes(1);
    });

    expect(updatePreferences).toHaveBeenCalledWith(
      expect.objectContaining({
        browserIntegration: expect.objectContaining({
          allowedSites: ["example.com", "foo.bar", "sub.domain"],
        }),
      })
    );
    expect(toastSuccess).toHaveBeenCalledWith("settings.toast.updated");
  });

  it("resets preferences to defaults", async () => {
    const resetPreferences = vi.fn().mockResolvedValue(makePreferences());
    const setTheme = vi.fn();
    mockUseTheme.mockReturnValue({
      theme: "dark",
      setTheme,
      setThemePreview: vi.fn(),
      colorPreset: "default",
      setColorPreset: vi.fn(),
      setColorPresetPreview: vi.fn(),
    });
    mockUseAppPreferences.mockReturnValue({
      preferences: makePreferences(),
      isLoading: false,
      error: null,
      updatePreferences: vi.fn().mockResolvedValue(undefined),
      isUpdating: false,
      resetPreferences,
      isResetting: false,
    });

    render(<SettingsView />);

    fireEvent.click(
      screen.getByRole("button", { name: "settings.resetDefaults" })
    );
    fireEvent.click(
      screen.getByRole("button", { name: "settings.resetPreferences" })
    );

    await waitFor(() => {
      expect(resetPreferences).toHaveBeenCalledTimes(1);
    });

    expect(setTheme).toHaveBeenCalledWith("system");
    expect(toastSuccess).toHaveBeenCalledWith("settings.toast.reset");
  });

  it("shows database config values and KDF details", () => {
    mockUseActiveDatabase.mockReturnValue({
      dbId: "db-1",
      tab: null,
      isUnlocking: false,
    });

    mockUseDatabaseConfig.mockReturnValue({
      data: {
        version: "KDBX 4.1",
        outerCipher: "aes256",
        innerCipher: "chaCha20",
        compression: "gZip",
        kdf: {
          type: "argon2id",
          memory: 65536,
          iterations: 4,
          parallelism: 2,
        },
      },
      isLoading: false,
      error: null,
    });

    render(<SettingsView />);

    expect(screen.getByText("KDBX 4.1")).toBeInTheDocument();
    expect(screen.getByText("aes256")).toBeInTheDocument();
    expect(
      screen.getByText("argon2id (65536 bytes, 4 iterations, 2 lanes)")
    ).toBeInTheDocument();
  });

  it("renders database loading and error states", () => {
    mockUseActiveDatabase.mockReturnValue({
      dbId: "db-1",
      tab: null,
      isUnlocking: false,
    });
    mockUseDatabaseConfig.mockReturnValue({
      data: null,
      isLoading: true,
      error: null,
    });

    const { rerender } = render(<SettingsView />);
    expect(
      screen.getByText("settings.database.loadingSettings")
    ).toBeInTheDocument();

    mockUseDatabaseConfig.mockReturnValue({
      data: null,
      isLoading: false,
      error: new Error("db failed"),
    });
    rerender(<SettingsView />);
    expect(screen.getByText("settings.database.loadError")).toBeInTheDocument();
  });

  it("shows toast error when update fails", async () => {
    const updatePreferences = vi
      .fn()
      .mockRejectedValue(new Error("update failed"));
    mockUseAppPreferences.mockReturnValue({
      preferences: makePreferences(),
      isLoading: false,
      error: null,
      updatePreferences,
      isUpdating: false,
      resetPreferences: vi.fn().mockResolvedValue(makePreferences()),
      isResetting: false,
    });

    render(<SettingsView />);

    // Change language from "en" to "de" via the select dropdown
    fireEvent.change(screen.getByLabelText("settings.general.language"), {
      target: { value: "de" },
    });
    fireEvent.click(
      screen.getByRole("button", { name: "settings.saveChanges" })
    );

    await waitFor(() => {
      expect(updatePreferences).toHaveBeenCalledTimes(1);
    });
    expect(toastError).toHaveBeenCalledWith("Error: update failed");
  });

  it("toggles preventScreenCapture and includes it in update", async () => {
    const updatePreferences = vi.fn().mockResolvedValue(undefined);
    mockUseAppPreferences.mockReturnValue({
      preferences: makePreferences(),
      isLoading: false,
      error: null,
      updatePreferences,
      isUpdating: false,
      resetPreferences: vi.fn().mockResolvedValue(makePreferences()),
      isResetting: false,
    });

    render(<SettingsView />);

    fireEvent.click(screen.getByText("settings.security.preventScreenCapture"));
    fireEvent.click(
      screen.getByRole("button", { name: "settings.saveChanges" })
    );

    await waitFor(() => {
      expect(updatePreferences).toHaveBeenCalledTimes(1);
    });
    expect(updatePreferences).toHaveBeenCalledWith(
      expect.objectContaining({
        security: expect.objectContaining({ preventScreenCapture: false }),
      })
    );
  });

  it("does not reset when reset dialog is cancelled", async () => {
    const resetPreferences = vi.fn().mockResolvedValue(makePreferences());
    mockUseAppPreferences.mockReturnValue({
      preferences: makePreferences(),
      isLoading: false,
      error: null,
      updatePreferences: vi.fn().mockResolvedValue(undefined),
      isUpdating: false,
      resetPreferences,
      isResetting: false,
    });

    render(<SettingsView />);
    fireEvent.click(
      screen.getByRole("button", { name: "settings.resetDefaults" })
    );
    fireEvent.click(screen.getByRole("button", { name: "common.cancel" }));

    await waitFor(() => {
      expect(resetPreferences).not.toHaveBeenCalled();
    });
  });

  it.each([0, 5, 30, 120])(
    "saves auto-lock timeout value %i",
    async (autoLockTimeout) => {
      const updatePreferences = vi.fn().mockResolvedValue(undefined);
      mockUseAppPreferences.mockReturnValue({
        preferences: makePreferences(),
        isLoading: false,
        error: null,
        updatePreferences,
        isUpdating: false,
        resetPreferences: vi.fn().mockResolvedValue(makePreferences()),
        isResetting: false,
      });

      render(<SettingsView />);

      fireEvent.change(
        screen.getByLabelText("settings.security.autoLockTimeout"),
        {
          target: { value: String(autoLockTimeout) },
        }
      );

      fireEvent.click(
        screen.getByRole("button", { name: "settings.saveChanges" })
      );

      await waitFor(() => {
        expect(updatePreferences).toHaveBeenCalledTimes(1);
      });

      expect(updatePreferences).toHaveBeenCalledWith(
        expect.objectContaining({
          security: expect.objectContaining({
            autoLockTimeout,
          }),
        })
      );
    }
  );

  it("updates core preference toggles and numeric fields", async () => {
    const updatePreferences = vi.fn().mockResolvedValue(undefined);
    const setTheme = vi.fn();
    const setThemePreview = vi.fn();
    mockUseTheme.mockReturnValue({
      theme: "system",
      setTheme,
      setThemePreview,
      colorPreset: "default",
      setColorPreset: vi.fn(),
      setColorPresetPreview: vi.fn(),
    });
    mockUseAppPreferences.mockReturnValue({
      preferences: makePreferences(),
      isLoading: false,
      error: null,
      updatePreferences,
      isUpdating: false,
      resetPreferences: vi.fn().mockResolvedValue(makePreferences()),
      isResetting: false,
    });

    render(<SettingsView />);

    fireEvent.change(
      screen.getByLabelText("settings.general.startupBehavior"),
      {
        target: { value: "openLastDatabase" },
      }
    );
    fireEvent.change(
      screen.getByLabelText("settings.security.autoLockTimeout"),
      {
        target: { value: "120" },
      }
    );
    fireEvent.change(
      screen.getByLabelText("settings.security.clipboardTimeout"),
      {
        target: { value: "15" },
      }
    );
    fireEvent.change(screen.getByLabelText("settings.appearance.theme"), {
      target: { value: "dark" },
    });
    fireEvent.change(screen.getByLabelText("settings.appearance.fontSize"), {
      target: { value: "18" },
    });

    fireEvent.click(
      screen.getByText("settings.security.showPasswordByDefault")
    );
    fireEvent.click(screen.getByText("settings.security.minimizeToTray"));
    fireEvent.click(screen.getByText("settings.security.startMinimized"));
    fireEvent.click(screen.getByText("settings.browser.enableIntegration"));
    fireEvent.click(screen.getByText("settings.advanced.enableDebugMode"));
    fireEvent.click(screen.getByText("settings.appearance.columns.username"));
    fireEvent.click(screen.getByText("settings.appearance.columns.url"));
    fireEvent.click(screen.getByText("settings.appearance.columns.modifiedAt"));
    fireEvent.click(screen.getByText("settings.appearance.columns.tags"));

    fireEvent.click(
      screen.getByRole("button", { name: "settings.saveChanges" })
    );

    await waitFor(() => {
      expect(updatePreferences).toHaveBeenCalledTimes(1);
    });

    expect(setThemePreview).toHaveBeenCalledWith("dark");
    expect(setTheme).toHaveBeenCalledWith("dark");
    expect(setTheme).toHaveBeenLastCalledWith("dark");
    expect(updatePreferences).toHaveBeenCalledWith(
      expect.objectContaining({
        general: expect.objectContaining({
          startupBehavior: "openLastDatabase",
        }),
        security: expect.objectContaining({
          autoLockTimeout: 120,
          clipboardClearTimeout: 15,
          showPasswordByDefault: true,
          minimizeToTray: false,
          startMinimized: true,
        }),
        appearance: expect.objectContaining({
          theme: "dark",
          fontSize: 18,
          entryListColumns: expect.objectContaining({
            username: false,
            url: false,
            modifiedAt: false,
            tags: false,
          }),
        }),
        browserIntegration: expect.objectContaining({
          enabled: true,
        }),
        advanced: expect.objectContaining({
          debugMode: true,
        }),
      })
    );
  });
});
