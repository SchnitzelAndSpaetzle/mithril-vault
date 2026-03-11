// SPDX-License-Identifier: MIT

import { type ColorPresetId, getPresetSwatchColors } from "@/lib/theme-presets";

interface ThemePresetSwatchProps {
  presetId: ColorPresetId;
  mode: "light" | "dark";
}

export function ThemePresetSwatch({
  presetId,
  mode,
}: Readonly<ThemePresetSwatchProps>) {
  const colors = getPresetSwatchColors(presetId, mode);

  return (
    <div className="flex gap-1">
      {colors.map((color, index) => (
        <span
          key={index}
          className="size-3 rounded-full border border-black/10"
          style={{ backgroundColor: color }}
        />
      ))}
    </div>
  );
}
