// SPDX-License-Identifier: MIT

import { beforeEach, describe, expect, it, vi } from "vitest";
import { renderHook } from "@testing-library/react";
import { useLanguageSync } from "../use-language-sync";

const mockChangeLanguage = vi.fn().mockResolvedValue(undefined);

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => key,
    i18n: {
      language: "en",
      changeLanguage: mockChangeLanguage,
    },
  }),
}));

const mockUseAppPreferences = vi.fn();

vi.mock("@/hooks/use-app-preferences", () => ({
  useAppPreferences: () => mockUseAppPreferences(),
}));

describe("useLanguageSync", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("calls changeLanguage when preference differs from current", () => {
    mockUseAppPreferences.mockReturnValue({
      preferences: { general: { language: "de" } },
    });

    renderHook(() => useLanguageSync());

    expect(mockChangeLanguage).toHaveBeenCalledWith("de");
  });

  it("does not call changeLanguage when already matching", () => {
    mockUseAppPreferences.mockReturnValue({
      preferences: { general: { language: "en" } },
    });

    renderHook(() => useLanguageSync());

    expect(mockChangeLanguage).not.toHaveBeenCalled();
  });

  it("falls back to default locale for unsupported language", () => {
    mockUseAppPreferences.mockReturnValue({
      preferences: { general: { language: "xx" } },
    });

    renderHook(() => useLanguageSync());

    // "xx" is not supported, falls back to "en" which matches i18n.language
    expect(mockChangeLanguage).not.toHaveBeenCalled();
  });

  it("does nothing when preferences are null", () => {
    mockUseAppPreferences.mockReturnValue({
      preferences: null,
    });

    renderHook(() => useLanguageSync());

    expect(mockChangeLanguage).not.toHaveBeenCalled();
  });
});
