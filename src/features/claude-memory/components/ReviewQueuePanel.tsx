import { useCallback, useState } from "react";
import { Loader2, RefreshCw } from "lucide-react";

import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

import type { ClaudeMemoryAsset, ReviewItem, ReviewQueue, ReviewState } from "../types";

import { ReviewStateBadge } from "./ReviewStateBadge";

interface ReviewQueuePanelProps {
  queue: ReviewQueue | null;
  isLoading: boolean;
  error: string | null;
  onSync: () => void;
  onUpdateState: (
    itemId: string,
    newState: string,
    snoozeDays?: number,
  ) => Promise<void>;
  assetsById: Map<string, ClaudeMemoryAsset>;
  onSelectAsset: (assetId: string) => void;
}

const FILTERS: { key: ReviewState | "all"; label: string }[] = [
  { key: "all", label: "全部" },
  { key: "pending", label: "待处理" },
  { key: "reviewed", label: "已标记" },
  { key: "ignored", label: "已忽略" },
  { key: "snoozed", label: "稍后" },
];

const SNOOZE_OPTIONS = [1, 3, 7, 30];

function formatDate(ts: number): string {
  const d = new Date(ts * 1000);
  const pad = (n: number) => n.toString().padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

function getSeverityDot(severity: string): string {
  if (severity === "critical") return "bg-red-500";
  if (severity === "warning") return "bg-amber-500";
  return "bg-blue-500";
}

export function ReviewQueuePanel({
  queue,
  isLoading,
  error,
  onSync,
  onUpdateState,
  assetsById,
  onSelectAsset,
}: ReviewQueuePanelProps) {
  const [filter, setFilter] = useState<ReviewState | "all">("pending");
  const [expandedId, setExpandedId] = useState<string | null>(null);
  const [updatingItemId, setUpdatingItemId] = useState<string | null>(null);

  const filteredItems =
    queue?.items.filter(
      (item) => filter === "all" || item.state === filter,
    ) ?? [];

  const handleToggle = useCallback((itemId: string) => {
    setExpandedId((prev) => (prev === itemId ? null : itemId));
  }, []);

  const handleUpdateState = useCallback(
    async (itemId: string, newState: string, snoozeDays?: number) => {
      setUpdatingItemId(itemId);
      try {
        await onUpdateState(itemId, newState, snoozeDays);
      } finally {
        setUpdatingItemId(null);
      }
    },
    [onUpdateState],
  );

  return (
    <div className="rounded-xl border border-border bg-card shadow-xs">
      {/* Header */}
      <div className="flex items-center justify-between border-b border-border px-4 py-3">
        <div className="flex items-center gap-3">
          <h2 className="text-sm font-semibold">审阅队列</h2>
          {queue && (
            <div className="flex items-center gap-2 text-xs text-muted-foreground">
              <span>待处理 {queue.pending_count}</span>
              <span>已标记 {queue.reviewed_count}</span>
              <span>已忽略 {queue.ignored_count}</span>
              <span>稍后 {queue.snoozed_count}</span>
            </div>
          )}
        </div>
        <div className="flex items-center gap-2">
          {queue?.last_sync_at && (
            <span className="text-xs text-muted-foreground">
              上次同步：{formatDate(queue.last_sync_at)}
            </span>
          )}
          <Button
            type="button"
            variant="outline"
            size="sm"
            disabled={isLoading}
            onClick={onSync}
          >
            {isLoading ? (
              <Loader2
                className="mr-1 size-3.5 animate-spin"
                aria-hidden="true"
              />
            ) : (
              <RefreshCw className="mr-1 size-3.5" aria-hidden="true" />
            )}
            同步
          </Button>
        </div>
      </div>

      {/* Filter Tabs */}
      <div className="flex items-center gap-1 border-b border-border px-4 py-2">
        {FILTERS.map((f) => (
          <button
            key={f.key}
            type="button"
            onClick={() => setFilter(f.key)}
            className={cn(
              "rounded-md px-2.5 py-1 text-xs font-medium transition-colors",
              filter === f.key
                ? "bg-primary/10 text-primary"
                : "text-muted-foreground hover:bg-muted/60",
            )}
          >
            {f.label}
          </button>
        ))}
      </div>

      {/* Content */}
      <div className="max-h-[320px] overflow-auto">
        {error && (
          <div className="flex items-center gap-2 p-4 text-xs text-destructive">
            <span>加载失败：{error}</span>
          </div>
        )}

        {!error && filteredItems.length === 0 && (
          <div className="flex flex-col items-center justify-center gap-2 py-8 text-sm text-muted-foreground">
            <p>
              {filter === "all"
                ? "当前没有审阅项"
                : `当前没有${FILTERS.find((f) => f.key === filter)?.label ?? ""}的审阅项`}
            </p>
          </div>
        )}

        {!error && filteredItems.length > 0 && (
          <div className="divide-y divide-border">
            {filteredItems.map((item) => (
              <ReviewItemRow
                key={item.id}
                item={item}
                assetsById={assetsById}
                expanded={expandedId === item.id}
                onToggle={() => handleToggle(item.id)}
                onSelectAsset={onSelectAsset}
                onUpdateState={handleUpdateState}
                isMutating={updatingItemId === item.id}
                isLoading={isLoading}
              />
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

function ReviewItemRow({
  item,
  assetsById,
  expanded,
  onToggle,
  onSelectAsset,
  onUpdateState,
  isMutating,
  isLoading,
}: {
  item: ReviewItem;
  assetsById: Map<string, ClaudeMemoryAsset>;
  expanded: boolean;
  onToggle: () => void;
  onSelectAsset: (assetId: string) => void;
  onUpdateState: (
    itemId: string,
    newState: string,
    snoozeDays?: number,
  ) => Promise<void>;
  isMutating: boolean;
  isLoading: boolean;
}) {
  const primaryAsset = assetsById.get(item.primary_asset_id);
  const assetName = primaryAsset?.logical_path ?? item.primary_asset_id;

  return (
    <div className="px-4 py-2">
      {/* Collapsed row */}
      <button
        type="button"
        onClick={onToggle}
        className="flex w-full items-center gap-2 text-left"
      >
        <div
          className={cn(
            "h-2 w-2 shrink-0 rounded-full",
            getSeverityDot(item.severity),
          )}
          title={item.severity}
        />
        <span className="shrink-0 rounded bg-muted px-1.5 py-0.5 text-[10px] font-medium text-muted-foreground">
          {item.issue_type}
        </span>
        <span className="min-w-0 flex-1 truncate text-xs">{assetName}</span>
        <ReviewStateBadge state={item.state} />
      </button>

      {/* Expanded detail */}
      {expanded && (
        <div className="mt-2 space-y-2 border-l-2 border-border pl-3">
          <p className="text-xs text-foreground">{item.message}</p>

          {item.suggestion && (
            <div className="rounded bg-muted/50 p-2 text-xs text-muted-foreground">
              <p className="mb-1 font-medium">建议</p>
              <p>{item.suggestion}</p>
            </div>
          )}

          {/* Asset links */}
          {item.asset_ids.length > 0 && (
            <div className="space-y-1">
              <p className="text-[10px] font-medium text-muted-foreground">
                涉及资产
              </p>
              <div className="flex flex-wrap gap-1">
                {item.asset_ids.map((aid) => {
                  const asset = assetsById.get(aid);
                  const canLocate = asset?.exists ?? false;
                  return (
                    <button
                      key={aid}
                      type="button"
                      disabled={!canLocate || isLoading || isMutating}
                      onClick={(e) => {
                        e.stopPropagation();
                        if (canLocate) onSelectAsset(aid);
                      }}
                      className={cn(
                        "inline-flex max-w-[200px] items-center gap-1 rounded bg-muted px-1.5 py-0.5 text-[10px]",
                        canLocate
                          ? "cursor-pointer text-foreground hover:bg-muted/80"
                          : "cursor-not-allowed text-muted-foreground/50",
                      )}
                      title={asset?.logical_path ?? aid}
                    >
                      <span className="truncate">
                        {asset?.logical_path ?? aid}
                      </span>
                      {!canLocate && (
                        <span className="text-muted-foreground/50">(不存在)</span>
                      )}
                    </button>
                  );
                })}
              </div>
            </div>
          )}

          {/* Actions */}
          <ReviewItemActions
            state={item.state}
            itemId={item.id}
            onUpdateState={onUpdateState}
            disabled={isLoading || isMutating}
          />
        </div>
      )}
    </div>
  );
}

function ReviewItemActions({
  state,
  itemId,
  onUpdateState,
  disabled,
}: {
  state: ReviewState;
  itemId: string;
  onUpdateState: (
    itemId: string,
    newState: string,
    snoozeDays?: number,
  ) => Promise<void>;
  disabled: boolean;
}) {
  const [showSnooze, setShowSnooze] = useState(false);

  const btnBase =
    "rounded px-2 py-0.5 text-[10px] font-medium transition-colors disabled:opacity-50";

  const handleReview = useCallback(() => {
    void onUpdateState(itemId, "reviewed");
  }, [itemId, onUpdateState]);

  const handleIgnore = useCallback(() => {
    void onUpdateState(itemId, "ignored");
  }, [itemId, onUpdateState]);

  const handleReopen = useCallback(() => {
    void onUpdateState(itemId, "pending");
  }, [itemId, onUpdateState]);

  const handleSnooze = useCallback(
    (days: number) => {
      setShowSnooze(false);
      void onUpdateState(itemId, "snoozed", days);
    },
    [itemId, onUpdateState],
  );

  if (state === "pending") {
    return (
      <div className="flex flex-wrap items-center gap-1.5">
        <button
          type="button"
          disabled={disabled}
          onClick={handleReview}
          className={`${btnBase} bg-green-100 text-green-700 hover:bg-green-200 dark:bg-green-950/40 dark:text-green-400`}
        >
          标记已审
        </button>
        <div className="flex items-center gap-1">
          <button
            type="button"
            disabled={disabled}
            onClick={() => setShowSnooze((p) => !p)}
            className={`${btnBase} bg-blue-100 text-blue-700 hover:bg-blue-200 dark:bg-blue-950/40 dark:text-blue-400`}
          >
            稍后处理
          </button>
          {showSnooze &&
            SNOOZE_OPTIONS.map((d) => (
              <button
                key={d}
                type="button"
                disabled={disabled}
                onClick={() => handleSnooze(d)}
                className={`${btnBase} bg-blue-50 text-blue-600 hover:bg-blue-100 dark:bg-blue-950/20 dark:text-blue-300`}
              >
                {d}天
              </button>
            ))}
        </div>
        <button
          type="button"
          disabled={disabled}
          onClick={handleIgnore}
          className={`${btnBase} bg-muted text-muted-foreground hover:bg-muted/80`}
        >
          忽略此项
        </button>
      </div>
    );
  }

  if (state === "reviewed" || state === "ignored") {
    return (
      <button
        type="button"
        disabled={disabled}
        onClick={handleReopen}
        className={`${btnBase} bg-muted text-muted-foreground hover:bg-muted/80`}
      >
        重新打开
      </button>
    );
  }

  // snoozed
  return (
    <div className="flex items-center gap-1.5">
      <button
        type="button"
        disabled={disabled}
        onClick={handleReview}
        className={`${btnBase} bg-green-100 text-green-700 hover:bg-green-200 dark:bg-green-950/40 dark:text-green-400`}
      >
        标记已审
      </button>
      <button
        type="button"
        disabled={disabled}
        onClick={handleReopen}
        className={`${btnBase} bg-muted text-muted-foreground hover:bg-muted/80`}
      >
        重新打开
      </button>
    </div>
  );
}
