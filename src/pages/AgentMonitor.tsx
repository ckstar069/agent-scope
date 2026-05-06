import { useEffect, useMemo, useState } from "react";
import { Activity, AlertTriangle, Bot, Clock, Gauge, Layers3, Radio, RotateCcw } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { useTauri } from "@/hooks/useTauri";
import { cn } from "@/lib/utils";

type RawAgentStatus = "Thinking" | "Executing" | "Waiting" | "RateLimited" | "Done";
type DisplayStatus = "Active" | "Idle" | "Offline";
type TokenRateUnit = "second" | "minute";

interface AgentInfo {
  agent_type: string;
  session_id: string;
  cwd: string;
  project_name: string;
  status: RawAgentStatus;
  model: string;
  context_percent: number;
  context_window: number;
  total_input_tokens: number;
  total_output_tokens: number;
  total_cache_read: number;
  total_cache_create: number;
  turn_count: number;
  current_tasks: string[];
  mem_mb: number;
  git_branch: string;
  git_added: number;
  git_modified: number;
  token_history: number[];
  context_history: number[];
  compaction_count: number;
  token_rate: number;
  pid: number;
  version: string;
  effort: string;
}

interface ProjectAgents {
  project_path: string;
  agents: AgentInfo[];
  count: number;
}

interface AgentUpdatePayload {
  projects: ProjectAgents[];
  unmapped: AgentInfo[];
  timestamp_ms: number;
  total_sessions: number;
}

const statusStyles: Record<DisplayStatus, string> = {
  Active: "border-stage-l1/40 bg-stage-l1/15 text-stage-l1",
  Idle: "border-stage-l3/40 bg-stage-l3/15 text-stage-l3",
  Offline: "border-muted-foreground/30 bg-muted/50 text-muted-foreground",
};

const statusText: Record<DisplayStatus, string> = {
  Active: "Active",
  Idle: "Idle",
  Offline: "Offline",
};

const rawStatusText: Record<RawAgentStatus, string> = {
  Thinking: "思考中",
  Executing: "执行工具",
  Waiting: "等待输入",
  RateLimited: "限流等待",
  Done: "已结束",
};

export function AgentMonitor() {
  const { listen } = useTauri();
  const [snapshot, setSnapshot] = useState<AgentUpdatePayload | null>(null);
  const [listenError, setListenError] = useState<string | null>(null);
  const [rateUnit, setRateUnit] = useState<TokenRateUnit>("second");
  const [now, setNow] = useState(() => Date.now());

  useEffect(() => {
    let isMounted = true;
    let unlisten: (() => void) | undefined;

    listen<AgentUpdatePayload>("agent-update", (event) => {
      setSnapshot(event.payload);
      setListenError(null);
    })
      .then((cleanup) => {
        if (isMounted) {
          unlisten = cleanup;
          return;
        }

        cleanup();
      })
      .catch((err) => {
        if (isMounted) {
          setListenError(err instanceof Error ? err.message : String(err));
        }
      });

    return () => {
      isMounted = false;
      unlisten?.();
    };
  }, [listen]);

  useEffect(() => {
    const timer = window.setInterval(() => setNow(Date.now()), 2_000);
    return () => window.clearInterval(timer);
  }, []);

  const activeProjects = useMemo(
    () => (snapshot?.projects ?? []).filter((project) => project.agents.length > 0),
    [snapshot?.projects],
  );
  const unmappedAgents = snapshot?.unmapped ?? [];
  const allAgents = useMemo(
    () => [...activeProjects.flatMap((project) => project.agents), ...unmappedAgents],
    [activeProjects, unmappedAgents],
  );
  const maxRate = useMemo(() => Math.max(1, ...allAgents.map((agent) => getDisplayRate(agent.token_rate, rateUnit))), [allAgents, rateUnit]);
  const totalSessions = snapshot?.total_sessions ?? 0;

  return (
    <section className="space-y-6">
      <div className="flex flex-col gap-4 lg:flex-row lg:items-end lg:justify-between">
        <div className="space-y-2">
          <p className="text-sm font-medium text-muted-foreground">Agents</p>
          <h1 className="text-3xl font-semibold tracking-tight">Agent 监控</h1>
          <p className="max-w-2xl text-sm text-muted-foreground">
            通过 Tauri 事件流展示实时 Token 速率、上下文窗口占用和会话在线状态。
          </p>
        </div>

        <div className="flex flex-wrap items-center gap-2">
          <Button
            type="button"
            variant={rateUnit === "second" ? "secondary" : "outline"}
            onClick={() => setRateUnit("second")}
          >
            token/s
          </Button>
          <Button
            type="button"
            variant={rateUnit === "minute" ? "secondary" : "outline"}
            onClick={() => setRateUnit("minute")}
          >
            token/min
          </Button>
        </div>
      </div>

      {listenError && (
        <Card className="border-destructive/40 bg-destructive/10">
          <CardContent className="flex items-center gap-3 p-4 text-sm text-destructive">
            <AlertTriangle className="size-4" aria-hidden="true" />
            Agent 事件监听失败：{listenError}
          </CardContent>
        </Card>
      )}

      <div className="grid gap-3 md:grid-cols-3">
        <SummaryTile icon={Radio} label="会话总数" value={`${totalSessions}`} detail="agent-update 实时快照" />
        <SummaryTile icon={Activity} label="关联项目" value={`${activeProjects.length}`} detail="仅显示有 Agent 的项目" />
        <SummaryTile icon={Clock} label="刷新时间" value={snapshot ? formatRelativeTime(snapshot.timestamp_ms, now) : "等待中"} detail={snapshot ? formatDateTime(snapshot.timestamp_ms) : "每 2 秒同步"} />
      </div>

      {totalSessions === 0 ? (
        <Card className="relative flex min-h-72 overflow-hidden border-dashed">
          <div className="absolute inset-x-8 top-0 h-px bg-gradient-to-r from-transparent via-primary/30 to-transparent" />
          <CardContent className="m-auto flex max-w-md flex-col items-center p-8 text-center">
            <div className="mb-4 flex size-12 items-center justify-center rounded-xl bg-muted text-muted-foreground">
              <Bot className="size-6" aria-hidden="true" />
            </div>
            <h2 className="text-xl font-semibold tracking-tight">暂无活跃 Agent</h2>
            <p className="mt-2 text-sm text-muted-foreground">
              Collector 会继续监听 agent-update 事件，有会话出现后将自动刷新到这里。
            </p>
          </CardContent>
        </Card>
      ) : (
        <div className="space-y-4">
          {activeProjects.map((project) => (
            <ProjectAgentCard key={project.project_path} project={project} maxRate={maxRate} rateUnit={rateUnit} />
          ))}

          {unmappedAgents.length > 0 && (
            <ProjectAgentCard
              project={{ project_path: "未关联", agents: unmappedAgents, count: unmappedAgents.length }}
              maxRate={maxRate}
              rateUnit={rateUnit}
              isUnmapped
            />
          )}
        </div>
      )}
    </section>
  );
}

interface SummaryTileProps {
  icon: typeof Radio;
  label: string;
  value: string;
  detail: string;
}

function SummaryTile({ icon: Icon, label, value, detail }: SummaryTileProps) {
  return (
    <Card>
      <CardContent className="p-4">
        <div className="mb-3 flex items-center gap-2 text-xs text-muted-foreground">
          <Icon className="size-3.5" aria-hidden="true" />
          {label}
        </div>
        <p className="text-2xl font-semibold tracking-tight">{value}</p>
        <p className="mt-1 text-xs text-muted-foreground">{detail}</p>
      </CardContent>
    </Card>
  );
}

interface ProjectAgentCardProps {
  project: ProjectAgents;
  maxRate: number;
  rateUnit: TokenRateUnit;
  isUnmapped?: boolean;
}

function ProjectAgentCard({ project, maxRate, rateUnit, isUnmapped = false }: ProjectAgentCardProps) {
  const displayName = isUnmapped ? "未关联" : getProjectName(project.project_path, project.agents[0]?.project_name);
  const cardAccent = isUnmapped ? "from-muted via-muted-foreground/30 to-muted" : "from-stage-l0 via-stage-l2 to-stage-l5";

  return (
    <Card className="overflow-hidden">
      <div className={cn("h-1 bg-gradient-to-r", cardAccent)} />
      <CardHeader className="flex-row items-start justify-between gap-4">
        <div className="min-w-0 space-y-2">
          <div className="flex items-center gap-3">
            <div className="flex size-10 shrink-0 items-center justify-center rounded-lg bg-muted text-muted-foreground">
              {isUnmapped ? <AlertTriangle className="size-5" aria-hidden="true" /> : <Bot className="size-5" aria-hidden="true" />}
            </div>
            <div className="min-w-0">
              <CardTitle className="truncate text-lg">{displayName}</CardTitle>
              <CardDescription className="truncate">{project.project_path}</CardDescription>
            </div>
          </div>
        </div>
        <span className="inline-flex shrink-0 items-center rounded-full border border-border bg-muted/40 px-2.5 py-1 text-xs font-medium text-muted-foreground">
          {project.count} sessions
        </span>
      </CardHeader>

      <CardContent className="space-y-3">
        {project.agents.map((agent) => (
          <AgentSessionRow key={agent.session_id} agent={agent} maxRate={maxRate} rateUnit={rateUnit} />
        ))}
      </CardContent>
    </Card>
  );
}

interface AgentSessionRowProps {
  agent: AgentInfo;
  maxRate: number;
  rateUnit: TokenRateUnit;
}

function AgentSessionRow({ agent, maxRate, rateUnit }: AgentSessionRowProps) {
  const displayStatus = toDisplayStatus(agent.status);
  const displayRate = getDisplayRate(agent.token_rate, rateUnit);
  const ratePercent = clamp((displayRate / maxRate) * 100, 0, 100);
  const context = getContextUsage(agent);
  const rateStyle = { "--rate-fill": `${ratePercent}%` } as React.CSSProperties;
  const contextStyle = { "--context-fill": `${context.percent}%` } as React.CSSProperties;

  return (
    <div className="rounded-lg border border-border bg-muted/30 p-4 transition-colors hover:bg-muted/45">
      <div className="flex flex-col gap-3 xl:flex-row xl:items-start xl:justify-between">
        <div className="min-w-0 space-y-2">
          <div className="flex flex-wrap items-center gap-2">
            <span className="font-mono text-sm font-semibold tracking-tight">{shortSessionId(agent.session_id)}</span>
            <span className="rounded-full border border-border bg-background px-2 py-0.5 text-xs text-muted-foreground">
              {agent.agent_type || "unknown"}
            </span>
            <StatusBadge status={displayStatus} rawStatus={agent.status} />
          </div>
          <p className="truncate text-xs text-muted-foreground">{agent.cwd}</p>
        </div>

        <div className="grid gap-2 text-xs text-muted-foreground sm:grid-cols-3 xl:min-w-96">
          <InlineMetric icon={Gauge} label="模型" value={agent.model || "未知"} />
          <InlineMetric icon={Layers3} label="轮次" value={`${agent.turn_count}`} />
          <InlineMetric icon={RotateCcw} label="PID" value={`${agent.pid || "-"}`} />
        </div>
      </div>

      <div className="mt-4 grid gap-4 lg:grid-cols-[minmax(0,1fr)_minmax(0,1fr)]">
        <div className="space-y-2">
          <div className="flex items-center justify-between gap-3 text-xs">
            <span className="text-muted-foreground">Token 速率</span>
            <span className="font-mono font-semibold text-foreground">{formatRate(displayRate)} {rateUnit === "second" ? "token/s" : "token/min"}</span>
          </div>
          <div className="h-2 overflow-hidden rounded-full bg-muted">
            <div className="h-full w-[var(--rate-fill)] rounded-full bg-gradient-to-r from-stage-l0 via-stage-l2 to-stage-l4 transition-[width] duration-500" style={rateStyle} />
          </div>
        </div>

        <div className="space-y-2">
          <div className="flex items-center justify-between gap-3 text-xs">
            <span className="text-muted-foreground">上下文窗口</span>
            <span className="font-mono font-semibold text-foreground">
              {formatTokens(context.current)} / {formatTokens(context.max)}
            </span>
          </div>
          <div className="h-2 overflow-hidden rounded-full bg-muted">
            <div className="h-full w-[var(--context-fill)] rounded-full bg-gradient-to-r from-stage-l1 via-stage-l3 to-stage-l5 transition-[width] duration-500" style={contextStyle} />
          </div>
          <p className="text-xs text-muted-foreground">{formatRate(context.percent)}% 使用率</p>
        </div>
      </div>
    </div>
  );
}

interface InlineMetricProps {
  icon: typeof Gauge;
  label: string;
  value: string;
}

function InlineMetric({ icon: Icon, label, value }: InlineMetricProps) {
  return (
    <div className="rounded-lg border border-border bg-background/60 p-2">
      <div className="mb-1 flex items-center gap-1.5">
        <Icon className="size-3.5" aria-hidden="true" />
        {label}
      </div>
      <p className="truncate font-medium text-foreground">{value}</p>
    </div>
  );
}

interface StatusBadgeProps {
  status: DisplayStatus;
  rawStatus: RawAgentStatus;
}

function StatusBadge({ status, rawStatus }: StatusBadgeProps) {
  return (
    <span className={cn("inline-flex items-center rounded-full border px-2 py-0.5 text-xs font-medium", statusStyles[status])}>
      <span className="mr-1.5 size-1.5 rounded-full bg-current" aria-hidden="true" />
      {statusText[status]} · {rawStatusText[rawStatus]}
    </span>
  );
}

function toDisplayStatus(status: RawAgentStatus): DisplayStatus {
  if (status === "Waiting") {
    return "Idle";
  }

  if (status === "Done") {
    return "Offline";
  }

  return "Active";
}

function getDisplayRate(tokenRate: number, unit: TokenRateUnit) {
  return unit === "second" ? tokenRate : tokenRate * 60;
}

function getContextUsage(agent: AgentInfo) {
  const max = Math.max(0, agent.context_window);
  const percentFromBackend = Number.isFinite(agent.context_percent) ? agent.context_percent : 0;
  const current = max > 0 ? Math.round((max * clamp(percentFromBackend, 0, 100)) / 100) : agent.total_input_tokens;
  const percent = max > 0 ? clamp((current / max) * 100, 0, 100) : 0;

  return { current, max, percent };
}

function clamp(value: number, min: number, max: number) {
  return Math.min(max, Math.max(min, value));
}

function getProjectName(path: string, fallback?: string) {
  const normalizedFallback = fallback?.trim();
  if (normalizedFallback) {
    return normalizedFallback;
  }

  const segments = path.split(/[\\/]/).filter(Boolean);
  return segments[segments.length - 1] ?? path;
}

function shortSessionId(sessionId: string) {
  if (sessionId.length <= 14) {
    return sessionId;
  }

  return `${sessionId.slice(0, 8)}…${sessionId.slice(-5)}`;
}

function formatRate(value: number) {
  if (!Number.isFinite(value)) {
    return "0";
  }

  if (value >= 100) {
    return Math.round(value).toLocaleString("zh-CN");
  }

  return value.toLocaleString("zh-CN", { maximumFractionDigits: 1 });
}

function formatTokens(value: number) {
  if (value >= 1_000_000) {
    return `${(value / 1_000_000).toLocaleString("zh-CN", { maximumFractionDigits: 1 })}M`;
  }

  if (value >= 1_000) {
    return `${(value / 1_000).toLocaleString("zh-CN", { maximumFractionDigits: 1 })}K`;
  }

  return value.toLocaleString("zh-CN");
}

function formatRelativeTime(timestampMs: number, now: number) {
  const diffSeconds = Math.max(0, Math.floor((now - timestampMs) / 1000));
  if (diffSeconds < 6) {
    return "刚刚";
  }

  if (diffSeconds < 60) {
    return `${diffSeconds} 秒前`;
  }

  const diffMinutes = Math.floor(diffSeconds / 60);
  if (diffMinutes < 60) {
    return `${diffMinutes} 分钟前`;
  }

  const diffHours = Math.floor(diffMinutes / 60);
  return `${diffHours} 小时前`;
}

function formatDateTime(timestampMs: number) {
  return new Intl.DateTimeFormat("zh-CN", {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  }).format(timestampMs);
}
