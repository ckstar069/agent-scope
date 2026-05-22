import { useState } from "react";
import {
  AlertTriangle,
  ChevronDown,
  ChevronRight,
  EyeOff,
  FileText,
  FolderOpen,
  Loader2,
  Play,
  Route,
  ShieldAlert,
} from "lucide-react";

import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible";
import { Input } from "@/components/ui/input";
import { ScrollArea } from "@/components/ui/scroll-area";
import { cn } from "@/lib/utils";

import type {
  ExcludedAsset,
  LoadChainResult,
  LoadChainStep,
  LoadChainWarning,
  PathScopedRule,
} from "../types";

import { useLoadChain } from "../hooks/useLoadChain";

export function LoadChainSimulator() {
  const { result, isLoading, error, simulate } = useLoadChain();
  const [cwd, setCwd] = useState("");

  const handleSimulate = () => {
    const path = cwd.trim() || ".";
    simulate(path);
  };

  return (
    <section className="flex h-full flex-col gap-4">
      {/* 页面标题 */}
      <div className="flex flex-col gap-4 lg:flex-row lg:items-end lg:justify-between">
        <div className="space-y-2">
          <p className="text-sm font-medium text-muted-foreground">
            Claude Code
          </p>
          <h1 className="text-3xl font-semibold tracking-tight">
            加载链模拟器
          </h1>
          <p className="max-w-2xl text-sm text-muted-foreground">
            模拟 Claude Code 从指定目录启动时的记忆加载顺序，包括启动链（A
            区域）和路径作用域规则（B 区域）。
          </p>
        </div>
      </div>

      {/* 输入区 */}
      <Card>
        <CardContent className="flex items-center gap-3 pt-6">
          <FolderOpen
            className="size-4 shrink-0 text-muted-foreground"
            aria-hidden="true"
          />
          <Input
            placeholder="输入目录路径（留空使用当前目录）"
            value={cwd}
            onChange={(e) => setCwd(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") handleSimulate();
            }}
            className="flex-1"
          />
          <Button
            type="button"
            disabled={isLoading}
            onClick={handleSimulate}
          >
            {isLoading ? (
              <Loader2 className="mr-2 size-4 animate-spin" aria-hidden="true" />
            ) : (
              <Play className="mr-2 size-4" aria-hidden="true" />
            )}
            模拟加载
          </Button>
        </CardContent>
      </Card>

      {/* 错误 */}
      {error && (
        <div className="flex items-center gap-2 rounded-xl border border-dashed border-destructive/30 bg-destructive/5 p-4 text-sm text-destructive">
          <AlertTriangle className="size-4" aria-hidden="true" />
          {error}
        </div>
      )}

      {/* 结果 */}
      {result && (
        <div className="flex flex-1 flex-col gap-4 overflow-hidden">
          <HostProfileBar profile={result.host_profile} cwd={result.cwd} />

          {result.warnings.length > 0 && (
            <WarningsPanel warnings={result.warnings} />
          )}

          <div className="grid flex-1 gap-4 lg:grid-cols-2">
            {/* A 区域：启动链 */}
            <Card className="flex flex-col overflow-hidden">
              <CardHeader className="py-3">
                <CardTitle className="flex items-center gap-2 text-sm font-medium">
                  <Route className="size-4 text-primary" aria-hidden="true" />
                  A 区域：启动链
                  <span className="ml-auto text-xs text-muted-foreground">
                    {result.startup_chain.length} 步
                  </span>
                </CardTitle>
              </CardHeader>
              <CardContent className="flex-1 p-0">
                <ScrollArea className="h-full">
                  <div className="space-y-1 p-4 pt-0">
                    {result.startup_chain.length === 0 && (
                      <p className="py-8 text-center text-sm text-muted-foreground">
                        无启动链步骤
                      </p>
                    )}
                    {result.startup_chain.map((step) => (
                      <StepCard key={step.order} step={step} />
                    ))}
                  </div>
                </ScrollArea>
              </CardContent>
            </Card>

            {/* B 区域：路径作用域规则 */}
            <Card className="flex flex-col overflow-hidden">
              <CardHeader className="py-3">
                <CardTitle className="flex items-center gap-2 text-sm font-medium">
                  <ShieldAlert
                    className="size-4 text-amber-500"
                    aria-hidden="true"
                  />
                  B 区域：路径作用域规则
                  <span className="ml-auto text-xs text-muted-foreground">
                    {result.path_scoped_rules.length} 条
                  </span>
                </CardTitle>
              </CardHeader>
              <CardContent className="flex-1 p-0">
                <ScrollArea className="h-full">
                  <div className="space-y-1 p-4 pt-0">
                    {result.path_scoped_rules.length === 0 && (
                      <p className="py-8 text-center text-sm text-muted-foreground">
                        无路径作用域规则
                      </p>
                    )}
                    {result.path_scoped_rules.map((rule, i) => (
                      <RuleCard key={i} rule={rule} />
                    ))}
                  </div>
                </ScrollArea>
              </CardContent>
            </Card>
          </div>

          {/* 被排除资产 */}
          {result.excluded_assets.length > 0 && (
            <ExcludedAssetsPanel assets={result.excluded_assets} />
          )}
        </div>
      )}
    </section>
  );
}

/* ─── Host Profile Bar ─── */
function HostProfileBar({
  profile,
  cwd,
}: {
  profile: LoadChainResult["host_profile"];
  cwd: string;
}) {
  return (
    <div className="flex flex-wrap items-center gap-x-4 gap-y-1 rounded-lg border bg-muted/50 px-4 py-2 text-xs text-muted-foreground">
      <span>
        <span className="font-medium">OS:</span> {profile.os}
      </span>
      <span>
        <span className="font-medium">主机:</span> {profile.hostname}
      </span>
      <span>
        <span className="font-medium">用户:</span> {profile.user_name}
      </span>
      <span className="truncate">
        <span className="font-medium">CWD:</span> {cwd}
      </span>
    </div>
  );
}

/* ─── Warnings Panel ─── */
function WarningsPanel({ warnings }: { warnings: LoadChainWarning[] }) {
  return (
    <div className="space-y-2">
      {warnings.map((w, i) => (
        <div
          key={i}
          className={cn(
            "flex items-start gap-2 rounded-lg border p-3 text-sm",
            w.level === "warning"
              ? "border-amber-200 bg-amber-50 text-amber-800 dark:border-amber-900/50 dark:bg-amber-950/20 dark:text-amber-400"
              : "border-blue-200 bg-blue-50 text-blue-800 dark:border-blue-900/50 dark:bg-blue-950/20 dark:text-blue-400",
          )}
        >
          <AlertTriangle
            className={cn(
              "mt-0.5 size-4 shrink-0",
              w.level === "warning"
                ? "text-amber-600 dark:text-amber-400"
                : "text-blue-600 dark:text-blue-400",
            )}
            aria-hidden="true"
          />
          <div>
            <p className="font-medium">[{w.code}]</p>
            <p
              className={cn(
                w.level === "warning"
                  ? "text-amber-700 dark:text-amber-500"
                  : "text-blue-700 dark:text-blue-500",
              )}
            >
              {w.message}
            </p>
          </div>
        </div>
      ))}
    </div>
  );
}

/* ─── Step Card ─── */
function StepCard({ step }: { step: LoadChainStep }) {
  const [open, setOpen] = useState(false);

  const scopeColor =
    {
      managed: "text-purple-600 dark:text-purple-400",
      user: "text-blue-600 dark:text-blue-400",
      project: "text-emerald-600 dark:text-emerald-400",
      local: "text-orange-600 dark:text-orange-400",
      auto: "text-pink-600 dark:text-pink-400",
    }[step.scope] ?? "text-muted-foreground";

  return (
    <Collapsible open={open} onOpenChange={setOpen}>
      <div className="flex items-center gap-2 rounded-md border px-3 py-2 hover:bg-muted/50">
        <span className="w-6 text-right text-xs text-muted-foreground">
          {step.order}
        </span>
        <FileText className="size-3.5 shrink-0 text-muted-foreground" />
        <div className="min-w-0 flex-1">
          <p className="truncate text-sm font-medium">{step.load_reason}</p>
          <p className="truncate text-xs text-muted-foreground">
            {step.logical_path}
          </p>
        </div>
        <span className={cn("shrink-0 text-xs font-medium", scopeColor)}>
          {step.scope}
        </span>
        {step.content_preview && (
          <CollapsibleTrigger>
            <Button variant="ghost" size="icon" className="size-7">
              {open ? (
                <ChevronDown className="size-3.5" />
              ) : (
                <ChevronRight className="size-3.5" />
              )}
            </Button>
          </CollapsibleTrigger>
        )}
      </div>
      {step.content_preview && (
        <CollapsibleContent>
          <pre className="mx-2 mb-2 mt-0.5 max-h-48 overflow-auto rounded-b-md border-x border-b bg-muted/30 p-3 text-xs">
            <code>{step.content_preview}</code>
          </pre>
        </CollapsibleContent>
      )}
    </Collapsible>
  );
}

/* ─── Rule Card ─── */
function RuleCard({ rule }: { rule: PathScopedRule }) {
  const [open, setOpen] = useState(false);

  return (
    <Collapsible open={open} onOpenChange={setOpen}>
      <div className="flex items-center gap-2 rounded-md border px-3 py-2 hover:bg-muted/50">
        <ShieldAlert
          className="size-3.5 shrink-0 text-amber-500"
          aria-hidden="true"
        />
        <div className="min-w-0 flex-1">
          <p className="truncate text-sm font-medium">
            {rule.name ?? "未命名规则"}
          </p>
          <p className="truncate text-xs text-muted-foreground">
            {rule.logical_path}
          </p>
        </div>
        <span
          className={cn(
            "shrink-0 text-xs font-medium",
            rule.scope === "user"
              ? "text-blue-600 dark:text-blue-400"
              : "text-emerald-600 dark:text-emerald-400",
          )}
        >
          {rule.scope}
        </span>
        <CollapsibleTrigger>
          <Button variant="ghost" size="icon" className="size-7">
            {open ? (
              <ChevronDown className="size-3.5" />
            ) : (
              <ChevronRight className="size-3.5" />
            )}
          </Button>
        </CollapsibleTrigger>
      </div>
      <CollapsibleContent>
        <div className="mx-2 mb-2 mt-0.5 space-y-1 rounded-b-md border-x border-b bg-muted/30 p-3">
          <p className="text-xs font-medium text-muted-foreground">paths:</p>
          {rule.paths.map((p, i) => (
            <code key={i} className="block text-xs">
              {p}
            </code>
          ))}
        </div>
      </CollapsibleContent>
    </Collapsible>
  );
}

/* ─── Excluded Assets Panel ─── */
function ExcludedAssetsPanel({ assets }: { assets: ExcludedAsset[] }) {
  const [open, setOpen] = useState(true);

  return (
    <Collapsible open={open} onOpenChange={setOpen}>
      <Card>
        <CardHeader className="py-3">
          <CollapsibleTrigger>
            <CardTitle className="flex cursor-pointer items-center gap-2 text-sm font-medium">
              <EyeOff
                className="size-4 text-destructive"
                aria-hidden="true"
              />
              被排除资产
              <span className="ml-auto text-xs text-muted-foreground">
                {assets.length} 项
              </span>
              {open ? (
                <ChevronDown className="size-3.5" />
              ) : (
                <ChevronRight className="size-3.5" />
              )}
            </CardTitle>
          </CollapsibleTrigger>
        </CardHeader>
        <CollapsibleContent>
          <CardContent className="p-0">
            <ScrollArea className="max-h-64">
              <div className="space-y-1 p-4 pt-0">
                {assets.map((asset, i) => (
                  <div
                    key={i}
                    className="flex items-center gap-2 rounded-md border px-3 py-2"
                  >
                    <EyeOff
                      className="size-3.5 shrink-0 text-destructive"
                      aria-hidden="true"
                    />
                    <div className="min-w-0 flex-1">
                      <p className="truncate text-xs text-muted-foreground">
                        {asset.logical_path}
                      </p>
                    </div>
                    <span className="shrink-0 text-xs text-muted-foreground">
                      {asset.excluded_by}
                    </span>
                    <code className="shrink-0 text-xs">{asset.pattern}</code>
                  </div>
                ))}
              </div>
            </ScrollArea>
          </CardContent>
        </CollapsibleContent>
      </Card>
    </Collapsible>
  );
}
