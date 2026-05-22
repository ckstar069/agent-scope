import { useCallback, useState } from "react";

import { simulateClaudeMemoryLoadChain } from "@/lib/api";

import type { LoadChainResult } from "../types";

interface UseLoadChainResult {
  result: LoadChainResult | null;
  isLoading: boolean;
  error: string | null;
  simulate: (cwd: string) => Promise<void>;
}

export function useLoadChain(): UseLoadChainResult {
  const [result, setResult] = useState<LoadChainResult | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const simulate = useCallback(async (cwd: string) => {
    setIsLoading(true);
    setError(null);
    setResult(null);
    try {
      const data = await simulateClaudeMemoryLoadChain<LoadChainResult>(cwd);
      setResult(data);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsLoading(false);
    }
  }, []);

  return { result, isLoading, error, simulate };
}
