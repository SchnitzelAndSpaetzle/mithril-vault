// SPDX-License-Identifier: MIT

import { afterEach, describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { DatabaseSettingsSection } from "../DatabaseSettingsSection";

const useVaultHistorySettings = vi.fn();
vi.mock("@/hooks/use-vault-history-settings", () => ({
  useVaultHistorySettings: () => useVaultHistorySettings(),
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
});
