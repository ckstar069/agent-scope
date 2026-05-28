import { useEffect, useMemo, useState } from "react";
import {
  AlertTriangle,
  Brain,
  EyeOff,
  FolderOpen,
  Heart,
  Loader2,
  RefreshCw,
  ShieldCheck,
  X,
  Zap,
} from "lucide-react";

import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Switch } from "@/components/ui/switch";
import { cn } from "@/lib/utils";

import type { ClaudeMemoryAsset, ContextPressure, MemoryDuplicateGroup, MemoryHealthReport } from "./types";
import { useClaudeMemory, useContextPressure, useMemoryHealth, useReviewQueue } from "./hooks/useClaudeMemory";

import { LoadChainSimulator } from "./components/LoadChainSimulator";
import { MemoryAssetDetail } from "./components/MemoryAssetDetail";
import { MemoryAssetTree } from "./components/MemoryAssetTree";
import { ReviewQueuePanel } from "./components/ReviewQueuePanel";

/** 从 healthReport 派生各资产的健康标记 */
function deriveHealthSets(report: MemoryHealthReport | null, assets: ClaudeMemoryAsset[]) {
  const staleAssetIds = new Set<string>();
  const duplicateAssetIds = new Set<string>();
  const staleDaysByAssetId = new Map<string, number>();
  const issueAssetIds = new Set<string>();
  const secretAssetIds = new Set<string>();
  const duplicateGroupsByAssetId = new Map<string, MemoryDuplicateGroup[]>();

  if (!report) return { staleAssetIds, duplicateAssetIds, staleDaysByAssetId, issueAssetIds, secretAssetIds, duplicateGroupsByAssetId };

  for (const s of report.stale_assets) {
    staleAssetIds.add(s.asset_id);
    if (s.stale_days != null) staleDaysByAssetId.set(s.asset_id, s.stale_days);
  }
  for (const g of report.duplicate_groups) {
    for (const aid of g.asset_ids) {
      duplicateAssetIds.add(aid);
      const list = duplicateGroupsByAssetId.get(aid) ?? [];
      list.push(g);
      duplicateGroupsByAssetId.set(aid, list);
    }
  }
  for (const issue of report.top_issues) {
    for (const aid of issue.asset_ids) {
      issueAssetIds.add(aid);
    }
  }
  for (const a of assets) {
    if (a.secret_issues.length > 0) secretAssetIds.add(a.id);
  }
  return { staleAssetIds, duplicateAssetIds, staleDaysByAssetId, issueAssetIds, secretAssetIds, duplicateGroupsByAssetId };
}

interface ClaudeMemoryProps {
  projectPath?: string;
  page?: "assets" | "load-chain";
}

export function ClaudeMemory({ projectPath, page = "assets" }: ClaudeMemoryProps) {
  if (page === "load-chain") {
    return <LoadChainSimulator />;
  }

  return <ClaudeMemoryAssets projectPath={projectPath} />;
}

function ClaudeMemoryAssets({ projectPath }: { projectPath?: string }) {
  const { overview, isLoading, error, refresh: refreshOverview } = useClaudeMemory(projectPath);
  const { report: healthReport, isLoading: healthLoading, refresh: refreshHealth } = useMemoryHealth(projectPath);
  const { pressure, isLoading: pressureLoading, refresh: refreshPressure } = useContextPressure(projectPath);
  const { queue, isLoading: rqLoading, error: rqError, sync: syncQueue, updateState } = useReviewQueue(projectPath);
  const [bannerDismissed, setBannerDismissed] = useState(false);
  const healthSets = useMemo(() => deriveHealthSets(healthReport, overview?.assets ?? []), [healthReport, overview?.assets]);
  const assetsById = useMemo(() => {
    if (!overview) return new Map<string, ClaudeMemoryAsset>();
    return new Map(overview.assets.map((a) => [a.id, a]));
  }, [overview]);
  const [selectedAsset, setSelectedAsset] = useState<ClaudeMemoryAsset | null>(null);
  const [hideMissing, setHideMissing] = useState(true);
  const [healthFilter, setHealthFilter] = useState<"all" | "stale" | "duplicate" | "issue" | "secret">("all");
  const isRefreshing = isLoading || healthLoading || pressureLoading || rqLoading;

  const visibleAssets = useMemo(() => {
    if (!overview) return [];
    let assets = overview.assets;
    if (hideMissing) assets = assets.filter((a) => a.exists);
    if (healthFilter === "stale") assets = assets.filter((a) => healthSets.staleAssetIds.has(a.id));
    else if (healthFilter === "duplicate") assets = assets.filter((a) => healthSets.duplicateAssetIds.has(a.id));
    else if (healthFilter === "issue") assets = assets.filter((a) => healthSets.issueAssetIds.has(a.id));
    else if (healthFilter === "secret") assets = assets.filter((a) => healthSets.secretAssetIds.has(a.id));
    return assets;
  }, [overview, hideMissing, healthFilter, healthSets]);

  // 切换隐藏不存在时，如果当前选中项被隐藏则清除选中
  useEffect(() => {
    if (selectedAsset && !visibleAssets.some((a) => a.id === selectedAsset.id)) {
      setSelectedAsset(null);
    }
  }, [visibleAssets, selectedAsset]);

  /** 从外部链接/ banner 定位到资产，先切回全部过滤确保资产可见 */
  const selectAssetFromExternalLink = (assetId: string) => {
    if (!overview) return;
    const target = overview.assets.find((a) => a.exists && a.id === assetId);
    if (target) {
      setHealthFilter("all");
      setSelectedAsset(target);
    }
  };

  /** 选中 issue 关联的第一个存在资产 */
  const selectFirstIssueAsset = (issueAssetIds: string[]) => {
    if (!overview || issueAssetIds.length === 0) return;
    const target = overview.assets.find(
      (a) => a.exists && issueAssetIds.includes(a.id),
    );
    if (target) selectAssetFromExternalLink(target.id);
  };

  return (
    <section className="flex h-full flex-col gap-4">
      {/* 页面标题 */}
      <div className="flex flex-col gap-4 lg:flex-row lg:items-end lg:justify-between">
        <div className="space-y-2">
          <p className="text-sm font-medium text-muted-foreground">
            Claude Code
          </p>
          <h1 className="text-3xl font-semibold tracking-tight">Claude 记忆</h1>
          <p className="max-w-2xl text-sm text-muted-foreground">
            扫描本机 Claude Code 记忆文件，包括 Instruction、Rules、Skills、Agents 和 Auto Memory。
          </p>
        </div>
        <Button
          type="button"
          variant="outline"
          disabled={isRefreshing}
          onClick={() => {
            void Promise.all([
              refreshOverview(true),
              refreshHealth(true),
              refreshPressure(true),
              syncQueue(true),
            ]);
          }}
        >
          {isRefreshing ? (
            <Loader2 className="mr-2 size-4 animate-spin" aria-hidden="true" />
          ) : (
            <RefreshCw className="mr-2 size-4" aria-hidden="true" />
          )}
          刷新
        </Button>
      </div>

      {/* 加载中 */}
      {isLoading && !overview && (
        <div className="flex min-h-56 items-center justify-center rounded-xl border border-dashed border-border text-sm text-muted-foreground">
          <Loader2 className="mr-2 size-4 animate-spin" aria-hidden="true" />
          正在扫描记忆文件…
        </div>
      )}

      {/* 错误 */}
      {error && !overview && (
        <div className="flex min-h-56 items-center justify-center rounded-xl border border-dashed border-destructive/30 bg-destructive/5 text-sm text-destructive">
          <AlertTriangle className="mr-2 size-4" aria-hidden="true" />
          {error}
        </div>
      )}

      {/* 内容 */}
      {overview && (
        <div className="flex flex-1 flex-col gap-4 overflow-hidden">
          {/* Context Pressure Banner */}
          {pressure && !bannerDismissed && pressure.level !== "normal" && (
            <ContextPressureBanner
              pressure={pressure}
              onDismiss={() => setBannerDismissed(true)}
              onSelectAsset={selectAssetFromExternalLink}
            />
          )}

          {/* 顶部统计 */}
          <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-5">
            <StatCard
              icon={Brain}
              label="总资产"
              value={overview.summary.total_assets}
            />
            <StatCard
              icon={ShieldCheck}
              label="已存在"
              value={overview.summary.total_existing}
            />
            <StatCard
              icon={AlertTriangle}
              label="风险项"
              value={overview.summary.total_secret_issues}
              tone={
                overview.summary.total_secret_issues > 0
                  ? "warning"
                  : "default"
              }
            />
            <StatCard
              icon={Heart}
              label="诊断评分"
              value={healthReport?.overall_score ?? "-"}
              tone={
                healthReport
                  ? healthReport.overall_score >= 80
                    ? "default"
                    : healthReport.overall_score >= 60
                      ? "warning"
                      : "danger"
                  : "default"
              }
            />
            <StatCard
              icon={FolderOpen}
              label="Claude 配置目录"
              value={overview.host_profile.claude_config_dir}
              isText
            />
          </div>

          {/* 扫描错误 */}
          {overview.errors.length > 0 && (
            <div className="space-y-2">
              {overview.errors.map((err, index) => (
                <div
                  key={index}
                  className="flex items-start gap-2 rounded-lg border border-amber-200 bg-amber-50 p-3 text-sm text-amber-800 dark:border-amber-900/50 dark:bg-amber-950/20 dark:text-amber-400"
                >
                  <AlertTriangle
                    className="mt-0.5 size-4 shrink-0"
                    aria-hidden="true"
                  />
                  <div>
                    <p className="font-medium">
                      [{err.scope}] {err.path}
                    </p>
                    <p className="text-amber-700 dark:text-amber-500">
                      {err.message}
                    </p>
                  </div>
                </div>
              ))}
            </div>
          )}

          {/* Review Queue */}
          <ReviewQueuePanel
            queue={queue}
            isLoading={rqLoading}
            error={rqError}
            onSync={() => void syncQueue(false)}
            onUpdateState={updateState}
            assetsById={assetsById}
            onSelectAsset={selectAssetFromExternalLink}
          />

          {/* 主内容：资产树 + 详情 */}
          <div className="flex flex-1 gap-4 overflow-hidden">
            <Card className="flex w-72 shrink-0 flex-col overflow-hidden shadow-xs">
              <CardHeader className="flex flex-row items-center justify-between border-b border-border bg-tile/70 py-3">
                <CardTitle className="text-sm font-medium">
                  记忆资产
                </CardTitle>
                <div className="flex items-center gap-2">
                  <EyeOff className="size-3.5 text-muted-foreground" aria-hidden="true" />
                  <span className="text-xs text-muted-foreground">隐藏不存在</span>
                  <Switch
                    checked={hideMissing}
                    onCheckedChange={setHideMissing}
                    aria-label="隐藏不存在的资产"
                  />
                </div>
              </CardHeader>
              <CardContent className="flex-1 p-0">
                <div className="flex items-center gap-1 border-b border-border px-3 py-2">
                  {(["all", "stale", "duplicate", "issue", "secret"] as const).map((f) => {
                    const labels: Record<typeof f, string> = {
                      all: "全部",
                      stale: "stale",
                      duplicate: "duplicate",
                      issue: "有问题",
                      secret: "secret",
                    };
                    return (
                      <button
                        key={f}
                        type="button"
                        onClick={() => setHealthFilter(f)}
                        className={cn(
                          "rounded-md px-2 py-1 text-xs font-medium transition-colors",
                          healthFilter === f
                            ? "bg-primary/10 text-primary"
                            : "text-muted-foreground hover:bg-muted/60",
                        )}
                      >
                        {labels[f]}
                      </button>
                    );
                  })}
                </div>
                <MemoryAssetTree
                  assets={visibleAssets}
                  selectedAsset={selectedAsset}
                  onSelectAsset={setSelectedAsset}
                  staleAssetIds={healthSets.staleAssetIds}
                  duplicateAssetIds={healthSets.duplicateAssetIds}
                  secretAssetIds={healthSets.secretAssetIds}
                />
              </CardContent>
            </Card>
            <Card className="flex flex-1 flex-col overflow-hidden shadow-xs">
              <CardContent className="flex-1 overflow-auto p-0">
                <MemoryAssetDetail
                  asset={selectedAsset}
                  projectPath={projectPath}
                  isStale={selectedAsset ? healthSets.staleAssetIds.has(selectedAsset.id) : false}
                  staleDays={selectedAsset ? healthSets.staleDaysByAssetId.get(selectedAsset.id) : undefined}
                  isDuplicate={selectedAsset ? healthSets.duplicateAssetIds.has(selectedAsset.id) : false}
                  isSecretRisk={selectedAsset ? healthSets.secretAssetIds.has(selectedAsset.id) : false}
                  duplicateGroupsForAsset={selectedAsset ? healthSets.duplicateGroupsByAssetId.get(selectedAsset.id) : undefined}
                  assetsById={assetsById}
                  onSelectAsset={setSelectedAsset}
                />
              </CardContent>
            </Card>
          </div>

          {/* Health Details */}
          {healthReport && (
            <details className="group rounded-xl border border-border bg-card shadow-xs">
              <summary className="flex cursor-pointer items-center gap-2 p-4 text-sm font-medium select-none">
                <span className="text-muted-foreground">健康诊断详情</span>
                <span className="text-xs text-muted-foreground">（启发式评估，非绝对质量分）</span>
                <span className="ml-auto text-xs text-muted-foreground group-open:rotate-180 transition-transform">▼</span>
              </summary>
              <div className="space-y-4 border-t border-border p-4">
                <div className="grid gap-3 sm:grid-cols-5">
                  {([healthReport.freshness, healthReport.quality, healthReport.coverage, healthReport.cleanliness, healthReport.safety] as const).map((dim) => (
                    <div key={dim.name} className="space-y-1">
                      <div className="flex items-center justify-between text-xs">
                        <span className="text-muted-foreground capitalize">{dim.name}</span>
                        <span className={`font-semibold ${dim.score >= 80 ? "text-green-600" : dim.score >= 60 ? "text-amber-600" : "text-red-600"}`}>{dim.score}</span>
                      </div>
                      <div className="h-2 rounded-full bg-muted overflow-hidden">
                        <div
                          className={`h-full rounded-full ${dim.score >= 80 ? "bg-green-500" : dim.score >= 60 ? "bg-amber-500" : "bg-red-500"}`}
                          style={{ width: `${dim.score}%` }}
                        />
                      </div>
                      <p className="text-xs text-muted-foreground truncate" title={dim.reason}>{dim.reason}</p>
                    </div>
                  ))}
                </div>
                {healthReport.top_issues.length > 0 && (
                  <div className="space-y-2">
                    <p className="text-xs font-medium text-muted-foreground">主要问题</p>
                    {healthReport.top_issues.map((issue, i) => {
                      const locatableCount = overview
                        ? issue.asset_ids.filter((id) =>
                            overview.assets.some((a) => a.exists && a.id === id),
                          ).length
                        : 0;
                      const canLocate = locatableCount > 0;
                      return (
                        <div
                          key={i}
                          className={cn(
                            "flex items-start gap-2 rounded-lg border p-2 text-xs",
                            issue.severity === "critical"
                              ? "border-red-200 bg-red-50 dark:border-red-900/50 dark:bg-red-950/20"
                              : issue.severity === "warning"
                                ? "border-amber-200 bg-amber-50 dark:border-amber-900/50 dark:bg-amber-950/20"
                                : "border-border bg-muted/30",
                            canLocate && "cursor-pointer hover:brightness-95",
                          )}
                          role={canLocate ? "button" : undefined}
                          tabIndex={canLocate ? 0 : undefined}
                          onClick={canLocate ? () => selectFirstIssueAsset(issue.asset_ids) : undefined}
                          onKeyDown={
                            canLocate
                              ? (e) => {
                                  if (e.key === "Enter" || e.key === " ") {
                                    e.preventDefault();
                                    selectFirstIssueAsset(issue.asset_ids);
                                  }
                                }
                              : undefined
                          }
                        >
                          <span className={`shrink-0 font-medium ${issue.severity === "critical" ? "text-red-600" : issue.severity === "warning" ? "text-amber-600" : "text-muted-foreground"}`}>{issue.severity}</span>
                          <div className="min-w-0 flex-1">
                            <p className="truncate" title={issue.message}>{issue.message}</p>
                            <p className="text-muted-foreground truncate" title={issue.suggestion}>→ {issue.suggestion}</p>
                          </div>
                          {issue.asset_ids.length > 0 && (
                            <span className="shrink-0 text-muted-foreground">{locatableCount} 个资产</span>
                          )}
                        </div>
                      );
                    })}
                  </div>
                )}
              </div>
            </details>
          )}
        </div>
      )}
    </section>
  );
}

function ContextPressureBanner({
  pressure,
  onDismiss,
  onSelectAsset,
}: {
  pressure: ContextPressure;
  onDismiss: () => void;
  onSelectAsset: (assetId: string) => void;
}) {
  const isCritical = pressure.level === "critical";
  const borderClass = isCritical
    ? "border-red-200 bg-red-50 dark:border-red-900/50 dark:bg-red-950/20"
    : "border-amber-200 bg-amber-50 dark:border-amber-900/50 dark:bg-amber-950/20";
  const textClass = isCritical
    ? "text-red-800 dark:text-red-400"
    : "text-amber-800 dark:text-amber-400";
  const iconClass = isCritical
    ? "text-red-600 dark:text-red-400"
    : "text-amber-600 dark:text-amber-400";

  return (
    <div className={`rounded-lg border px-4 py-3 ${borderClass}`}>
      <div className="flex items-start gap-3">
        <Zap className={`mt-0.5 size-4 shrink-0 ${iconClass}`} aria-hidden="true" />
        <div className="min-w-0 flex-1 space-y-2">
          <div className="flex items-center gap-2">
            <span className={`text-sm font-semibold ${textClass}`}>
              上下文压力 {pressure.level === "critical" ? "过高" : "偏高"}
            </span>
            <span className="text-xs text-muted-foreground">
              {pressure.estimated_tokens >= 1000
                ? `${(pressure.estimated_tokens / 1000).toFixed(0)}K`
                : pressure.estimated_tokens}{" "}
              tokens / 200K
            </span>
          </div>
          {pressure.alerts.length > 0 && (
            <div className="space-y-1">
              {pressure.alerts.map((alert, i) => (
                <p key={i} className="text-xs text-muted-foreground">
                  {alert.message}
                </p>
              ))}
            </div>
          )}
          {pressure.heavy_assets.length > 0 && (
            <div className="flex flex-wrap gap-1.5">
              {pressure.heavy_assets.slice(0, 3).map((asset) => (
                <button
                  key={asset.asset_id}
                  type="button"
                  onClick={() => onSelectAsset(asset.asset_id)}
                  className="inline-flex max-w-[200px] items-center gap-1 rounded bg-background/60 px-2 py-0.5 text-xs text-muted-foreground hover:bg-background"
                  title={asset.logical_path}
                >
                  <span className="truncate">{asset.logical_path}</span>
                </button>
              ))}
              {pressure.heavy_assets.length > 3 && (
                <span className="text-xs text-muted-foreground">
                  +{pressure.heavy_assets.length - 3}
                </span>
              )}
            </div>
          )}
        </div>
        <button
          type="button"
          onClick={onDismiss}
          className="shrink-0 rounded p-1 text-muted-foreground hover:bg-background/60"
          aria-label="关闭提示"
        >
          <X className="size-3.5" aria-hidden="true" />
        </button>
      </div>
    </div>
  );
}

function StatCard({
  icon: Icon,
  label,
  value,
  tone = "default",
  isText = false,
}: {
  icon: typeof Brain;
  label: string;
  value: string | number;
  tone?: "default" | "warning" | "danger";
  isText?: boolean;
}) {
  const toneClass =
    tone === "danger"
      ? "border-red-200 bg-red-50 dark:border-red-900/50 dark:bg-red-950/20"
      : tone === "warning"
        ? "border-amber-200 bg-amber-50 dark:border-amber-900/50 dark:bg-amber-950/20"
        : "border-border bg-card";
  const iconBorderClass =
    tone === "danger"
      ? "border-red-200 bg-red-100/70 dark:border-red-900/60 dark:bg-red-950/40"
      : tone === "warning"
        ? "border-amber-200 bg-amber-100/70 dark:border-amber-900/60 dark:bg-amber-950/40"
        : "border-border bg-tile";
  const iconClass =
    tone === "danger"
      ? "text-red-600 dark:text-red-400"
      : tone === "warning"
        ? "text-amber-600 dark:text-amber-400"
        : "text-muted-foreground";
  const valueClass =
    tone === "danger"
      ? "text-red-700 dark:text-red-400"
      : tone === "warning"
        ? "text-amber-700 dark:text-amber-400"
        : "";

  return (
    <div
      className={`rounded-xl border p-4 shadow-xs ${toneClass}`}
    >
      <div className="flex min-h-20 items-start justify-between gap-3">
        <div className="min-w-0">
          <span className="text-xs text-muted-foreground">{label}</span>
          <p
            data-stat={label}
            className={`mt-2 truncate font-semibold tracking-tight ${isText ? "font-mono text-xs" : "text-2xl"} ${valueClass}`}
          >
            {value}
          </p>
        </div>
        <span
          className={`flex size-8 shrink-0 items-center justify-center rounded-md border ${iconBorderClass}`}
        >
          <Icon
            className={`size-4 ${iconClass}`}
            aria-hidden="true"
          />
        </span>
      </div>
    </div>
  );
}
