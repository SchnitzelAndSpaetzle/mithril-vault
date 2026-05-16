// SPDX-License-Identifier: MIT

import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";

import { AuditLogSettingsSection } from "@/components/settings/sections/AuditLogSettingsSection";
import type { AppPreferences } from "@/lib/types";

function makeDraft(
  audit: { enabled: boolean; retentionDays: number } = {
    enabled: true,
    retentionDays: 90,
  }
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
    backups: { enabled: true, maxVersions: 10, onOpen: false },
    audit,
  };
}

describe("AuditLogSettingsSection", () => {
  it("renders the enable toggle reflecting draft.audit.enabled", () => {
    render(
      <AuditLogSettingsSection
        draft={makeDraft({ enabled: true, retentionDays: 90 })}
        updateDraft={vi.fn()}
      />
    );

    const checkbox = screen.getByRole("checkbox", {
      name: "settings.audit.enabled.label",
    });
    expect(checkbox).toBeChecked();
  });

  it("renders unchecked when draft.audit.enabled is false", () => {
    render(
      <AuditLogSettingsSection
        draft={makeDraft({ enabled: false, retentionDays: 90 })}
        updateDraft={vi.fn()}
      />
    );

    const checkbox = screen.getByRole("checkbox", {
      name: "settings.audit.enabled.label",
    });
    expect(checkbox).not.toBeChecked();
  });

  it("toggles draft.audit.enabled when the checkbox is clicked", () => {
    // Acceptance criterion: a single toggle in Settings drives
    // draft.audit.enabled. The functional-updater shape matches the rest
    // of the settings sections so SettingsEditor can compose updates
    // without per-section special-casing.
    const updateDraft = vi.fn();
    render(
      <AuditLogSettingsSection draft={makeDraft()} updateDraft={updateDraft} />
    );

    const checkbox = screen.getByRole("checkbox", {
      name: "settings.audit.enabled.label",
    });
    fireEvent.click(checkbox);

    const calls = updateDraft.mock.calls;
    const lastCall = calls[calls.length - 1];
    if (!lastCall) throw new Error("expected updateDraft to be called");
    const updater = lastCall[0] as (prev: AppPreferences) => AppPreferences;
    const next = updater(makeDraft());
    expect(next.audit.enabled).toBe(false);
    // Unrelated audit fields stay intact.
    expect(next.audit.retentionDays).toBe(90);
  });

  it("renders the retention-days input with the current value", () => {
    render(
      <AuditLogSettingsSection
        draft={makeDraft({ enabled: true, retentionDays: 30 })}
        updateDraft={vi.fn()}
      />
    );
    const input = screen.getByRole("spinbutton", {
      name: "settings.audit.retentionDays.label",
    });
    expect(input).toHaveValue(30);
  });

  it("updates draft.audit.retentionDays when the input changes", () => {
    const updateDraft = vi.fn();
    render(
      <AuditLogSettingsSection draft={makeDraft()} updateDraft={updateDraft} />
    );

    const input = screen.getByRole("spinbutton", {
      name: "settings.audit.retentionDays.label",
    });
    fireEvent.change(input, { target: { value: "180" } });

    const calls = updateDraft.mock.calls;
    const lastCall = calls[calls.length - 1];
    if (!lastCall) throw new Error("expected updateDraft to be called");
    const updater = lastCall[0] as (prev: AppPreferences) => AppPreferences;
    const next = updater(makeDraft());
    expect(next.audit.retentionDays).toBe(180);
    expect(next.audit.enabled).toBe(true);
  });

  it("declares the documented retention range on the input", () => {
    // Mirroring the backend's 1..=365 boundary keeps invalid drafts from
    // ever reaching validate_preferences. The spinbutton's min/max
    // attributes communicate the range to assistive tech and stop browser
    // up/down arrows from stepping past it.
    render(
      <AuditLogSettingsSection draft={makeDraft()} updateDraft={vi.fn()} />
    );
    const input = screen.getByRole("spinbutton", {
      name: "settings.audit.retentionDays.label",
    });
    expect(input).toHaveAttribute("min", "1");
    expect(input).toHaveAttribute("max", "365");
  });
});
