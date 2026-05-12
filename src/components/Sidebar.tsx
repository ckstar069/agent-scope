import { Bot, History, LayoutDashboard, PanelLeftClose, PanelLeftOpen, Settings } from "lucide-react";

import type { AppRoute } from "@/App";
import { Button } from "@/components/ui/button";
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
      <div className="flex h-16 items-center gap-3 border-b border-sidebar-border px-3">
        <div className="flex size-9 shrink-0 items-center justify-center rounded-lg bg-sidebar-primary text-sidebar-primary-foreground">
          <LayoutDashboard className="size-4" aria-hidden="true" />
        </div>
        {isExpanded && (
          <div className="min-w-0">
            <p className="truncate text-sm font-semibold">ptv</p>
            <p className="truncate text-xs text-muted-foreground">Project Visualizer</p>
          </div>
        )}
      </div>

      <nav className="flex flex-1 flex-col gap-1 p-3" aria-label="主导航">
        {navigationItems.map((item) => {
          const Icon = item.icon;
          const isActive = activeRoute === item.route;

          return (
            <Button
              key={item.route}
              type="button"
              variant={isActive ? "secondary" : "ghost"}
              className={cn(
                "h-10 justify-start text-sidebar-foreground hover:bg-sidebar-accent hover:text-sidebar-accent-foreground",
                !isExpanded && "justify-center px-0",
                isActive && "bg-sidebar-accent text-sidebar-accent-foreground",
              )}
              aria-current={isActive ? "page" : undefined}
              title={!isExpanded ? item.label : undefined}
              onClick={() => onRouteChange(item.route)}
            >
              <Icon className="size-4" aria-hidden="true" />
              {isExpanded && <span>{item.label}</span>}
            </Button>
          );
        })}
      </nav>

      <div className="border-t border-sidebar-border p-3">
        <Button
          type="button"
          variant="outline"
          className={cn("w-full justify-start", !isExpanded && "justify-center px-0")}
          aria-label={isExpanded ? "收起侧边栏" : "展开侧边栏"}
          onClick={onToggle}
        >
          {isExpanded ? <PanelLeftClose className="size-4" /> : <PanelLeftOpen className="size-4" />}
          {isExpanded && <span>收起</span>}
        </Button>
      </div>
    </aside>
  );
}
