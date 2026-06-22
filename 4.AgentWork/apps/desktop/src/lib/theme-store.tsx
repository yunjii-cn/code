// 主题系统
// 亮/暗双模式，localStorage 持久化，默认暗黑

import { createContext, useContext, useState, useEffect, type ReactNode } from "react";

// ============================================================================
// 主题定义
// ============================================================================

export type ThemeName = "dark" | "light";

export interface ThemeDef {
  name: ThemeName;
  label: string;
  // 背景色系
  bg: {
    primary: string;
    secondary: string;
    surface: string;
    elevated: string;
  };
  // 文字色系
  text: {
    primary: string;
    secondary: string;
    muted: string;
    inverse: string;
  };
  // 边框色
  border: string;
  // 品牌强调色
  brand: {
    primary: string;
    hover: string;
    light: string;
  };
  // 成功/警告/错误
  status: {
    success: string;
    warning: string;
    error: string;
  };
}

// 2 套主题
export const THEMES: Record<ThemeName, ThemeDef> = {
  // 暗黑（默认）
  dark: {
    name: "dark",
    label: "暗黑",
    bg: {
      primary: "#09090b",       // zinc-950
      secondary: "#18181b",     // zinc-900
      surface: "#1f1f23",
      elevated: "#27272a",      // zinc-800
    },
    text: {
      primary: "#fafafa",       // zinc-50
      secondary: "#a1a1aa",     // zinc-400
      muted: "#71717a",         // zinc-500
      inverse: "#09090b",       // zinc-950
    },
    border: "#27272a",          // zinc-800
    brand: {
      primary: "#3b82f6",       // blue-500
      hover: "#2563eb",         // blue-600
      light: "#1e3a8a",         // blue-950
    },
    status: {
      success: "#22c55e",       // green-500
      warning: "#eab308",       // yellow-500
      error: "#ef4444",         // red-500
    },
  },

  // 明亮
  light: {
    name: "light",
    label: "明亮",
    bg: {
      primary: "#ffffff",
      secondary: "#f8fafc",     // slate-50
      surface: "#f1f5f9",       // slate-100
      elevated: "#e2e8f0",      // slate-200
    },
    text: {
      primary: "#0f172a",       // slate-900
      secondary: "#475569",     // slate-600
      muted: "#94a3b8",         // slate-400
      inverse: "#ffffff",
    },
    border: "#cbd5e1",          // slate-300
    brand: {
      primary: "#3b82f6",       // blue-500
      hover: "#2563eb",         // blue-600
      light: "#dbeafe",         // blue-100
    },
    status: {
      success: "#10b981",       // emerald-500
      warning: "#f59e0b",       // amber-500
      error: "#ef4444",         // red-500
    },
  },
};

// ============================================================================
// Context
// ============================================================================

interface ThemeContextValue {
  currentTheme: ThemeName;
  themeDef: ThemeDef;
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

  const themeDef = THEMES[currentTheme];
  const isLight = currentTheme === "light";

  // 应用主题到 data-theme attribute（CSS 中已定义所有变量）
  useEffect(() => {
    const root = document.documentElement;
    if (currentTheme === "dark") {
      root.removeAttribute("data-theme");
    } else {
      root.setAttribute("data-theme", currentTheme);
    }

    // 保存到 localStorage
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
    <ThemeContext.Provider value={{ currentTheme, themeDef, setTheme, toggleTheme, isLight }}>
      {children}
    </ThemeContext.Provider>
  );
}

export function useTheme(): ThemeContextValue {
  const ctx = useContext(ThemeContext);
  if (!ctx) throw new Error("useTheme must be used within ThemeProvider");
  return ctx;
}
