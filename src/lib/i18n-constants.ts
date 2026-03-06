// SPDX-License-Identifier: MIT

export const SUPPORTED_LOCALES = ["en", "de", "es", "fr", "sr"] as const;

export type SupportedLocale = (typeof SUPPORTED_LOCALES)[number];

export const DEFAULT_LOCALE: SupportedLocale = "en";

export const LOCALE_LABELS: Record<SupportedLocale, string> = {
  en: "English",
  de: "Deutsch",
  es: "Español",
  fr: "Français",
  sr: "Srpski",
};

export function isSupportedLocale(value: string): value is SupportedLocale {
  return (SUPPORTED_LOCALES as readonly string[]).includes(value);
}
