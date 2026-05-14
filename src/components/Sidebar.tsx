import { Bot, History, LayoutDashboard, PanelLeftClose, PanelLeftOpen, Settings } from "lucide-react";

import type { AppRoute } from "@/App";
import { Button } from "@/components/ui/button";
import { ThemeToggle } from "@/components/ThemeToggle";
import { cn } from "@/lib/utils";

interface SidebarProps {
  activeRoute: AppRoute;
  isExpanded: boolean;
  onRouteChange: (route: AppRoute) => void;
  onToggle: () => void;
}

const navigationItems: Array<{
  icon: typeof LayoutDashboard;
  label: string;
  route: AppRoute;
}> = [
  { icon: LayoutDashboard, label: "仪表盘", route: "dashboard" },
  { icon: Bot, label: "代理监控", route: "agents" },
  { icon: History, label: "会话管理", route: "claude-history" },
  { icon: Settings, label: "设置", route: "settings" },
];

export function Sidebar({ activeRoute, isExpanded, onRouteChange, onToggle }: SidebarProps) {
  return (
    <aside
      className={cn(
        "flex h-screen shrink-0 flex-col border-r border-sidebar-border bg-sidebar text-sidebar-foreground transition-[width] duration-200",
        isExpanded ? "w-[var(--sidebar-width-expanded)]" : "w-[var(--sidebar-width-collapsed)]",
      )}
    >
      <div className="flex h-14 items-center gap-3 border-b border-sidebar-border px-3">
        <div className="flex size-8 shrink-0 items-center justify-center rounded-md bg-primary text-primary-foreground">
          <LayoutDashboard className="size-4" aria-hidden="true" />
        </div>
        {isExpanded && (
          <div className="min-w-0">
            <p className="truncate text-sm font-semibold">AgentScope</p>
          </div>
        )}
      </div>

      <nav className="flex flex-1 flex-col gap-0.5 p-2" aria-label="主导航">
        {navigationItems.map((item) => {
          const Icon = item.icon;
          const isActive = activeRoute === item.route;

          return (
            <Button
              key={item.route}
              type="button"
              variant="ghost"
              className={cn(
                "h-9 justify-start gap-3 rounded-md px-3 text-sidebar-foreground/80 hover:bg-sidebar-accent hover:text-sidebar-foreground",
                !isExpanded && "justify-center px-0",
                isActive && "bg-sidebar-accent text-sidebar-accent-foreground font-medium",
              )}
              aria-current={isActive ? "page" : undefined}
              title={!isExpanded ? item.label : undefined}
              onClick={() => onRouteChange(item.route)}
            >
              <Icon className="size-4 shrink-0" aria-hidden="true" />
              {isExpanded && <span className="text-sm">{item.label}</span>}
            </Button>
          );
        })}
      </nav>

      <div className="border-t border-sidebar-border p-2">
        <div className={cn("flex items-center gap-1", !isExpanded && "justify-center")}>
          <ThemeToggle />
          <Button
            type="button"
            variant="ghost"
            size="icon"
            className={cn("h-9 w-9", !isExpanded && "hidden")}
            aria-label={isExpanded ? "收起侧边栏" : "展开侧边栏"}
            onClick={onToggle}
          >
            <PanelLeftClose className="size-4" />
          </Button>
          <Button
            type="button"
            variant="ghost"
            size="icon"
            className={cn("h-9 w-9", isExpanded && "hidden")}
            aria-label={isExpanded ? "收起侧边栏" : "展开侧边栏"}
            onClick={onToggle}
          >
            <PanelLeftOpen className="size-4" />
          </Button>
        </div>
      </div>
    </aside>
  );
}
