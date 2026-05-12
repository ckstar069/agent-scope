import { useEffect, useRef, useState } from "react";
import { Circle, Download, Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import type { ClaudeSession } from "@/hooks/useClaudeHistory";

interface SessionTimelineProps {
  sessions: ClaudeSession[];
  onDelete: (sessionId: string) => void;
  onExport: (sessionId: string, format: "Jsonl" | "Markdown") => void;
}

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
            onClick={() => {
              onExport(sessionId, "Jsonl");
              setOpen(false);
            }}
          >
            导出为 JSONL
          </button>
          <button
            type="button"
            className="w-full px-3 py-2 text-left text-sm hover:bg-accent"
            onClick={() => {
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

export function SessionTimeline({ sessions, onDelete, onExport }: SessionTimelineProps) {
  return (
    <div className="flex flex-col gap-2">
      {sessions.map((session) => (
        <div
          key={session.session_id}
          className={cn(
            "flex items-start gap-3 rounded-lg border p-3 transition-colors",
            session.is_active
              ? "border-primary/30 bg-primary/5"
              : "border-border bg-card hover:bg-accent/50"
          )}
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
                <span className="ml-2">{session.turn_count} 轮对话</span>
              )}
            </p>
            <p className="mt-1 truncate text-xs text-muted-foreground">
              {session.cwd}
            </p>
          </div>
          <div className="flex shrink-0 items-center gap-1">
            <ExportMenu sessionId={session.session_id} onExport={onExport} />
            {!session.is_active && (
              <Button
                type="button"
                variant="ghost"
                size="icon"
                className="size-8 shrink-0 text-muted-foreground hover:text-destructive"
                title="删除会话"
                onClick={() => onDelete(session.session_id)}
              >
                <Trash2 className="size-4" />
              </Button>
            )}
          </div>
        </div>
      ))}
    </div>
  );
}
