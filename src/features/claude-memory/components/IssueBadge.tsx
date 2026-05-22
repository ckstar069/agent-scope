import { AlertTriangle, ShieldCheck } from "lucide-react";

import { cn } from "@/lib/utils";

interface IssueBadgeProps {
  count: number;
  className?: string;
}

export function IssueBadge({ count, className }: IssueBadgeProps) {
  if (count === 0) {
    return (
      <span
        className={cn(
          "inline-flex items-center gap-1 rounded-full bg-emerald-50 px-2 py-0.5 text-xs font-medium text-emerald-700 dark:bg-emerald-950/30 dark:text-emerald-400",
          className,
        )}
      >
        <ShieldCheck className="size-3" aria-hidden="true" />
        安全
      </span>
    );
  }

  return (
    <span
      className={cn(
        "inline-flex items-center gap-1 rounded-full bg-amber-50 px-2 py-0.5 text-xs font-medium text-amber-700 dark:bg-amber-950/30 dark:text-amber-400",
        className,
      )}
    >
      <AlertTriangle className="size-3" aria-hidden="true" />
      {count} 项风险
    </span>
  );
}
