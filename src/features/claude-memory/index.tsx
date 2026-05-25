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
} from "lucide-react";

import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Switch } from "@/components/ui/switch";

import type { ClaudeMemoryAsset } from "./types";
import { useMemoryHealth } from "./hooks/useClaudeMemory";

import { LoadChainSimulator } from "./components/LoadChainSimulator";
import { MemoryAssetDetail } from "./components/MemoryAssetDetail";
import { MemoryAssetTree } from "./components/MemoryAssetTree";
import { useClaudeMemory } from "./hooks/useClaudeMemory";

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
  const { overview, isLoading, error, refresh } = useClaudeMemory(projectPath);
  const { report: healthReport } = useMemoryHealth(projectPath);
  const [selectedAsset, setSelectedAsset] = useState<ClaudeMemoryAsset | null>(
    null,
  );
  const [hideMissing, setHideMissing] = useState(true);

  const visibleAssets = useMemo(() => {
    if (!overview) return [];
    return hideMissing
      ? overview.assets.filter((a) => a.exists)
      : overview.assets;
  }, [overview, hideMissing]);

  // 切换隐藏不存在时，如果当前选中项被隐藏则清除选中
  useEffect(() => {
    if (selectedAsset && !visibleAssets.some((a) => a.id === selectedAsset.id)) {
      setSelectedAsset(null);
    }
  }, [visibleAssets, selectedAsset]);

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
          disabled={isLoading}
          onClick={() => refresh(true)}
        >
          {isLoading ? (
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
                <MemoryAssetTree
                  assets={visibleAssets}
                  selectedAsset={selectedAsset}
                  onSelectAsset={setSelectedAsset}
                />
              </CardContent>
            </Card>
            <Card className="flex flex-1 flex-col overflow-hidden shadow-xs">
              <CardContent className="flex-1 overflow-auto p-0">
                <MemoryAssetDetail
                  asset={selectedAsset}
                  projectPath={projectPath}
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
                    {healthReport.top_issues.map((issue, i) => (
                      <div key={i} className={`flex items-start gap-2 rounded-lg border p-2 text-xs ${issue.severity === "critical" ? "border-red-200 bg-red-50 dark:border-red-900/50 dark:bg-red-950/20" : issue.severity === "warning" ? "border-amber-200 bg-amber-50 dark:border-amber-900/50 dark:bg-amber-950/20" : "border-border bg-muted/30"}`}>
                        <span className={`shrink-0 font-medium ${issue.severity === "critical" ? "text-red-600" : issue.severity === "warning" ? "text-amber-600" : "text-muted-foreground"}`}>{issue.severity}</span>
                        <div className="min-w-0">
                          <p className="truncate" title={issue.message}>{issue.message}</p>
                          <p className="text-muted-foreground truncate" title={issue.suggestion}>→ {issue.suggestion}</p>
                        </div>
                      </div>
                    ))}
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
