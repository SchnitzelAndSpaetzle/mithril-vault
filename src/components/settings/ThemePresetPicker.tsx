// SPDX-License-Identifier: MIT

import { useTranslation } from "react-i18next";
import { Check } from "lucide-react";
import {
  COLOR_PRESET_IDS,
  type ColorPresetId,
  THEME_PRESETS,
} from "@/lib/theme-presets";
import { ThemePresetSwatch } from "@/components/settings/ThemePresetSwatch";
import { cn } from "@/lib/utils";

interface ThemePresetPickerProps {
  value: ColorPresetId;
  onChange: (preset: ColorPresetId) => void;
  onPreview: (preset: ColorPresetId) => void;
}

export function ThemePresetPicker({
  value,
  onChange,
  onPreview,
}: Readonly<ThemePresetPickerProps>) {
  const { t } = useTranslation();

  return (
    <div
      className="grid gap-3 grid-cols-2 md:grid-cols-3 lg:grid-cols-4"
      onMouseLeave={() => onPreview(value)}
    >
      {COLOR_PRESET_IDS.map((presetId) => {
        const preset = THEME_PRESETS[presetId];
        const isSelected = presetId === value;

        return (
          <button
            key={presetId}
            type="button"
            className={cn(
              "relative flex flex-col gap-2 rounded-lg border p-3 text-left transition-colors hover:bg-accent/50",
              isSelected
                ? "border-primary ring-2 ring-primary/20"
                : "border-border"
            )}
            onClick={() => {
              onChange(presetId);
              onPreview(presetId);
            }}
            onMouseEnter={() => onPreview(presetId)}
          >
            {isSelected && (
              <Check className="absolute top-2 right-2 size-3.5 text-primary" />
            )}
            <span className="text-sm font-medium">{t(preset.labelKey)}</span>
            <div className="flex flex-col gap-1">
              <ThemePresetSwatch presetId={presetId} mode="light" />
              <ThemePresetSwatch presetId={presetId} mode="dark" />
            </div>
          </button>
        );
      })}
    </div>
  );
}
