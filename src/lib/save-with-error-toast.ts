// SPDX-License-Identifier: MIT

import { toast } from "sonner";
import type { TFunction } from "i18next";
import { database } from "@/lib/tauri";

export class SaveError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "SaveError";
  }
}

const BACKUP_FAILED_RE = /^Backup failed for (.+?): (.+)$/s;

export async function saveWithErrorToast(
  dbId: string,
  t: TFunction
): Promise<void> {
  try {
    await database.save(dbId);
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
    throw new SaveError(message);
  }
}
