// SPDX-License-Identifier: MIT

import { useCallback } from "react";
import { toast } from "sonner";
import { useTranslation } from "react-i18next";
import { useAppPreferences } from "@/hooks/use-app-preferences";

const TOAST_ID = "clipboard-countdown";
let activeInterval: ReturnType<typeof setInterval> | null = null;

export function useClipboardCountdown() {
  const { t } = useTranslation();
  const { preferences } = useAppPreferences();

  const startCountdown = useCallback(
    (seconds: number) => {
      if (!preferences?.security.showClipboardCountdown) return;

      if (activeInterval) {
        clearInterval(activeInterval);
      }

      let remaining = seconds;
      toast.success(t("clipboard.countdown", { seconds: remaining }), {
        id: TOAST_ID,
        duration: Infinity,
      });

      activeInterval = setInterval(() => {
        remaining -= 1;
        if (remaining <= 0) {
          if (activeInterval) {
            clearInterval(activeInterval);
            activeInterval = null;
          }
          toast.dismiss(TOAST_ID);
        } else {
          toast.success(t("clipboard.countdown", { seconds: remaining }), {
            id: TOAST_ID,
            duration: Infinity,
          });
        }
      }, 1000);
    },
    [preferences?.security.showClipboardCountdown, t]
  );

  return startCountdown;
}
