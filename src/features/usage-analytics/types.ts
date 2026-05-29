export type TimeRange = "today" | "last7days" | "all";
export type GroupBy = "project" | "model" | "session";

export type ConfidenceLevel = "high" | "medium" | "low";
export type RealtimeLevel = "delayed" | "near_realtime" | "realtime";
export type DirErrorReason =
  | "not_found"
  | "not_a_directory"
  | "permission_denied"
  | "invalid_path"
  | "missing_structure"
  | "empty";

export type DirIssueSeverity = "info" | "warning" | "error";

export interface UnreadableDir {
  path: string;
  canonical_path?: string;
  reason: DirErrorReason;
  detail?: string;
  source: string;
  severity: DirIssueSeverity;
}

export interface UsageSourceStatus {
  source_type: string;
  config_dirs: string[];
  readable_dirs: string[];
  unreadable_dirs: UnreadableDir[];
  last_scan_at?: string;
  last_usage_at?: string;
  confidence: ConfidenceLevel;
  realtime_level: RealtimeLevel;
  notes: string[];
}

export interface UsageScanSummary {
  source_status: UsageSourceStatus;
  scanned_files: number;
  scanned_lines: number;
  record_count: number;
  error_count: number;
  errors: string[];
}

export interface UsageGroup {
  group_key: string;
  group_label: string;
  group_detail?: string;
  input_tokens: number;
  output_tokens: number;
  cache_read_tokens: number;
  cache_create_tokens: number;
  total_tokens: number;
  session_count: number;
  first_seen: string;
  last_seen: string;
}

export interface UsageAggregate {
  time_range: TimeRange;
  group_by: GroupBy;
  input_tokens: number;
  output_tokens: number;
  cache_read_tokens: number;
  cache_create_tokens: number;
  total_tokens: number;
  session_count: number;
  project_count: number;
  model_count: number;
  groups: UsageGroup[];
}
