use serde::Serialize;
use std::collections::HashMap;

/// 单次扫描结果（v0.1 实时生成，无服务端缓存）
#[derive(Debug, Clone, Serialize)]
pub struct SerClaudeMemoryScanResult {
    pub scanned_at_ms: u64,
    pub host_profile: SerHostProfile,
    pub assets: Vec<SerClaudeMemoryAsset>,
    pub summary: SerMemorySummary,
    pub errors: Vec<SerMemoryScanError>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SerHostProfile {
    pub host_id: String,
    pub hostname: String,
    pub os: String,
    pub home_dir: String,
    pub claude_config_dir: String,
    pub user_name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SerClaudeMemoryAsset {
    pub id: String,
    pub scope: String,
    pub asset_type: String,
    pub logical_path: String,
    pub native_path: String,
    pub content_hash: Option<String>,
    pub content_preview: Option<String>,
    pub content_truncated: bool,
    pub line_count: Option<usize>,
    pub byte_size: Option<u64>,
    pub mtime_ms: Option<u64>,
    pub frontmatter: Option<SerFrontmatter>,
    pub secret_issues: Vec<SerSecretIssue>,
    pub exists: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct SerFrontmatter {
    pub name: Option<String>,
    pub description: Option<String>,
    pub trigger: Option<String>,
    pub paths: Option<Vec<String>>,
    pub memory_scope: Option<String>,
    pub raw: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SerSecretIssue {
    pub issue_type: String,
    pub line_number: usize,
    pub column_start: usize,
    pub column_end: usize,
    pub matched_text: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SerMemorySummary {
    pub total_assets: usize,
    pub total_existing: usize,
    pub by_scope: HashMap<String, usize>,
    pub by_type: HashMap<String, usize>,
    pub total_secret_issues: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct SerMemoryScanError {
    pub scope: String,
    pub path: String,
    pub message: String,
}

// ============================================================================
// P1: Load Chain Simulation 数据结构
// ============================================================================

/// 加载链模拟结果
#[derive(Debug, Clone, Serialize)]
pub struct SerLoadChain {
    pub cwd: String,
    pub host_profile: SerHostProfile,
    pub startup_chain: Vec<SerLoadChainStep>,
    pub path_scoped_rules: Vec<SerPathScopedRule>,
    pub excluded_assets: Vec<SerExcludedAsset>,
    pub warnings: Vec<SerLoadChainWarning>,
}

/// 启动链中的单个步骤
#[derive(Debug, Clone, Serialize)]
pub struct SerLoadChainStep {
    pub order: usize,
    pub scope: String,
    pub asset_type: String,
    pub native_path: String,
    pub logical_path: String,
    pub load_reason: String,
    pub line_count: Option<usize>,
    pub byte_size: Option<u64>,
    pub content_preview: Option<String>,
    pub content_truncated: bool,
    pub exists: bool,
}

/// 可能触发的 path-scoped rule（不在启动链中）
#[derive(Debug, Clone, Serialize)]
pub struct SerPathScopedRule {
    pub scope: String,
    pub native_path: String,
    pub logical_path: String,
    pub name: Option<String>,
    pub paths: Vec<String>,
    pub exists: bool,
}

/// 被排除的资产（含排除来源）
#[derive(Debug, Clone, Serialize)]
pub struct SerExcludedAsset {
    pub native_path: String,
    pub logical_path: String,
    pub scope: String,
    pub excluded_by: String,
    pub pattern: String,
}

/// 加载链警告
#[derive(Debug, Clone, Serialize)]
pub struct SerLoadChainWarning {
    pub level: String, // "warning" | "info"
    pub code: String,
    pub message: String,
}

/// claudeMdExcludes 配置
#[derive(Debug, Clone, Serialize)]
pub struct ClaudeMdExcludesConfig {
    pub patterns: Vec<ExcludePattern>,
    pub managed_accessible: Option<bool>,
}

/// 单个排除模式
#[derive(Debug, Clone, Serialize)]
pub struct ExcludePattern {
    pub pattern: String,
    pub source: String,
}

// ============================================================================
// Memory Health Phase 1 数据结构
// ============================================================================

/// 记忆健康报告
#[derive(Debug, Clone, Serialize)]
pub struct SerMemoryHealthReport {
    pub overall_score: u8,
    pub freshness: SerHealthDimension,
    pub quality: SerHealthDimension,
    pub coverage: SerHealthDimension,
    pub cleanliness: SerHealthDimension,
    pub safety: SerHealthDimension,
    pub top_issues: Vec<SerMemoryHealthIssue>,
    pub stale_assets: Vec<SerMemoryStaleness>,
    pub duplicate_groups: Vec<SerMemoryDuplicateGroup>,
}

/// 健康维度评分
#[derive(Debug, Clone, Serialize)]
pub struct SerHealthDimension {
    pub name: String,
    pub score: u8,
    pub reason: String,
    pub contributing_assets: Vec<String>,
}

/// 健康问题
#[derive(Debug, Clone, Serialize)]
pub struct SerMemoryHealthIssue {
    pub issue_type: String,
    pub severity: String,
    pub asset_ids: Vec<String>,
    pub message: String,
    pub suggestion: String,
}

/// 过期资产信息
#[derive(Debug, Clone, Serialize)]
pub struct SerMemoryStaleness {
    pub asset_id: String,
    pub asset_type: String,
    pub scope: String,
    pub logical_path: String,
    pub mtime_ms: Option<u64>,
    pub stale_days: Option<u64>,
    pub threshold_days: u64,
}

/// 重复资产组
#[derive(Debug, Clone, Serialize)]
pub struct SerMemoryDuplicateGroup {
    pub group_id: String,
    pub asset_ids: Vec<String>,
    pub similarity: f64,
    pub overlap_content: String,
    pub suggestion: String,
}
