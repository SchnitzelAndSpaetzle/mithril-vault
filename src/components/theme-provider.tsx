import {
  useCallback,
  createContext,
  type ReactNode,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import {
  type ColorPresetId,
  isColorPresetId,
  THEME_PRESETS,
} from "@/lib/theme-presets";

export type Theme = "dark" | "light" | "system";

interface ThemeProviderProps {
  children: ReactNode;
  defaultTheme?: Theme;
  storageKey?: string;
  presetStorageKey?: string;
}

export interface ThemeProviderState {
  theme: Theme;
  setTheme: (theme: Theme) => void;
  setThemePreview: (theme: Theme) => void;
  colorPreset: ColorPresetId;
  setColorPreset: (preset: ColorPresetId) => void;
  setColorPresetPreview: (preset: ColorPresetId) => void;
}

export const ThemeProviderContext = createContext<
  ThemeProviderState | undefined
>(undefined);

const DARK_STYLE_ID = "theme-preset-dark";

function resolveEffectiveMode(theme: Theme): "light" | "dark" {
  if (theme === "system") {
    return window.matchMedia("(prefers-color-scheme: dark)").matches
      ? "dark"
      : "light";
  }
  return theme;
}

export function ThemeProvider({
  children,
  defaultTheme = "system",
  storageKey = "vite-ui-theme",
  presetStorageKey = "vite-ui-color-preset",
  ...props
}: ThemeProviderProps) {
  const [theme, setThemeState] = useState<Theme>(
    () => (localStorage.getItem(storageKey) as Theme) || defaultTheme
  );

  const [colorPreset, setColorPresetState] = useState<ColorPresetId>(() => {
    const stored = localStorage.getItem(presetStorageKey);
    return stored && isColorPresetId(stored) ? stored : "default";
  });

  const previousVarsRef = useRef<Set<string>>(new Set());

  // Apply theme mode class and color preset in a single effect to avoid flash
  useEffect(() => {
    const root = document.documentElement;

    // 1. Toggle mode class (only when mode actually changed to avoid repaint)
    const mode = resolveEffectiveMode(theme);
    const currentMode = root.classList.contains("dark") ? "dark" : "light";
    if (currentMode !== mode) {
      root.classList.remove("light", "dark");
      root.classList.add(mode);
    }

    // 2. Apply preset vars without flash
    const presetData = THEME_PRESETS[colorPreset];
    const varsForInline =
      mode === "dark"
        ? { ...presetData.light, ...presetData.dark }
        : presetData.light;
    const newVarNames = new Set(Object.keys(varsForInline));

    // Remove only vars from previous preset that aren't in new preset
    for (const varName of previousVarsRef.current) {
      if (!newVarNames.has(varName)) {
        root.style.removeProperty(`--${varName}`);
      }
    }

    // Apply new vars (overwrite in place)
    for (const [varName, value] of Object.entries(varsForInline)) {
      root.style.setProperty(`--${varName}`, value);
    }

    // 3. Update dark <style> tag (reuse, don't recreate)
    let darkStyle = document.getElementById(
      DARK_STYLE_ID
    ) as HTMLStyleElement | null;
    if (colorPreset !== "default" && Object.keys(presetData.dark).length > 0) {
      if (!darkStyle) {
        darkStyle = document.createElement("style");
        darkStyle.id = DARK_STYLE_ID;
        document.head.appendChild(darkStyle);
      }
      const rules = Object.entries(presetData.dark)
        .map(([v, val]) => `  --${v}: ${val};`)
        .join("\n");
      const newContent = `.dark {\n${rules}\n}`;
      if (darkStyle.textContent !== newContent) {
        darkStyle.textContent = newContent;
      }
    } else if (darkStyle) {
      darkStyle.textContent = "";
    }

    previousVarsRef.current = newVarNames;
  }, [theme, colorPreset]);

  const setTheme = useCallback(
    (nextTheme: Theme) => {
      localStorage.setItem(storageKey, nextTheme);
      setThemeState(nextTheme);
    },
    [storageKey]
  );

  const setThemePreview = useCallback((nextTheme: Theme) => {
    setThemeState(nextTheme);
  }, []);

  const setColorPreset = useCallback(
    (preset: ColorPresetId) => {
      localStorage.setItem(presetStorageKey, preset);
      setColorPresetState(preset);
    },
    [presetStorageKey]
  );

  const setColorPresetPreview = useCallback((preset: ColorPresetId) => {
    setColorPresetState(preset);
  }, []);

  const value = useMemo<ThemeProviderState>(
    () => ({
      theme,
      setTheme,
      setThemePreview,
      colorPreset,
      setColorPreset,
      setColorPresetPreview,
    }),
    [
      theme,
      setTheme,
      setThemePreview,
      colorPreset,
      setColorPreset,
      setColorPresetPreview,
    ]
  );

  return (
    <ThemeProviderContext.Provider {...props} value={value}>
      {children}
    </ThemeProviderContext.Provider>
  );
}
