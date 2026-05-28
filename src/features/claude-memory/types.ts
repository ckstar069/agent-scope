export interface Frontmatter {
  name: string | null;
  description: string | null;
  trigger: string | null;
  memory_scope: string | null;
  paths: string[] | null;
  raw: string;
}

export interface SecretIssue {
  issue_type: string;
  line_number: number;
  column_start: number;
  column_end: number;
  matched_text: string;
}

export interface ClaudeMemoryAsset {
  id: string;
  scope: string;
  asset_type: string;
  logical_path: string;
  native_path: string;
  content_hash: string | null;
  content_preview: string | null;
  content_truncated: boolean;
  line_count: number | null;
  byte_size: number | null;
  mtime_ms: number | null;
  frontmatter: Frontmatter | null;
  secret_issues: SecretIssue[];
  exists: boolean;
}

export interface HostProfile {
  host_id: string;
  hostname: string;
  os: string;
  home_dir: string;
  claude_config_dir: string;
  user_name: string;
}

export interface MemorySummary {
  total_assets: number;
  total_existing: number;
  by_scope: Record<string, number>;
  by_type: Record<string, number>;
  total_secret_issues: number;
}

export interface MemoryScanError {
  scope: string;
  path: string;
  message: string;
}

export interface ClaudeMemoryOverview {
  scanned_at_ms: number;
  host_profile: HostProfile;
  assets: ClaudeMemoryAsset[];
  summary: MemorySummary;
  errors: MemoryScanError[];
}

export type AssetGroup =
  | "instruction"
  | "rules"
  | "auto_memory"
  | "skills_agents";

export interface GroupedAssets {
  group: AssetGroup;
  label: string;
  assets: ClaudeMemoryAsset[];
}

// ─── Load Chain (P1) Types ───

export interface LoadChainStep {
  order: number;
  scope: string;
  asset_type: string;
  native_path: string;
  logical_path: string;
  load_reason: string;
  line_count: number | null;
  byte_size: number | null;
  content_preview: string | null;
  content_truncated: boolean;
  exists: boolean;
}

export interface PathScopedRule {
  scope: string;
  native_path: string;
  logical_path: string;
  name: string | null;
  paths: string[];
  exists: boolean;
}

export interface ExcludedAsset {
  native_path: string;
  logical_path: string;
  scope: string;
  excluded_by: string;
  pattern: string;
}

export interface LoadChainWarning {
  level: string;
  code: string;
  message: string;
}

export interface LoadChainResult {
  cwd: string;
  host_profile: HostProfile;
  startup_chain: LoadChainStep[];
  path_scoped_rules: PathScopedRule[];
  excluded_assets: ExcludedAsset[];
  warnings: LoadChainWarning[];
}

// ─── Memory Health Phase 1 Types ───

export interface HealthDimension {
  name: string;
  score: number;
  reason: string;
  contributing_assets: string[];
}

export interface MemoryHealthIssue {
  issue_type: string;
  severity: string;
  asset_ids: string[];
  message: string;
  suggestion: string;
}

export interface MemoryStaleness {
  asset_id: string;
  asset_type: string;
  scope: string;
  logical_path: string;
  mtime_ms: number | null;
  stale_days: number | null;
  threshold_days: number;
}

export interface MemoryDuplicateGroup {
  group_id: string;
  asset_ids: string[];
  similarity: number;
  overlap_content: string;
  suggestion: string;
}

export interface MemoryHealthReport {
  overall_score: number;
  freshness: HealthDimension;
  quality: HealthDimension;
  coverage: HealthDimension;
  cleanliness: HealthDimension;
  safety: HealthDimension;
  top_issues: MemoryHealthIssue[];
  stale_assets: MemoryStaleness[];
  duplicate_groups: MemoryDuplicateGroup[];
}

// ─── Context Pressure (Phase 3 Batch 1) ───

export interface PressureHeavyAsset {
  asset_id: string;
  asset_type: string;
  logical_path: string;
  line_count: number | null;
  byte_size: number | null;
}

export interface PressureAlert {
  metric: string;
  current: number;
  threshold: number;
  severity: string;
  message: string;
}

export interface ContextPressure {
  total_assets: number;
  existing_assets: number;
  total_lines: number;
  total_bytes: number;
  estimated_tokens: number;
  pressure_ratio: number;
  level: string;
  heavy_assets: PressureHeavyAsset[];
  alerts: PressureAlert[];
}

// ─── Review Queue (Phase 3 Batch 2) ───

export type ReviewState = "pending" | "reviewed" | "ignored" | "snoozed";

export interface ReviewItem {
  id: string;
  source_key: string;
  project_id: string;
  issue_type: string;
  severity: string;
  message: string;
  suggestion: string;
  asset_ids: string[];
  primary_asset_id: string;
  group_id: string | null;
  state: ReviewState;
  created_at: number;
  updated_at: number;
  snooze_until: number | null;
  review_note: string | null;
}

export interface ReviewQueue {
  items: ReviewItem[];
  pending_count: number;
  reviewed_count: number;
  ignored_count: number;
  snoozed_count: number;
  last_sync_at: number | null;
}

export interface ReviewQueueCounts {
  pending: number;
  reviewed: number;
  ignored: number;
  snoozed: number;
  total: number;
}

export interface ClaudeMemoryDashboard {
  overview: ClaudeMemoryOverview;
  health_report: MemoryHealthReport;
  context_pressure: ContextPressure;
  review_queue: ReviewQueue;
}

export interface ReviewQueueSyncResult {
  created: number;
  updated: number;
  unchanged: number;
  expired_snoozes: number;
  queue: ReviewQueue;
}
