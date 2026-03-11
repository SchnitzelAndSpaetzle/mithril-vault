// SPDX-License-Identifier: MIT

import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { ThemePresetPicker } from "@/components/settings/ThemePresetPicker";
import { COLOR_PRESET_IDS } from "@/lib/theme-presets";

describe("ThemePresetPicker", () => {
  it("renders all preset options", () => {
    render(
      <ThemePresetPicker
        value="default"
        onChange={vi.fn()}
        onPreview={vi.fn()}
      />
    );

    for (const id of COLOR_PRESET_IDS) {
      expect(
        screen.getByText(`settings.appearance.presets.${id}`)
      ).toBeInTheDocument();
    }
  });

  it("calls onChange when a preset is clicked", () => {
    const onChange = vi.fn();
    render(
      <ThemePresetPicker
        value="default"
        onChange={onChange}
        onPreview={vi.fn()}
      />
    );

    fireEvent.click(screen.getByText("settings.appearance.presets.catppuccin"));
    expect(onChange).toHaveBeenCalledWith("catppuccin");
  });

  it("calls onPreview on mouse enter", () => {
    const onPreview = vi.fn();
    render(
      <ThemePresetPicker
        value="default"
        onChange={vi.fn()}
        onPreview={onPreview}
      />
    );

    const nordButton = screen
      .getByText("settings.appearance.presets.nord")
      .closest("button")!;
    fireEvent.mouseEnter(nordButton);
    expect(onPreview).toHaveBeenCalledWith("nord");
  });

  it("reverts preview on mouse leave from grid", () => {
    const onPreview = vi.fn();
    render(
      <ThemePresetPicker
        value="dracula"
        onChange={vi.fn()}
        onPreview={onPreview}
      />
    );

    // The grid container has the onMouseLeave handler
    const grid = screen
      .getByText("settings.appearance.presets.default")
      .closest("div.grid")!;
    fireEvent.mouseLeave(grid);
    expect(onPreview).toHaveBeenCalledWith("dracula");
  });
});
