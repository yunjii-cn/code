// 极简 i18n：自研 Context + 资源切换，不引入 react-i18next 等重依赖
import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import zhCN from "./zh-CN";
import enUS from "./en-US";
import jaJP from "./ja-JP";
import koKR from "./ko-KR";
import esES from "./es-ES";
import type { TranslationDict } from "./types";

export type Locale = "zh-CN" | "en-US" | "ja-JP" | "ko-KR" | "es-ES";
export type DisplayLanguage = Locale;

export const SUPPORTED_LOCALES: DisplayLanguage[] = [
  "zh-CN",
  "en-US",
  "ja-JP",
  "ko-KR",
  "es-ES",
];
export const LOCALE_STORAGE_KEY = "yunji-locale";

const DICTS: Record<Locale, TranslationDict> = {
  "zh-CN": zhCN as unknown as TranslationDict,
  "en-US": enUS as unknown as TranslationDict,
  "ja-JP": jaJP as unknown as TranslationDict,
  "ko-KR": koKR as unknown as TranslationDict,
  "es-ES": esES as unknown as TranslationDict,
};

interface I18nContextValue {
  locale: Locale;
  displayLanguage: DisplayLanguage;
  setLocale: (l: Locale) => void;
  setDisplayLanguage: (d: DisplayLanguage) => void;
  t: (key: string, vars?: Record<string, string | number>) => string;
}

const I18nContext = createContext<I18nContextValue | null>(null);

function detectInitialLocale(): Locale {
  try {
    const stored = localStorage.getItem(LOCALE_STORAGE_KEY) as Locale | null;
    if (stored && stored in DICTS) return stored;
  } catch {
    // ignore
  }
  try {
    const nav = (navigator.language || "").toLowerCase();
    if (nav.startsWith("zh")) return "zh-CN";
    if (nav.startsWith("ja")) return "ja-JP";
    if (nav.startsWith("ko")) return "ko-KR";
    if (nav.startsWith("es")) return "es-ES";
    return "en-US";
  } catch {
    return "zh-CN";
  }
}

export function I18nProvider({ children }: { children: ReactNode }) {
  const [locale, setLocaleState] = useState<Locale>(detectInitialLocale);

  useEffect(() => {
    try {
      document.documentElement.lang = locale;
    } catch {
      // ignore
    }
    try {
      localStorage.setItem(LOCALE_STORAGE_KEY, locale);
    } catch {
      // ignore
    }
  }, [locale]);

  const setLocale = useCallback((l: Locale) => {
    setLocaleState(l);
  }, []);

  const setDisplayLanguage = useCallback(
    (d: DisplayLanguage) => setLocale(d),
    [setLocale]
  );

  const dict = DICTS[locale];

  const t = useCallback(
    (key: string, vars?: Record<string, string | number>) => {
      const raw = dict[key] ?? DICTS["en-US"][key] ?? key;
      if (!vars) return raw;
      return Object.entries(vars).reduce(
        (acc, [k, v]) => acc.replace(new RegExp(`\\{${k}\\}`, "g"), String(v)),
        raw
      );
    },
    [dict]
  );

  const value = useMemo<I18nContextValue>(
    () => ({
      locale,
      displayLanguage: locale,
      setLocale,
      setDisplayLanguage,
      t,
    }),
    [locale, setLocale, setDisplayLanguage, t]
  );

  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
}

export function useI18n(): I18nContextValue {
  const ctx = useContext(I18nContext);
  if (!ctx) throw new Error("useI18n must be used within I18nProvider");
  return ctx;
}

export function useT() {
  return useI18n().t;
}