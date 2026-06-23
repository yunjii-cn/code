// 主题系统
// 暗/亮双模式 + Tailwind darkMode:class，localStorage 持久化，默认暗黑

import { createContext, useContext, useState, useEffect, type ReactNode } from "react";

export type ThemeName = "dark" | "light";

interface ThemeContextValue {
  currentTheme: ThemeName;
  setTheme: (name: ThemeName) => void;
  toggleTheme: () => void;
  isLight: boolean;
}

const ThemeContext = createContext<ThemeContextValue | null>(null);

export function ThemeProvider({ children }: { children: ReactNode }) {
  const [currentTheme, setCurrentTheme] = useState<ThemeName>(() => {
    try {
      return (localStorage.getItem("yunji-theme") as ThemeName) || "dark";
    } catch {
      return "dark";
    }
  });

  const isLight = currentTheme === "light";

  useEffect(() => {
    const root = document.documentElement;
    if (currentTheme === "dark") {
      root.classList.add("dark");
      root.classList.remove("light");
    } else {
      root.classList.add("light");
      root.classList.remove("dark");
    }
    try {
      localStorage.setItem("yunji-theme", currentTheme);
    } catch {
      // ignore
    }
  }, [currentTheme]);

  const setTheme = (name: ThemeName) => {
    setCurrentTheme(name);
  };

  const toggleTheme = () => {
    setCurrentTheme((prev) => (prev === "dark" ? "light" : "dark"));
  };

  return (
    <ThemeContext.Provider value={{ currentTheme, setTheme, toggleTheme, isLight }}>
      {children}
    </ThemeContext.Provider>
  );
}

export function useTheme(): ThemeContextValue {
  const ctx = useContext(ThemeContext);
  if (!ctx) throw new Error("useTheme must be used within ThemeProvider");
  return ctx;
}