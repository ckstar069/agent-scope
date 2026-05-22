import type { ProjectEntry, StageInfo, GitStatus, ProjectConfig, ProjectAgents, AgentUpdatePayload } from "@/lib/types";

export type { ProjectEntry, StageInfo, GitStatus, ProjectConfig, ProjectAgents, AgentUpdatePayload };

export interface DashboardProps {
  onNavigateSettings: () => void;
  onSelectProject: (projectPath: string) => void;
}

export interface TemplateDataPayload {
  project_path: string;
  stage: StageInfo | null;
  stage_error: string | null;
  config: ProjectConfig | null;
  config_error: string | null;
  git: GitStatus;
  git_error: string | null;
  timestamp_ms: number;
}
