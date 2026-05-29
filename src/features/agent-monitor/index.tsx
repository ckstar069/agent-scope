import { useEffect, useMemo, useState } from "react";
import { Activity, AlertTriangle, Bot, ChevronDown, ChevronRight, Clock, Radio, Search, X } from "lucide-react";

import { getAgentSnapshot } from "@/lib/api";

import { AgentFileAudit } from "@/components/AgentFileAudit";
import { AgentSubTree } from "@/components/AgentSubTree";
import { AgentToolTimeline } from "@/components/AgentToolTimeline";
import { InfoHint } from "@/components/InfoHint";
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
  RateType,
  RawAgentStatus,
  TokenRateUnit,
} from "./types";

const statusStyles: Record<DisplayStatus, string> = {
  Active: "border-stage-l1/40 bg-stage-l1/15 text-stage-l1",
  Idle: "border-stage-l3/40 bg-stage-l3/15 text-stage-l3",
  Offline: "border-muted-foreground/30 bg-muted/50 text-muted-foreground",
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

    // 先读取最近一次快照，避免等待下一次 2 秒轮询
    getAgentSnapshot<AgentUpdatePayload>()
      .then((payload) => {
        if (isMounted && payload) {
          setSnapshot(payload);
        }
      })
      .catch(() => {
        // 静默忽略，后续事件流会补数据
      });

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

  const totalSessions = snapshot?.total_sessions ?? 0;

  // 所有 agent 按 cwd 分组为平级目录卡片
  const directoryGroups = useMemo(() => {
    const groups = new Map<string, { title: string; subtitle: string; tag: string; agents: AgentInfo[] }>();

    // 已注册项目的 session
    for (const project of snapshot?.projects ?? []) {
      if (project.agents.length === 0) continue;
      const title = project.agents[0]?.project_name?.trim() || getProjectName(project.project_path);
      groups.set(project.project_path, {
        title,
        subtitle: project.project_path,
        tag: "项目",
        agents: project.agents,
      });
    }

    // 未匹配 session 按 cwd 分组
    for (const agent of snapshot?.unmapped ?? []) {
      const cwd = agent.cwd;
      const existing = groups.get(cwd);
      if (existing) {
        existing.agents.push(agent);
      } else {
        groups.set(cwd, {
          title: getProjectName(cwd),
          subtitle: cwd,
          tag: "工作目录",
          agents: [agent],
        });
      }
    }

    return Array.from(groups.entries())
      .map(([key, group]) => ({ key, ...group }))
      .sort((a, b) => a.title.localeCompare(b.title, "zh-CN"));
  }, [snapshot?.projects, snapshot?.unmapped]);

  const allAgents = useMemo(
    () => directoryGroups.flatMap((group) => group.agents),
    [directoryGroups],
  );
  const totalCount = allAgents.length;

  const filteredGroups = useMemo(() => {
    if (!normalizedFilter) return directoryGroups;

    return directoryGroups
      .map((group) => {
        const agents = group.agents.filter((agent) => matchesAgentFilter(agent, normalizedFilter));
        return { ...group, agents };
      })
      .filter((group) => group.agents.length > 0);
  }, [directoryGroups, normalizedFilter]);

  const matchedCount = filteredGroups.reduce((sum, group) => sum + group.agents.length, 0);

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
          <p className="text-sm font-medium text-muted-foreground">Claude Code</p>
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
            <InfoHint content="选择 Token 消耗速率的统计窗口。采样不足时会显示「采样中」。" />
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
        <SummaryTile icon={Activity} label="活动目录" value={`${directoryGroups.length}`} detail="当前有会话的工作目录" />
        <SummaryTile icon={Clock} label="刷新时间" value={snapshot ? formatRelativeTime(snapshot.timestamp_ms, now) : "等待中"} detail={snapshot ? formatDateTime(snapshot.timestamp_ms) : "每 2 秒同步"} />
      </div>

      {totalCount > 0 && (
        <>
          <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
            <TokenStatTile label="Input Tokens" value={tokenStats.input} color="text-stage-l1" hint="会话累计接收的输入 token 总量，通常来自用户消息和上下文。" />
            <TokenStatTile label="Output Tokens" value={tokenStats.output} color="text-stage-l3" hint="会话累计生成的输出 token 总量，来自模型回复内容。" />
            <TokenStatTile label="Cache Read" value={tokenStats.cacheRead} color="text-stage-l5" hint="从缓存中读取的 token 数，命中缓存通常可降低实际调用成本。" />
            <TokenStatTile label="Cache Create" value={tokenStats.cacheCreate} color="text-primary" hint="新写入缓存的 token 数，首次写入通常会产生额外成本。" />
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
          {filteredGroups.map((group) => (
            <DirectoryCard
              key={group.key}
              group={group}
              rateUnit={rateUnit}
              rateType={rateType}
              expandedSessionId={expandedSessionId}
              onToggleSession={handleToggle}
            />
          ))}
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
    <Card className="shadow-xs">
      <CardContent className="flex min-h-32 flex-col justify-between p-4">
        <div className="flex items-start justify-between gap-3">
          <div className="space-y-1">
            <p className="text-xs text-muted-foreground">{label}</p>
            <p className="text-2xl font-semibold tracking-tight">{value}</p>
          </div>
          <span className="flex size-8 shrink-0 items-center justify-center rounded-md border border-border bg-tile text-muted-foreground">
            <Icon className="size-4" aria-hidden="true" />
          </span>
        </div>
        <p className="text-xs text-muted-foreground">{detail}</p>
      </CardContent>
    </Card>
  );
}

interface TokenStatTileProps {
  label: string;
  value: number;
  color: string;
  hint?: string;
}

function TokenStatTile({ label, value, color, hint }: TokenStatTileProps) {
  return (
    <Card className="shadow-xs">
      <CardContent className="space-y-3 p-4">
        <div className="flex items-center justify-between gap-3">
          <div className="flex items-center gap-1">
            <p className="text-xs text-muted-foreground">{label}</p>
            {hint && <InfoHint content={hint} />}
          </div>
          <span className={cn("size-1.5 rounded-full bg-current", color)} aria-hidden="true" />
        </div>
        <p className={cn("text-2xl font-semibold tracking-tight", color)}>{formatTokens(value)}</p>
        <p className="text-xs text-muted-foreground">累计消耗</p>
      </CardContent>
    </Card>
  );
}

interface DirectoryCardProps {
  group: {
    key: string;
    title: string;
    subtitle: string;
    tag: string;
    agents: AgentInfo[];
  };
  rateUnit: TokenRateUnit;
  rateType: RateType;
  expandedSessionId: string | null;
  onToggleSession: (sessionId: string) => void;
}

function DirectoryCard({ group, rateUnit, rateType, expandedSessionId, onToggleSession }: DirectoryCardProps) {
  const [isExpanded, setIsExpanded] = useState(true);

  return (
    <Card className="overflow-hidden shadow-xs">
      <div className="h-1 bg-gradient-to-r from-stage-l0 via-stage-l2 to-stage-l5" />
      <CardHeader className="flex-row items-center justify-between gap-4 py-3">
        <div className="min-w-0">
          <div className="flex items-center gap-2">
            <div className="flex size-7 shrink-0 items-center justify-center rounded-md bg-muted text-muted-foreground">
              <Bot className="size-4" aria-hidden="true" />
            </div>
            <div className="min-w-0">
              <div className="flex items-center gap-2">
                <CardTitle className="truncate text-base">{group.title}</CardTitle>
                <span className="shrink-0 rounded-full border border-border bg-muted/40 px-1.5 py-0 text-[10px] font-medium text-muted-foreground">
                  {group.tag}
                </span>
              </div>
              <CardDescription className="truncate text-xs">{group.subtitle}</CardDescription>
            </div>
          </div>
        </div>
        <div className="flex shrink-0 items-center gap-2">
          <span className="inline-flex items-center rounded-full border border-border bg-muted/40 px-2 py-0.5 text-[10px] font-medium text-muted-foreground">
            {group.agents.length} sessions
          </span>
          <Button
            type="button"
            variant="ghost"
            size="icon"
            className="size-7"
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
        <CardContent className="space-y-2 pb-3 pt-0">
          {group.agents.map((agent) => (
            <AgentSessionRow
              key={agent.session_id}
              agent={agent}
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

function AgentSessionRow({ agent, rateUnit, rateType, isExpanded, onToggle }: Omit<AgentSessionRowProps, "maxRate">) {
  const [activeTab, setActiveTab] = useState<AgentDetailTab>("timeline");
  const displayStatus = toDisplayStatus(agent.status);
  const isIdle = displayStatus === "Idle" || displayStatus === "Offline";

  // 根据用户选择的 rateType 获取用于染色的速率值；reason 无效时返回 0，不使用 fallback
  function getRateForColor(type: RateType): number {
    switch (type) {
      case "1min":
        return agent.token_rate_1m_reason === "fixed_window" ? getDisplayRate(agent, "1min", "minute") : 0;
      case "5min":
        return agent.token_rate_5m_reason === "fixed_window" ? getDisplayRate(agent, "5min", "minute") : 0;
      case "total":
        return agent.token_rate_total_reason === "observed_baseline" ? getDisplayRate(agent, "total", "minute") : 0;
      case "realtime":
        return agent.token_rate > 0 ? getDisplayRate(agent, "realtime", "minute") : 0;
      default:
        return 0;
    }
  }

  const rateForColor = getRateForColor(rateType);
  const rateColor = getRateColor(rateForColor);
  const context = getContextUsage(agent);
  const ctxColor = getContextColor(context.percent);
  const detailId = `agent-detail-${agent.session_id}`;

  return (
    <div
      className={cn(
        "rounded-lg border transition-colors outline-none focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50",
        isExpanded ? "border-l-2 border-l-primary border-border bg-muted/40" : "border-border/60 bg-muted/20 cursor-pointer hover:bg-muted/40",
      )}
    >
      {/* 紧凑行 */}
      <button type="button" className="block w-full text-left" aria-expanded={isExpanded} aria-controls={detailId} onClick={onToggle}>
        <div className="flex items-center gap-3 px-3 py-2.5">
          {/* 左侧：展开箭头 + 基本信息 */}
          <div className="flex min-w-0 flex-1 items-center gap-2"
>
            {isExpanded ? (
              <ChevronDown className="size-3.5 shrink-0 text-muted-foreground" aria-hidden="true" />
            ) : (
              <ChevronRight className="size-3.5 shrink-0 text-muted-foreground" aria-hidden="true" />
            )}
            <span className="shrink-0 whitespace-nowrap font-mono text-xs font-semibold tracking-tight">{shortSessionId(agent.session_id)}</span>
            <span className={cn("shrink-0 rounded-full border px-1.5 py-0 text-[10px] font-medium whitespace-nowrap", getAgentTypeStyle(agent.agent_type))}>
              {agent.agent_type || "unknown"}
            </span>
            <StatusBadge status={displayStatus} rawStatus={agent.status} />
            <span className="hidden min-w-0 truncate text-[10px] text-muted-foreground sm:inline" title={agent.cwd}>{agent.cwd}</span>
          </div>

          {/* 中间：迷你进度条 */}
          <div className="hidden min-w-0 flex-[1.2] items-center gap-3 md:flex"
>
            {/* Token 速率 */}
            <div className="min-w-0 flex-1"
>
              <div className="flex items-center justify-between gap-1 text-[10px]"
>
                <span className="text-muted-foreground">速率</span>
                <span className={cn("font-mono font-semibold", rateColor.text)}>
                  {isIdle
                    ? `Idle · ${formatIdleRate(agent, rateType, rateUnit)}`
                    : formatActiveRate(agent, rateType, rateUnit)}
                </span>
              </div>
              <div className="mt-0.5 h-1.5 overflow-hidden rounded-full bg-muted"
>
                <div
                  className={cn("h-full rounded-full transition-[width] duration-500", rateColor.bar)}
                  style={{ width: `${Math.min((rateForColor / 100_000) * 100, 100)}%` }}
                />
              </div>
            </div>
            {/* 上下文窗口 */}
            <div className="min-w-0 flex-1"
>
              <div className="flex items-center justify-between gap-1 text-[10px]"
>
                <span className="flex items-center gap-1 text-muted-foreground">上下文<InfoHint content="当前会话已使用的上下文比例，接近上限时可能触发压缩或影响后续对话质量。" interactive={false} /></span>
                <span className={cn("font-mono font-semibold", ctxColor.text)}>
                  {formatRate(context.percent)}%
                </span>
              </div>
              <div className="mt-0.5 h-1.5 overflow-hidden rounded-full bg-muted"
>
                <div
                  className={cn("h-full rounded-full transition-[width] duration-500", ctxColor.bar)}
                  style={{ width: `${context.percent}%` }}
                />
              </div>
            </div>
          </div>

          {/* 右侧：模型/轮次/PID */}
          <div className="hidden shrink-0 items-center gap-3 text-[10px] text-muted-foreground lg:flex"
>
            <span className="truncate max-w-[100px]" title={agent.model || "未知"}>{agent.model || "未知"}</span>
            <span className="rounded bg-muted px-1 py-0">{agent.turn_count} 轮</span>
            <span className="font-mono">{agent.pid || "-"}</span>
          </div>
        </div>
      </button>

      {/* 展开详情 */}
      <div
        id={detailId}
        className={cn(
          "overflow-hidden transition-[max-height,opacity] duration-300 ease-in-out",
          isExpanded ? "max-h-screen opacity-100" : "max-h-0 opacity-0",
        )}
      >
        <div className="border-t border-border px-3 py-3"
>
          {/* 移动端补充信息 */}
          <div className="mb-3 flex flex-wrap items-center gap-2 text-xs text-muted-foreground lg:hidden"
>
            <span className="truncate max-w-[200px]" title={agent.model || "未知"}>模型: {agent.model || "未知"}</span>
            <span>{agent.turn_count} 轮</span>
            <span className="font-mono">PID: {agent.pid || "-"}</span>
          </div>

          {/* 移动端进度条 */}
          <div className="mb-3 grid gap-3 md:hidden"
>
            <div>
              <div className="flex items-center justify-between gap-2 text-xs"
>
                <span className="text-muted-foreground">Token 消耗速率</span>
                <span className={cn("font-mono font-semibold", rateColor.text)}>
                  {isIdle
                    ? `Idle · ${formatIdleRate(agent, rateType, rateUnit)}`
                    : formatActiveRate(agent, rateType, rateUnit)}
                </span>
              </div>
              <div className="mt-1 h-2 overflow-hidden rounded-full bg-muted"
>
                <div className={cn("h-full rounded-full transition-[width] duration-500", rateColor.bar)} style={{ width: `${Math.min((rateForColor / 100_000) * 100, 100)}%` }} />
              </div>
            </div>
            <div>
              <div className="flex items-center justify-between gap-2 text-xs"
>
                <span className="text-muted-foreground">上下文窗口</span>
                <span className="font-mono font-semibold text-foreground">{formatTokens(context.current)} / {formatTokens(context.max)}</span>
              </div>
              <div className="mt-1 h-2 overflow-hidden rounded-full bg-muted"
>
                <div className={cn("h-full rounded-full transition-[width] duration-500", ctxColor.bar)} style={{ width: `${context.percent}%` }} />
              </div>
              <p className="mt-0.5 text-xs text-muted-foreground">{formatRate(context.percent)}% 使用率</p>
            </div>
          </div>

          <div className="mb-3"
>
            <TokenTrendSparkline tokenHistory={agent.token_history} />
          </div>
          <div className="mb-3 grid gap-2 sm:grid-cols-2 lg:grid-cols-4"
>
            <DetailMetric label="Input Tokens" value={formatTokens(agent.total_input_tokens)} />
            <DetailMetric label="Output Tokens" value={formatTokens(agent.total_output_tokens)} />
            <DetailMetric label="Cache Read" value={formatTokens(agent.total_cache_read)} />
            <DetailMetric label="Cache Create" value={formatTokens(agent.total_cache_create)} />
          </div>
          <div className="mb-2 flex gap-1 border-b border-border pb-2"
>
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

interface StatusBadgeProps {
  status: DisplayStatus;
  rawStatus: RawAgentStatus;
}

function StatusBadge({ status, rawStatus }: StatusBadgeProps) {
  return (
    <span className={cn("inline-flex shrink-0 items-center rounded-full border px-1.5 py-0 text-[10px] font-medium whitespace-nowrap", statusStyles[status])}>
      <span className="mr-1 size-1 rounded-full bg-current" aria-hidden="true" />
      {rawStatusText[rawStatus]}
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

function formatActiveRate(agent: AgentInfo, rateType: RateType, unit: TokenRateUnit): string {
  const rate = getDisplayRate(agent, rateType, unit);
  const suffix = unit === "second" ? "token/s" : "token/min";

  switch (rateType) {
    case "1min":
      if (agent.token_rate_1m_reason === "insufficient_samples") return "采样中";
      if (agent.token_rate_1m_reason === "short_span") return "采样中";
      if (agent.token_rate_1m_reason === "no_activity") return `0 ${suffix}`;
      break;
    case "5min":
      if (agent.token_rate_5m_reason === "insufficient_samples") return "采样中";
      if (agent.token_rate_5m_reason === "short_span") return "采样中";
      if (agent.token_rate_5m_reason === "no_activity") return `0 ${suffix}`;
      break;
    case "total":
      if (agent.token_rate_total_reason === "warming_up") return "采样中";
      if (agent.token_rate_total_reason === "no_activity") return `0 ${suffix}`;
      break;
    case "realtime":
      if (rate <= 0) return `0 ${suffix}`;
      break;
  }

  return `${formatRate(rate)} ${suffix}`;
}

function formatIdleRate(agent: AgentInfo, rateType: RateType, unit: TokenRateUnit): string {
  const suffix = unit === "second" ? "token/s" : "token/min";

  switch (rateType) {
    case "realtime": {
      const r = agent.token_rate;
      if (r > 0) return `瞬时 ${formatRate(unit === "second" ? r : r * 60)} ${suffix}`;
      return "当前无消耗";
    }
    case "1min": {
      switch (agent.token_rate_1m_reason) {
        case "fixed_window": {
          const rate = getDisplayRate(agent, "1min", unit);
          return rate > 0 ? `1m 均速 ${formatRate(rate)} ${suffix}` : "当前无消耗";
        }
        case "insufficient_samples":
        case "short_span":
          return "采样中";
        case "no_activity":
          return "当前无消耗";
        default:
          return "当前无消耗";
      }
    }
    case "5min": {
      switch (agent.token_rate_5m_reason) {
        case "fixed_window": {
          const rate = getDisplayRate(agent, "5min", unit);
          return rate > 0 ? `5m 均速 ${formatRate(rate)} ${suffix}` : "当前无消耗";
        }
        case "insufficient_samples":
        case "short_span":
          return "采样中";
        case "no_activity":
          return "当前无消耗";
        default:
          return "当前无消耗";
      }
    }
    case "total": {
      switch (agent.token_rate_total_reason) {
        case "observed_baseline": {
          const rate = getDisplayRate(agent, "total", unit);
          return rate > 0 ? `观察期均速 ${formatRate(rate)} ${suffix}` : "当前无消耗";
        }
        case "warming_up":
          return "采样中";
        case "no_activity":
          return "当前无消耗";
        default:
          return "当前无消耗";
      }
    }
    default:
      return "当前无消耗";
  }
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

// Token 消耗速率绝对分级（统一换算为 token/min）
function getRateColor(ratePerMin: number): { bar: string; text: string; level: string } {
  if (ratePerMin >= 80_000) return { bar: "bg-red-500", text: "text-red-600", level: "极高" };
  if (ratePerMin >= 20_000) return { bar: "bg-orange-500", text: "text-orange-600", level: "高" };
  if (ratePerMin >= 1_000) return { bar: "bg-amber-500", text: "text-amber-600", level: "中" };
  return { bar: "bg-green-500", text: "text-green-600", level: "低" };
}

// 上下文窗口分级
function getContextColor(percent: number): { bar: string; text: string } {
  if (percent >= 90) return { bar: "bg-red-500", text: "text-red-600" };
  if (percent >= 75) return { bar: "bg-orange-500", text: "text-orange-600" };
  if (percent >= 50) return { bar: "bg-amber-500", text: "text-amber-600" };
  return { bar: "bg-green-500", text: "text-green-600" };
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

  // 采样压缩到最多 120 个点，防止柱状条溢出
  const MAX_POINTS = 120;
  const sampled =
    tokenHistory.length <= MAX_POINTS
      ? tokenHistory
      : sampleToMax(tokenHistory, MAX_POINTS);

  const max = Math.max(...sampled);
  const min = Math.min(...sampled);
  const range = max - min || 1;

  return (
    <div className="space-y-2">
      <p className="text-xs font-medium text-muted-foreground">Token 生成趋势</p>
      <div className="flex items-end gap-px h-16 overflow-hidden rounded-lg border border-border bg-background/60 p-2">
        {/* eslint-disable-next-line react/no-array-index-key */}
        {sampled.map((value, i) => {
          const height = `${((value - min) / range) * 100}%`;
          return (
            <div
              key={i}
              className="min-w-0 flex-1 bg-primary/50 rounded-t-sm transition-all duration-300"
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

// 将数据采样到最多 maxPoints 个点，使用 bucket max 采样
function sampleToMax(data: number[], maxPoints: number): number[] {
  if (data.length <= maxPoints) return data;
  const result: number[] = [];
  const bucketSize = data.length / maxPoints;
  for (let i = 0; i < maxPoints; i++) {
    const start = Math.floor(i * bucketSize);
    const end = Math.floor((i + 1) * bucketSize);
    let max = data[start];
    for (let j = start + 1; j < end; j++) {
      if (data[j] > max) max = data[j];
    }
    result.push(max);
  }
  return result;
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
