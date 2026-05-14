import { invoke as tauriInvoke } from "@tauri-apps/api/core";

export function invoke<TResponse, TArgs extends Record<string, unknown> | undefined = undefined>(
  command: string,
  args?: TArgs,
): Promise<TResponse> {
  return tauriInvoke<TResponse>(command, args);
}

// Claude History API
export function listClaudeSessions<T = unknown>() {
  return tauriInvoke<T>("list_claude_sessions_cmd");
}

export function getClaudeSessionDetail<T = unknown>(sessionId: string) {
  return tauriInvoke<T>("get_claude_session_detail_cmd", { sessionId });
}

export function searchClaudeHistory<T = unknown>(query: string) {
  return tauriInvoke<T>("search_claude_history_cmd", { query });
}

export function deleteClaudeSession(sessionId: string) {
  return tauriInvoke<void>("delete_claude_session_cmd", { sessionId });
}

export function exportClaudeSession(sessionId: string, format: "Jsonl" | "Markdown", outputPath: string) {
  return tauriInvoke<void>("export_claude_session_cmd", { sessionId, format, outputPath });
}

export function previewClaudeSession<T = unknown>(sessionId: string) {
  return tauriInvoke<T>("preview_claude_session_cmd", { sessionId });
}
