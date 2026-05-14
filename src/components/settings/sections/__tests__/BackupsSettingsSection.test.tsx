// SPDX-License-Identifier: MIT

import { beforeAll, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";

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

function makeDraft(maxVersions = 10): AppPreferences {
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
    backups: { enabled: true, maxVersions },
  };
}

describe("BackupsSettingsSection", () => {
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
    const updater = updateDraft.mock.calls[0][0] as (
      prev: AppPreferences
    ) => AppPreferences;
    const next = updater(makeDraft(10));
    expect(next.backups.maxVersions).toBe(50);
    // Other backup settings stay untouched.
    expect(next.backups.enabled).toBe(true);
  });
});
