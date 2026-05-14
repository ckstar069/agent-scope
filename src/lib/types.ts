export interface ProjectEntry {
  path: string;
  added_at: number;
}

export interface StageInfo {
  name: string;
  description: string;
  ordinal: number;
}

export interface GitStatus {
  branch: string;
  modified_count: number;
  staged_count: number;
  untracked_count: number;
  conflict_count: number;
  is_clean: boolean;
}

export interface ProjectConfig {
  project_name: string;
}

export interface ProjectAgents {
  project_path: string;
  count: number;
}

export interface AgentUpdatePayload {
  projects: ProjectAgents[];
  timestamp_ms: number;
  total_sessions: number;
}
