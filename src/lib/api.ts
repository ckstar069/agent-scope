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

// Claude Memory API
export function getClaudeMemoryOverview<T = unknown>(projectPath?: string, force = false) {
  return tauriInvoke<T>("get_claude_memory_overview", { projectPath, force });
}

export function getClaudeMemoryFileContent<T = string>(nativePath: string, projectPath?: string) {
  return tauriInvoke<T>("get_claude_memory_file_content", { nativePath, projectPath });
}

export function simulateClaudeMemoryLoadChain<T = unknown>(cwd: string) {
  return tauriInvoke<T>("simulate_claude_memory_load_chain", { cwd });
}

export function getMemoryHealthReport<T = unknown>(projectPath?: string, force = false) {
  return tauriInvoke<T>("get_memory_health_report", { projectPath, force });
}

export function getContextPressure<T = unknown>(projectPath?: string, force = false) {
  return tauriInvoke<T>("get_context_pressure", { projectPath, force });
}

export function getClaudeMemoryDashboard<T = unknown>(projectPath?: string, force = false) {
  return tauriInvoke<T>("get_claude_memory_dashboard", { projectPath, force });
}

// Review Queue API
export function getReviewQueue<T = unknown>(projectPath?: string, filter?: string) {
  return tauriInvoke<T>("get_review_queue", { projectPath, filter });
}

export function syncReviewQueue<T = unknown>(projectPath?: string, force = false) {
  return tauriInvoke<T>("sync_review_queue", { projectPath, force });
}

export function updateReviewItemState<T = unknown>(
  itemId: string,
  newState: string,
  snoozeDays?: number,
  note?: string,
) {
  return tauriInvoke<T>("update_review_item_state", { itemId, newState, snoozeDays, note });
}

export function getReviewQueueCounts<T = unknown>(projectPath?: string) {
  return tauriInvoke<T>("get_review_queue_counts", { projectPath });
}

export function getAgentSnapshot<T = unknown>() {
  return tauriInvoke<T>("get_agent_snapshot")
    .catch((err) => {
      console.warn("[getAgentSnapshot] 读取快照失败:", err);
      return null;
    });
}
