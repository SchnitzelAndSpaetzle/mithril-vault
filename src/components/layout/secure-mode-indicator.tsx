// SPDX-License-Identifier: MIT

import { ShieldCheck } from "lucide-react";
import { useTranslation } from "react-i18next";

import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { useWindowProtection } from "@/hooks/use-window-protection";

export function SecureModeIndicator() {
  const { t } = useTranslation();
  const { enabled, isSupported } = useWindowProtection();

  if (!enabled) {
    return null;
  }

  const tooltipKey = isSupported
    ? "secureMode.indicator.activeTooltip"
    : "secureMode.indicator.notSupportedTooltip";

  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <div
          aria-label={t(tooltipKey)}
          className="pointer-events-auto fixed bottom-3 right-3 z-50 inline-flex size-7 items-center justify-center rounded-full bg-background/80 text-foreground/80 shadow-sm ring-1 ring-border backdrop-blur"
          role="status"
        >
          <ShieldCheck className="size-4" aria-hidden />
        </div>
      </TooltipTrigger>
      <TooltipContent side="left">{t(tooltipKey)}</TooltipContent>
    </Tooltip>
  );
}
