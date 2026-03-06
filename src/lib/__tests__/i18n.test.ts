// SPDX-License-Identifier: MIT

import { beforeEach, describe, expect, it, vi } from "vitest";

// Unmock react-i18next for this test so we can test real i18n init
vi.unmock("react-i18next");

describe("i18n", () => {
  beforeEach(() => {
    vi.resetModules();
  });

  it("initializes with English as default language", async () => {
    const { default: i18n } = await import("../i18n");
    expect(i18n.language).toBe("en");
  });

  it("has all locale resource bundles loaded", async () => {
    const { default: i18n } = await import("../i18n");
    expect(i18n.hasResourceBundle("en", "common")).toBe(true);
    expect(i18n.hasResourceBundle("de", "common")).toBe(true);
    expect(i18n.hasResourceBundle("es", "common")).toBe(true);
    expect(i18n.hasResourceBundle("fr", "common")).toBe(true);
    expect(i18n.hasResourceBundle("sr", "common")).toBe(true);
  });

  it("resolves translation keys in English", async () => {
    const { default: i18n } = await import("../i18n");
    expect(i18n.t("common.cancel")).toBe("Cancel");
    expect(i18n.t("settings.title")).toBe("Settings");
  });

  it("switches language and resolves keys in German", async () => {
    const { default: i18n } = await import("../i18n");
    await i18n.changeLanguage("de");
    expect(i18n.language).toBe("de");
    expect(i18n.t("common.cancel")).toBe("Abbrechen");
  });

  it("falls back to English for unsupported language", async () => {
    const { default: i18n } = await import("../i18n");
    await i18n.changeLanguage("xx");
    expect(i18n.t("common.cancel")).toBe("Cancel");
  });
});
