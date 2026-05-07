import { cn } from "@/lib/utils";

interface AgentToolTimelineProps {
  tool_calls: { name: string; arg: string; duration_ms: number }[];
  pending_since_ms?: number;
  thinking_since_ms?: number;
}

const TOOL_LABEL_MAP: Record<string, string> = {
  exec_command: "Bash",
  read_file: "Read",
  write_to_file: "Write",
  edit_file: "Edit",
  search: "Grep",
  update_plan: "Plan",
};

const TOOL_COLOR_MAP: Record<string, string> = {
  Bash: "bg-blue-500",
  Read: "bg-green-500",
  Write: "bg-orange-500",
  Edit: "bg-yellow-500",
  Grep: "bg-purple-500",
  Plan: "bg-pink-500",
};

export function AgentToolTimeline({ tool_calls, pending_since_ms = 0, thinking_since_ms = 0 }: AgentToolTimelineProps) {
  if (tool_calls.length === 0) {
    return null;
  }

  const maxDuration = Math.max(...tool_calls.map((toolCall) => toolCall.duration_ms));
  const safeMaxDuration = Math.max(1, maxDuration);
  const shouldShowThinking = thinking_since_ms > 0 || pending_since_ms > 0;

  return (
    <div className="space-y-2 rounded-lg border border-border bg-background/60 p-3">
      <style>{`@keyframes pulse { 0%, 100% { opacity: 0.3 } 50% { opacity: 0.7 } }`}</style>
      {shouldShowThinking && (
        <div className="flex items-center gap-2 rounded-md border border-border bg-muted/40 px-2.5 py-2 text-xs text-muted-foreground [animation:pulse_1.5s_infinite]">
          <span className="size-2 rounded-full bg-current" aria-hidden="true" />
          <span className="font-medium">Thinking...</span>
        </div>
      )}

      <div className="space-y-1.5">
        {tool_calls.map((toolCall) => {
          const label = TOOL_LABEL_MAP[toolCall.name] ?? toolCall.name;
          const colorClass = TOOL_COLOR_MAP[label] ?? "bg-muted-foreground";
          const widthPercent = Math.max(5, (toolCall.duration_ms / safeMaxDuration) * 100);
          const barStyle = { "--tool-width": `${widthPercent}%` } as React.CSSProperties;
          const isLongest = toolCall.duration_ms === maxDuration;

          return (
            <div
              key={`${toolCall.name}-${toolCall.arg}-${toolCall.duration_ms}`}
              className="grid items-center gap-2 rounded-md px-2 py-1.5 text-xs transition-colors hover:bg-muted/35 sm:grid-cols-[4.75rem_minmax(0,1fr)_3.5rem_minmax(5rem,8rem)_0.75rem]"
            >
              <span className={cn("inline-flex w-fit items-center rounded-full px-2 py-0.5 font-mono text-[0.7rem] font-semibold text-white shadow-xs", colorClass)}>
                {label}
              </span>
              <span className="min-w-0 truncate font-mono text-muted-foreground" title={toolCall.arg}>
                {truncateArg(toolCall.arg)}
              </span>
              <span className="font-mono font-semibold text-foreground">{formatDuration(toolCall.duration_ms)}</span>
              <div className="h-2 overflow-hidden rounded-full bg-muted">
                <div className={cn("h-full w-[var(--tool-width)] rounded-full transition-[width] duration-500", colorClass)} style={barStyle} />
              </div>
              <span className="font-mono font-semibold text-primary" title={isLongest ? "最长工具调用" : undefined}>
                {isLongest ? "*" : ""}
              </span>
            </div>
          );
        })}
      </div>
    </div>
  );
}

function truncateArg(arg: string): string {
  if (arg.length <= 20) {
    return arg;
  }

  return `${arg.slice(0, 20)}…`;
}

function formatDuration(ms: number): string {
  if (ms < 1000) return "<1s";
  if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`;
  return `${Math.floor(ms / 60000)}m ${Math.floor((ms % 60000) / 1000)}s`;
}
