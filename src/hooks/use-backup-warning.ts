// SPDX-License-Identifier: MIT

import { listen } from "@tauri-apps/api/event";
import { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";

interface BackupWarningPayload {
  path: string;
  reason: string;
}

/**
 * Subscribes to the `backup-warning` Tauri event and renders a non-blocking
 * warning toast. Open-side backup failures (issue #193) must surface to the
 * user without interrupting them — never a modal, never an error dialog.
 */
export function useBackupWarning(): void {
  const { t } = useTranslation();

  useEffect(() => {
    const unlisten = listen<BackupWarningPayload>("backup-warning", (event) => {
      const { path, reason } = event.payload;
      toast.warning(t("settings.backups.warning.openSide", { path, reason }));
    });

    return () => {
      void unlisten.then((dispose) => dispose());
    };
  }, [t]);
}
