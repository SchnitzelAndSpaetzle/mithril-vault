// SPDX-License-Identifier: MIT

import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { SettingsView } from "@/views/SettingsView";
import type { AppPreferences } from "@/lib/types";

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
      showPasswordByDefault: false,
      minimizeToTray: true,
      startMinimized: false,
    },
    appearance: {
      theme: "system",
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
      dataLocation: "/tmp/mithril-vault",
    },
  };
}

describe("SettingsView", () => {
  beforeEach(() => {
    vi.clearAllMocks();

    mockUseTheme.mockReturnValue({
      theme: "system",
      setTheme: vi.fn(),
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
      screen.getByRole("heading", { name: "Settings" })
    ).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { name: "General" })
    ).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { name: "Security" })
    ).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { name: "Appearance" })
    ).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { name: "Browser Integration" })
    ).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { name: "Advanced" })
    ).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { name: "Database Settings" })
    ).toBeInTheDocument();
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
    expect(screen.getByText("Loading settings...")).toBeInTheDocument();
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
    expect(
      screen.getByText("Failed to load settings: Error: boom")
    ).toBeInTheDocument();
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

    const allowedSites = screen.getByLabelText("Allowed sites (one per line)");
    fireEvent.change(allowedSites, {
      target: { value: "example.com\nfoo.bar\n  \nsub.domain" },
    });

    fireEvent.click(screen.getByRole("button", { name: "Save changes" }));

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
    expect(toastSuccess).toHaveBeenCalledWith("Settings updated");
  });

  it("resets preferences to defaults", async () => {
    const resetPreferences = vi.fn().mockResolvedValue(makePreferences());
    const setTheme = vi.fn();
    mockUseTheme.mockReturnValue({ theme: "dark", setTheme });
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

    fireEvent.click(screen.getByRole("button", { name: "Reset defaults" }));
    fireEvent.click(screen.getByRole("button", { name: "Reset preferences" }));

    await waitFor(() => {
      expect(resetPreferences).toHaveBeenCalledTimes(1);
    });

    expect(setTheme).toHaveBeenCalledWith("system");
    expect(toastSuccess).toHaveBeenCalledWith("Preferences reset to defaults");
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
      screen.getByText("Loading database settings...")
    ).toBeInTheDocument();

    mockUseDatabaseConfig.mockReturnValue({
      data: null,
      isLoading: false,
      error: new Error("db failed"),
    });
    rerender(<SettingsView />);
    expect(
      screen.getByText("Failed to load database settings: Error: db failed")
    ).toBeInTheDocument();
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

    fireEvent.change(screen.getByLabelText("Language"), {
      target: { value: "de" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save changes" }));

    await waitFor(() => {
      expect(updatePreferences).toHaveBeenCalledTimes(1);
    });
    expect(toastError).toHaveBeenCalledWith("Error: update failed");
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
    fireEvent.click(screen.getByRole("button", { name: "Reset defaults" }));
    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));

    await waitFor(() => {
      expect(resetPreferences).not.toHaveBeenCalled();
    });
  });

  it("updates core preference toggles and numeric fields", async () => {
    const updatePreferences = vi.fn().mockResolvedValue(undefined);
    const setTheme = vi.fn();
    mockUseTheme.mockReturnValue({ theme: "system", setTheme });
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

    fireEvent.change(screen.getByLabelText("Startup behavior"), {
      target: { value: "openLastDatabase" },
    });
    fireEvent.change(screen.getByLabelText("Auto-lock timeout (seconds)"), {
      target: { value: "120" },
    });
    fireEvent.change(
      screen.getByLabelText("Clipboard clear timeout (seconds)"),
      {
        target: { value: "15" },
      }
    );
    fireEvent.change(screen.getByLabelText("Theme"), {
      target: { value: "dark" },
    });
    fireEvent.change(screen.getByLabelText("Font size"), {
      target: { value: "18" },
    });

    fireEvent.click(screen.getByText("Show passwords by default"));
    fireEvent.click(screen.getByText("Minimize to tray"));
    fireEvent.click(screen.getByText("Start minimized"));
    fireEvent.click(screen.getByText("Enable browser integration"));
    fireEvent.click(screen.getByText("Enable debug mode"));
    fireEvent.click(screen.getByText("Username"));
    fireEvent.click(screen.getByText("URL"));
    fireEvent.click(screen.getByText("Modified date"));
    fireEvent.click(screen.getByText("Tags"));

    fireEvent.click(screen.getByRole("button", { name: "Save changes" }));

    await waitFor(() => {
      expect(updatePreferences).toHaveBeenCalledTimes(1);
    });

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
