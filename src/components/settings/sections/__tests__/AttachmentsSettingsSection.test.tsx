// SPDX-License-Identifier: MIT

import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";

import { AttachmentsSettingsSection } from "@/components/settings/sections/AttachmentsSettingsSection";
import type { AppPreferences } from "@/lib/types";

function makeDraft(
  attachments: { softWarnBytes: number; hardCapBytes: number } = {
    softWarnBytes: 5_000_000,
    hardCapBytes: 25_000_000,
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
    audit: { enabled: true, retentionDays: 90 },
    attachments,
  };
}

describe("AttachmentsSettingsSection", () => {
  it("renders the thresholds as decimal MB from the byte-valued draft", () => {
    // The draft stores bytes; the UI surfaces decimal MB. 5_000_000 B -> 5 MB,
    // 25_000_000 B -> 25 MB (the documented defaults).
    render(
      <AttachmentsSettingsSection draft={makeDraft()} updateDraft={vi.fn()} />
    );

    expect(
      screen.getByRole("spinbutton", {
        name: "settings.attachments.softWarn.label",
      })
    ).toHaveValue(5);
    expect(
      screen.getByRole("spinbutton", {
        name: "settings.attachments.hardCap.label",
      })
    ).toHaveValue(25);
  });

  it("converts the soft-warning MB input back to bytes in the draft", () => {
    const updateDraft = vi.fn();
    render(
      <AttachmentsSettingsSection
        draft={makeDraft()}
        updateDraft={updateDraft}
      />
    );

    const input = screen.getByRole("spinbutton", {
      name: "settings.attachments.softWarn.label",
    });
    fireEvent.change(input, { target: { value: "10" } });

    const calls = updateDraft.mock.calls;
    const lastCall = calls[calls.length - 1];
    if (!lastCall) throw new Error("expected updateDraft to be called");
    const updater = lastCall[0] as (prev: AppPreferences) => AppPreferences;
    const next = updater(makeDraft());
    expect(next.attachments.softWarnBytes).toBe(10_000_000);
    // The hard cap is left untouched.
    expect(next.attachments.hardCapBytes).toBe(25_000_000);
  });

  it("converts the hard-cap MB input back to bytes in the draft", () => {
    const updateDraft = vi.fn();
    render(
      <AttachmentsSettingsSection
        draft={makeDraft()}
        updateDraft={updateDraft}
      />
    );

    const input = screen.getByRole("spinbutton", {
      name: "settings.attachments.hardCap.label",
    });
    fireEvent.change(input, { target: { value: "50" } });

    const calls = updateDraft.mock.calls;
    const lastCall = calls[calls.length - 1];
    if (!lastCall) throw new Error("expected updateDraft to be called");
    const updater = lastCall[0] as (prev: AppPreferences) => AppPreferences;
    const next = updater(makeDraft());
    expect(next.attachments.hardCapBytes).toBe(50_000_000);
    expect(next.attachments.softWarnBytes).toBe(5_000_000);
  });

  it("ignores a non-numeric MB input rather than writing NaN bytes", () => {
    const updateDraft = vi.fn();
    render(
      <AttachmentsSettingsSection
        draft={makeDraft()}
        updateDraft={updateDraft}
      />
    );

    const input = screen.getByRole("spinbutton", {
      name: "settings.attachments.softWarn.label",
    });
    fireEvent.change(input, { target: { value: "" } });

    expect(updateDraft).not.toHaveBeenCalled();
  });
});
