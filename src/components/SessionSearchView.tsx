import { useEffect, useState, useCallback, useRef } from "react";
import { Search, Clock, FileText, MessageSquare } from "lucide-react";

import { useTauri } from "@/hooks/useTauri";
import { Skeleton } from "@/components/ui/skeleton";
import { Input } from "@/components/ui/input";
import { cn } from "@/lib/utils";

export interface SerSessionSummary {
  session_id: string;
  initial_prompt: string;
  custom_title?: string;
  model?: string;
  turn_count: number;
  modified_files: string[];
  created_at: number;
}

interface SessionSearchViewProps {
  projectPath: string;
  onSelectSession: (sessionId: string) => void;
}

function formatRelativeTime(timestampMs: number): string {
  const now = Date.now();
  const diff = now - timestampMs;

  const minutes = Math.floor(diff / 60000);
  if (minutes < 1) return "刚刚";
  if (minutes < 60) return `${minutes}分钟前`;

  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}小时前`;

  const days = Math.floor(hours / 24);
  if (days < 30) return `${days}天前`;

  const months = Math.floor(days / 30);
  if (months < 12) return `${months}个月前`;

  const years = Math.floor(months / 12);
  return `${years}年前`;
}

function truncateText(text: string, maxLength: number): string {
  if (!text || text.length <= maxLength) return text || "";
  return text.slice(0, maxLength) + "...";
}

export function SessionSearchView({ projectPath, onSelectSession }: SessionSearchViewProps) {
  const { invoke } = useTauri();
  const [query, setQuery] = useState("");
  const [sessions, setSessions] = useState<SerSessionSummary[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const debounceTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const loadSessions = useCallback(
    async (searchQuery: string) => {
      if (!projectPath) return;

      setIsLoading(true);
      setError(null);

      try {
        let result: SerSessionSummary[];

        if (searchQuery.trim()) {
          result = await invoke<SerSessionSummary[], { path: string; query: string }>(
            "search_sessions",
            { path: projectPath, query: searchQuery.trim() }
          );
        } else {
          result = await invoke<SerSessionSummary[], { path: string }>(
            "list_project_sessions",
            { path: projectPath }
          );
        }

        setSessions(result);
      } catch (err) {
        setError(err instanceof Error ? err.message : String(err));
        setSessions([]);
      } finally {
        setIsLoading(false);
      }
    },
    [invoke, projectPath]
  );

  useEffect(() => {
    if (debounceTimerRef.current) {
      clearTimeout(debounceTimerRef.current);
    }

    debounceTimerRef.current = setTimeout(() => {
      void loadSessions(query);
    }, 300);

    return () => {
      if (debounceTimerRef.current) {
        clearTimeout(debounceTimerRef.current);
      }
    };
  }, [query, loadSessions]);

  return (
    <div className="space-y-4">
      <div className="relative">
        <Search
          className="absolute left-3 top-1/2 size-4 -translate-y-1/2 text-muted-foreground"
          aria-hidden="true"
        />
        <Input
          type="text"
          placeholder="搜索对话内容..."
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          className="pl-9"
        />
      </div>

      {error && (
        <div className="rounded-xl border border-amber-500/30 bg-amber-500/10 p-3 text-sm text-amber-700 dark:text-amber-300">
          搜索失败: {error}
        </div>
      )}

      {isLoading && sessions.length === 0 && <SessionListSkeleton />}

      {!isLoading && sessions.length === 0 && !error && (
        <div className="flex flex-col items-center justify-center py-8 text-sm text-muted-foreground">
          {query.trim() ? (
            <>
              <MessageSquare className="mb-2 size-8 opacity-50" aria-hidden="true" />
              <p>未找到匹配 &quot;{query}&quot; 的对话</p>
            </>
          ) : (
            <>
              <Search className="mb-2 size-8 opacity-50" aria-hidden="true" />
              <p>输入关键词搜索对话内容</p>
            </>
          )}
        </div>
      )}

      {sessions.length > 0 && (
        <div className="space-y-2">
          {sessions.map((session) => (
            <SessionListItem
              key={session.session_id}
              session={session}
              onClick={() => onSelectSession(session.session_id)}
            />
          ))}
        </div>
      )}
    </div>
  );
}

function SessionListItem({
  session,
  onClick,
}: {
  session: SerSessionSummary;
  onClick: () => void;
}) {
  const displayTitle = session.custom_title || truncateText(session.initial_prompt, 80);
  const sessionIdShort = session.session_id.slice(0, 8);

  return (
    <button
      type="button"
      onClick={onClick}
      className={cn(
        "w-full rounded-xl border border-border bg-card p-4 text-left transition-colors",
        "hover:bg-accent/50 focus-visible:outline-none focus-visible:ring-3 focus-visible:ring-ring/50"
      )}
    >
      <div className="mb-2 flex items-center gap-2 text-xs text-muted-foreground">
        <span className="font-mono">{sessionIdShort}</span>
        {session.model && (
          <span className="rounded-md bg-muted px-1.5 py-0.5">{session.model}</span>
        )}
        <span className="ml-auto flex items-center gap-1">
          <Clock className="size-3" aria-hidden="true" />
          {formatRelativeTime(session.created_at)}
        </span>
      </div>

      <p className="mb-2 text-sm font-medium">{displayTitle}</p>

      <div className="flex flex-wrap items-center gap-2">
        <span className="flex items-center gap-1 text-xs text-muted-foreground">
          <MessageSquare className="size-3" aria-hidden="true" />
          {session.turn_count} 轮
        </span>

        {session.modified_files.length > 0 && (
          <div className="flex flex-wrap gap-1">
            {session.modified_files.slice(0, 3).map((file) => (
              <span
                key={file}
                className="flex items-center gap-1 rounded-md bg-muted px-1.5 py-0.5 text-xs text-muted-foreground"
              >
                <FileText className="size-3" aria-hidden="true" />
                {truncateText(file.split("/").pop() || file, 20)}
              </span>
            ))}
            {session.modified_files.length > 3 && (
              <span className="rounded-md bg-muted px-1.5 py-0.5 text-xs text-muted-foreground">
                +{session.modified_files.length - 3}
              </span>
            )}
          </div>
        )}
      </div>
    </button>
  );
}

function SessionListSkeleton() {
  const items = ["a", "b", "c", "d", "e", "f"];
  
  return (
    <div className="space-y-2">
      {items.map((key) => (
        <div key={key} className="rounded-xl border border-border bg-card p-4">
          <div className="mb-2 flex items-center gap-2">
            <Skeleton className="h-3 w-16" />
            <Skeleton className="h-3 w-12" />
            <Skeleton className="ml-auto h-3 w-20" />
          </div>
          <Skeleton className="mb-2 h-4 w-3/4" />
          <div className="flex gap-2">
            <Skeleton className="h-3 w-16" />
            <Skeleton className="h-3 w-24" />
          </div>
        </div>
      ))}
    </div>
  );
}
