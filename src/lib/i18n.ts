// SPDX-License-Identifier: MIT

import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import { DEFAULT_LOCALE } from "./i18n-constants";

import en from "@/locales/en/common.json";
import de from "@/locales/de/common.json";
import es from "@/locales/es/common.json";
import fr from "@/locales/fr/common.json";
import sr from "@/locales/sr/common.json";

void i18n.use(initReactI18next).init({
  resources: {
    en: { common: en },
    de: { common: de },
    es: { common: es },
    fr: { common: fr },
    sr: { common: sr },
  },
  lng: DEFAULT_LOCALE,
  defaultNS: "common",
  fallbackLng: DEFAULT_LOCALE,
  interpolation: {
    escapeValue: false,
  },
});

export default i18n;
