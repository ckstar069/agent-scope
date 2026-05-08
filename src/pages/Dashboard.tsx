import type { CSSProperties } from "react";
import { useEffect, useMemo, useState } from "react";
import { Activity, AlertTriangle, Bot, ChevronRight, Clock, FolderKanban, FolderPlus, GitBranch, Loader2, Plus } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { useTauri } from "@/hooks/useTauri";

interface DashboardProps {
  onSelectProject: (projectPath: string) => void;
  onNavigateSettings: () => void;
}

interface ProjectEntry {
  path: string;
  added_at: number;
}

interface StageInfo {
  name: string;
  description: string;
  ordinal: number;
}

interface GitStatus {
  branch: string;
  modified_count: number;
  staged_count: number;
  untracked_count: number;
  conflict_count: number;
  is_clean: boolean;
}

interface ProjectConfig {
  project_name: string;
}

interface TemplateDataPayload {
  project_path: string;
  stage: StageInfo | null;
  stage_error: string | null;
  config: ProjectConfig | null;
  config_error: string | null;
  git: GitStatus;
  git_error: string | null;
  timestamp_ms: number;
}

interface ProjectAgents {
  project_path: string;
  count: number;
}

interface AgentUpdatePayload {
  projects: ProjectAgents[];
  timestamp_ms: number;
  total_sessions: number;
}

const stageTokenPairs = [
  ["var(--chart-1)", "var(--sidebar-primary)"],
  ["var(--sidebar-primary)", "var(--chart-1)"],
  ["var(--chart-2)", "var(--primary)"],
  ["var(--primary)", "var(--chart-2)"],
  ["var(--destructive)", "var(--chart-3)"],
  ["var(--chart-3)", "var(--destructive)"],
  ["var(--sidebar-primary)", "var(--destructive)"],
];

const collator = new Intl.Collator("zh-CN", { numeric: true, sensitivity: "base" });

export function Dashboard({ onSelectProject, onNavigateSettings }: DashboardProps) {
  const { invoke, listen } = useTauri();
  const [projects, setProjects] = useState<ProjectEntry[]>([]);
  const [projectData, setProjectData] = useState<Record<string, TemplateDataPayload>>({});
  const [agentCounts, setAgentCounts] = useState<Record<string, number>>({});
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let isMounted = true;

    async function loadProjects() {
      setIsLoading(true);
      setError(null);

      try {
        const entries = await invoke<ProjectEntry[]>("list_projects");
        if (!isMounted) {
          return;
        }

        setProjects(entries);

        const snapshots = await Promise.allSettled(
          entries.map((entry) => invoke<TemplateDataPayload, { path: string }>("get_project_data", { path: entry.path })),
        );

        if (!isMounted) {
          return;
        }

        const nextData = snapshots.reduce<Record<string, TemplateDataPayload>>((acc, snapshot, index) => {
          if (snapshot.status === "fulfilled") {
            acc[entries[index].path] = snapshot.value;
          }
          return acc;
        }, {});

        setProjectData(nextData);
      } catch (err) {
        if (isMounted) {
          setError(err instanceof Error ? err.message : String(err));
        }
      } finally {
        if (isMounted) {
          setIsLoading(false);
        }
      }
    }

    loadProjects();

    return () => {
      isMounted = false;
    };
  }, [invoke]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;

    listen<AgentUpdatePayload>("agent-update", (event) => {
      const nextCounts = event.payload.projects.reduce<Record<string, number>>((acc, project) => {
        acc[project.project_path] = project.count;
        return acc;
      }, {});

      setAgentCounts(nextCounts);
    }).then((cleanup) => {
      unlisten = cleanup;
    });

    return () => {
      unlisten?.();
    };
  }, [listen]);

  const dashboardProjects = useMemo(
    () =>
      projects
        .map((project) => {
          const data = projectData[project.path];
          const displayName = data?.config?.project_name?.trim() || getProjectName(project.path);
          const totalChanges = data
            ? data.git.modified_count + data.git.staged_count + data.git.untracked_count + data.git.conflict_count
            : 0;

          return {
            ...project,
            data,
            displayName,
            agentCount: agentCounts[project.path] ?? 0,
            totalChanges,
            recentMs: data?.timestamp_ms ?? project.added_at * 1000,
          };
        })
        .sort((a, b) => collator.compare(a.displayName, b.displayName)),
    [agentCounts, projectData, projects],
  );

  return (
    <section className="space-y-6">
      <div className="flex flex-col gap-4 lg:flex-row lg:items-end lg:justify-between">
        <div className="space-y-2">
          <p className="text-sm font-medium text-muted-foreground">Dashboard</p>
          <h1 className="text-3xl font-semibold tracking-tight">项目仪表盘</h1>
          <p className="max-w-2xl text-sm text-muted-foreground">
            汇总已注册 FPGA 项目的 Stage、Git 工作区状态和实时 Agent 活跃度。
          </p>
        </div>
          <Button type="button" variant="outline" onClick={onNavigateSettings}>
            <Plus className="size-4" aria-hidden="true" />
            添加项目
          </Button>
      </div>

      {error && (
        <Card className="border-destructive/40 bg-destructive/10">
          <CardContent className="flex items-center gap-3 p-4 text-sm text-destructive">
            <AlertTriangle className="size-4" aria-hidden="true" />
            项目列表加载失败：{error}
          </CardContent>
        </Card>
      )}

      {isLoading ? (
        <Card className="flex min-h-72 items-center justify-center border-dashed">
          <div className="flex items-center gap-3 text-sm text-muted-foreground">
            <Loader2 className="size-4 animate-spin" aria-hidden="true" />
            正在加载项目列表…
          </div>
        </Card>
      ) : dashboardProjects.length === 0 ? (
        <Card className="relative flex min-h-72 overflow-hidden border-dashed">
          <div className="absolute inset-x-8 top-0 h-px bg-gradient-to-r from-transparent via-primary/30 to-transparent" />
          <CardContent className="m-auto flex max-w-md flex-col items-center p-8 text-center">
            <div className="mb-4 flex size-12 items-center justify-center rounded-xl bg-muted text-muted-foreground">
              <FolderPlus className="size-6" aria-hidden="true" />
            </div>
            <h2 className="text-xl font-semibold tracking-tight">还没有项目</h2>
            <p className="mt-2 text-sm text-muted-foreground">
              添加从 ai_project_template 创建的项目后，这里会按名称排序展示 Stage、Git 状态和 Agent 数量。
            </p>
            <Button type="button" className="mt-5" onClick={onNavigateSettings}>
              <FolderPlus className="size-4" aria-hidden="true" />
              添加项目
            </Button>
          </CardContent>
        </Card>
      ) : (
        <div className="grid gap-4 xl:grid-cols-2">
          {dashboardProjects.map((project) => (
            <ProjectCard key={project.path} project={project} onOpen={() => onSelectProject(project.path)} />
          ))}
        </div>
      )}
    </section>
  );
}

interface ProjectCardProps {
  project: {
    path: string;
    data?: TemplateDataPayload;
    displayName: string;
    agentCount: number;
    totalChanges: number;
    recentMs: number;
  };
  onOpen: () => void;
}

function ProjectCard({ project, onOpen }: ProjectCardProps) {
  const stage = project.data?.stage;
  const stageOrdinal = stage?.ordinal ?? null;
  const stageVisual = getStageVisual(stageOrdinal);
  const branch = project.data?.git.branch || "未知分支";

  return (
    <Card
      role="button"
      tabIndex={0}
      className="group overflow-hidden transition-all hover:-translate-y-0.5 hover:border-primary/30 hover:shadow-lg hover:shadow-primary/5"
      onClick={onOpen}
      onKeyDown={(event) => {
        if (event.key === "Enter" || event.key === " ") {
          event.preventDefault();
          onOpen();
        }
      }}
    >
      <div className="h-1" style={stageVisual.bar} />
      <CardHeader className="flex-row items-start justify-between gap-4">
        <div className="min-w-0 space-y-2">
          <div className="flex items-center gap-3">
            <div className="flex size-10 shrink-0 items-center justify-center rounded-lg bg-muted text-muted-foreground">
              <FolderKanban className="size-5" aria-hidden="true" />
            </div>
            <div className="min-w-0">
              <CardTitle className="truncate text-lg">{project.displayName}</CardTitle>
              <CardDescription className="truncate">{project.path}</CardDescription>
            </div>
          </div>
        </div>
        <ChevronRight className="mt-2 size-4 shrink-0 text-muted-foreground transition-transform group-hover:translate-x-1" aria-hidden="true" />
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="flex flex-wrap gap-2">
          <span className="inline-flex items-center rounded-full border px-2.5 py-1 text-xs font-medium" style={stageVisual.badge}>
            {stage ? stage.name : "Stage 未知"}
          </span>
          {project.data?.stage_error && <span className="text-xs text-destructive">Stage 读取失败</span>}
        </div>

        <div className="grid gap-3 sm:grid-cols-3">
          <StatusTile icon={GitBranch} label="Git 分支" value={branch} detail={`${project.totalChanges} 项变更`} />
          <StatusTile icon={Bot} label="活跃 Agent" value={`${project.agentCount}`} detail="实时事件更新" />
          <StatusTile icon={Clock} label="最近活动" value={formatRelativeTime(project.recentMs)} detail={formatDateTime(project.recentMs)} />
        </div>

        <div className="flex items-center justify-between border-t border-border pt-4 text-xs text-muted-foreground">
          <span className="inline-flex items-center gap-1.5">
            <Activity className="size-3.5" aria-hidden="true" />
            {project.data?.git.is_clean ? "工作区干净" : `${project.totalChanges} 个文件状态变化`}
          </span>
          <span>查看详情</span>
        </div>
      </CardContent>
    </Card>
  );
}

interface StatusTileProps {
  icon: typeof GitBranch;
  label: string;
  value: string;
  detail: string;
}

function StatusTile({ icon: Icon, label, value, detail }: StatusTileProps) {
  return (
    <div className="rounded-lg border border-border bg-muted/40 p-3">
      <div className="mb-2 flex items-center gap-2 text-xs text-muted-foreground">
        <Icon className="size-3.5" aria-hidden="true" />
        {label}
      </div>
      <p className="truncate text-sm font-semibold">{value}</p>
      <p className="mt-1 truncate text-xs text-muted-foreground">{detail}</p>
    </div>
  );
}

function getProjectName(path: string) {
  const segments = path.split(/[\\/]/).filter(Boolean);
  return segments[segments.length - 1] ?? path;
}

function getStageVisual(stageOrdinal: number | null): { badge: CSSProperties; bar: CSSProperties } {
  const [from, to] = stageOrdinal === null ? ["var(--muted)", "var(--secondary)"] : stageTokenPairs[stageOrdinal] ?? stageTokenPairs[0];
  const textColor = stageOrdinal === null ? "var(--muted-foreground)" : from;

  return {
    badge: {
      background: `linear-gradient(90deg, color-mix(in oklch, ${from} 18%, transparent), color-mix(in oklch, ${to} 12%, transparent))`,
      borderColor: `color-mix(in oklch, ${from} 42%, transparent)`,
      color: textColor,
    },
    bar: {
      background: `linear-gradient(90deg, ${from}, ${to})`,
    },
  };
}

function formatRelativeTime(timestampMs: number) {
  const diffSeconds = Math.max(0, Math.floor((Date.now() - timestampMs) / 1000));
  if (diffSeconds < 60) {
    return "刚刚";
  }

  const diffMinutes = Math.floor(diffSeconds / 60);
  if (diffMinutes < 60) {
    return `${diffMinutes} 分钟前`;
  }

  const diffHours = Math.floor(diffMinutes / 60);
  if (diffHours < 24) {
    return `${diffHours} 小时前`;
  }

  return `${Math.floor(diffHours / 24)} 天前`;
}

function formatDateTime(timestampMs: number) {
  return new Intl.DateTimeFormat("zh-CN", {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  }).format(timestampMs);
}
