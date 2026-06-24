// SPDX-License-Identifier: MIT

import { afterEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { DatabaseSettingsSection } from "../DatabaseSettingsSection";

const useVaultHistorySettings = vi.fn();
vi.mock("@/hooks/use-vault-history-settings", () => ({
  useVaultHistorySettings: () => useVaultHistorySettings(),
}));

const askMock = vi.fn();
vi.mock("@tauri-apps/plugin-dialog", () => ({
  ask: (...args: unknown[]) => askMock(...args),
}));

function renderSection() {
  return render(
    <DatabaseSettingsSection
      dbId="/vault.kdbx"
      databaseConfig={null}
      isDatabaseConfigLoading={false}
      databaseConfigError={null}
    />
  );
}

describe("DatabaseSettingsSection — history control gating", () => {
  afterEach(() => {
    useVaultHistorySettings.mockReset();
    askMock.mockReset();
  });

  it("hides the history control until vault metadata has loaded", () => {
    // e.g. the vault is locked, so the backend `with_vault` read errors and
    // settings stay null — the writable control must not render enabled.
    useVaultHistorySettings.mockReturnValue({
      settings: null,
      isLoading: false,
      error: new Error("DatabaseLocked"),
      update: vi.fn(),
      isUpdating: false,
    });

    renderSection();

    expect(
      screen.queryByText("settings.database.history.title")
    ).not.toBeInTheDocument();
  });

  it("shows the history control once settings have loaded", () => {
    useVaultHistorySettings.mockReturnValue({
      settings: { maxItems: null },
      isLoading: false,
      error: null,
      update: vi.fn(),
      isUpdating: false,
    });

    renderSection();

    expect(
      screen.getByText("settings.database.history.title")
    ).toBeInTheDocument();
  });

  it("clears all history vault-wide after the user confirms", async () => {
    const clearAll = vi.fn();
    useVaultHistorySettings.mockReturnValue({
      settings: { maxItems: null },
      isLoading: false,
      error: null,
      update: vi.fn(),
      isUpdating: false,
      clearAll,
      isClearing: false,
    });
    askMock.mockResolvedValue(true);

    renderSection();

    fireEvent.click(
      screen.getByRole("button", {
        name: "settings.database.history.clearAll",
      })
    );

    await waitFor(() => {
      expect(askMock).toHaveBeenCalledTimes(1);
    });
    await waitFor(() => {
      expect(clearAll).toHaveBeenCalledTimes(1);
    });
  });

  it("does not clear all history when the user cancels", async () => {
    const clearAll = vi.fn();
    useVaultHistorySettings.mockReturnValue({
      settings: { maxItems: null },
      isLoading: false,
      error: null,
      update: vi.fn(),
      isUpdating: false,
      clearAll,
      isClearing: false,
    });
    askMock.mockResolvedValue(false);

    renderSection();

    fireEvent.click(
      screen.getByRole("button", {
        name: "settings.database.history.clearAll",
      })
    );

    await waitFor(() => {
      expect(askMock).toHaveBeenCalledTimes(1);
    });
    expect(clearAll).not.toHaveBeenCalled();
  });
});
