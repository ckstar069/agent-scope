import { CheckCircle2, Monitor, Moon, Sun, Type } from "lucide-react";

import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { useFontSize, type FontSize } from "@/hooks/useFontSize";
import { useTheme, type Theme } from "@/hooks/useTheme";
import { cn } from "@/lib/utils";

const fontSizeOptions: { value: FontSize; label: string; description: string }[] = [
  { value: "compact", label: "紧凑", description: "13px — 适合小屏 / 更多信息密度" },
  { value: "normal", label: "标准", description: "15px — 平衡阅读与信息密度" },
  { value: "large", label: "大字号", description: "17px — 大屏高分辨率首选" },
  { value: "xlarge", label: "超大字号", description: "19px — 远距离 / 视力辅助" },
];

const themeOptions: { value: Theme; label: string; icon: typeof Sun; description: string }[] = [
  { value: "light", label: "浅色", icon: Sun, description: "始终使用浅色主题" },
  { value: "dark", label: "深色", icon: Moon, description: "始终使用深色主题" },
  { value: "system", label: "跟随系统", icon: Monitor, description: "根据系统设置自动切换" },
];

export function GeneralSettings() {
  const { fontSize, setFontSize } = useFontSize();

  return (
    <section className="space-y-6">
      <div className="space-y-2">
        <p className="text-sm font-medium text-muted-foreground">Settings</p>
        <h1 className="text-3xl font-semibold tracking-tight">设置</h1>
        <p className="max-w-2xl text-sm text-muted-foreground">
          调整 AgentScope 的界面外观与交互偏好。
        </p>
      </div>

      <Card className="overflow-hidden shadow-xs">
        <div className="h-1 bg-gradient-to-r from-primary/80 via-primary/30 to-transparent" />
        <CardHeader>
          <div className="flex items-start gap-3">
            <div className="flex size-10 shrink-0 items-center justify-center rounded-lg bg-muted text-muted-foreground">
              <Type className="size-5" aria-hidden="true" />
            </div>
            <div className="space-y-1">
              <CardTitle>界面字号</CardTitle>
              <CardDescription>调整全局文字大小。较大字号会自动优化布局以防止内容溢出。</CardDescription>
            </div>
          </div>
        </CardHeader>
        <CardContent>
          <div className="grid gap-3 sm:grid-cols-3">
            {fontSizeOptions.map((option) => (
              <button
                key={option.value}
                type="button"
                onClick={() => setFontSize(option.value)}
                className={cn(
                  "relative flex flex-col items-start gap-1 rounded-xl border p-4 text-left shadow-xs transition-all",
                  fontSize === option.value
                    ? "border-primary bg-primary/8"
                    : "border-border bg-tile hover:border-primary/30 hover:bg-muted/40",
                )}
              >
                {fontSize === option.value && (
                  <span className="absolute right-3 top-3 flex size-5 items-center justify-center rounded-full bg-primary text-primary-foreground">
                    <CheckCircle2 className="size-3.5" aria-hidden="true" />
                  </span>
                )}
                <span className="text-sm font-semibold">{option.label}</span>
                <span className="text-xs text-muted-foreground">{option.description}</span>
              </button>
            ))}
          </div>
        </CardContent>
      </Card>

      <Card className="overflow-hidden shadow-xs">
        <div className="h-1 bg-gradient-to-r from-primary/80 via-primary/30 to-transparent" />
        <CardHeader>
          <div className="flex items-start gap-3">
            <div className="flex size-10 shrink-0 items-center justify-center rounded-lg bg-muted text-muted-foreground">
              <Sun className="size-5" aria-hidden="true" />
            </div>
            <div className="space-y-1">
              <CardTitle>界面主题</CardTitle>
              <CardDescription>选择浅色、深色或跟随系统偏好。</CardDescription>
            </div>
          </div>
        </CardHeader>
        <CardContent>
          <ThemeSelector />
        </CardContent>
      </Card>
    </section>
  );
}

function ThemeSelector() {
  const { theme, setTheme } = useTheme();

  return (
    <div className="grid gap-3 sm:grid-cols-3">
      {themeOptions.map((option) => {
        const Icon = option.icon;
        return (
          <button
            key={option.value}
            type="button"
            onClick={() => setTheme(option.value)}
            className={cn(
              "relative flex flex-col items-start gap-2 rounded-xl border p-4 text-left shadow-xs transition-all",
              theme === option.value
                ? "border-primary bg-primary/8"
                : "border-border bg-tile hover:border-primary/30 hover:bg-muted/40",
            )}
          >
            {theme === option.value && (
              <span className="absolute right-3 top-3 flex size-5 items-center justify-center rounded-full bg-primary text-primary-foreground">
                <CheckCircle2 className="size-3.5" aria-hidden="true" />
              </span>
            )}
            <Icon className="size-5 text-foreground/80" aria-hidden="true" />
            <div className="space-y-0.5">
              <span className="text-sm font-semibold">{option.label}</span>
              <span className="block text-xs text-muted-foreground">{option.description}</span>
            </div>
          </button>
        );
      })}
    </div>
  );
}
