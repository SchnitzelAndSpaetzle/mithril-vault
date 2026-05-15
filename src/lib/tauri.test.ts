// SPDX-License-Identifier: MIT

import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import {
  audit,
  backups,
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
          onOpen: false,
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

  it("parses list_backups payload into a typed BackupListEntry[]", async () => {
    const dbPath = "/tmp/vault.kdbx";
    const payload = [
      {
        path: "/tmp/.kdbx-backups/vault.kdbx.backup.20260512T143045.123Z.kdbx",
        timestamp: "2026-05-12T14:30:45.123Z",
        sizeBytes: 4096,
        kind: "auto",
      },
      {
        path: "/tmp/.kdbx-backups/vault.kdbx.backup.manual.20260101T000000.000Z.kdbx",
        timestamp: "2026-01-01T00:00:00.000Z",
        sizeBytes: 8192,
        kind: "manual",
      },
    ];
    vi.mocked(invoke).mockResolvedValueOnce(payload);

    await expect(backups.list(dbPath)).resolves.toEqual(payload);
    expect(invoke).toHaveBeenCalledWith("list_backups", {
      databasePath: dbPath,
    });
  });

  it("rejects list_backups payloads with an unknown kind discriminator", async () => {
    vi.mocked(invoke).mockResolvedValueOnce([
      {
        path: "/tmp/.kdbx-backups/vault.kdbx.backup.20260512T143045.123Z.kdbx",
        timestamp: "2026-05-12T14:30:45.123Z",
        sizeBytes: 4096,
        kind: "bogus",
      },
    ]);

    await expect(backups.list("/tmp/vault.kdbx")).rejects.toThrow();
  });

  it("invokes create_manual_backup and returns the typed BackupInfo", async () => {
    const dbPath = "/tmp/vault.kdbx";
    const payload = {
      path: "/tmp/.kdbx-backups/vault.kdbx.backup.manual.20260515T120000.000Z.kdbx",
    };
    vi.mocked(invoke).mockResolvedValueOnce(payload);

    await expect(backups.createManual(dbPath)).resolves.toEqual(payload);
    expect(invoke).toHaveBeenCalledWith("create_manual_backup", {
      databasePath: dbPath,
    });
  });

  it("audit.list parses get_audit_events into typed events", async () => {
    const payload = [
      {
        kind: "vaultUnlockFailed",
        timestamp: "2026-05-15T12:00:00.000Z",
        attemptCount: 2,
      },
      {
        kind: "vaultUnlockFailed",
        timestamp: "2026-05-15T11:59:00.000Z",
        attemptCount: 1,
      },
    ];
    vi.mocked(invoke).mockResolvedValueOnce(payload);

    await expect(audit.list("/tmp/vault.kdbx")).resolves.toEqual(payload);
    expect(invoke).toHaveBeenCalledWith("get_audit_events", {
      vaultPath: "/tmp/vault.kdbx",
      filter: null,
    });
  });

  it("audit.list rejects payloads with an unknown kind", async () => {
    vi.mocked(invoke).mockResolvedValueOnce([
      {
        kind: "vaultDeleted",
        timestamp: "2026-05-15T12:00:00.000Z",
        attemptCount: 1,
      },
    ]);
    await expect(audit.list("/tmp/vault.kdbx")).rejects.toThrow();
  });

  it("invokes delete_backup with the supplied path", async () => {
    vi.mocked(invoke).mockResolvedValueOnce(undefined);

    await backups.delete(
      "/tmp/.kdbx-backups/vault.kdbx.backup.20260512T143045.123Z.kdbx"
    );

    expect(invoke).toHaveBeenCalledWith("delete_backup", {
      backupPath:
        "/tmp/.kdbx-backups/vault.kdbx.backup.20260512T143045.123Z.kdbx",
    });
  });
});
