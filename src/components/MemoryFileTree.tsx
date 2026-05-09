import { useMemo, useState } from "react";
import { ChevronDown, ChevronRight, FileText, FolderTree, AlertTriangle } from "lucide-react";

import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "@/components/ui/collapsible";
import { cn } from "@/lib/utils";

const SOURCE_GROUP_ORDER = ["root", "rules", "notepads", "plans", "drafts", "docs"] as const;

type KnownSourceGroup = (typeof SOURCE_GROUP_ORDER)[number];

export interface SerProjectFile {
  relative_path: string;
  source_group: KnownSourceGroup | string;
  origin?: string;
  content_preview?: string;
  content_truncated?: boolean;
  mtime_ms?: number;
}

type OriginFilter = "all" | "template" | "project";

interface MemoryFileTreeProps {
  files: SerProjectFile[];
  selectedPath: string | null;
  onSelect: (path: string) => void;
  changedPaths: Set<string>;
}

interface GroupedFiles {
  group: KnownSourceGroup;
  label: string;
  files: SerProjectFile[];
}

const SOURCE_GROUP_LABELS: Record<KnownSourceGroup, string> = {
  root: "根文件",
  rules: "规则",
  notepads: "笔记",
  plans: "计划",
  drafts: "草稿",
  docs: "文档",
};

export function MemoryFileTree({ files, selectedPath, onSelect, changedPaths }: MemoryFileTreeProps) {
  const [originFilter, setOriginFilter] = useState<OriginFilter>("all");
  const [openGroups, setOpenGroups] = useState<Record<KnownSourceGroup, boolean>>(() => ({
    root: true,
    rules: true,
    notepads: true,
    plans: true,
    drafts: true,
    docs: true,
  }));

  const groupedFiles = useMemo<GroupedFiles[]>(() => {
    const visibleFiles = files.filter((file) => isVisibleByOrigin(file, originFilter));

    return SOURCE_GROUP_ORDER.map((group) => ({
      group,
      label: SOURCE_GROUP_LABELS[group],
      files: visibleFiles
        .filter((file) => file.source_group === group)
        .sort((left, right) => left.relative_path.localeCompare(right.relative_path, "zh-CN")),
    })).filter((group) => group.files.length > 0);
  }, [files, originFilter]);

  if (files.length === 0) {
    return (
      <section className="rounded-xl border border-dashed border-border bg-card/60 p-5 text-center text-sm text-muted-foreground" aria-label="记忆文件树空状态">
        此项目未找到记忆文件
      </section>
    );
  }

  // 检测是否所有文件的来源都是 "unknown"（未配置模板路径）
  const allUnknown = files.length > 0 && files.every((f) => f.origin === "unknown");

  return (
    <nav className="rounded-xl border border-border bg-card/70 p-2 text-card-foreground shadow-sm" aria-label="记忆文件导航">
      {allUnknown && (
        <div className="mb-2 flex items-center gap-2 rounded-lg border border-amber-200 bg-amber-50 px-2.5 py-2 text-xs text-amber-800">
          <AlertTriangle className="size-4 shrink-0" aria-hidden="true" />
          <span>未配置模板项目路径，无法区分文件来源。请在「设置 → 模板项目路径」中进行配置。</span>
        </div>
      )}
      <div className="mb-2 flex items-center justify-between gap-3 rounded-lg border border-border/70 bg-muted/30 px-2.5 py-2">
        <label htmlFor="memory-origin-filter" className="text-xs font-medium text-muted-foreground">
          来源筛选
        </label>
        <select
          id="memory-origin-filter"
          value={originFilter}
          onChange={(event) => setOriginFilter(event.target.value as OriginFilter)}
          className="h-8 rounded-md border border-border bg-background px-2 text-xs text-foreground shadow-sm outline-none transition-colors hover:bg-muted focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50"
        >
          <option value="all">全部</option>
          <option value="template">模板记忆</option>
          <option value="project">项目记忆</option>
        </select>
      </div>
      <div className="space-y-1" role="tree">
        {groupedFiles.map((group) => (
          <Collapsible
            key={group.group}
            open={openGroups[group.group]}
            onOpenChange={(open) => setOpenGroups((current) => ({ ...current, [group.group]: open }))}
          >
            <CollapsibleTrigger className="flex h-9 w-full items-center justify-between rounded-lg px-2.5 text-left text-sm font-medium transition-colors outline-none hover:bg-muted focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50">
              <span className="flex min-w-0 items-center gap-2">
                {openGroups[group.group] ? (
                  <ChevronDown className="size-4 shrink-0 text-muted-foreground" aria-hidden="true" />
                ) : (
                  <ChevronRight className="size-4 shrink-0 text-muted-foreground" aria-hidden="true" />
                )}
                <FolderTree className="size-4 shrink-0 text-muted-foreground" aria-hidden="true" />
                <span className="truncate">{group.label}</span>
              </span>
              <span className="shrink-0 rounded-full bg-muted px-2 py-0.5 text-xs text-muted-foreground">{group.files.length}</span>
            </CollapsibleTrigger>

            <CollapsibleContent>
              <div className="space-y-1 border-l border-border/80 py-1 pl-3 ml-4">
                {group.files.map((file) => (
                  <MemoryFileTreeItem
                    key={file.relative_path}
                    file={file}
                    isSelected={selectedPath === file.relative_path}
                    isChanged={changedPaths.has(file.relative_path)}
                    onSelect={onSelect}
                  />
                ))}
              </div>
            </CollapsibleContent>
          </Collapsible>
        ))}
      </div>
    </nav>
  );
}

interface MemoryFileTreeItemProps {
  file: SerProjectFile;
  isSelected: boolean;
  isChanged: boolean;
  onSelect: (path: string) => void;
}

function MemoryFileTreeItem({ file, isSelected, isChanged, onSelect }: MemoryFileTreeItemProps) {
  const originMeta = getOriginMeta(file.origin);

  return (
    <button
      type="button"
      className={cn(
        "grid h-8 w-full grid-cols-[auto_minmax(0,1fr)_auto] items-center gap-2 rounded-md px-2 text-left text-sm transition-colors outline-none hover:bg-muted/70 focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50",
        isSelected && "bg-accent text-accent-foreground hover:bg-accent",
      )}
      role="treeitem"
      aria-selected={isSelected}
      title={file.relative_path}
      onClick={() => onSelect(file.relative_path)}
    >
      <FileText className="size-4 shrink-0 text-muted-foreground" aria-hidden="true" />
      <span className="flex min-w-0 items-center gap-1.5">
        <span className={cn("text-[10px] leading-none", originMeta.dotClass)} aria-hidden="true">
          ●
        </span>
        <span className="min-w-0 truncate font-mono text-xs">{getFilename(file.relative_path)}</span>
        <span className={cn("shrink-0 text-[10px]", originMeta.labelClass)}>{originMeta.label}</span>
      </span>
      {isChanged && <span className="size-2 rounded-full bg-destructive" title="文件已变更" aria-hidden="true" />}
    </button>
  );
}

function isVisibleByOrigin(file: SerProjectFile, filter: OriginFilter) {
  if (filter === "all") {
    return true;
  }

  if (filter === "template") {
    return file.origin === "template";
  }

  return file.origin === "project" || file.origin === "unknown";
}

function getOriginMeta(origin: string | undefined) {
  if (origin === "project") {
    return {
      label: "项目",
      dotClass: "text-blue-500",
      labelClass: "text-blue-500",
    };
  }

  if (origin === "template") {
    return {
      label: "模板",
      dotClass: "text-green-500",
      labelClass: "text-green-500",
    };
  }

  return {
    label: "未知",
    dotClass: "text-muted-foreground",
    labelClass: "text-muted-foreground",
  };
}

function getFilename(relativePath: string) {
  const normalizedPath = relativePath.replace(/\/+$/, "");

  if (!normalizedPath) {
    return "未知文件";
  }

  const parts = normalizedPath.split("/");
  return parts[parts.length - 1] || normalizedPath;
}
