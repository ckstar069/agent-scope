import { AlertTriangle, FileWarning, Loader2, Ruler, Scale } from "lucide-react";

import { MarkdownRenderer } from "@/components/MarkdownRenderer";

import type { ClaudeMemoryAsset, Frontmatter } from "../types";

import { useClaudeMemoryFile } from "../hooks/useClaudeMemory";

interface MemoryAssetDetailProps {
  asset: ClaudeMemoryAsset | null;
  projectPath?: string;
}

export function MemoryAssetDetail({ asset, projectPath }: MemoryAssetDetailProps) {
  const { content, isLoading, error } = useClaudeMemoryFile(asset, projectPath);

  if (!asset) {
    return (
      <div className="flex h-full items-center justify-center text-sm text-muted-foreground">
        点击左侧资产查看详情
      </div>
    );
  }

  return (
    <div className="flex h-full flex-col">
      {/* 元数据头部 —— 始终显示 */}
      <AssetMetaHeader asset={asset} />

      {/* 内容区域 */}
      {!asset.exists && (
        <div className="flex flex-1 flex-col items-center justify-center gap-3 p-8 text-center">
          <div className="flex size-12 items-center justify-center rounded-xl bg-muted text-muted-foreground">
            <FileWarning className="size-6" aria-hidden="true" />
          </div>
          <h3 className="text-lg font-semibold">文件不存在</h3>
          <p className="max-w-md text-sm text-muted-foreground">{asset.logical_path}</p>
        </div>
      )}

      {asset.exists && isLoading && (
        <div className="flex flex-1 items-center justify-center gap-2 text-sm text-muted-foreground">
          <Loader2 className="size-4 animate-spin" aria-hidden="true" />
          正在读取文件内容…
        </div>
      )}

      {asset.exists && !isLoading && error && (
        <div className="flex flex-1 flex-col gap-4 overflow-auto p-4">
          {/* 大文件预览 */}
          {asset.content_truncated && asset.content_preview != null && (
            <div className="rounded-lg border border-amber-200 bg-amber-50 p-3 dark:border-amber-900/50 dark:bg-amber-950/20">
              <p className="text-sm font-medium text-amber-700 dark:text-amber-400">
                文件过大，仅展示扫描预览
              </p>
              <pre className="mt-2 max-h-48 overflow-auto rounded bg-muted/50 p-2 text-xs">
                {asset.content_preview}
              </pre>
            </div>
          )}
          <div className="flex flex-col items-center justify-center gap-3 p-8 text-center">
            <div className="flex size-12 items-center justify-center rounded-xl bg-destructive/10 text-destructive">
              <AlertTriangle className="size-6" aria-hidden="true" />
            </div>
            <h3 className="text-lg font-semibold">读取失败</h3>
            <p className="max-w-md text-sm text-muted-foreground">{error}</p>
          </div>
        </div>
      )}

      {asset.exists && !isLoading && !error && content !== null && (
        <div className="flex-1 overflow-auto">
          <MarkdownRenderer content={content} />
        </div>
      )}

      {asset.exists && !isLoading && !error && content === null && (
        <div className="flex flex-1 items-center justify-center text-sm text-muted-foreground">
          无法加载内容
        </div>
      )}
    </div>
  );
}

function AssetMetaHeader({ asset }: { asset: ClaudeMemoryAsset }) {
  const isTooLong =
    asset.asset_type === "auto_memory_index" &&
    asset.line_count != null &&
    asset.line_count > 200;

  return (
    <div className="border-b border-border bg-muted/30 p-4">
      {/* 基本信息 */}
      <div className="mb-3 flex flex-wrap items-center gap-2">
        <span className="rounded bg-primary/10 px-2 py-0.5 text-xs font-medium text-primary">
          {asset.asset_type}
        </span>
        <span className="rounded bg-muted px-2 py-0.5 text-xs text-muted-foreground">
          {asset.scope}
        </span>
        {asset.line_count != null && (
          <span className="inline-flex items-center gap-1 rounded bg-muted px-2 py-0.5 text-xs text-muted-foreground">
            <Ruler className="size-3" aria-hidden="true" />
            {asset.line_count} 行
          </span>
        )}
        {asset.byte_size != null && (
          <span className="inline-flex items-center gap-1 rounded bg-muted px-2 py-0.5 text-xs text-muted-foreground">
            <Scale className="size-3" aria-hidden="true" />
            {formatFileSize(asset.byte_size)}
          </span>
        )}
        {asset.content_truncated && (
          <span className="rounded bg-amber-100 px-2 py-0.5 text-xs font-medium text-amber-700 dark:bg-amber-950/40 dark:text-amber-400">
            预览已截断
          </span>
        )}
        {isTooLong && (
          <span className="inline-flex items-center gap-1 rounded bg-red-100 px-2 py-0.5 text-xs font-medium text-red-700 dark:bg-red-950/40 dark:text-red-400">
            <AlertTriangle className="size-3" aria-hidden="true" />
            MEMORY.md 过长（{asset.line_count} 行）
          </span>
        )}
      </div>

      {/* 路径 */}
      <p className="mb-3 font-mono text-xs text-muted-foreground break-all">{asset.native_path}</p>

      {/* Frontmatter —— 对 rule 类型即使 null 也渲染加载模式 */}
      <FrontmatterMeta frontmatter={asset.frontmatter} assetType={asset.asset_type} />

      {/* Secret Issues */}
      {asset.secret_issues.length > 0 && <SecretIssuesList issues={asset.secret_issues} />}
    </div>
  );
}

function FrontmatterMeta({
  frontmatter,
  assetType,
}: {
  frontmatter: Frontmatter | null;
  assetType: string;
}) {
  const isRule = assetType.includes("rule");
  const isSkill = assetType.includes("skill");
  const isAgent = assetType.includes("agent");

  // 如果不是需要 frontmatter 的类型且 frontmatter 为 null，不渲染
  if (!frontmatter && !isRule) return null;

  const name = frontmatter?.name ?? "—";
  const description = frontmatter?.description ?? "未声明";
  const trigger = frontmatter?.trigger ?? "未声明";
  const memoryScope = frontmatter?.memory_scope ?? "未声明";

  return (
    <div className="space-y-1.5 text-xs">
      {(isSkill || isAgent || frontmatter?.name) && (
        <div className="flex gap-4">
          <span className="w-16 shrink-0 text-muted-foreground">名称</span>
          <span className="font-medium">{name}</span>
        </div>
      )}

      {(isSkill || isAgent) && (
        <div className="flex gap-4">
          <span className="w-16 shrink-0 text-muted-foreground">描述</span>
          <span className="text-muted-foreground">{description}</span>
        </div>
      )}

      {isSkill && (
        <div className="flex gap-4">
          <span className="w-16 shrink-0 text-muted-foreground">触发</span>
          <span className="text-muted-foreground">{trigger}</span>
        </div>
      )}

      {isAgent && (
        <div className="flex gap-4">
          <span className="w-16 shrink-0 text-muted-foreground">作用域</span>
          <span className="text-muted-foreground">{memoryScope}</span>
        </div>
      )}

      {isRule && (
        <div className="flex gap-4">
          <span className="w-16 shrink-0 text-muted-foreground">加载模式</span>
          {frontmatter?.paths != null && frontmatter.paths.length > 0 ? (
            <span className="inline-flex items-center gap-1 text-amber-700 dark:text-amber-400">
              <AlertTriangle className="size-3" aria-hidden="true" />
              路径触发：{frontmatter.paths.join(", ")}
            </span>
          ) : (
            <span className="text-emerald-700 dark:text-emerald-400">全局加载</span>
          )}
        </div>
      )}
    </div>
  );
}

function SecretIssuesList({ issues }: { issues: ClaudeMemoryAsset["secret_issues"] }) {
  return (
    <div className="mt-3 space-y-1.5">
      <p className="text-xs font-medium text-amber-700 dark:text-amber-400">敏感信息检测</p>
      {issues.map((issue, index) => (
        <div
          key={index}
          className="flex items-center gap-3 rounded bg-amber-50 px-2 py-1 text-xs dark:bg-amber-950/20"
        >
          <span className="rounded bg-amber-100 px-1.5 py-0 text-[10px] font-medium text-amber-700 dark:bg-amber-900/50 dark:text-amber-400">
            {issue.issue_type}
          </span>
          <span className="text-muted-foreground">
            第 {issue.line_number} 行，列 {issue.column_start}–{issue.column_end}
          </span>
          <span className="font-mono text-amber-700 dark:text-amber-400">{issue.matched_text}</span>
        </div>
      ))}
    </div>
  );
}

function formatFileSize(bytes: number | null): string {
  if (bytes == null) return "—";
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}
