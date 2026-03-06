// SPDX-License-Identifier: MIT

import { describe, expect, it } from "vitest";
import {
  DEFAULT_LOCALE,
  isSupportedLocale,
  LOCALE_LABELS,
  SUPPORTED_LOCALES,
} from "../i18n-constants";

describe("i18n-constants", () => {
  it("has a default locale of 'en'", () => {
    expect(DEFAULT_LOCALE).toBe("en");
  });

  it("has labels for every supported locale", () => {
    for (const locale of SUPPORTED_LOCALES) {
      expect(LOCALE_LABELS[locale]).toBeDefined();
      expect(typeof LOCALE_LABELS[locale]).toBe("string");
      expect(LOCALE_LABELS[locale].length).toBeGreaterThan(0);
    }
  });

  describe("isSupportedLocale", () => {
    it("returns true for supported locales", () => {
      for (const locale of SUPPORTED_LOCALES) {
        expect(isSupportedLocale(locale)).toBe(true);
      }
    });

    it("returns false for unsupported values", () => {
      expect(isSupportedLocale("xx")).toBe(false);
      expect(isSupportedLocale("")).toBe(false);
      expect(isSupportedLocale("english")).toBe(false);
    });
  });
});
