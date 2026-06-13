import { createContext, useContext, useState, useCallback } from "react";
import zh from "./zh";
import en from "./en";

const LOCALE_KEY = "quickshare-locale";

const messages = { zh, en };

const defaultLocale = () => {
  // 1. 用户之前的选择
  const saved = localStorage.getItem(LOCALE_KEY);
  if (saved && messages[saved]) return saved;
  // 2. 浏览器语言偏好
  const lang = navigator.language || navigator.userLanguage || "zh";
  if (lang.startsWith("zh")) return "zh";
  return "en";
};

const I18nContext = createContext();

export function I18nProvider({ children }) {
  const [locale, setLocaleState] = useState(defaultLocale());

  const setLocale = useCallback((newLocale) => {
    if (messages[newLocale]) {
      setLocaleState(newLocale);
      localStorage.setItem(LOCALE_KEY, newLocale);
    }
  }, []);

  const t = useCallback(
    (key) => {
      const parts = key.split(".");
      let val = messages[locale];
      for (const p of parts) {
        val = val?.[p];
      }
      // fallback to English
      if (val === undefined) {
        let enVal = messages.en;
        for (const p of parts) {
          enVal = enVal?.[p];
        }
        return enVal ?? key;
      }
      return val;
    },
    [locale]
  );

  // t.withCount for plural-aware strings like "{count} device(s)"
  const tc = useCallback(
    (key, vars) => {
      let str = t(key);
      if (vars) {
        Object.entries(vars).forEach(([k, v]) => {
          str = str.replace(`{${k}}`, v);
        });
      }
      return str;
    },
    [t]
  );

  return (
    <I18nContext.Provider value={{ locale, setLocale, t, tc }}>
      {children}
    </I18nContext.Provider>
  );
}

export function useI18n() {
  const ctx = useContext(I18nContext);
  if (!ctx) throw new Error("useI18n must be used within I18nProvider");
  return ctx;
}

export const availableLocales = [
  { code: "zh", label: "简体中文" },
  { code: "en", label: "English" },
];
