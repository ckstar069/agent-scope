import type { ReactNode } from "react";
import { useState } from "react";

import type { AppDomain } from "@/App";
import { ScrollArea } from "@/components/ui/scroll-area";
import { cn } from "@/lib/utils";
import { useTheme } from "@/hooks/useTheme";

import { Sidebar } from "./Sidebar";
import { TopNav } from "./TopNav";

interface LayoutProps {
  activeDomain: AppDomain;
  activePage: string;
  selectedProject: string;
  onDomainChange: (domain: AppDomain) => void;
  onProjectPageChange: (page: string) => void;
  onMonitoringPageChange: (page: string) => void;
  onSettingsPageChange: (page: string) => void;
  onSelectProject: (projectPath: string) => void;
  children: ReactNode;
}

export function Layout({
  activeDomain,
  activePage,
  selectedProject,
  onDomainChange,
  onProjectPageChange,
  onMonitoringPageChange,
  onSettingsPageChange,
  onSelectProject,
  children,
}: LayoutProps) {
  const [isSidebarExpanded, setIsSidebarExpanded] = useState(true);
  useTheme();

  return (
    <div className="flex h-screen flex-col overflow-hidden bg-background text-foreground">
      <TopNav activeDomain={activeDomain} onDomainChange={onDomainChange} />
      <div className="flex min-h-0 flex-1">
        <Sidebar
          activeDomain={activeDomain}
          activePage={activePage}
          selectedProject={selectedProject}
          isExpanded={isSidebarExpanded}
          onToggle={() => setIsSidebarExpanded((current) => !current)}
          onProjectPageChange={onProjectPageChange}
          onMonitoringPageChange={onMonitoringPageChange}
          onSettingsPageChange={onSettingsPageChange}
          onSelectProject={onSelectProject}
        />
        <main className="flex min-w-0 flex-1 flex-col bg-background">
          <ScrollArea className="h-full">
            <div className={cn("min-h-full p-4 sm:p-6 lg:p-8")}>{children}</div>
          </ScrollArea>
        </main>
      </div>
    </div>
  );
}
