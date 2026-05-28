import { useCallback, useEffect, useState } from "react";

import {
  getClaudeMemoryDashboard,
  getClaudeMemoryFileContent,
  getClaudeMemoryOverview,
  getContextPressure,
  getMemoryHealthReport,
  getReviewQueue,
  getReviewQueueCounts,
  syncReviewQueue,
  updateReviewItemState,
} from "@/lib/api";

import type {
  ClaudeMemoryOverview,
  ClaudeMemoryAsset,
  ClaudeMemoryDashboard,
  MemoryHealthReport,
  ContextPressure,
  ReviewQueue,
  ReviewQueueCounts,
  ReviewQueueSyncResult,
  ReviewItem,
} from "../types";

interface UseClaudeMemoryResult {
  overview: ClaudeMemoryOverview | null;
  isLoading: boolean;
  error: string | null;
  refresh: (force?: boolean) => Promise<void>;
}

export function useClaudeMemory(projectPath?: string): UseClaudeMemoryResult {
  const [overview, setOverview] = useState<ClaudeMemoryOverview | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(
    async (force = true) => {
      setIsLoading(true);
      setError(null);
      try {
        const result = await getClaudeMemoryOverview<ClaudeMemoryOverview>(
          projectPath,
          force,
        );
        setOverview(result);
      } catch (err) {
        setError(err instanceof Error ? err.message : String(err));
      } finally {
        setIsLoading(false);
      }
    },
    [projectPath],
  );

  useEffect(() => {
    refresh(true);
  }, [refresh]);

  return { overview, isLoading, error, refresh };
}

interface UseClaudeMemoryFileResult {
  content: string | null;
  isLoading: boolean;
  error: string | null;
}

export function useClaudeMemoryFile(
  asset: ClaudeMemoryAsset | null,
  projectPath?: string,
): UseClaudeMemoryFileResult {
  const [content, setContent] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!asset || !asset.exists) {
      setContent(null);
      setError(null);
      return;
    }

    let cancelled = false;
    setIsLoading(true);
    setError(null);

    getClaudeMemoryFileContent(asset.native_path, projectPath)
      .then((result) => {
        if (!cancelled) setContent(result);
      })
      .catch((err) => {
        if (!cancelled) setError(err instanceof Error ? err.message : String(err));
      })
      .finally(() => {
        if (!cancelled) setIsLoading(false);
      });

    return () => {
      cancelled = true;
    };
  }, [asset?.native_path, asset?.exists, projectPath]);

  return { content, isLoading, error };
}

interface UseMemoryHealthResult {
  report: MemoryHealthReport | null;
  isLoading: boolean;
  error: string | null;
  refresh: (force?: boolean) => Promise<void>;
}

export function useMemoryHealth(projectPath?: string): UseMemoryHealthResult {
  const [report, setReport] = useState<MemoryHealthReport | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async (force = true) => {
    setIsLoading(true);
    setError(null);
    try {
      const result = await getMemoryHealthReport<MemoryHealthReport>(projectPath, force);
      setReport(result);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsLoading(false);
    }
  }, [projectPath]);

  useEffect(() => {
    refresh(true);
  }, [refresh]);

  return { report, isLoading, error, refresh };
}

interface UseContextPressureResult {
  pressure: ContextPressure | null;
  isLoading: boolean;
  error: string | null;
  refresh: (force?: boolean) => Promise<void>;
}

export function useContextPressure(projectPath?: string): UseContextPressureResult {
  const [pressure, setPressure] = useState<ContextPressure | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async (force = true) => {
    setIsLoading(true);
    setError(null);
    try {
      const result = await getContextPressure<ContextPressure>(projectPath, force);
      setPressure(result);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsLoading(false);
    }
  }, [projectPath]);

  useEffect(() => {
    refresh(true);
  }, [refresh]);

  return { pressure, isLoading, error, refresh };
}

// ─── Review Queue Hooks (Phase 3 Batch 2) ───

interface UseReviewQueueResult {
  queue: ReviewQueue | null;
  isLoading: boolean;
  error: string | null;
  refresh: () => Promise<void>;
  sync: (force?: boolean) => Promise<void>;
  updateState: (itemId: string, newState: string, snoozeDays?: number, note?: string) => Promise<void>;
}

export function useReviewQueue(projectPath?: string): UseReviewQueueResult {
  const [queue, setQueue] = useState<ReviewQueue | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    try {
      const result = await getReviewQueue<ReviewQueue>(projectPath);
      setQueue(result);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsLoading(false);
    }
  }, [projectPath]);

  const sync = useCallback(async (force = false) => {
    setIsLoading(true);
    setError(null);
    try {
      const result = await syncReviewQueue<ReviewQueueSyncResult>(projectPath, force);
      setQueue(result.queue);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsLoading(false);
    }
  }, [projectPath]);

  const updateState = useCallback(
    async (itemId: string, newState: string, snoozeDays?: number, note?: string) => {
      try {
        await updateReviewItemState<ReviewItem>(itemId, newState, snoozeDays, note);
        await refresh();
      } catch (err) {
        setError(err instanceof Error ? err.message : String(err));
      }
    },
    [refresh],
  );

  useEffect(() => {
    refresh();
  }, [refresh]);

  return { queue, isLoading, error, refresh, sync, updateState };
}

interface UseReviewQueueCountsResult {
  counts: ReviewQueueCounts | null;
  isLoading: boolean;
  error: string | null;
  refresh: () => Promise<void>;
}

export function useReviewQueueCounts(projectPath?: string): UseReviewQueueCountsResult {
  const [counts, setCounts] = useState<ReviewQueueCounts | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    try {
      const result = await getReviewQueueCounts<ReviewQueueCounts>(projectPath);
      setCounts(result);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsLoading(false);
    }
  }, [projectPath]);

  useEffect(() => {
    refresh();
  }, [refresh]);

  return { counts, isLoading, error, refresh };
}

// ─── Combined Dashboard Hook (Phase 3 Batch 3) ───

interface UseClaudeMemoryDashboardResult {
  dashboard: ClaudeMemoryDashboard | null;
  isLoading: boolean;
  error: string | null;
  refresh: (force?: boolean) => Promise<void>;
}

export function useClaudeMemoryDashboard(projectPath?: string): UseClaudeMemoryDashboardResult {
  const [dashboard, setDashboard] = useState<ClaudeMemoryDashboard | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async (force = true) => {
    setIsLoading(true);
    setError(null);
    try {
      const result = await getClaudeMemoryDashboard<ClaudeMemoryDashboard>(projectPath, force);
      setDashboard(result);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsLoading(false);
    }
  }, [projectPath]);

  useEffect(() => {
    refresh(true);
  }, [refresh]);

  return { dashboard, isLoading, error, refresh };
}
