import { useCallback, useEffect, useMemo, useState } from "react";
import {
  BarChart,
  Bar,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
} from "recharts";
import {
  AlertTriangle,
  BarChart3,
  Bookmark,
  Boxes,
  FilePlus,
  FolderKanban,
  Hash,
  Layers,
  Loader2,
  RefreshCw,
  Server,
  Users,
  Zap,
} from "lucide-react";

import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { useTauri } from "@/hooks/useTauri";
import { cn } from "@/lib/utils";

import type {
  GroupBy,
  TimeRange,
  UsageAggregate,
  UsageScanSummary,
  UsageSourceStatus,
} from "./types";

const CONFIDENCE_LABELS: Record<string, string> = {
  high: "高",
  medium: "中",
  low: "低",
};

const REALTIME_LABELS: Record<string, string> = {
  delayed: "延迟更新",
  near_realtime: "近实时",
  realtime: "实时",
};

const REASON_LABELS: Record<string, string> = {
  not_found: "目录不存在",
  not_a_directory: "不是目录",
  permission_denied: "权限不足",
  invalid_path: "路径无效",
  missing_structure: "缺少结构",
  empty: "目录为空",
};

function formatNumber(n: number): string {
  if (n < 1000) return String(n);
  if (n < 1_000_000) return `${(n / 1000).toFixed(1)}K`;
  return `${(n / 1_000_000).toFixed(1)}M`;
}

function formatDate(iso?: string): string {
  if (!iso) return "—";
  try {
    return new Date(iso).toLocaleString("zh-CN");
  } catch {
    return iso;
  }
}

/** 按 groupBy 生成去重后的显示行（用于 tooltip / title） */
function buildDisplayParts(
  groupBy: GroupBy,
  label: string,
  detail: string | undefined,
  key: string,
): string[] {
  const lines: string[] = [];
  const seen = new Set<string>();

  const add = (s: string) => {
    const trimmed = s.trim();
    if (trimmed && !seen.has(trimmed)) {
      seen.add(trimmed);
      lines.push(trimmed);
    }
  };

  add(label);

  if (groupBy === "project") {
    // detail 是路径，优先展示；key 与 detail 相同则不重复
    if (detail) add(detail);
    // key 只在 detail 为空且 key 与 label 不同时展示
    else if (key !== label) add(key);
  } else if (groupBy === "session") {
    // detail 是 project_name · short_id
    if (detail) add(detail);
    // key 是完整 session_id，只要 key 未包含在 detail 中就展示
    if (!detail || !detail.includes(key.slice(0, 8))) {
      if (key !== label && !lines.includes(key)) add(key);
    }
  }
  // groupBy === "model": 不展示 detail 和 key

  return lines;
}

export function UsageAnalytics() {
  const { invoke } = useTauri();

  const [timeRange, setTimeRange] = useState<TimeRange>("today");
  const [groupBy, setGroupBy] = useState<GroupBy>("project");

  const [summary, setSummary] = useState<UsageScanSummary | null>(null);
  const [aggregate, setAggregate] = useState<UsageAggregate | null>(null);

  const [isLoading, setIsLoading] = useState(true);
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const loadAnalytics = useCallback(
    async (range: TimeRange, grouping: GroupBy) => {
      const result = await invoke<UsageAggregate, { timeRange: string; groupBy: string }>(
        "get_usage_analytics",
        {
          timeRange: range,
          groupBy: grouping,
        },
      );
      setAggregate(result);
    },
    [invoke],
  );

  const loadInitial = useCallback(async () => {
    setIsLoading(true);
    setError(null);

    try {
      // 先 scan 获取 summary（含 source_status），避免 status + analytics 各自触发扫描
      const scanResult = await invoke<UsageScanSummary>("scan_usage_data");
      setSummary(scanResult);

      // 再拉取 analytics
      await loadAnalytics(timeRange, groupBy);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsLoading(false);
    }
  }, [invoke, loadAnalytics, timeRange, groupBy]);

  const handleRefresh = useCallback(async () => {
    setIsRefreshing(true);
    setError(null);

    try {
      const scanResult = await invoke<UsageScanSummary>("scan_usage_data");
      setSummary(scanResult);
      await loadAnalytics(timeRange, groupBy);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsRefreshing(false);
    }
  }, [invoke, loadAnalytics, timeRange, groupBy]);

  const handleTimeRangeChange = useCallback(
    async (range: TimeRange) => {
      setTimeRange(range);
      setError(null);
      try {
        await loadAnalytics(range, groupBy);
      } catch (err) {
        setError(err instanceof Error ? err.message : String(err));
      }
    },
    [loadAnalytics, groupBy],
  );

  const handleGroupByChange = useCallback(
    async (grouping: GroupBy) => {
      setGroupBy(grouping);
      setError(null);
      try {
        await loadAnalytics(timeRange, grouping);
      } catch (err) {
        setError(err instanceof Error ? err.message : String(err));
      }
    },
    [loadAnalytics, timeRange],
  );

  useEffect(() => {
    let isMounted = true;

    async function init() {
      try {
        const scanResult = await invoke<UsageScanSummary>("scan_usage_data");
        if (!isMounted) return;
        setSummary(scanResult);

        const agg = await invoke<UsageAggregate, { timeRange: string; groupBy: string }>(
          "get_usage_analytics",
          {
            timeRange: "today",
            groupBy: "project",
          },
        );
        if (!isMounted) return;
        setAggregate(agg);
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

    init();

    return () => {
      isMounted = false;
    };
  }, [invoke]);

  const chartData = useMemo(() => {
    if (!aggregate) return [];
    const gb = aggregate.group_by;
    return aggregate.groups.slice(0, 10).map((g) => {
      // 轴标签截断：去掉开头 "/"，中文/长文本最多 16 字符
      let label = g.group_label.trim().replace(/^\/+/, "");
      if (label.length > 16) {
        label = label.slice(0, 15) + "…";
      }
      return {
        label,
        displayParts: buildDisplayParts(gb, g.group_label, g.group_detail, g.group_key),
        total: g.total_tokens,
        input: g.input_tokens,
        output: g.output_tokens,
        cacheRead: g.cache_read_tokens,
        cacheCreate: g.cache_create_tokens,
      };
    });
  }, [aggregate]);

  const hasData = aggregate && aggregate.groups.length > 0;

  return (
    <section className="space-y-6">
      {/* 标题 */}
      <div className="flex flex-col gap-4 lg:flex-row lg:items-end lg:justify-between">
        <div className="space-y-2">
          <p className="text-sm font-medium text-muted-foreground">Usage Analytics</p>
          <h1 className="text-3xl font-semibold tracking-tight">Usage 分析</h1>
          <p className="max-w-2xl text-sm text-muted-foreground">
            基于 Claude Code 本地 session transcript JSONL 的 token usage 统计。最终用量较可靠，实时数据可能有延迟。
          </p>
        </div>
        <Button
          type="button"
          variant="outline"
          onClick={handleRefresh}
          disabled={isRefreshing}
        >
          {isRefreshing ? (
            <Loader2 className="size-4 animate-spin" aria-hidden="true" />
          ) : (
            <RefreshCw className="size-4" aria-hidden="true" />
          )}
          刷新
        </Button>
      </div>

      {/* 错误提示 */}
      {error && (
        <Card className="border-destructive/40 bg-destructive/10">
          <CardContent className="flex items-center gap-3 p-4 text-sm text-destructive">
            <AlertTriangle className="size-4" aria-hidden="true" />
            <span className="flex-1">{error}</span>
            <Button type="button" variant="ghost" size="sm" onClick={loadInitial}>
              重试
            </Button>
          </CardContent>
        </Card>
      )}

      {/* 数据源状态卡片 */}
      {summary && <SourceStatusCard status={summary.source_status} />}

      {/* 时间范围和分组切换 */}
      <div className="flex flex-wrap items-center gap-3">
        <div className="flex items-center gap-1 rounded-md border bg-muted p-1">
          <FilterButton
            active={timeRange === "today"}
            onClick={() => handleTimeRangeChange("today")}
          >
            今日
          </FilterButton>
          <FilterButton
            active={timeRange === "last7days"}
            onClick={() => handleTimeRangeChange("last7days")}
          >
            近 7 天
          </FilterButton>
        </div>

        <div className="flex items-center gap-1 rounded-md border bg-muted p-1">
          <FilterButton
            active={groupBy === "project"}
            onClick={() => handleGroupByChange("project")}
          >
            按项目
          </FilterButton>
          <FilterButton
            active={groupBy === "model"}
            onClick={() => handleGroupByChange("model")}
          >
            按模型
          </FilterButton>
          <FilterButton
            active={groupBy === "session"}
            onClick={() => handleGroupByChange("session")}
          >
            按会话
          </FilterButton>
        </div>
      </div>

      {/* 汇总卡片 */}
      {aggregate && (
        <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
          <SummaryTile
            icon={Zap}
            label="Total Tokens"
            value={formatNumber(aggregate.total_tokens)}
            fullValue={String(aggregate.total_tokens)}
            detail="所有 token 总和"
          />
          <SummaryTile
            icon={BarChart3}
            label="Input"
            value={formatNumber(aggregate.input_tokens)}
            fullValue={String(aggregate.input_tokens)}
            detail="输入 token"
          />
          <SummaryTile
            icon={Layers}
            label="Output"
            value={formatNumber(aggregate.output_tokens)}
            fullValue={String(aggregate.output_tokens)}
            detail="输出 token"
          />
          <SummaryTile
            icon={Bookmark}
            label="Cache Read"
            value={formatNumber(aggregate.cache_read_tokens)}
            fullValue={String(aggregate.cache_read_tokens)}
            detail="缓存读取 token"
          />
          <SummaryTile
            icon={FilePlus}
            label="Cache Create"
            value={formatNumber(aggregate.cache_create_tokens)}
            fullValue={String(aggregate.cache_create_tokens)}
            detail="缓存创建 token"
          />
          <SummaryTile
            icon={Users}
            label="Sessions"
            value={formatNumber(aggregate.session_count)}
            fullValue={String(aggregate.session_count)}
            detail="会话数量"
          />
          <SummaryTile
            icon={FolderKanban}
            label="Projects"
            value={formatNumber(aggregate.project_count)}
            fullValue={String(aggregate.project_count)}
            detail="项目数量"
          />
          <SummaryTile
            icon={Boxes}
            label="Models"
            value={formatNumber(aggregate.model_count)}
            fullValue={String(aggregate.model_count)}
            detail="模型数量"
          />
        </div>
      )}

      {/* 加载中 */}
      {isLoading && (
        <Card className="flex min-h-72 items-center justify-center border-dashed">
          <div className="flex items-center gap-3 text-sm text-muted-foreground">
            <Loader2 className="size-4 animate-spin" aria-hidden="true" />
            正在扫描本地 Claude Code usage 数据…
          </div>
        </Card>
      )}

      {/* 空状态 */}
      {!isLoading && !error && !hasData && (
        <Card className="flex min-h-72 items-center justify-center border-dashed">
          <div className="text-center text-sm text-muted-foreground">
            <BarChart3 className="mx-auto mb-3 size-8 opacity-40" aria-hidden="true" />
            <p>暂无 usage 数据</p>
            <p className="mt-1 text-xs">未在配置目录中发现有效的 session JSONL 文件</p>
          </div>
        </Card>
      )}

      {/* 图表 */}
      {!isLoading && hasData && chartData.length > 0 && (
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2 text-base">
              <BarChart3 className="size-4" aria-hidden="true" />
              Token 分布（Top 10）
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="h-64 w-full">
              <ResponsiveContainer width="100%" height="100%">
                <BarChart data={chartData} margin={{ top: 5, right: 5, left: 0, bottom: 5 }}>
                  <CartesianGrid strokeDasharray="3 3" stroke="var(--border)" />
                  <XAxis
                    dataKey="label"
                    tick={{ fontSize: 12, fill: "var(--muted-foreground)" }}
                    axisLine={{ stroke: "var(--border)" }}
                    tickLine={false}
                  />
                  <YAxis
                    tick={{ fontSize: 12, fill: "var(--muted-foreground)" }}
                    axisLine={false}
                    tickLine={false}
                    tickFormatter={(v: number) => formatNumber(v)}
                  />
                  <Tooltip content={<ChartTooltipContent />} />
                  <Bar dataKey="total" fill="hsl(var(--chart-1))" radius={[4, 4, 0, 0]} />
                </BarChart>
              </ResponsiveContainer>
            </div>
          </CardContent>
        </Card>
      )}

      {/* 分组表格 */}
      {!isLoading && hasData && (
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2 text-base">
              <Hash className="size-4" aria-hidden="true" />
              分组明细
              {aggregate && aggregate.groups.length > 50 && (
                <span className="text-xs font-normal text-muted-foreground">
                  （仅显示前 50 条）
                </span>
              )}
            </CardTitle>
          </CardHeader>
          <CardContent className="p-0">
            <div className="overflow-x-auto">
              <table className="w-full text-sm">
                <thead>
                  <tr className="border-b bg-muted/50 text-left text-muted-foreground">
                    <th className="px-4 py-2.5 font-medium">名称</th>
                    <th className="px-4 py-2.5 font-medium text-right">Total</th>
                    <th className="px-4 py-2.5 font-medium text-right">Input</th>
                    <th className="px-4 py-2.5 font-medium text-right">Output</th>
                    <th className="px-4 py-2.5 font-medium text-right">Cache Read</th>
                    <th className="px-4 py-2.5 font-medium text-right">Cache Create</th>
                    <th className="px-4 py-2.5 font-medium text-right">Sessions</th>
                    <th className="px-4 py-2.5 font-medium text-right">最近时间</th>
                  </tr>
                </thead>
                <tbody>
                  {aggregate?.groups.slice(0, 50).map((group) => (
                    <tr
                      key={group.group_key}
                      className="border-b transition-colors hover:bg-muted/30"
                    >
                      <td className="px-4 py-2.5">
                        <div
                          className="max-w-[300px]"
                          title={buildDisplayParts(
                            aggregate!.group_by,
                            group.group_label,
                            group.group_detail,
                            group.group_key,
                          ).join("\n")}
                        >
                          <div
                            className="break-words font-semibold leading-snug"
                            data-testid="group-label"
                          >
                            {group.group_label}
                          </div>
                          {group.group_detail && (
                            <div
                              className="mt-0.5 truncate text-xs text-muted-foreground"
                              data-testid="group-detail"
                            >
                              {group.group_detail}
                            </div>
                          )}
                        </div>
                      </td>
                      <td className="px-4 py-2.5 text-right font-semibold">
                        {formatNumber(group.total_tokens)}
                      </td>
                      <td className="px-4 py-2.5 text-right text-muted-foreground">
                        {formatNumber(group.input_tokens)}
                      </td>
                      <td className="px-4 py-2.5 text-right text-muted-foreground">
                        {formatNumber(group.output_tokens)}
                      </td>
                      <td className="px-4 py-2.5 text-right text-muted-foreground">
                        {formatNumber(group.cache_read_tokens)}
                      </td>
                      <td className="px-4 py-2.5 text-right text-muted-foreground">
                        {formatNumber(group.cache_create_tokens)}
                      </td>
                      <td className="px-4 py-2.5 text-right">{group.session_count}</td>
                      <td className="px-4 py-2.5 text-right text-muted-foreground">
                        {formatDate(group.last_seen)}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </CardContent>
        </Card>
      )}
    </section>
  );
}

/* ─── 数据源状态卡片 ─── */
function SourceStatusCard({ status }: { status: UsageSourceStatus }) {
  return (
    <Card>
      <CardHeader className="pb-2">
        <CardTitle className="flex items-center gap-2 text-base">
          <Server className="size-4" aria-hidden="true" />
          数据源状态
        </CardTitle>
      </CardHeader>
      <CardContent>
        <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
          <StatusItem label="数据源" value={status.source_type} />
          <StatusItem
            label="可信度"
            value={CONFIDENCE_LABELS[status.confidence] ?? status.confidence}
          />
          <StatusItem
            label="实时性"
            value={REALTIME_LABELS[status.realtime_level] ?? status.realtime_level}
          />
          <StatusItem label="已识别目录" value={`${status.config_dirs.length}`} />
          <StatusItem label="可读目录" value={`${status.readable_dirs.length}`} />
          <StatusItem label="不可读目录" value={`${status.unreadable_dirs.length}`} />
          <StatusItem label="最近扫描" value={formatDate(status.last_scan_at)} />
          <StatusItem label="最近 usage" value={formatDate(status.last_usage_at)} />
        </div>

        {status.unreadable_dirs.length > 0 && (
          <div className="mt-3 rounded-md border border-amber-200 bg-amber-50 p-3 text-xs text-amber-800 dark:border-amber-900 dark:bg-amber-950/30 dark:text-amber-200">
            <div className="mb-1.5 flex items-center gap-1.5 font-medium">
              <AlertTriangle className="size-3.5" aria-hidden="true" />
              部分目录不可读
            </div>
            <ul className="space-y-1">
              {status.unreadable_dirs.slice(0, 3).map((dir, i) => (
                <li key={i} className="truncate" title={dir.path}>
                  {dir.path} — {REASON_LABELS[dir.reason] ?? dir.reason}
                </li>
              ))}
              {status.unreadable_dirs.length > 3 && (
                <li className="text-muted-foreground">
                  还有 {status.unreadable_dirs.length - 3} 个…
                </li>
              )}
            </ul>
          </div>
        )}

        {status.notes.length > 0 && (
          <div className="mt-3 space-y-1 text-xs text-muted-foreground">
            {status.notes.map((note, i) => (
              <p key={i}>• {note}</p>
            ))}
          </div>
        )}
      </CardContent>
    </Card>
  );
}

function StatusItem({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <div className="text-xs text-muted-foreground">{label}</div>
      <div className="mt-0.5 text-sm font-medium">{value}</div>
    </div>
  );
}

/* ─── 汇总卡片 ─── */
function SummaryTile({
  icon: Icon,
  label,
  value,
  fullValue,
  detail,
}: {
  icon: typeof BarChart3;
  label: string;
  value: string;
  fullValue: string;
  detail: string;
}) {
  return (
    <Card className="transition-shadow hover:shadow-sm">
      <CardContent className="flex items-center gap-3 p-4">
        <div className="flex size-9 shrink-0 items-center justify-center rounded-lg bg-muted text-muted-foreground">
          <Icon className="size-4" aria-hidden="true" />
        </div>
        <div className="min-w-0">
          <div className="text-xs text-muted-foreground">{label}</div>
          <div className="text-lg font-semibold tracking-tight" title={fullValue}>
            {value}
          </div>
          <div className="text-xs text-muted-foreground/70">{detail}</div>
        </div>
      </CardContent>
    </Card>
  );
}

/* ─── 自定义图表 Tooltip ─── */
interface ChartTooltipPayloadItem {
  payload?: Record<string, unknown>;
}

interface ChartTooltipProps {
  active?: boolean;
  payload?: ChartTooltipPayloadItem[];
  label?: string;
}

function ChartTooltipContent({ active, payload }: ChartTooltipProps) {
  if (!active || !payload || payload.length === 0) return null;

  const p = payload[0].payload ?? {};
  const displayParts = (p.displayParts as string[] | undefined) ?? [];

  const items: { label: string; value: number }[] = [
    { label: "Total", value: Number(p.total ?? 0) },
    { label: "Input", value: Number(p.input ?? 0) },
    { label: "Output", value: Number(p.output ?? 0) },
    { label: "Cache Read", value: Number(p.cacheRead ?? 0) },
    { label: "Cache Create", value: Number(p.cacheCreate ?? 0) },
  ];

  return (
    <div
      className="rounded-lg border bg-card p-3 text-xs shadow-sm"
      style={{ borderColor: "var(--border)" }}
    >
      {displayParts.map((line, i) => (
        <div
          key={i}
          className={
            i === 0
              ? "mb-1.5 font-semibold"
              : i === displayParts.length - 1 && line.length > 20
                ? "mb-1.5 font-mono text-[10px] text-muted-foreground"
                : "mb-1.5 text-muted-foreground"
          }
        >
          {line}
        </div>
      ))}
      <div className="space-y-0.5">
        {items.map((item) => (
          <div key={item.label} className="flex items-center justify-between gap-4">
            <span className="text-muted-foreground">{item.label}</span>
            <span className="font-medium tabular-nums">
              {formatNumber(item.value)} ({item.value.toLocaleString("zh-CN")})
            </span>
          </div>
        ))}
      </div>
    </div>
  );
}

/* ─── 过滤按钮 ─── */
function FilterButton({
  active,
  children,
  onClick,
}: {
  active: boolean;
  children: React.ReactNode;
  onClick: () => void;
}) {
  return (
    <Button
      type="button"
      variant="ghost"
      size="sm"
      className={cn(
        "h-7 px-3 text-xs font-medium",
        active && "bg-background text-foreground shadow-sm",
      )}
      onClick={onClick}
    >
      {children}
    </Button>
  );
}
