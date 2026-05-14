// SPDX-License-Identifier: MIT

import { beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";

const openMock = vi.fn();
vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: (...args: unknown[]) => openMock(...args),
}));

// jsdom doesn't implement these APIs that Radix Select reaches for when
// the listbox opens. Stub them so the production component can be exercised
// end-to-end without rewriting the UI just for tests.
beforeAll(() => {
  if (!Element.prototype.scrollIntoView) {
    Element.prototype.scrollIntoView = vi.fn();
  }
  if (!Element.prototype.hasPointerCapture) {
    Element.prototype.hasPointerCapture = vi.fn(() => false);
  }
  if (!Element.prototype.releasePointerCapture) {
    Element.prototype.releasePointerCapture = vi.fn();
  }
});

import { BackupsSettingsSection } from "@/components/settings/sections/BackupsSettingsSection";
import { BACKUP_MAX_VERSIONS_PRESETS, type AppPreferences } from "@/lib/types";

function makeDraft(
  maxVersions = 10,
  directory: string | null = null
): AppPreferences {
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
    browserIntegration: { enabled: false, allowedSites: [] },
    advanced: { debugMode: false, dataLocation: "/tmp" },
    backups: { enabled: true, maxVersions, directory },
  };
}

describe("BackupsSettingsSection", () => {
  beforeEach(() => {
    openMock.mockReset();
  });

  it("renders the max-versions trigger with the current preset value", () => {
    render(
      <BackupsSettingsSection draft={makeDraft(25)} updateDraft={vi.fn()} />
    );

    const trigger = screen.getByRole("combobox", {
      name: "settings.backups.maxVersions.label",
    });
    // The Radix Select trigger renders the selected value as its text.
    expect(trigger).toHaveTextContent("25");
  });

  it("offers exactly the documented presets when opened", () => {
    render(
      <BackupsSettingsSection draft={makeDraft(10)} updateDraft={vi.fn()} />
    );

    const trigger = screen.getByRole("combobox", {
      name: "settings.backups.maxVersions.label",
    });
    fireEvent.click(trigger);

    const options = screen.getAllByRole("option");
    expect(options).toHaveLength(BACKUP_MAX_VERSIONS_PRESETS.length);
    for (const preset of BACKUP_MAX_VERSIONS_PRESETS) {
      expect(
        screen.getByRole("option", { name: String(preset) })
      ).toBeInTheDocument();
    }
  });

  it("renders a directory text input with the default-placeholder hint", () => {
    render(
      <BackupsSettingsSection draft={makeDraft()} updateDraft={vi.fn()} />
    );

    const input = screen.getByRole("textbox", {
      name: "settings.backups.directory.label",
    });
    // i18n is mocked to echo keys, so the placeholder key is what appears.
    expect(input).toHaveAttribute(
      "placeholder",
      "settings.backups.directory.placeholder"
    );
    expect(input).toHaveValue("");
  });

  it("populates the directory input from draft.backups.directory", () => {
    render(
      <BackupsSettingsSection
        draft={makeDraft(10, "/mnt/backups")}
        updateDraft={vi.fn()}
      />
    );
    const input = screen.getByRole("textbox", {
      name: "settings.backups.directory.label",
    });
    expect(input).toHaveValue("/mnt/backups");
  });

  it("propagates typed directory text into draft.backups.directory", () => {
    const updateDraft = vi.fn();
    render(
      <BackupsSettingsSection draft={makeDraft()} updateDraft={updateDraft} />
    );

    const input = screen.getByRole("textbox", {
      name: "settings.backups.directory.label",
    });
    fireEvent.change(input, { target: { value: "/mnt/backups" } });

    const firstCall = updateDraft.mock.calls.at(-1);
    if (!firstCall) throw new Error("expected updateDraft to be called");
    const updater = firstCall[0] as (prev: AppPreferences) => AppPreferences;
    const next = updater(makeDraft());
    expect(next.backups.directory).toBe("/mnt/backups");
  });

  it("normalizes an empty directory input back to undefined", () => {
    const updateDraft = vi.fn();
    render(
      <BackupsSettingsSection
        draft={makeDraft(10, "/mnt/backups")}
        updateDraft={updateDraft}
      />
    );

    const input = screen.getByRole("textbox", {
      name: "settings.backups.directory.label",
    });
    fireEvent.change(input, { target: { value: "" } });

    const lastCall = updateDraft.mock.calls.at(-1);
    if (!lastCall) throw new Error("expected updateDraft to be called");
    const updater = lastCall[0] as (prev: AppPreferences) => AppPreferences;
    const next = updater(makeDraft(10, "/mnt/backups"));
    expect(next.backups.directory).toBeUndefined();
  });

  it("writes the picked directory into the draft when Browse is clicked", async () => {
    const updateDraft = vi.fn();
    openMock.mockResolvedValueOnce("/Volumes/ExternalDrive/backups");

    render(
      <BackupsSettingsSection draft={makeDraft()} updateDraft={updateDraft} />
    );

    const browse = screen.getByRole("button", {
      name: "settings.backups.directory.browse",
    });
    fireEvent.click(browse);

    // Let the awaited open() promise resolve.
    await Promise.resolve();
    await Promise.resolve();

    expect(openMock).toHaveBeenCalledWith({
      directory: true,
      multiple: false,
    });

    const lastCall = updateDraft.mock.calls.at(-1);
    if (!lastCall) throw new Error("expected updateDraft after Browse");
    const updater = lastCall[0] as (prev: AppPreferences) => AppPreferences;
    const next = updater(makeDraft());
    expect(next.backups.directory).toBe("/Volumes/ExternalDrive/backups");
  });

  it("does not change the draft when the Browse picker is cancelled", async () => {
    const updateDraft = vi.fn();
    openMock.mockResolvedValueOnce(null);

    render(
      <BackupsSettingsSection draft={makeDraft()} updateDraft={updateDraft} />
    );

    const browse = screen.getByRole("button", {
      name: "settings.backups.directory.browse",
    });
    fireEvent.click(browse);

    await Promise.resolve();
    await Promise.resolve();

    expect(updateDraft).not.toHaveBeenCalled();
  });

  it("updates draft.backups.maxVersions when a preset is picked", () => {
    const updateDraft = vi.fn();
    render(
      <BackupsSettingsSection draft={makeDraft(10)} updateDraft={updateDraft} />
    );

    const trigger = screen.getByRole("combobox", {
      name: "settings.backups.maxVersions.label",
    });
    fireEvent.click(trigger);
    fireEvent.click(screen.getByRole("option", { name: "50" }));

    expect(updateDraft).toHaveBeenCalledTimes(1);
    // The section invokes updateDraft with a functional updater so the
    // surrounding SettingsView can compose draft changes safely.
    const firstCall = updateDraft.mock.calls[0];
    if (!firstCall) throw new Error("expected updateDraft to be called");
    const updater = firstCall[0] as (prev: AppPreferences) => AppPreferences;
    const next = updater(makeDraft(10));
    expect(next.backups.maxVersions).toBe(50);
    // Other backup settings stay untouched.
    expect(next.backups.enabled).toBe(true);
  });
});
