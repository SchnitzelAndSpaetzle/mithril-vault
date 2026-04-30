// SPDX-License-Identifier: MIT

import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { groups, settings, tags } from "./tauri";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

describe("tauri wrappers validation", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("validates groups.update payload before invoking", async () => {
    await expect(
      groups.update(crypto.randomUUID(), crypto.randomUUID(), {})
    ).rejects.toThrow();
    expect(invoke).not.toHaveBeenCalled();

    await expect(
      groups.update(crypto.randomUUID(), crypto.randomUUID(), {
        name: "   ",
      })
    ).rejects.toThrow();
    expect(invoke).not.toHaveBeenCalled();

    await expect(
      groups.update(crypto.randomUUID(), crypto.randomUUID(), {
        icon: "folder",
      })
    ).rejects.toThrow();
    expect(invoke).not.toHaveBeenCalled();
  });

  it("trims and validates tag names for rename/delete", async () => {
    vi.mocked(invoke)
      .mockResolvedValueOnce(1) // tags.rename
      .mockResolvedValueOnce(1); // tags.delete

    await tags.rename(crypto.randomUUID(), "  old-tag  ", "  new-tag  ");
    expect(invoke).toHaveBeenNthCalledWith(1, "rename_tag", {
      dbId: expect.any(String),
      oldName: "old-tag",
      newName: "new-tag",
    });

    await tags.delete(crypto.randomUUID(), "  stale-tag  ");
    expect(invoke).toHaveBeenNthCalledWith(2, "delete_tag", {
      dbId: expect.any(String),
      tagName: "stale-tag",
    });
  });

  it("rejects empty tag names before invoking backend", async () => {
    await expect(
      tags.rename(crypto.randomUUID(), " ", "new")
    ).rejects.toThrow();
    await expect(
      tags.rename(crypto.randomUUID(), "old", " ")
    ).rejects.toThrow();
    await expect(tags.delete(crypto.randomUUID(), " ")).rejects.toThrow();
    expect(invoke).not.toHaveBeenCalled();
  });

  it("gets, updates, and resets app preferences through settings wrappers", async () => {
    const preferences = {
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
    } as const;

    vi.mocked(invoke)
      .mockResolvedValueOnce(preferences)
      .mockResolvedValueOnce(undefined)
      .mockResolvedValueOnce(preferences);

    const loaded = await settings.getPreferences();
    expect(loaded).toEqual(preferences);
    expect(invoke).toHaveBeenNthCalledWith(1, "get_app_preferences");

    await settings.updatePreferences(loaded);
    expect(invoke).toHaveBeenNthCalledWith(2, "update_app_preferences", {
      newPreferences: loaded,
    });

    const reset = await settings.resetPreferences();
    expect(reset).toEqual(preferences);
    expect(invoke).toHaveBeenNthCalledWith(3, "reset_app_preferences");
  });

  it("validates app preferences before invoking update", async () => {
    await expect(
      settings.updatePreferences({
        general: {
          language: "",
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
          dataLocation: "/tmp",
        },
      })
    ).rejects.toThrow();

    expect(invoke).not.toHaveBeenCalled();
  });
});
