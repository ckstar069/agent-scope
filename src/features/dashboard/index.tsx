import type { CSSProperties } from "react";
import { useEffect, useMemo, useState } from "react";
import { Activity, AlertTriangle, Bot, CheckCircle2, Clock, FileWarning, FolderKanban, FolderPlus, GitBranch, Loader2, Plus, TrendingUp, Zap } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { useTauri } from "@/hooks/useTauri";
import { cn } from "@/lib/utils";
import type {
  DashboardProps,
  TemplateDataPayload,
} from "./types";
import type { ProjectEntry, AgentUpdatePayload } from "@/lib/types";

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

export function Dashboard({ onNavigateSettings }: DashboardProps) {
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

  const summaryStats = useMemo(() => {
    const totalAgents = Object.values(agentCounts).reduce((sum, count) => sum + count, 0);
    const totalChanges = dashboardProjects.reduce((sum, p) => sum + p.totalChanges, 0);
    const alertCount = dashboardProjects.filter(
      (p) => (p.data?.git.conflict_count ?? 0) > 0 || p.data?.stage_error,
    ).length;
    const cleanCount = dashboardProjects.filter((p) => p.data?.git.is_clean).length;

    return { totalAgents, totalChanges, alertCount, cleanCount };
  }, [agentCounts, dashboardProjects]);

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

      {!isLoading && dashboardProjects.length > 0 && (
        <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
          <SummaryTile icon={FolderKanban} label="项目总数" value={`${dashboardProjects.length}`} detail="已注册项目" color="text-primary" />
          <SummaryTile icon={Bot} label="活跃 Agent" value={`${summaryStats.totalAgents}`} detail="实时会话总数" color="text-stage-l1" />
          <SummaryTile icon={GitBranch} label="未提交变更" value={`${summaryStats.totalChanges}`} detail="所有项目累计" color="text-stage-l3" />
          <SummaryTile
            icon={summaryStats.alertCount > 0 ? FileWarning : CheckCircle2}
            label="项目状态"
            value={summaryStats.alertCount > 0 ? `${summaryStats.alertCount} 个告警` : `${summaryStats.cleanCount} 个干净`}
            detail={summaryStats.alertCount > 0 ? "Git 冲突或 Stage 错误" : "工作区无异常"}
            color={summaryStats.alertCount > 0 ? "text-destructive" : "text-stage-l5"}
          />
        </div>
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
            <ProjectCard key={project.path} project={project} />
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
}

function ProjectCard({ project }: ProjectCardProps) {
  const stage = project.data?.stage;
  const stageOrdinal = stage?.ordinal ?? null;
  const stageVisual = getStageVisual(stageOrdinal);
  const branch = project.data?.git.branch || "未知分支";
  const activity = getActivityScore(project);

  return (
    <Card
      className="group overflow-hidden transition-all hover:border-primary/40"
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
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="flex flex-wrap items-center gap-2">
          <span className="inline-flex items-center rounded-full border px-2.5 py-1 text-xs font-medium" style={stageVisual.badge}>
            {stage ? stage.name : "Stage 未知"}
          </span>
          <span className={cn("inline-flex items-center gap-1 rounded-full border px-2 py-0.5 text-xs font-medium", activity.style)}>
            {activity.icon}
            {activity.label}
          </span>
          {project.data?.git.conflict_count ? (
            <span className="inline-flex items-center gap-1 rounded-full border border-destructive/50 bg-destructive/10 px-2 py-0.5 text-xs font-medium text-destructive">
              <AlertTriangle className="size-3" />
              {project.data.git.conflict_count} 个冲突
            </span>
          ) : null}
          {project.data?.stage_error ? (
            <span className="inline-flex items-center gap-1 rounded-full border border-stage-l3/50 bg-stage-l3/10 px-2 py-0.5 text-xs font-medium text-stage-l3">
              <FileWarning className="size-3" />
              Stage 异常
            </span>
          ) : null}
        </div>

        <div className="grid gap-3 sm:grid-cols-3">
          <StatusTile icon={GitBranch} label="Git 分支" value={branch} detail={`${project.totalChanges} 项变更`} />
          <StatusTile icon={Bot} label="活跃 Agent" value={`${project.agentCount}`} detail="实时事件更新" />
          <StatusTile icon={Clock} label="最近活动" value={formatRelativeTime(project.recentMs)} detail={formatDateTime(project.recentMs)} />
        </div>

        <div className="flex items-center gap-1.5 border-t border-border pt-4 text-xs text-muted-foreground">
          <Activity className="size-3.5" aria-hidden="true" />
          {project.data?.git.is_clean ? "工作区干净" : `${project.totalChanges} 个文件状态变化`}
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
    <div className="rounded-md border border-border/60 bg-muted/30 p-3">
      <div className="mb-2 flex items-center gap-2 text-xs text-muted-foreground">
        <Icon className="size-3.5" aria-hidden="true" />
        {label}
      </div>
      <p className="truncate text-sm font-medium">{value}</p>
      <p className="mt-1 truncate text-xs text-muted-foreground">{detail}</p>
    </div>
  );
}

interface SummaryTileProps {
  icon: typeof GitBranch;
  label: string;
  value: string;
  detail: string;
  color: string;
}

function SummaryTile({ icon: Icon, label, value, detail, color }: SummaryTileProps) {
  return (
    <Card>
      <CardContent className="p-4">
        <div className="mb-3 flex items-center gap-2 text-xs text-muted-foreground">
          <Icon className={cn("size-3.5", color)} aria-hidden="true" />
          {label}
        </div>
        <p className={cn("text-2xl font-semibold tracking-tight", color)}>{value}</p>
        <p className="mt-1 text-xs text-muted-foreground">{detail}</p>
      </CardContent>
    </Card>
  );
}

function getActivityScore(project: ProjectCardProps["project"]): { label: string; style: string; icon: React.ReactNode } {
  const agentScore = Math.min(project.agentCount * 30, 60);
  const changeScore = Math.min(project.totalChanges * 5, 25);
  const now = Date.now();
  const diffHours = (now - project.recentMs) / (1000 * 60 * 60);
  const recencyScore = diffHours < 1 ? 15 : diffHours < 24 ? 10 : diffHours < 72 ? 5 : 0;
  const score = agentScore + changeScore + recencyScore;

  if (score >= 60) {
    return {
      label: "活跃",
      style: "border-stage-l1/50 bg-stage-l1/10 text-stage-l1",
      icon: <Zap className="size-3" />,
    };
  }

  if (score >= 35) {
    return {
      label: "进行中",
      style: "border-stage-l3/50 bg-stage-l3/10 text-stage-l3",
      icon: <TrendingUp className="size-3" />,
    };
  }

  if (score >= 15) {
    return {
      label: "低活跃",
      style: "border-stage-l5/50 bg-stage-l5/10 text-stage-l5",
      icon: <Clock className="size-3" />,
    };
  }

  return {
    label: "闲置",
    style: "border-border bg-muted/50 text-muted-foreground",
    icon: <Clock className="size-3" />,
  };
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
