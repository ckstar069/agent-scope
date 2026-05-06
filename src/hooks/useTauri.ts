import { useCallback } from "react";
import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { listen as tauriListen, type Event, type UnlistenFn } from "@tauri-apps/api/event";

export function useTauri() {
  const invoke = useCallback(
    <TResponse, TArgs extends Record<string, unknown> | undefined = undefined>(command: string, args?: TArgs) =>
      tauriInvoke<TResponse>(command, args),
    [],
  );

  const listen = useCallback(
    <TPayload>(event: string, handler: (event: Event<TPayload>) => void): Promise<UnlistenFn> =>
      tauriListen<TPayload>(event, handler),
    [],
  );

  return { invoke, listen };
}
