import { useEffect, useRef, useState } from "react";
import { Circle, Download, Trash2, ChevronDown, ChevronUp, Wrench, FileCode, ArrowUpDown } from "lucide-react";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

import type { SessionTimelineProps, SessionPreview, ToolCallStat, FileReference } from "../types";

type PreviewOrder = "newest-first" | "oldest-first";

const ORDER_STORAGE_KEY = "claude-history-preview-order";

function formatDate(timestamp: number | null): string {
  if (!timestamp) return "未知时间";
  return new Date(timestamp).toLocaleString("zh-CN");
}

function ExportMenu({ sessionId, onExport }: { sessionId: string; onExport: (sessionId: string, format: "Jsonl" | "Markdown") => void }) {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    function handleClickOutside(event: MouseEvent) {
      if (ref.current && !ref.current.contains(event.target as Node)) {
        setOpen(false);
      }
    }
    if (open) {
      document.addEventListener("mousedown", handleClickOutside);
    }
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, [open]);

  return (
    <div className="relative" ref={ref}>
      <Button
        type="button"
        variant="ghost"
        size="icon"
        className="size-8 shrink-0 text-muted-foreground hover:text-primary"
        title="导出会话"
        onClick={() => setOpen(!open)}
      >
        <Download className="size-4" />
      </Button>
      {open && (
        <div className="absolute right-0 z-10 mt-1 w-32 rounded-md border bg-popover shadow-md">
          <button
            type="button"
            className="w-full px-3 py-2 text-left text-sm hover:bg-accent"
            onClick={(e) => {
              e.stopPropagation();
              onExport(sessionId, "Jsonl");
              setOpen(false);
            }}
          >
            导出为 JSONL
          </button>
          <button
            type="button"
            className="w-full px-3 py-2 text-left text-sm hover:bg-accent"
            onClick={(e) => {
              e.stopPropagation();
              onExport(sessionId, "Markdown");
              setOpen(false);
            }}
          >
            导出为 Markdown
          </button>
        </div>
      )}
    </div>
  );
}

function PreviewPanel({ preview }: { preview: SessionPreview }) {
  const [order, setOrder] = useState<PreviewOrder>(() => {
    const saved = localStorage.getItem(ORDER_STORAGE_KEY);
    return (saved as PreviewOrder) || "newest-first";
  });

  const roleLabel: Record<string, string> = {
    user: "用户",
    assistant: "助手",
    tool: "工具",
  };

  const roleClass: Record<string, string> = {
    user: "text-blue-600",
    assistant: "text-green-600",
    tool: "text-amber-600",
  };

  const toggleOrder = () => {
    const next = order === "newest-first" ? "oldest-first" : "newest-first";
    setOrder(next);
    localStorage.setItem(ORDER_STORAGE_KEY, next);
  };

  const displayMessages =
    order === "newest-first" ? [...preview.messages].reverse() : preview.messages;

  return (
    <div className="mt-3 rounded-xl border border-border bg-tile p-3">
      <div className="mb-2 flex items-center justify-between">
        <p className="text-xs text-muted-foreground">
          预览（共 {preview.total_turns} 轮对话）
        </p>
        <Button
          type="button"
          variant="ghost"
          size="sm"
          className="h-6 gap-1 px-2 text-xs text-muted-foreground hover:text-foreground"
          onClick={toggleOrder}
          title={order === "newest-first" ? "当前：最新在上" : "当前：最早在上"}
        >
          <ArrowUpDown className="size-3" />
          {order === "newest-first" ? "最新在上" : "最早在上"}
        </Button>
      </div>
      <div className="flex max-h-[600px] flex-col gap-4 overflow-y-auto pr-1">
        {displayMessages.map((msg, idx) => (
          <div key={idx} className="rounded-lg border border-transparent bg-card/70 p-2 text-sm hover:border-border hover:bg-card">
            <span
              className={cn(
                "font-medium",
                roleClass[msg.role] || "text-muted-foreground"
              )}
            >
              {roleLabel[msg.role] || msg.role}：
            </span>
            <span className="text-foreground">{msg.content}</span>
          </div>
        ))}
      </div>
    </div>
  );
}

export function SessionTimeline({ sessions, onDelete, onExport, onPreview, previewCache }: SessionTimelineProps) {
  const [expandedSession, setExpandedSession] = useState<string | null>(null);

  const handleTogglePreview = async (sessionId: string) => {
    if (expandedSession === sessionId) {
      setExpandedSession(null);
      return;
    }
    setExpandedSession(sessionId);
    if (!previewCache[sessionId]) {
      await onPreview(sessionId);
    }
  };

  return (
    <div className="flex flex-col gap-2">
      {sessions.map((session) => {
        const isExpanded = expandedSession === session.session_id;
        const preview = previewCache[session.session_id];

        return (
          <div
            key={session.session_id}
            className={cn(
              "rounded-xl border p-3 shadow-xs transition-colors",
              session.is_active
                ? "border-primary/30 bg-primary/5"
                : "border-border bg-card hover:bg-tile"
            )}
          >
            {/* 主行：可点击展开预览 */}
            <div
              className="flex cursor-pointer items-start gap-3"
              onClick={() => handleTogglePreview(session.session_id)}
            >
              <div className="mt-1 shrink-0">
                <Circle
                  className={cn(
                    "size-3",
                    session.is_active ? "fill-green-500 text-green-500" : "fill-muted text-muted"
                  )}
                />
              </div>
              <div className="min-w-0 flex-1">
                <p className="text-sm font-medium">
                  {session.name || "未命名会话"}
                </p>
                <p className="text-xs text-muted-foreground">
                  {formatDate(session.started_at)}
                  {session.is_active && (
                    <span className="ml-2 text-green-600">运行中</span>
                  )}
                  {!session.is_active && session.turn_count !== null && (
                    <span className="ml-2">
                      {session.turn_count} 轮对话
                      {session.status === "Idle" && (
                        <span
                          className="ml-1 text-blue-600"
                          title="此会话可在 Claude Code 中使用 /resume 恢复"
                        >
                          · 闲置
                        </span>
                      )}
                    </span>
                  )}
                </p>
                <p className="mt-1 truncate text-xs text-muted-foreground">
                  {session.cwd}
                </p>
              </div>
              <div className="flex shrink-0 items-center gap-1">
                <Button
                  type="button"
                  variant="ghost"
                  size="icon"
                  className="size-8 shrink-0 text-muted-foreground hover:text-primary"
                  title={isExpanded ? "收起预览" : "展开预览"}
                  onClick={(e) => {
                    e.stopPropagation();
                    handleTogglePreview(session.session_id);
                  }}
                >
                  {isExpanded ? <ChevronUp className="size-4" /> : <ChevronDown className="size-4" />}
                </Button>
                <ExportMenu sessionId={session.session_id} onExport={onExport} />
                {!session.is_active && (
                  <Button
                    type="button"
                    variant="ghost"
                    size="icon"
                    className="size-8 shrink-0 text-muted-foreground hover:text-destructive"
                    title="删除会话"
                    onClick={(e) => {
                      e.stopPropagation();
                      onDelete(session.session_id);
                    }}
                  >
                    <Trash2 className="size-4" />
                  </Button>
                )}
              </div>
            </div>

            {/* 预览面板 */}
            {isExpanded && preview && (
              <>
                <PreviewPanel preview={preview} />
                <SessionAnalysisPanel preview={preview} />
              </>
            )}
            {isExpanded && !preview && (
              <div className="mt-3 rounded-xl border border-border bg-tile p-3">
                <p className="text-sm text-muted-foreground">加载中...</p>
              </div>
            )}
          </div>
        );
      })}
    </div>
  );
}

function extractToolStats(preview: SessionPreview): ToolCallStat[] {
  const counts = new Map<string, { count: number; details: string[] }>();

  for (const msg of preview.messages) {
    if (msg.role === "tool" && msg.content.startsWith("调用工具:")) {
      const match = msg.content.match(/^调用工具:\s*(\S+)(?:\s*\((.*)\))?$/);
      if (match) {
        const name = match[1];
        const detail = match[2] || "";
        const existing = counts.get(name);
        if (existing) {
          existing.count++;
          if (detail) existing.details.push(detail);
        } else {
          counts.set(name, { count: 1, details: detail ? [detail] : [] });
        }
      }
    }
  }

  return Array.from(counts.entries())
    .map(([name, data]) => ({ name, count: data.count, details: data.details.slice(0, 3) }))
    .sort((a, b) => b.count - a.count);
}

function extractFileReferences(preview: SessionPreview): FileReference[] {
  const counts = new Map<string, number>();
  const filePattern = /(?:[\w-]+\/)*[\w-]+\.(?:rs|ts|tsx|js|jsx|py|md|json|toml|yaml|yml|css|html|go|java|c|cpp|h|hpp)/gi;

  for (const msg of preview.messages) {
    const matches = msg.content.match(filePattern);
    if (matches) {
      for (const match of matches) {
        const normalized = match.toLowerCase();
        counts.set(normalized, (counts.get(normalized) ?? 0) + 1);
      }
    }
  }

  return Array.from(counts.entries())
    .map(([path, count]) => ({ path, count }))
    .sort((a, b) => b.count - a.count)
    .slice(0, 8);
}

function SessionAnalysisPanel({ preview }: { preview: SessionPreview }) {
  const toolStats = extractToolStats(preview);
  const fileRefs = extractFileReferences(preview);

  if (toolStats.length === 0 && fileRefs.length === 0) {
    return null;
  }

  return (
    <div className="mt-3 grid gap-3 sm:grid-cols-2">
      {toolStats.length > 0 && (
        <div className="rounded-xl border border-border bg-tile p-3">
          <div className="mb-2 flex items-center gap-1.5 text-xs font-medium text-muted-foreground">
            <Wrench className="size-3.5" />
            工具调用 ({toolStats.reduce((s, t) => s + t.count, 0)} 次)
          </div>
          <div className="space-y-1.5">
            {toolStats.map((tool) => (
              <div key={tool.name} className="flex items-center justify-between text-xs">
                <span className="truncate font-medium">{tool.name}</span>
                <span className="shrink-0 rounded-full bg-primary/10 px-1.5 py-0.5 font-mono text-[10px] text-primary">{tool.count}</span>
              </div>
            ))}
          </div>
        </div>
      )}
      {fileRefs.length > 0 && (
        <div className="rounded-xl border border-border bg-tile p-3">
          <div className="mb-2 flex items-center gap-1.5 text-xs font-medium text-muted-foreground">
            <FileCode className="size-3.5" />
            热点文件
          </div>
          <div className="space-y-1.5">
            {fileRefs.map((file) => (
              <div key={file.path} className="flex items-center justify-between text-xs">
                <span className="truncate font-mono text-muted-foreground">{file.path}</span>
                <span className="shrink-0 rounded-full bg-stage-l3/10 px-1.5 py-0.5 font-mono text-[10px] text-stage-l3">{file.count}</span>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
