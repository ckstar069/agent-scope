import { AlertTriangle, FileText, FileWarning, FolderOpen } from "lucide-react";

import { ScrollArea } from "@/components/ui/scroll-area";
import { cn } from "@/lib/utils";

import type { AssetGroup, ClaudeMemoryAsset, GroupedAssets } from "../types";

import { IssueBadge } from "./IssueBadge";

interface MemoryAssetTreeProps {
  assets: ClaudeMemoryAsset[];
  selectedAsset: ClaudeMemoryAsset | null;
  onSelectAsset: (asset: ClaudeMemoryAsset) => void;
  staleAssetIds?: Set<string>;
  duplicateAssetIds?: Set<string>;
}

const GROUP_ORDER: AssetGroup[] = [
  "instruction",
  "rules",
  "auto_memory",
  "skills_agents",
];

const GROUP_LABELS: Record<AssetGroup, string> = {
  instruction: "Instruction",
  rules: "Rules",
  auto_memory: "Auto Memory",
  skills_agents: "Skills & Agents",
};

function getAssetGroup(assetType: string): AssetGroup {
  switch (assetType) {
    case "user_claude_md":
    case "project_claude_md":
    case "project_dot_claude_md":
    case "local_md":
      return "instruction";
    case "global_rule":
    case "project_rule":
      return "rules";
    case "auto_memory_index":
    case "auto_memory_topic":
      return "auto_memory";
    case "global_skill":
    case "project_skill":
    case "global_agent":
    case "project_agent":
      return "skills_agents";
    default:
      return "instruction";
  }
}

function groupAssets(assets: ClaudeMemoryAsset[]): GroupedAssets[] {
  const map = new Map<AssetGroup, ClaudeMemoryAsset[]>();

  for (const asset of assets) {
    const group = getAssetGroup(asset.asset_type);
    const list = map.get(group) ?? [];
    list.push(asset);
    map.set(group, list);
  }

  return GROUP_ORDER.map((group) => ({
    group,
    label: GROUP_LABELS[group],
    assets: map.get(group) ?? [],
  })).filter((g) => g.assets.length > 0);
}

function formatFileSize(bytes: number | null): string {
  if (bytes == null) return "—";
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function getFileName(path: string): string {
  return path.split(/[\\/]/).pop() ?? path;
}

function getAssetContext(asset: ClaudeMemoryAsset): string {
  if (asset.scope === "user") return "user";
  if (asset.scope === "auto") {
    // /home/user/.claude/projects/p1/memory/MEMORY.md -> p1
    const match = asset.logical_path.match(/\.claude\/projects\/([^/]+)/);
    return match ? match[1] : "auto";
  }
  // project / local:
  // 如果路径包含 /.claude/，取 /.claude/ 前一个目录作为项目名
  const claudeIdx = asset.logical_path.indexOf("/.claude/");
  if (claudeIdx > 0) {
    const prefix = asset.logical_path.slice(0, claudeIdx);
    const segments = prefix.split(/[\\/]/).filter(Boolean);
    const projectName = segments[segments.length - 1];
    if (projectName) return projectName;
  }
  // 否则取文件父目录名
  const segments = asset.logical_path.split(/[\\/]/).filter(Boolean);
  if (segments.length >= 2) {
    return segments[segments.length - 2];
  }
  return asset.scope;
}

export function MemoryAssetTree({
  assets,
  selectedAsset,
  onSelectAsset,
  staleAssetIds,
  duplicateAssetIds,
}: MemoryAssetTreeProps) {
  const grouped = groupAssets(assets);

  return (
    <ScrollArea className="h-full">
      <div className="space-y-4 p-3">
        {grouped.map(({ group, label, assets: groupAssets }) => (
          <div key={group}>
            <h3 className="mb-2 flex items-center gap-2 px-1 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
              <FolderOpen className="size-3.5" aria-hidden="true" />
              {label}
              <span className="ml-auto rounded-full bg-muted px-1.5 py-0 text-[10px] tabular-nums text-muted-foreground">
                {groupAssets.length}
              </span>
            </h3>
            <div className="space-y-0.5">
              {groupAssets.map((asset) => {
                const isSelected = selectedAsset?.id === asset.id;
                return (
                  <button
                    key={asset.id}
                    type="button"
                    onClick={() => onSelectAsset(asset)}
                    className={cn(
                      "flex w-full flex-col gap-1 rounded-lg px-2.5 py-2 text-left transition-colors",
                      isSelected
                        ? "bg-primary/10 text-primary"
                        : "hover:bg-muted/60 text-foreground/80",
                    )}
                  >
                    <div className="flex items-center gap-2">
                      {asset.exists ? (
                        <FileText
                          className={cn(
                            "size-3.5 shrink-0",
                            isSelected ? "text-primary" : "text-muted-foreground",
                          )}
                          aria-hidden="true"
                        />
                      ) : (
                        <FileWarning
                          className={cn(
                            "size-3.5 shrink-0",
                            isSelected ? "text-primary" : "text-muted-foreground/50",
                          )}
                          aria-hidden="true"
                        />
                      )}
                      <span className="min-w-0 flex-1 truncate text-sm font-medium">
                        {getFileName(asset.logical_path)}
                      </span>
                      {asset.exists && (staleAssetIds?.has(asset.id) || duplicateAssetIds?.has(asset.id)) && (
                        <span className="flex shrink-0 items-center gap-1">
                          {staleAssetIds?.has(asset.id) && (
                            <span className="rounded bg-amber-100 px-1 text-[10px] font-medium text-amber-700 dark:bg-amber-900/40 dark:text-amber-400">
                              stale
                            </span>
                          )}
                          {duplicateAssetIds?.has(asset.id) && (
                            <span className="rounded bg-blue-100 px-1 text-[10px] font-medium text-blue-700 dark:bg-blue-900/40 dark:text-blue-400">
                              dup
                            </span>
                          )}
                        </span>
                      )}
                    </div>
                    <div className="flex items-center gap-2 pl-5">
                      <span className="text-[10px] text-muted-foreground">
                        {getAssetContext(asset)}
                      </span>
                      {asset.line_count != null && (
                        <span className="text-[10px] text-muted-foreground">
                          {asset.line_count} 行
                        </span>
                      )}
                      {asset.byte_size != null && (
                        <span className="text-[10px] text-muted-foreground">
                          {formatFileSize(asset.byte_size)}
                        </span>
                      )}
                      {asset.content_truncated && (
                        <span className="rounded bg-amber-100 px-1 text-[10px] font-medium text-amber-700 dark:bg-amber-950/40 dark:text-amber-400">
                          截断
                        </span>
                      )}
                      {asset.asset_type === "auto_memory_index" &&
                        asset.line_count != null &&
                        asset.line_count > 200 && (
                          <span className="inline-flex items-center gap-0.5 rounded bg-red-100 px-1 text-[10px] font-medium text-red-700 dark:bg-red-950/40 dark:text-red-400">
                            <AlertTriangle className="size-2.5" aria-hidden="true" />
                            过长
                          </span>
                        )}
                      {!asset.exists && (
                        <span className="rounded bg-muted px-1 text-[10px] font-medium text-muted-foreground">
                          不存在
                        </span>
                      )}
                    </div>
                    {asset.secret_issues.length > 0 && (
                      <div className="pl-5">
                        <IssueBadge count={asset.secret_issues.length} />
                      </div>
                    )}
                  </button>
                );
              })}
            </div>
          </div>
        ))}
      </div>
    </ScrollArea>
  );
}
