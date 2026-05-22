import { useCallback, useEffect, useState } from "react";

import { getClaudeMemoryFileContent, getClaudeMemoryOverview } from "@/lib/api";

import type { ClaudeMemoryOverview, ClaudeMemoryAsset } from "../types";

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
