// SPDX-License-Identifier: MIT

import { beforeEach, describe, expect, it, vi } from "vitest";
import type { TFunction } from "i18next";

const { mockDatabaseSave, mockToastError } = vi.hoisted(() => ({
  mockDatabaseSave: vi.fn(),
  mockToastError: vi.fn(),
}));

vi.mock("@/lib/tauri", () => ({
  database: { save: mockDatabaseSave },
}));

vi.mock("sonner", () => ({
  toast: { error: mockToastError, success: vi.fn() },
}));

// Stub matching the i18next signature. Encodes the key + options into a
// predictable string so tests can assert on the args the helper used.
const fakeT = ((key: string, opts?: Record<string, unknown>) => {
  if (!opts) return key;
  const args = Object.entries(opts)
    .map(([k, v]) => `${k}=${String(v)}`)
    .join("|");
  return `${key}[${args}]`;
}) as unknown as TFunction;

describe("saveWithErrorToast", () => {
  beforeEach(() => {
    mockDatabaseSave.mockReset();
    mockToastError.mockReset();
  });

  it("resolves quietly when database.save succeeds", async () => {
    mockDatabaseSave.mockResolvedValueOnce(undefined);
    const { saveWithErrorToast } = await import("../save-with-error-toast");

    await expect(saveWithErrorToast("db-1", fakeT)).resolves.toBeUndefined();

    expect(mockDatabaseSave).toHaveBeenCalledWith("db-1");
    expect(mockToastError).not.toHaveBeenCalled();
  });

  it("toasts the backup i18n key and throws SaveError on BackupFailed", async () => {
    mockDatabaseSave.mockRejectedValueOnce(
      new Error("Backup failed for /vaults/work.kdbx: No space left on device")
    );
    const { saveWithErrorToast, SaveError } =
      await import("../save-with-error-toast");

    await expect(saveWithErrorToast("db-1", fakeT)).rejects.toBeInstanceOf(
      SaveError
    );

    expect(mockToastError).toHaveBeenCalledTimes(1);
    expect(mockToastError).toHaveBeenCalledWith(
      "settings.backups.error.failed[path=/vaults/work.kdbx|reason=No space left on device]"
    );
  });

  it("toasts the generic save i18n key and throws SaveError for non-backup errors", async () => {
    mockDatabaseSave.mockRejectedValueOnce(new Error("Lock error"));
    const { saveWithErrorToast, SaveError } =
      await import("../save-with-error-toast");

    await expect(saveWithErrorToast("db-1", fakeT)).rejects.toBeInstanceOf(
      SaveError
    );

    expect(mockToastError).toHaveBeenCalledTimes(1);
    expect(mockToastError).toHaveBeenCalledWith(
      "database.save.failed[error=Lock error]"
    );
  });
});
