import { Sun, Moon, Monitor } from "lucide-react";

import { Button } from "@/components/ui/button";
import { useTheme } from "@/hooks/useTheme";

/**
 * 主题切换按钮
 *
 * 循环切换 light → dark → system，显示对应图标。
 */
export function ThemeToggle() {
  const { theme, toggleTheme } = useTheme();

  const icon =
    theme === "light" ? (
      <Sun className="size-4" />
    ) : theme === "dark" ? (
      <Moon className="size-4" />
    ) : (
      <Monitor className="size-4" />
    );

  const label =
    theme === "light" ? "浅色模式" : theme === "dark" ? "深色模式" : "跟随系统";

  return (
    <Button
      variant="ghost"
      size="icon"
      onClick={toggleTheme}
      title={label}
      aria-label={label}
    >
      {icon}
    </Button>
  );
}
