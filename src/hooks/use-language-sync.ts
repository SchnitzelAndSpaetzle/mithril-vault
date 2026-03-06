// SPDX-License-Identifier: MIT

import { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { useAppPreferences } from "@/hooks/use-app-preferences";
import { DEFAULT_LOCALE, isSupportedLocale } from "@/lib/i18n-constants";

export function useLanguageSync() {
  const { i18n } = useTranslation();
  const { preferences } = useAppPreferences();
  const language = preferences?.general.language;

  useEffect(() => {
    if (!language) return;
    const locale = isSupportedLocale(language) ? language : DEFAULT_LOCALE;
    if (i18n.language !== locale) {
      void i18n.changeLanguage(locale);
    }
  }, [language, i18n]);
}
