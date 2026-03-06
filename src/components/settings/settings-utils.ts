// SPDX-License-Identifier: MIT

import type { AppPreferences, KdfSettings } from "@/lib/types";

export const THEME_OPTIONS = ["system", "light", "dark"] as const;
export const FONT_SIZE_MIN = 10;
export const FONT_SIZE_MAX = 24;

export function isThemePreference(
  value: string
): value is AppPreferences["appearance"]["theme"] {
  return THEME_OPTIONS.includes(value as (typeof THEME_OPTIONS)[number]);
}

export function parseAllowedSites(input: string): string[] {
  return input
    .split(/[\n,]/)
    .map((site) => site.trim())
    .filter((site) => site.length > 0);
}

export function formatAllowedSites(sites: string[]): string {
  return sites.join("\n");
}

export function formatKdf(kdf: KdfSettings): string {
  if (kdf.type === "aesKdf") {
    return `AES-KDF (${kdf.rounds} rounds)`;
  }

  return `${kdf.type} (${kdf.memory} bytes, ${kdf.iterations} iterations, ${kdf.parallelism} lanes)`;
}
