import { useState, useEffect } from "react";

/**
 * 主题类型：light（浅色）、dark（深色）、system（跟随系统）
 */
type Theme = "light" | "dark" | "system";

const STORAGE_KEY = "ptv-theme";

/**
 * 从 localStorage 读取初始主题，若无则返回 system
 */
function getInitialTheme(): Theme {
  const stored = localStorage.getItem(STORAGE_KEY);
  if (stored === "light" || stored === "dark" || stored === "system") return stored;
  return "system";
}

/**
 * 将指定主题应用到 document.documentElement
 * - dark：添加 .dark 类
 * - light：移除 .dark 类
 * - system：根据 prefers-color-scheme 自动切换
 */
function applyTheme(theme: Theme) {
  const root = document.documentElement;
  if (theme === "dark") {
    root.classList.add("dark");
  } else if (theme === "light") {
    root.classList.remove("dark");
  } else {
    // system
    const prefersDark = window.matchMedia("(prefers-color-scheme: dark)").matches;
    root.classList.toggle("dark", prefersDark);
  }
}

/**
 * 主题管理 Hook
 *
 * 功能：
 * - 读取 localStorage 持久化主题
 * - 监听系统主题变化（当选择 system 时）
 * - 提供 toggleTheme() 循环切换 light → dark → system
 */
export function useTheme() {
  const [theme, setThemeState] = useState<Theme>(getInitialTheme);

  // 主题变化时应用并持久化
  useEffect(() => {
    applyTheme(theme);
    localStorage.setItem(STORAGE_KEY, theme);
  }, [theme]);

  // 监听系统主题变化（仅在 system 模式下响应）
  useEffect(() => {
    const media = window.matchMedia("(prefers-color-scheme: dark)");
    const handler = () => {
      if (theme === "system") applyTheme("system");
    };
    media.addEventListener("change", handler);
    return () => media.removeEventListener("change", handler);
  }, [theme]);

  /**
   * 循环切换主题：light → dark → system → light
   */
  const toggleTheme = () => {
    setThemeState((prev) => {
      if (prev === "light") return "dark";
      if (prev === "dark") return "system";
      return "light";
    });
  };

  return { theme, setTheme: setThemeState, toggleTheme };
}
