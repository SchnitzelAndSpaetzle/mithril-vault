// SPDX-License-Identifier: MIT

import { toast } from "sonner";
import type { TFunction } from "i18next";
import { database } from "@/lib/tauri";

const BACKUP_FAILED_RE = /^Backup failed for (.+?): (.+)$/s;

/**
 * Persists the given database to disk. If the save fails (e.g., backup
 * directory unwritable), surfaces an error toast and returns — does NOT
 * throw or reject. The backend mutation that preceded this call has already
 * succeeded in memory, so the caller's success path (close form, clear
 * selection, navigate, show "X created" toast) must still run; the surfaced
 * error tells the user their work is in memory but not on disk.
 *
 * This is intentional: React Query mutations using this helper resolve
 * normally, `onSuccess` fires, and the UI reflects backend memory. The
 * save-failure toast is the user's signal to act (retry, fix the backup
 * directory, or disable backups in Settings).
 */
export async function saveWithErrorToast(
  dbId: string,
  t: TFunction
): Promise<boolean> {
  try {
    await database.save(dbId);
    return true;
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    const match = BACKUP_FAILED_RE.exec(message);
    if (match) {
      toast.error(
        t("settings.backups.error.failed", {
          path: match[1],
          reason: match[2],
        })
      );
    } else {
      toast.error(t("database.save.failed", { error: message }));
    }
    return false;
  }
}
