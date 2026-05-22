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
