// SPDX-License-Identifier: MIT

import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import {
  database,
  entries,
  groups,
  settings,
  tags,
  windowProtection,
} from "./tauri";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

describe("tauri wrappers validation", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("gets custom icons as MIME-aware payloads", async () => {
    const dbId = "/tmp/test.kdbx";
    const customIcons = {
      "icon-1": {
        mimeType: "image/svg+xml",
        data: "PHN2Zy8+",
      },
    };
    vi.mocked(invoke).mockResolvedValueOnce(customIcons);

    await expect(database.getCustomIcons(dbId)).resolves.toEqual(customIcons);
    expect(invoke).toHaveBeenCalledWith("get_custom_icons", { dbId });
  });

  it("fetches and clears entry custom icons through entry wrappers", async () => {
    const dbId = "/tmp/test.kdbx";
    const entryId = crypto.randomUUID();
    vi.mocked(invoke)
      .mockResolvedValueOnce("updated")
      .mockResolvedValueOnce(false)
      .mockResolvedValueOnce(true);

    await expect(entries.fetchFavicon(dbId, entryId, true)).resolves.toBe(
      "updated"
    );
    expect(invoke).toHaveBeenNthCalledWith(1, "fetch_entry_favicon", {
      dbId,
      id: entryId,
      force: true,
    });

    await expect(entries.clearCustomIcon(dbId, entryId)).resolves.toBe(false);
    expect(invoke).toHaveBeenNthCalledWith(2, "clear_entry_custom_icon", {
      dbId,
      id: entryId,
    });

    const iconUuid = crypto.randomUUID();
    await expect(entries.setCustomIcon(dbId, entryId, iconUuid)).resolves.toBe(
      true
    );
    expect(invoke).toHaveBeenNthCalledWith(3, "set_entry_custom_icon", {
      dbId,
      id: entryId,
      iconUuid,
    });
  });

  it("validates favicon entry ids before invoking backend", async () => {
    await expect(
      entries.fetchFavicon("/tmp/test.kdbx", "not-a-uuid")
    ).rejects.toThrow();
    await expect(
      entries.clearCustomIcon("/tmp/test.kdbx", "not-a-uuid")
    ).rejects.toThrow();
    expect(invoke).not.toHaveBeenCalled();
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
        maxVersions: 10,
        onOpen: false,
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
          dataLocation: "/tmp",
        },
        backups: {
          enabled: true,
          maxVersions: 10,
        },
      })
    ).rejects.toThrow();

    expect(invoke).not.toHaveBeenCalled();
  });

  it("sets window protection through invoke", async () => {
    vi.mocked(invoke).mockResolvedValueOnce(undefined);

    await windowProtection.setProtected(true);

    expect(invoke).toHaveBeenCalledWith("set_window_content_protected", {
      enabled: true,
    });
  });

  it("parses window protection support response as boolean", async () => {
    vi.mocked(invoke).mockResolvedValueOnce(true);

    await expect(windowProtection.isSupported()).resolves.toBe(true);
    expect(invoke).toHaveBeenCalledWith(
      "get_window_content_protection_supported"
    );
  });

  it("rejects invalid window protection support payloads", async () => {
    vi.mocked(invoke).mockResolvedValueOnce("yes");

    await expect(windowProtection.isSupported()).rejects.toThrow();
    expect(invoke).toHaveBeenCalledWith(
      "get_window_content_protection_supported"
    );
  });
});
