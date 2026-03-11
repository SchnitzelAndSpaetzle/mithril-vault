// SPDX-License-Identifier: MIT

import type { ReactNode } from "react";
import { act, renderHook } from "@testing-library/react";
import { beforeEach, describe, expect, it } from "vitest";
import { ThemeProvider } from "@/components/theme-provider";
import { useTheme } from "@/hooks/use-theme";

function Wrapper({ children }: Readonly<{ children: ReactNode }>) {
  return <ThemeProvider defaultTheme="light">{children}</ThemeProvider>;
}

describe("ThemeProvider", () => {
  beforeEach(() => {
    localStorage.clear();
    document.documentElement.className = "";
    document.documentElement.style.cssText = "";
    document.getElementById("theme-preset-dark")?.remove();
  });

  it("keeps setter references stable during preview updates", () => {
    const { result } = renderHook(() => useTheme(), { wrapper: Wrapper });

    const initialSetTheme = result.current.setTheme;
    const initialSetColorPreset = result.current.setColorPreset;

    act(() => {
      result.current.setThemePreview("dark");
      result.current.setColorPresetPreview("mithril");
    });

    expect(result.current.theme).toBe("dark");
    expect(result.current.colorPreset).toBe("mithril");
    expect(result.current.setTheme).toBe(initialSetTheme);
    expect(result.current.setColorPreset).toBe(initialSetColorPreset);
  });
});
