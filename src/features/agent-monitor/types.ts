export type RawAgentStatus = "Thinking" | "Executing" | "Waiting" | "RateLimited" | "Done";
export type DisplayStatus = "Active" | "Idle" | "Offline";
export type TokenRateUnit = "second" | "minute";
export type RateType = "realtime" | "1min" | "5min" | "total";
export type AgentDetailTab = "timeline" | "subagents" | "fileaudit";

export interface AgentInfo {
  agent_type: string;
  session_id: string;
  cwd: string;
  project_name: string;
  status: RawAgentStatus;
  model: string;
  context_percent: number;
  context_window: number;
  total_input_tokens: number;
  total_output_tokens: number;
  total_cache_read: number;
  total_cache_create: number;
  turn_count: number;
  current_tasks: string[];
  mem_mb: number;
  git_branch: string;
  git_added: number;
  git_modified: number;
  token_history: number[];
  context_history: number[];
  compaction_count: number;
  token_rate: number;
  token_rate_1m: number;
  token_rate_5m: number;
  token_rate_total: number;
  token_rate_1m_reason: string;
  token_rate_5m_reason: string;
  token_rate_total_reason: string;
  pid: number;
  version: string;
  effort: string;
  tool_calls: { name: string; arg: string; duration_ms: number }[];
  subagents: { name: string; status: string; tokens: number }[];
  file_accesses: { path: string; operation: string; turn_index: number }[];
  pending_since_ms: number;
  thinking_since_ms: number;
}

export interface ProjectAgents {
  project_path: string;
  agents: AgentInfo[];
  count: number;
}

export interface AgentUpdatePayload {
  projects: ProjectAgents[];
  unmapped: AgentInfo[];
  timestamp_ms: number;
  total_sessions: number;
}
