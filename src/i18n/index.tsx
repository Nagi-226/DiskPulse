import i18n from "i18next";
import { I18nextProvider, initReactI18next } from "react-i18next";
import type { ReactNode } from "react";
import en from "./locales/en.json";
import zhCN from "./locales/zh-CN.json";

export const LANGUAGE_OPTIONS = [
  { id: "auto", labelKey: "settings.auto" },
  { id: "en", labelKey: "settings.english" },
  { id: "zh-CN", labelKey: "settings.chinese" },
] as const;

export type LanguageId = (typeof LANGUAGE_OPTIONS)[number]["id"];

export function resolveLanguage(language: string): "en" | "zh-CN" {
  if (language === "zh-CN") return "zh-CN";
  if (language === "en") return "en";
  return navigator.language.toLowerCase().startsWith("zh") ? "zh-CN" : "en";
}

void i18n.use(initReactI18next).init({
  resources: {
    en: { translation: en },
    "zh-CN": { translation: zhCN },
  },
  lng: resolveLanguage(localStorage.getItem("diskpulse.language") ?? "auto"),
  fallbackLng: "en",
  interpolation: { escapeValue: false },
  returnNull: false,
});

export function applyLanguage(language: string) {
  localStorage.setItem("diskpulse.language", language);
  void i18n.changeLanguage(resolveLanguage(language));
}

export function DiskPulseI18nProvider({ children }: { children: ReactNode }) {
  return <I18nextProvider i18n={i18n}>{children}</I18nextProvider>;
}

export default i18n;
