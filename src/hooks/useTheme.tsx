import { createContext, useCallback, useContext, useEffect, useMemo, useState } from "react";
import type { ReactNode } from "react";

export const THEME_OPTIONS = [
  { id: "auto", labelKey: "settings.auto" },
  { id: "light", labelKey: "settings.light" },
  { id: "dark", labelKey: "settings.dark" },
] as const;

export type ThemeId = (typeof THEME_OPTIONS)[number]["id"];

type ResolvedTheme = "light" | "dark";

interface ThemeContextValue {
  theme: ThemeId;
  resolvedTheme: ResolvedTheme;
  setTheme: (theme: ThemeId) => void;
  toggleTheme: () => void;
}

const ThemeContext = createContext<ThemeContextValue | null>(null);

function systemTheme(): ResolvedTheme {
  return window.matchMedia("(prefers-color-scheme: light)").matches ? "light" : "dark";
}

function resolveTheme(theme: ThemeId): ResolvedTheme {
  return theme === "auto" ? systemTheme() : theme;
}

function applyThemeAttribute(theme: ResolvedTheme) {
  document.documentElement.dataset.theme = theme;
}

function readStoredTheme(): ThemeId {
  const stored = localStorage.getItem("diskpulse.theme");
  return stored === "light" || stored === "dark" || stored === "auto" ? stored : "auto";
}

export function ThemeProvider({ children }: { children: ReactNode }) {
  const [theme, setThemeState] = useState<ThemeId>(readStoredTheme);
  const [resolvedTheme, setResolvedTheme] = useState<ResolvedTheme>(() => resolveTheme(readStoredTheme()));

  const setTheme = useCallback((nextTheme: ThemeId) => {
    localStorage.setItem("diskpulse.theme", nextTheme);
    setThemeState(nextTheme);
    const resolved = resolveTheme(nextTheme);
    setResolvedTheme(resolved);
    applyThemeAttribute(resolved);
  }, []);

  const toggleTheme = useCallback(() => {
    setTheme(resolvedTheme === "dark" ? "light" : "dark");
  }, [resolvedTheme, setTheme]);

  useEffect(() => {
    const media = window.matchMedia("(prefers-color-scheme: light)");
    const sync = () => {
      const resolved = resolveTheme(theme);
      setResolvedTheme(resolved);
      applyThemeAttribute(resolved);
    };
    sync();
    media.addEventListener("change", sync);
    return () => media.removeEventListener("change", sync);
  }, [theme]);

  const value = useMemo(() => ({ theme, resolvedTheme, setTheme, toggleTheme }), [theme, resolvedTheme, setTheme, toggleTheme]);

  return <ThemeContext.Provider value={value}>{children}</ThemeContext.Provider>;
}

export function useTheme() {
  const value = useContext(ThemeContext);
  if (!value) {
    throw new Error("useTheme must be used inside ThemeProvider");
  }
  return value;
}
