import type { ReactNode } from "react";
import { useState } from "react";

import type { AppRoute } from "@/App";
import { ScrollArea } from "@/components/ui/scroll-area";
import { cn } from "@/lib/utils";
import { useTheme } from "@/hooks/useTheme";

import { Sidebar } from "./Sidebar";

interface LayoutProps {
  activeRoute: AppRoute;
  children: ReactNode;
  onRouteChange: (route: AppRoute) => void;
}

export function Layout({ activeRoute, children, onRouteChange }: LayoutProps) {
  const [isSidebarExpanded, setIsSidebarExpanded] = useState(true);
  useTheme(); // 初始化主题（应用 localStorage 或系统偏好）

  return (
    <div className="flex h-screen overflow-hidden bg-background text-foreground">
      <Sidebar
        activeRoute={activeRoute}
        isExpanded={isSidebarExpanded}
        onRouteChange={onRouteChange}
        onToggle={() => setIsSidebarExpanded((current) => !current)}
      />
      <main className="flex min-w-0 flex-1 flex-col bg-background">
        <ScrollArea className="h-full">
          <div
            className={cn(
              "min-h-screen p-4 sm:p-6 lg:p-8",
            )}
          >
            {children}
          </div>
        </ScrollArea>
      </main>
    </div>
  );
}
