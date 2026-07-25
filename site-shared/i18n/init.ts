import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import LanguageDetector from "i18next-browser-languagedetector";

/**
 * Initialize i18n with site-specific locales.
 * @param zh - Chinese locale object
 * @param en - English locale object
 */
export function initI18n(zh: Record<string, unknown>, en: Record<string, unknown>) {
  i18n
    .use(LanguageDetector)
    .use(initReactI18next)
    .init({
      resources: { zh: { translation: zh }, en: { translation: en } },
      fallbackLng: "zh",
      interpolation: { escapeValue: false },
      detection: {
        order: ["localStorage", "navigator"],
        caches: ["localStorage"],
      },
    });

  return i18n;
}
