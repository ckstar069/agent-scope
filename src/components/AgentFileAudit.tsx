import { FileText } from "lucide-react";

import { cn } from "@/lib/utils";

interface AgentFileAuditProps {
  file_accesses: { path: string; operation: string; turn_index: number }[];
}

interface FileAuditEntry {
  path: string;
  operation: string;
  turn_index: number;
}

const MAX_VISIBLE_ENTRIES = 50;

const operationStyles: Record<string, string> = {
  R: "bg-blue-100 text-blue-700 dark:bg-blue-500/15 dark:text-blue-300",
  W: "bg-green-100 text-green-700 dark:bg-green-500/15 dark:text-green-300",
  E: "bg-orange-100 text-orange-700 dark:bg-orange-500/15 dark:text-orange-300",
};

export function AgentFileAudit({ file_accesses }: AgentFileAuditProps) {
  if (file_accesses.length === 0) {
    return null;
  }

  const latestByPath = file_accesses.reduce<Map<string, FileAuditEntry>>((acc, access) => {
    const current = acc.get(access.path);

    if (!current || access.turn_index > current.turn_index) {
      acc.set(access.path, access);
    }

    return acc;
  }, new Map<string, FileAuditEntry>());

  const uniqueEntries = Array.from(latestByPath.values()).sort((left, right) => {
    if (left.turn_index !== right.turn_index) {
      return left.turn_index - right.turn_index;
    }

    return left.path.localeCompare(right.path, "zh-CN");
  });

  const visibleEntries = uniqueEntries.slice(-MAX_VISIBLE_ENTRIES);
  const hiddenCount = uniqueEntries.length - visibleEntries.length;

  return (
    <section className="rounded-lg border border-border bg-background/60 p-3" aria-label="文件审计日志">
      <div className="mb-3 flex flex-wrap items-center justify-between gap-2 border-b border-border pb-3">
        <div className="flex items-center gap-2 text-sm font-semibold tracking-tight">
          <span className="flex size-7 items-center justify-center rounded-lg bg-muted text-muted-foreground">
            <FileText className="size-3.5" aria-hidden="true" />
          </span>
          文件审计
        </div>
        <p className="text-xs text-muted-foreground">
          操作: {file_accesses.length} 次 | 文件: {latestByPath.size} 个（去重）
        </p>
      </div>

      <div className="max-h-[360px] space-y-1.5 overflow-auto">
        {hiddenCount > 0 && (
          <p className="rounded-md border border-dashed border-border bg-muted/30 px-2.5 py-1.5 text-xs text-muted-foreground">
            …及另外 {hiddenCount} 条
          </p>
        )}

        {visibleEntries.map((entry) => (
          <FileAuditRow key={`${entry.path}-${entry.turn_index}-${entry.operation}`} entry={entry} />
        ))}
      </div>
    </section>
  );
}

interface FileAuditRowProps {
  entry: FileAuditEntry;
}

function FileAuditRow({ entry }: FileAuditRowProps) {
  return (
    <div className="grid grid-cols-[auto_minmax(0,1fr)_auto] items-center gap-2 rounded-md border border-border/70 bg-muted/25 px-2.5 py-2 text-xs transition-colors hover:bg-muted/40">
      <span
        className={cn(
          "inline-flex h-5 min-w-5 items-center justify-center rounded px-1.5 font-mono text-[0.7rem] font-bold",
          operationStyles[entry.operation] ?? "bg-muted text-muted-foreground",
        )}
      >
        {entry.operation || "-"}
      </span>
      <span className="min-w-0 truncate font-mono text-foreground/90" title={entry.path}>
        {entry.path}
      </span>
      <span className="shrink-0 rounded-full bg-background px-2 py-0.5 font-mono text-[0.7rem] text-muted-foreground">
        turn {entry.turn_index}
      </span>
    </div>
  );
}
