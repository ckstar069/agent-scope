import { useCallback, useEffect, useMemo, useState } from "react";
import {
  Bot,
  Brain,
  FolderKanban,
  History,
  LayoutDashboard,
  PanelLeftClose,
  PanelLeftOpen,
  Route,
  Settings,
} from "lucide-react";

import type { AppDomain } from "@/App";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import { useTauri } from "@/hooks/useTauri";
import { cn } from "@/lib/utils";
import type { ProjectEntry } from "@/lib/types";

import { ThemeToggle } from "./ThemeToggle";

interface SidebarProps {
  activeDomain: AppDomain;
  activePage: string;
  selectedProject: string;
  isExpanded: boolean;
  onToggle: () => void;
  onProjectPageChange: (page: string) => void;
  onMonitoringPageChange: (page: string) => void;
  onSettingsPageChange: (page: string) => void;
  onSelectProject: (projectPath: string) => void;
}

const collator = new Intl.Collator("zh-CN", { numeric: true, sensitivity: "base" });

export function Sidebar({
  activeDomain,
  activePage,
  selectedProject,
  isExpanded,
  onToggle,
  onProjectPageChange,
  onMonitoringPageChange,
  onSettingsPageChange,
  onSelectProject,
}: SidebarProps) {
  const { invoke } = useTauri();
  const [projects, setProjects] = useState<ProjectEntry[]>([]);

  const loadProjects = useCallback(async () => {
    try {
      const entries = await invoke<ProjectEntry[]>("list_projects");
      setProjects(entries);
    } catch {
      // 静默失败，侧边栏不阻断主流程
    }
  }, [invoke]);

  useEffect(() => {
    loadProjects();
    const interval = setInterval(loadProjects, 10_000);
    return () => clearInterval(interval);
  }, [loadProjects]);

  const sortedProjects = useMemo(() => [...projects].sort((a, b) => collator.compare(a.path, b.path)), [projects]);

  return (
    <aside
      className={cn(
        "flex h-full shrink-0 flex-col border-r border-sidebar-border bg-sidebar text-sidebar-foreground transition-[width] duration-200",
        isExpanded ? "w-[var(--sidebar-width-expanded)]" : "w-[var(--sidebar-width-collapsed)]",
      )}
    >


      {/* 子导航内容 */}
      <nav className="flex flex-1 flex-col gap-0.5 overflow-hidden p-2" aria-label="子导航">
        {activeDomain === "projects" && (
          <ProjectSidebarContent
            isExpanded={isExpanded}
            activePage={activePage}
            selectedProject={selectedProject}
            projects={sortedProjects}
            onPageChange={onProjectPageChange}
            onSelectProject={onSelectProject}
          />
        )}
        {activeDomain === "monitoring" && (
          <MonitoringSidebarContent
            isExpanded={isExpanded}
            activePage={activePage}
            onPageChange={onMonitoringPageChange}
          />
        )}
        {activeDomain === "settings" && (
          <SettingsSidebarContent
            isExpanded={isExpanded}
            activePage={activePage}
            onPageChange={onSettingsPageChange}
          />
        )}
      </nav>

      {/* 底部工具 */}
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

/* ─── 项目监控域侧边栏 ─── */
function ProjectSidebarContent({
  isExpanded,
  activePage,
  selectedProject,
  projects,
  onPageChange,
  onSelectProject,
}: {
  isExpanded: boolean;
  activePage: string;
  selectedProject: string;
  projects: ProjectEntry[];
  onPageChange: (page: string) => void;
  onSelectProject: (path: string) => void;
}) {
  const isOverview = activePage === "overview";

  return (
    <>
      <SidebarButton
        icon={LayoutDashboard}
        label="项目概览"
        isExpanded={isExpanded}
        isActive={isOverview}
        onClick={() => onPageChange("overview")}
      />

      {isExpanded && projects.length > 0 && (
        <div className="my-1 h-px bg-sidebar-border/50" />
      )}

      {isExpanded && projects.length > 0 && (
        <ScrollArea className="flex-1">
          <div className="space-y-0.5">
            {projects.map((project) => {
              const isActive = activePage === "detail" && selectedProject === project.path;
              const name = getProjectName(project.path);
              return (
                <button
                  key={project.path}
                  type="button"
                  onClick={() => {
                    onSelectProject(project.path);
                  }}
                  className={cn(
                    "flex w-full items-center gap-2 rounded-md px-3 py-1.5 text-left text-sm transition-colors",
                    isActive
                      ? "bg-sidebar-accent text-sidebar-accent-foreground font-medium"
                      : "text-sidebar-foreground/70 hover:bg-sidebar-accent/50 hover:text-sidebar-foreground",
                  )}
                  title={project.path}
                >
                  <FolderKanban className="size-3.5 shrink-0" aria-hidden="true" />
                  <span className="truncate">{name}</span>
                </button>
              );
            })}
          </div>
        </ScrollArea>
      )}
    </>
  );
}

/* ─── Claude Code 域侧边栏（监控 + 记忆） ─── */
function MonitoringSidebarContent({
  isExpanded,
  activePage,
  onPageChange,
}: {
  isExpanded: boolean;
  activePage: string;
  onPageChange: (page: string) => void;
}) {
  return (
    <>
      {/* 监控分组 */}
      {isExpanded && (
        <div className="px-3 py-1.5 text-xs font-medium text-sidebar-foreground/50">监控</div>
      )}
      <SidebarButton
        icon={Bot}
        label="Agent 监控"
        isExpanded={isExpanded}
        isActive={activePage === "agents"}
        onClick={() => onPageChange("agents")}
      />
      <SidebarButton
        icon={History}
        label="会话管理"
        isExpanded={isExpanded}
        isActive={activePage === "claude-history"}
        onClick={() => onPageChange("claude-history")}
      />

      {/* 分隔 */}
      <div className={cn("my-1 h-px bg-sidebar-border/50", !isExpanded && "mx-2")} />

      {/* 记忆分组 */}
      {isExpanded && (
        <div className="px-3 py-1.5 text-xs font-medium text-sidebar-foreground/50">记忆</div>
      )}
      <SidebarButton
        icon={Brain}
        label="记忆资产"
        isExpanded={isExpanded}
        isActive={activePage === "assets"}
        onClick={() => onPageChange("assets")}
      />
      <SidebarButton
        icon={Route}
        label="加载链模拟器"
        isExpanded={isExpanded}
        isActive={activePage === "load-chain"}
        onClick={() => onPageChange("load-chain")}
      />
    </>
  );
}

/* ─── 设置域侧边栏 ─── */
function SettingsSidebarContent({
  isExpanded,
  activePage,
  onPageChange,
}: {
  isExpanded: boolean;
  activePage: string;
  onPageChange: (page: string) => void;
}) {
  const items = [
    { id: "project", label: "项目设置", icon: FolderKanban },
    { id: "general", label: "通用设置", icon: Settings },
  ] as const;

  return (
    <>
      {items.map((item) => (
        <SidebarButton
          key={item.id}
          icon={item.icon}
          label={item.label}
          isExpanded={isExpanded}
          isActive={activePage === item.id}
          onClick={() => onPageChange(item.id)}
        />
      ))}
    </>
  );
}

/* ─── 通用侧边栏按钮 ─── */
function SidebarButton({
  icon: Icon,
  label,
  isExpanded,
  isActive,
  onClick,
}: {
  icon: typeof LayoutDashboard;
  label: string;
  isExpanded: boolean;
  isActive: boolean;
  onClick: () => void;
}) {
  return (
    <Button
      type="button"
      variant="ghost"
      className={cn(
        "h-9 justify-start gap-3 rounded-md px-3 text-sidebar-foreground/80 hover:bg-sidebar-accent hover:text-sidebar-foreground",
        !isExpanded && "justify-center px-0",
        isActive && "bg-sidebar-accent text-sidebar-accent-foreground font-medium",
      )}
      title={!isExpanded ? label : undefined}
      onClick={onClick}
    >
      <Icon className="size-4 shrink-0" aria-hidden="true" />
      {isExpanded && <span className="text-sm">{label}</span>}
    </Button>
  );
}

function getProjectName(path: string) {
  const segments = path.split(/[\\/]/).filter(Boolean);
  return segments[segments.length - 1] ?? path;
}
