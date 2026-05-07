import { useEffect, useState } from "react";
import { AlertCircle, Bookmark } from "lucide-react";

import { MarkdownRenderer } from "@/components/MarkdownRenderer";
import { useTauri } from "@/hooks/useTauri";
import { Skeleton } from "@/components/ui/skeleton";
import { cn } from "@/lib/utils";

export interface SessionTurn {
  role: "user" | "assistant";
  text: string;
  tools: string[];
  timestamp?: number;
}

interface SerTranscript {
  session_id: string;
  initial_prompt: string;
  custom_title?: string;
  model?: string;
  turns: SessionTurn[];
  modified_files: string[];
  created_at: number;
}

interface TranscriptDetailViewProps {
  sessionId: string;
  projectPath: string;
  onMarkMemory: (turn: SessionTurn, turnIndex: number) => void;
  markedTurns?: Set<number>;
}

export function TranscriptDetailView({
  sessionId,
  projectPath,
  onMarkMemory,
  markedTurns = new Set(),
}: TranscriptDetailViewProps) {
  const { invoke } = useTauri();
  const [transcript, setTranscript] = useState<SerTranscript | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [displayCount, setDisplayCount] = useState(50);

  useEffect(() => {
    if (!sessionId || !projectPath) {
      setTranscript(null);
      return;
    }

    let isActive = true;

    async function loadTranscript() {
      setIsLoading(true);
      setError(null);
      setDisplayCount(50);

      try {
        const result = await invoke<SerTranscript, { path: string; session_id: string }>(
          "get_session_transcript",
          { path: projectPath, session_id: sessionId }
        );

        if (isActive) {
          setTranscript(result);
        }
      } catch (err) {
        if (isActive) {
          setError(err instanceof Error ? err.message : String(err));
        }
      } finally {
        if (isActive) {
          setIsLoading(false);
        }
      }
    }

    void loadTranscript();

    return () => {
      isActive = false;
    };
  }, [invoke, sessionId, projectPath]);

  if (isLoading) {
    return <TranscriptSkeleton />;
  }

  if (error) {
    return (
      <div className="flex items-start gap-3 rounded-xl border border-amber-500/30 bg-amber-500/10 p-4 text-sm text-amber-700 dark:text-amber-300">
        <AlertCircle className="mt-0.5 size-4 shrink-0" />
        <div>
          <p className="font-medium">加载对话失败</p>
          <p className="mt-1 text-xs opacity-85">{error}</p>
        </div>
      </div>
    );
  }

  if (!transcript) {
    return (
      <div className="flex min-h-64 items-center justify-center rounded-xl border border-dashed border-border bg-card/60 p-5 text-sm text-muted-foreground">
        选择一个会话查看对话详情
      </div>
    );
  }

  const visibleTurns = transcript.turns.slice(0, displayCount);
  const hasMore = transcript.turns.length > displayCount;

  return (
    <div className="space-y-4">
      <div className="rounded-xl border border-border bg-card p-4">
        <h3 className="text-sm font-semibold">
          {transcript.custom_title || truncateText(transcript.initial_prompt, 60)}
        </h3>
        {transcript.model && (
          <p className="mt-1 text-xs text-muted-foreground">模型: {transcript.model}</p>
        )}
      </div>

      <div className="space-y-4">
          {visibleTurns.map((turn, index) => (
          <TurnBubble
            key={`${turn.role}-${turn.text.slice(0, 20).replace(/\s/g, "-")}`}
            turn={turn}
            isMarked={markedTurns.has(index)}
            onMark={() => onMarkMemory(turn, index)}
          />
        ))}
      </div>

      {hasMore && (
        <button
          type="button"
          onClick={() => setDisplayCount((prev) => prev + 50)}
          className="w-full rounded-xl border border-border bg-card py-3 text-sm text-muted-foreground transition-colors hover:bg-accent/50"
        >
          加载更多 ({transcript.turns.length - displayCount} 条剩余)
        </button>
      )}
    </div>
  );
}

function TurnBubble({
  turn,
  isMarked,
  onMark,
}: {
  turn: SessionTurn;
  isMarked: boolean;
  onMark: () => void;
}) {
  const isUser = turn.role === "user";

  return (
    <div
      className={cn(
        "flex",
        isUser ? "justify-end" : "justify-start"
      )}
    >
      <div
        className={cn(
          "max-w-[85%] space-y-2 rounded-2xl p-4",
          isUser
            ? "rounded-tr-sm bg-primary text-primary-foreground"
            : "rounded-tl-sm bg-muted text-foreground",
          isMarked && "ring-2 ring-amber-500/50"
        )}
      >
        {isUser ? (
          <p className="whitespace-pre-wrap text-sm">{turn.text}</p>
        ) : (
          <div className="prose prose-sm dark:prose-invert max-w-none">
            <MarkdownRenderer content={turn.text} />
          </div>
        )}

        {turn.tools.length > 0 && (
          <div className="flex flex-wrap gap-1">
            {turn.tools.map((tool) => (
              <span
                key={tool}
                className={cn(
                  "rounded-md px-2 py-0.5 text-xs",
                  isUser
                    ? "bg-primary-foreground/20 text-primary-foreground"
                    : "bg-background/80 text-muted-foreground"
                )}
              >
                {tool}
              </span>
            ))}
          </div>
        )}

        <div className="flex items-center justify-between gap-2">
          {turn.timestamp && (
            <span className="text-xs opacity-60">
              {new Date(turn.timestamp).toLocaleTimeString("zh-CN")}
            </span>
          )}

          <button
            type="button"
            onClick={onMark}
            className={cn(
              "flex items-center gap-1 rounded-md px-2 py-1 text-xs transition-colors",
              isMarked
                ? "bg-amber-500/20 text-amber-700 dark:text-amber-300"
                : "hover:bg-black/10 dark:hover:bg-white/10",
              !isUser && !isMarked && "text-muted-foreground"
            )}
          >
            <Bookmark className="size-3" />
            {isMarked ? "已标记" : "标记"}
          </button>
        </div>
      </div>
    </div>
  );
}

function TranscriptSkeleton() {
  const items = ["a", "b", "c"];

  return (
    <div className="space-y-4">
      <div className="rounded-xl border border-border bg-card p-4">
        <Skeleton className="h-4 w-1/2" />
        <Skeleton className="mt-2 h-3 w-24" />
      </div>

      {items.map((key) => (
        <div key={key} className={cn("flex", key === "b" ? "justify-end" : "justify-start")}>
          <div className="max-w-[85%] space-y-2 rounded-2xl bg-muted p-4">
            <Skeleton className="h-4 w-64" />
            <Skeleton className="h-4 w-48" />
            <Skeleton className="h-4 w-32" />
          </div>
        </div>
      ))}
    </div>
  );
}

function truncateText(text: string, maxLength: number): string {
  if (!text || text.length <= maxLength) return text || "";
  return text.slice(0, maxLength) + "...";
}
