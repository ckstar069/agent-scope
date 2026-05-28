import { cn } from "@/lib/utils";

import type { ReviewState } from "../types";

interface ReviewStateBadgeProps {
  state: ReviewState;
}

const STATE_CONFIG: Record<
  ReviewState,
  { label: string; className: string }
> = {
  pending: {
    label: "待处理",
    className:
      "bg-amber-100 text-amber-700 dark:bg-amber-950/40 dark:text-amber-400",
  },
  reviewed: {
    label: "已标记",
    className:
      "bg-green-100 text-green-700 dark:bg-green-950/40 dark:text-green-400",
  },
  ignored: {
    label: "已忽略",
    className:
      "bg-muted text-muted-foreground",
  },
  snoozed: {
    label: "稍后",
    className:
      "bg-blue-100 text-blue-700 dark:bg-blue-950/40 dark:text-blue-400",
  },
};

export function ReviewStateBadge({ state }: ReviewStateBadgeProps) {
  const config = STATE_CONFIG[state];
  return (
    <span
      className={cn(
        "inline-flex shrink-0 items-center rounded px-1.5 py-0.5 text-[10px] font-medium",
        config.className,
      )}
    >
      {config.label}
    </span>
  );
}
