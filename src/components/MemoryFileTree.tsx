import { useMemo, useState } from "react";
import { ChevronDown, ChevronRight, FileText, FolderTree } from "lucide-react";

import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "@/components/ui/collapsible";
import { cn } from "@/lib/utils";

const SOURCE_GROUP_ORDER = ["root", "rules", "notepads", "plans", "drafts", "docs"] as const;

type KnownSourceGroup = (typeof SOURCE_GROUP_ORDER)[number];

export interface SerProjectFile {
  relative_path: string;
  source_group: KnownSourceGroup | string;
  content_preview?: string;
  content_truncated?: boolean;
  mtime_ms?: number;
}

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
  const [openGroups, setOpenGroups] = useState<Record<KnownSourceGroup, boolean>>(() => ({
    root: true,
    rules: true,
    notepads: true,
    plans: true,
    drafts: true,
    docs: true,
  }));

  const groupedFiles = useMemo<GroupedFiles[]>(() => {
    return SOURCE_GROUP_ORDER.map((group) => ({
      group,
      label: SOURCE_GROUP_LABELS[group],
      files: files
        .filter((file) => file.source_group === group)
        .sort((left, right) => left.relative_path.localeCompare(right.relative_path, "zh-CN")),
    })).filter((group) => group.files.length > 0);
  }, [files]);

  if (files.length === 0) {
    return (
      <section className="rounded-xl border border-dashed border-border bg-card/60 p-5 text-center text-sm text-muted-foreground" aria-label="记忆文件树空状态">
        此项目未找到记忆文件
      </section>
    );
  }

  return (
    <nav className="rounded-xl border border-border bg-card/70 p-2 text-card-foreground shadow-sm" aria-label="记忆文件导航">
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
      <span className="min-w-0 truncate font-mono text-xs">{getFilename(file.relative_path)}</span>
      {isChanged && <span className="size-2 rounded-full bg-destructive" title="文件已变更" aria-hidden="true" />}
    </button>
  );
}

function getFilename(relativePath: string) {
  const normalizedPath = relativePath.replace(/\/+$/, "");

  if (!normalizedPath) {
    return "未知文件";
  }

  const parts = normalizedPath.split("/");
  return parts[parts.length - 1] || normalizedPath;
}
