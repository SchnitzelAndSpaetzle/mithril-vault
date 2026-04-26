// SPDX-License-Identifier: MIT

import { useCallback, useRef } from "react";
import { toast } from "sonner";
import { useTranslation } from "react-i18next";
import { useAppPreferences } from "@/hooks/use-app-preferences";

const TOAST_ID = "clipboard-countdown";

export function useClipboardCountdown() {
  const { t } = useTranslation();
  const { preferences } = useAppPreferences();
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const startCountdown = useCallback(
    (seconds: number) => {
      if (!preferences?.security.showClipboardCountdown) return;

      if (intervalRef.current) {
        clearInterval(intervalRef.current);
      }

      let remaining = seconds;
      toast.success(t("clipboard.countdown", { seconds: remaining }), {
        id: TOAST_ID,
        duration: Infinity,
      });

      intervalRef.current = setInterval(() => {
        remaining -= 1;
        if (remaining <= 0) {
          if (intervalRef.current) {
            clearInterval(intervalRef.current);
            intervalRef.current = null;
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
