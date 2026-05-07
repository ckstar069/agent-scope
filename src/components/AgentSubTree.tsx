import { useState } from "react";
import { CheckCircle2, ChevronDown, ChevronRight, Circle, Loader2 } from "lucide-react";

import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

interface AgentSubTreeProps {
  subagents: { name: string; status: string; tokens: number }[];
}

function formatTokens(n: number): string {
  if (n < 1000) return `${n}`;
  if (n < 1000000) return `${(n / 1000).toFixed(1)}K`;
  return `${(n / 1000000).toFixed(1)}M`;
}

function getStatusIcon(status: string) {
  const normalizedStatus = status.toLowerCase();

  if (normalizedStatus === "working" || normalizedStatus === "in_progress") {
    return <Loader2 className="size-4 animate-spin text-blue-500" aria-label="运行中" />;
  }

  if (normalizedStatus === "completed" || normalizedStatus === "done") {
    return <CheckCircle2 className="size-4 text-green-500" aria-label="已完成" />;
  }

  return <Circle className="size-4 text-muted-foreground" aria-label="未知状态" />;
}

export function AgentSubTree({ subagents }: AgentSubTreeProps) {
  const [isOpen, setIsOpen] = useState(false);

  if (subagents.length === 0) {
    return null;
  }

  return (
    <section className="rounded-lg border border-border bg-card/60 text-card-foreground" aria-label="子 Agent 列表">
      <Button
        type="button"
        variant="ghost"
        size="sm"
        className="h-9 w-full justify-between rounded-lg px-3 text-left hover:bg-muted"
        aria-expanded={isOpen}
        onClick={() => setIsOpen((current) => !current)}
      >
        <span className="flex min-w-0 items-center gap-2">
          {isOpen ? <ChevronDown className="size-4 text-muted-foreground" /> : <ChevronRight className="size-4 text-muted-foreground" />}
          <span className="text-sm font-medium">子 Agent ({subagents.length})</span>
        </span>
        <span className="text-xs text-muted-foreground">{isOpen ? "收起" : "展开"}</span>
      </Button>

      {isOpen && (
        <div className="grid gap-1 border-t border-border p-2" role="tree">
          {subagents.map((subagent) => (
            <div
              key={`${subagent.name}-${subagent.status}-${subagent.tokens}`}
              className={cn(
                "ml-5 flex items-center gap-2 rounded-md px-2 py-1.5 text-sm",
                "hover:bg-muted/70",
              )}
              role="treeitem"
              tabIndex={0}
            >
              {getStatusIcon(subagent.status)}
              <span className="min-w-0 flex-1 truncate font-mono text-sm">{subagent.name}</span>
              <span className="shrink-0 font-mono text-xs text-muted-foreground">{formatTokens(subagent.tokens)}</span>
            </div>
          ))}
        </div>
      )}
    </section>
  );
}
