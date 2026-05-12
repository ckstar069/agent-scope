import { useEffect, useRef, useState } from "react";
import { Circle, Download, Trash2, ChevronDown, ChevronUp } from "lucide-react";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import type { ClaudeSession, SessionPreview } from "@/hooks/useClaudeHistory";

interface SessionTimelineProps {
  sessions: ClaudeSession[];
  onDelete: (sessionId: string) => void;
  onExport: (sessionId: string, format: "Jsonl" | "Markdown") => void;
  onPreview: (sessionId: string) => Promise<SessionPreview | null>;
  previewCache: Record<string, SessionPreview>;
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

  return (
    <div className="mt-2 rounded-md border bg-muted/30 p-3">
      <p className="mb-2 text-xs text-muted-foreground">
        预览（共 {preview.total_turns} 轮对话）
      </p>
      <div className="flex max-h-[600px] flex-col gap-4 overflow-y-auto pr-1">
        {preview.messages.map((msg, idx) => (
          <div key={idx} className="rounded-sm p-2 text-sm hover:bg-muted/50">
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
              "rounded-lg border p-3 transition-colors",
              session.is_active
                ? "border-primary/30 bg-primary/5"
                : "border-border bg-card hover:bg-accent/50"
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
                    <span className="ml-2">{session.turn_count} 轮对话</span>
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
              <PreviewPanel preview={preview} />
            )}
            {isExpanded && !preview && (
              <div className="mt-2 rounded-md border bg-muted/30 p-3">
                <p className="text-sm text-muted-foreground">加载中...</p>
              </div>
            )}
          </div>
        );
      })}
    </div>
  );
}
