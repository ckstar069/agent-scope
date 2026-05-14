export interface PreviewMessage {
  role: string;
  content: string;
  timestamp: number | null;
}

export interface SessionPreview {
  session_id: string;
  messages: PreviewMessage[];
  total_turns: number;
}

export interface ClaudeSession {
  session_id: string;
  name: string | null;
  cwd: string;
  status: "Active" | "Idle" | "Exited" | "Unknown";
  started_at: number | null;
  updated_at: number | null;
  turn_count: number | null;
  is_active: boolean;
}

export interface ProjectSessionGroup {
  project_path: string;
  project_name: string;
  sessions: ClaudeSession[];
  session_count: number;
  is_orphaned: boolean;
}

export interface SearchBarProps {
  value: string;
  onChange: (value: string) => void;
}

export interface ProjectListProps {
  groups: ProjectSessionGroup[];
  selectedPath: string | null;
  onSelect: (path: string) => void;
}

export interface SessionTimelineProps {
  sessions: ClaudeSession[];
  onDelete: (sessionId: string) => void;
  onExport: (sessionId: string, format: "Jsonl" | "Markdown") => void;
  onPreview: (sessionId: string) => Promise<SessionPreview | null>;
  previewCache: Record<string, SessionPreview>;
}
