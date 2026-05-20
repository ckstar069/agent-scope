import { useEffect, useMemo, useState } from "react";
import { Activity, AlertTriangle, Bot, ChevronDown, ChevronRight, Clock, Gauge, Layers3, Radio, RotateCcw, Search, X } from "lucide-react";

import { AgentFileAudit } from "@/components/AgentFileAudit";
import { AgentSubTree } from "@/components/AgentSubTree";
import { AgentToolTimeline } from "@/components/AgentToolTimeline";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { useTauri } from "@/hooks/useTauri";
import { cn } from "@/lib/utils";
import type {
  AgentDetailTab,
  AgentInfo,
  AgentUpdatePayload,
  DisplayStatus,
  ProjectAgents,
  RateType,
  RawAgentStatus,
  TokenRateUnit,
} from "./types";

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
  const [rateUnit, setRateUnit] = useState<TokenRateUnit>("minute");
  const [rateType, setRateType] = useState<RateType>("5min");
  const [filterText, setFilterText] = useState("");
  const [expandedSessionId, setExpandedSessionId] = useState<string | null>(null);
  const [now, setNow] = useState(() => Date.now());
  const normalizedFilter = filterText.trim().toLowerCase();

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

  useEffect(() => {
    if (filterText.length >= 0) {
      setExpandedSessionId(null);
    }
  }, [filterText]);

  const activeProjects = useMemo(
    () => (snapshot?.projects ?? []).filter((project) => project.agents.length > 0),
    [snapshot?.projects],
  );
  const unmappedAgents = snapshot?.unmapped ?? [];
  const allAgents = useMemo(
    () => [...activeProjects.flatMap((project) => project.agents), ...unmappedAgents],
    [activeProjects, unmappedAgents],
  );
  const filteredProjects = useMemo(() => {
    if (!normalizedFilter) {
      return activeProjects;
    }

    return activeProjects
      .map((project) => {
        const agents = project.agents.filter((agent) => matchesAgentFilter(agent, normalizedFilter));
        return { ...project, agents, count: agents.length };
      })
      .filter((project) => project.agents.length > 0);
  }, [activeProjects, normalizedFilter]);
  const filteredUnmappedAgents = useMemo(() => {
    if (!normalizedFilter) {
      return unmappedAgents;
    }

    return unmappedAgents.filter((agent) => matchesAgentFilter(agent, normalizedFilter));
  }, [normalizedFilter, unmappedAgents]);
  const maxRate = useMemo(() => Math.max(1, ...allAgents.map((agent) => getDisplayRate(agent, rateType, rateUnit))), [allAgents, rateType, rateUnit]);
  const totalSessions = snapshot?.total_sessions ?? 0;
  const totalCount = allAgents.length;
  const matchedCount = filteredProjects.reduce((sum, project) => sum + project.agents.length, 0) + filteredUnmappedAgents.length;

  // Token 用量统计
  const tokenStats = useMemo(() => {
    return allAgents.reduce(
      (acc, agent) => ({
        input: acc.input + (agent.total_input_tokens ?? 0),
        output: acc.output + (agent.total_output_tokens ?? 0),
        cacheRead: acc.cacheRead + (agent.total_cache_read ?? 0),
        cacheCreate: acc.cacheCreate + (agent.total_cache_create ?? 0),
      }),
      { input: 0, output: 0, cacheRead: 0, cacheCreate: 0 },
    );
  }, [allAgents]);

  // Agent 类型分布统计
  const agentTypeStats = useMemo(() => {
    const counts = new Map<string, number>();
    for (const agent of allAgents) {
      const type = agent.agent_type || "unknown";
      counts.set(type, (counts.get(type) ?? 0) + 1);
    }
    return counts;
  }, [allAgents]);

  function handleToggle(sessionId: string) {
    setExpandedSessionId((previous) => (previous === sessionId ? null : sessionId));
  }

  return (
    <section className="space-y-6">
      <div className="flex flex-col gap-4 lg:flex-row lg:items-end lg:justify-between">
        <div className="space-y-3">
          <p className="text-sm font-medium text-muted-foreground">Agents</p>
          <h1 className="text-3xl font-semibold tracking-tight">Agent 监控</h1>
          <p className="max-w-2xl text-sm text-muted-foreground">
            通过 Tauri 事件流展示 Token 消耗速率（burn rate）、上下文窗口占用和会话在线状态。
          </p>
          <div className="flex max-w-2xl items-center gap-2">
            <Search className="size-4 shrink-0 text-muted-foreground" aria-hidden="true" />
            <Input
              aria-label="搜索 Agent 会话"
              placeholder="搜索会话 ID、项目、模型、状态…"
              value={filterText}
              onChange={(event) => setFilterText(event.target.value)}
            />
            {filterText && (
              <Button type="button" variant="ghost" size="icon" aria-label="清空搜索" onClick={() => setFilterText("")}>
                <X className="size-4" aria-hidden="true" />
              </Button>
            )}
            <span className="shrink-0 font-mono text-xs text-muted-foreground">{matchedCount}/{totalCount}</span>
          </div>
        </div>

        <div className="flex flex-wrap items-center gap-3">
          <div className="flex items-center gap-1">
            <span className="text-xs font-medium text-muted-foreground mr-1">速率类型</span>
            {(["5min", "1min", "total", "realtime"] as RateType[]).map((type) => (
              <Button
                key={type}
                type="button"
                variant={rateType === type ? "secondary" : "outline"}
                size="sm"
                className="h-8 px-2.5 text-xs"
                onClick={() => setRateType(type)}
              >
                {type === "5min" ? "5分钟" : type === "1min" ? "1分钟" : type === "total" ? "全程" : "瞬时"}
              </Button>
            ))}
          </div>
          <div className="h-6 w-px bg-border" />
          <div className="flex items-center gap-1">
            <span className="text-xs font-medium text-muted-foreground mr-1">单位</span>
            <Button
              type="button"
              variant={rateUnit === "second" ? "secondary" : "outline"}
              size="sm"
              className="h-8 px-2.5 text-xs"
              onClick={() => setRateUnit("second")}
            >
              token/s
            </Button>
            <Button
              type="button"
              variant={rateUnit === "minute" ? "secondary" : "outline"}
              size="sm"
              className="h-8 px-2.5 text-xs"
              onClick={() => setRateUnit("minute")}
            >
              token/min
            </Button>
          </div>
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

      {totalCount > 0 && (
        <>
          <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
            <TokenStatTile label="Input Tokens" value={tokenStats.input} color="text-stage-l1" />
            <TokenStatTile label="Output Tokens" value={tokenStats.output} color="text-stage-l3" />
            <TokenStatTile label="Cache Read" value={tokenStats.cacheRead} color="text-stage-l5" />
            <TokenStatTile label="Cache Create" value={tokenStats.cacheCreate} color="text-primary" />
          </div>
          {agentTypeStats.size > 0 && (
            <div className="flex flex-wrap items-center gap-2">
              <span className="text-xs text-muted-foreground">Agent 类型分布:</span>
              {Array.from(agentTypeStats.entries()).map(([type, count]) => (
                <span
                  key={type}
                  className={cn(
                    "inline-flex items-center gap-1 rounded-full border px-2.5 py-0.5 text-xs font-medium",
                    getAgentTypeStyle(type),
                  )}
                >
                  {type} <span className="font-mono opacity-70">{count}</span>
                </span>
              ))}
            </div>
          )}
        </>
      )}

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
      ) : normalizedFilter && matchedCount === 0 ? (
        <Card className="relative flex min-h-72 overflow-hidden border-dashed">
          <div className="absolute inset-x-8 top-0 h-px bg-gradient-to-r from-transparent via-primary/30 to-transparent" />
          <CardContent className="m-auto flex max-w-md flex-col items-center p-8 text-center">
            <div className="mb-4 flex size-12 items-center justify-center rounded-xl bg-muted text-muted-foreground">
              <Search className="size-6" aria-hidden="true" />
            </div>
            <h2 className="text-xl font-semibold tracking-tight">没有匹配的会话</h2>
            <p className="mt-2 text-sm text-muted-foreground">
              当前筛选条件未命中 session_id、项目、模型、状态或工作目录。
            </p>
            <Button type="button" variant="outline" className="mt-5" onClick={() => setFilterText("")}>
              <X className="size-4" aria-hidden="true" />
              清空搜索
            </Button>
          </CardContent>
        </Card>
      ) : (
        <div className="space-y-4">
          {filteredProjects.map((project) => (
            <ProjectAgentCard
              key={project.project_path}
              project={project}
              maxRate={maxRate}
              rateUnit={rateUnit}
              rateType={rateType}
              expandedSessionId={expandedSessionId}
              onToggleSession={handleToggle}
            />
          ))}

          {filteredUnmappedAgents.length > 0 && (
            <ProjectAgentCard
              project={{ project_path: "未关联", agents: filteredUnmappedAgents, count: filteredUnmappedAgents.length }}
              maxRate={maxRate}
              rateUnit={rateUnit}
              rateType={rateType}
              expandedSessionId={expandedSessionId}
              onToggleSession={handleToggle}
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

interface TokenStatTileProps {
  label: string;
  value: number;
  color: string;
}

function TokenStatTile({ label, value, color }: TokenStatTileProps) {
  return (
    <Card>
      <CardContent className="p-4">
        <div className="mb-3 text-xs text-muted-foreground">{label}</div>
        <p className={cn("text-2xl font-semibold tracking-tight", color)}>{formatTokens(value)}</p>
        <p className="mt-1 text-xs text-muted-foreground">累计消耗</p>
      </CardContent>
    </Card>
  );
}

interface ProjectAgentCardProps {
  project: ProjectAgents;
  maxRate: number;
  rateUnit: TokenRateUnit;
  rateType: RateType;
  expandedSessionId: string | null;
  onToggleSession: (sessionId: string) => void;
  isUnmapped?: boolean;
}

function ProjectAgentCard({ project, maxRate, rateUnit, rateType, expandedSessionId, onToggleSession, isUnmapped = false }: ProjectAgentCardProps) {
  const [isExpanded, setIsExpanded] = useState(true);
  const displayName = isUnmapped ? "未关联" : getProjectName(project.project_path, project.agents[0]?.project_name);
  const cardAccent = isUnmapped ? "from-muted via-muted-foreground/30 to-muted" : "from-stage-l0 via-stage-l2 to-stage-l5";

  return (
    <Card className="overflow-hidden">
      <div className={cn("h-1 bg-gradient-to-r", cardAccent)} />
      <CardHeader className="flex-row items-center justify-between gap-4">
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
        <div className="flex shrink-0 items-center gap-2">
          <span className="inline-flex items-center rounded-full border border-border bg-muted/40 px-2.5 py-1 text-xs font-medium text-muted-foreground">
            {project.count} sessions
          </span>
          <Button
            type="button"
            variant="ghost"
            size="icon"
            className="size-8"
            aria-label={isExpanded ? "收起会话" : "展开会话"}
            onClick={() => setIsExpanded((prev) => !prev)}
          >
            {isExpanded ? <ChevronDown className="size-4" /> : <ChevronRight className="size-4" />}
          </Button>
        </div>
      </CardHeader>

      <div
        className={cn(
          "overflow-hidden transition-[max-height,opacity] duration-300 ease-in-out",
          isExpanded ? "max-h-[9999px] opacity-100" : "max-h-0 opacity-0",
        )}
      >
        <CardContent className="space-y-3">
          {project.agents.map((agent) => (
            <AgentSessionRow
              key={agent.session_id}
              agent={agent}
              maxRate={maxRate}
              rateUnit={rateUnit}
              rateType={rateType}
              isExpanded={expandedSessionId === agent.session_id}
              onToggle={() => onToggleSession(agent.session_id)}
            />
          ))}
        </CardContent>
      </div>
    </Card>
  );
}

interface AgentSessionRowProps {
  agent: AgentInfo;
  maxRate: number;
  rateUnit: TokenRateUnit;
  rateType: RateType;
  isExpanded: boolean;
  onToggle: () => void;
}

function AgentSessionRow({ agent, maxRate, rateUnit, rateType, isExpanded, onToggle }: AgentSessionRowProps) {
  const [activeTab, setActiveTab] = useState<AgentDetailTab>("timeline");
  const displayStatus = toDisplayStatus(agent.status);
  const isIdle = displayStatus === "Idle" || displayStatus === "Offline";

  // Idle 状态使用 session average 显示，避免展示 0 rate
  // Idle 固定使用 token/min，不受全局 unit 切换影响
  const effectiveRateType: RateType = isIdle && rateType !== "total" ? "total" : rateType;
  const displayRate = getDisplayRate(agent, effectiveRateType, isIdle ? "minute" : rateUnit);
  const ratePercent = clamp((displayRate / maxRate) * 100, 0, 100);
  const context = getContextUsage(agent);
  const rateStyle = { "--rate-fill": `${ratePercent}%` } as React.CSSProperties;
  const contextStyle = { "--context-fill": `${context.percent}%` } as React.CSSProperties;
  const detailId = `agent-detail-${agent.session_id}`;

  return (
    <div
      className={cn(
        "rounded-lg border border-border bg-muted/30 p-4 transition-colors outline-none focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50",
        isExpanded ? "border-l-2 border-l-primary bg-muted/40" : "cursor-pointer hover:bg-muted/50",
      )}
    >
      <button type="button" className="block w-full text-left" aria-expanded={isExpanded} aria-controls={detailId} onClick={onToggle}>
        <div className="flex flex-col gap-3 xl:flex-row xl:items-start xl:justify-between">
          <div className="min-w-0 space-y-2">
            <div className="flex flex-wrap items-center gap-2">
              {isExpanded ? (
                <ChevronDown className="size-4 shrink-0 text-muted-foreground" aria-hidden="true" />
              ) : (
                <ChevronRight className="size-4 shrink-0 text-muted-foreground" aria-hidden="true" />
              )}
              <span className="font-mono text-sm font-semibold tracking-tight">{shortSessionId(agent.session_id)}</span>
              <span className={cn("rounded-full border px-2 py-0.5 text-xs font-medium", getAgentTypeStyle(agent.agent_type))}>
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
              <span className="text-muted-foreground">Token 消耗速率</span>
              <span className="font-mono font-semibold text-foreground">
                {isIdle ? (
                  <>
                    Idle · {formatRate(displayRate)} token/min
                  </>
                ) : (
                  <>{formatRate(displayRate)} {rateUnit === "second" ? "token/s" : "token/min"}</>
                )}
              </span>
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
      </button>

      <div
        id={detailId}
        className={cn(
          "overflow-hidden transition-[max-height,opacity] duration-300 ease-in-out",
          isExpanded ? "max-h-screen opacity-100" : "max-h-0 opacity-0",
        )}
      >
        <div className="mt-4 border-t border-border pt-4">
          <div className="mb-4">
            <TokenTrendSparkline tokenHistory={agent.token_history} />
          </div>
          <div className="mb-4 grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
            <DetailMetric label="Input Tokens" value={formatTokens(agent.total_input_tokens)} />
            <DetailMetric label="Output Tokens" value={formatTokens(agent.total_output_tokens)} />
            <DetailMetric label="Cache Read" value={formatTokens(agent.total_cache_read)} />
            <DetailMetric label="Cache Create" value={formatTokens(agent.total_cache_create)} />
          </div>
          <div className="mb-3 flex gap-1 border-b border-border pb-2">
            <DetailTabButton active={activeTab === "timeline"} onClick={() => setActiveTab("timeline")}>
              工具调用
            </DetailTabButton>
            <DetailTabButton active={activeTab === "subagents"} onClick={() => setActiveTab("subagents")}>
              子 Agent
            </DetailTabButton>
            <DetailTabButton active={activeTab === "fileaudit"} onClick={() => setActiveTab("fileaudit")}>
              文件审计
            </DetailTabButton>
          </div>

          {activeTab === "timeline" && (
            agent.tool_calls?.length ? (
              <AgentToolTimeline
                tool_calls={agent.tool_calls ?? []}
                pending_since_ms={agent.pending_since_ms}
                thinking_since_ms={agent.thinking_since_ms}
              />
            ) : (
              <DetailEmptyState label="暂无工具调用记录" />
            )
          )}
          {activeTab === "subagents" && (
            agent.subagents?.length ? <AgentSubTree subagents={agent.subagents ?? []} /> : <DetailEmptyState label="暂无子 Agent" />
          )}
          {activeTab === "fileaudit" && (
            agent.file_accesses?.length ? <AgentFileAudit file_accesses={agent.file_accesses ?? []} /> : <DetailEmptyState label="暂无文件审计记录" />
          )}
        </div>
      </div>
    </div>
  );
}

interface DetailTabButtonProps {
  active: boolean;
  children: React.ReactNode;
  onClick: () => void;
}

interface DetailMetricProps {
  label: string;
  value: string;
}

function DetailMetric({ label, value }: DetailMetricProps) {
  return (
    <div className="rounded-lg border border-border bg-muted/20 p-3">
      <div className="text-xs text-muted-foreground">{label}</div>
      <div className="mt-1 font-mono text-sm font-semibold">{value}</div>
    </div>
  );
}

function DetailTabButton({ active, children, onClick }: DetailTabButtonProps) {
  return (
    <button
      type="button"
      className={cn(
        "rounded-md px-3 py-1 text-xs font-medium transition-colors",
        active ? "bg-primary text-primary-foreground" : "text-muted-foreground hover:bg-muted hover:text-foreground",
      )}
      onClick={onClick}
    >
      {children}
    </button>
  );
}

function DetailEmptyState({ label }: { label: string }) {
  return (
    <div className="rounded-lg border border-dashed border-border bg-background/60 px-3 py-6 text-center text-xs text-muted-foreground">
      {label}
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

function getDisplayRate(agent: AgentInfo, rateType: RateType, unit: TokenRateUnit): number {
  let baseRate: number;
  switch (rateType) {
    case "realtime":
      baseRate = agent.token_rate;
      break;
    case "1min":
      baseRate = agent.token_rate_1m;
      break;
    case "5min":
      baseRate = agent.token_rate_5m;
      break;
    case "total":
      baseRate = agent.token_rate_total;
      break;
  }
  return unit === "second" ? baseRate : baseRate * 60;
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

function matchesAgentFilter(agent: AgentInfo, filter: string): boolean {
  const lower = filter.toLowerCase();
  return (
    agent.session_id.toLowerCase().includes(lower) ||
    agent.project_name.toLowerCase().includes(lower) ||
    agent.model.toLowerCase().includes(lower) ||
    agent.status.toLowerCase().includes(lower) ||
    agent.cwd.toLowerCase().includes(lower) ||
    agent.agent_type.toLowerCase().includes(lower)
  );
}

function getAgentTypeStyle(agentType: string): string {
  switch (agentType.toLowerCase()) {
    case "claude":
      return "border-stage-l1/50 bg-stage-l1/10 text-stage-l1";
    case "codex":
      return "border-stage-l3/50 bg-stage-l3/10 text-stage-l3";
    case "mcp":
      return "border-stage-l5/50 bg-stage-l5/10 text-stage-l5";
    default:
      return "border-border bg-muted/50 text-muted-foreground";
  }
}

function TokenTrendSparkline({ tokenHistory }: { tokenHistory: number[] }) {
  if (tokenHistory.length < 2) {
    return (
      <div className="rounded-lg border border-dashed border-border bg-background/60 px-3 py-4 text-center text-xs text-muted-foreground">
        历史数据不足，无法生成趋势图
      </div>
    );
  }

  const max = Math.max(...tokenHistory);
  const min = Math.min(...tokenHistory);
  const range = max - min || 1;

  return (
    <div className="space-y-2">
      <p className="text-xs font-medium text-muted-foreground">Token 生成趋势</p>
      <div className="flex items-end gap-px h-16 rounded-lg border border-border bg-background/60 p-2">
        {/* eslint-disable-next-line react/no-array-index-key */}
        {tokenHistory.map((value, i) => {
          const height = `${((value - min) / range) * 100}%`;
          return (
            <div
              key={i}
              className="flex-1 bg-primary/50 rounded-t-sm min-w-[2px] transition-all duration-300"
              style={{ height }}
              title={`Turn ${i + 1}: ${value.toLocaleString("zh-CN")} tokens`}
            />
          );
        })}
      </div>
      <div className="flex justify-between text-xs text-muted-foreground">
        <span>最低: {formatTokens(min)}</span>
        <span>最高: {formatTokens(max)}</span>
      </div>
    </div>
  );
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
