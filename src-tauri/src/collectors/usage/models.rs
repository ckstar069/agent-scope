use chrono::{DateTime, Utc};
use serde::Serialize;
use std::path::PathBuf;

// ============================================================================
// Config Dir Discovery
// ============================================================================

/// 候选配置目录
#[derive(Debug, Clone)]
pub struct CandidateConfigDir {
    /// 原始路径（用户输入或默认值）
    pub raw_path: PathBuf,
    /// canonicalize 成功时填充
    pub canonical_path: Option<PathBuf>,
    /// 来源类型
    pub source: ConfigDirSource,
}

/// 配置目录来源
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigDirSource {
    /// ~/.claude
    DefaultClaude,
    /// ~/.config/claude
    DefaultXdg,
    /// Windows 默认目录（待验证）
    DefaultWindows,
    /// CLAUDE_CONFIG_DIR 环境变量
    EnvClaudeConfigDir,
    /// 遗留或全局 usage 源
    LegacyOrGlobal,
}

/// 目录错误原因
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DirErrorReason {
    /// 目录不存在
    NotFound,
    /// 路径存在但不是目录
    NotADirectory,
    /// 权限不足
    PermissionDenied,
    /// 路径格式无效（canonicalize 失败）
    InvalidPath,
    /// 目录存在但不含 sessions/ 或 projects/ 子目录
    MissingStructure,
    /// 目录存在但为空
    Empty,
}

/// 不可读/无效的配置目录
#[derive(Debug, Clone, Serialize)]
pub struct UnreadableDir {
    /// 原始路径
    pub path: PathBuf,
    /// canonicalize 后的路径（如成功）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub canonical_path: Option<PathBuf>,
    /// 错误类型
    pub reason: DirErrorReason,
    /// 额外说明
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

// ============================================================================
// Usage Data Models
// ============================================================================

/// Usage 数据来源
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UsageSource {
    /// Claude Code 项目 transcript JSONL
    ClaudeJsonl,
    /// 遗留或全局 usage 源
    LegacyOrGlobalUsage,
}

/// 单条 usage 记录
#[derive(Debug, Clone, Serialize)]
pub struct UsageRecord {
    /// 数据来源
    pub source: UsageSource,
    /// 数据来源目录
    pub config_dir: PathBuf,
    /// 项目路径（从 JSONL 路径推断）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_path: Option<String>,
    /// 项目名称（与 ProjectRegistry 匹配后填充）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_name: Option<String>,
    /// 会话 ID
    pub session_id: String,
    /// 模型名称
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// usage 记录时间
    pub timestamp: DateTime<Utc>,
    /// 输入 token
    pub input_tokens: u64,
    /// 输出 token
    pub output_tokens: u64,
    /// cache 读取 token
    pub cache_read_tokens: u64,
    /// cache 创建 token
    pub cache_create_tokens: u64,
    /// 总 token
    pub total_tokens: u64,
    /// 原始 JSONL 文件路径
    pub raw_file_path: PathBuf,
    /// 行号（用于调试和对账）
    pub line_no: u64,
}

/// Token 汇总
#[derive(Debug, Clone, Default, Serialize, PartialEq)]
pub struct UsageTotals {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_create_tokens: u64,
    pub total_tokens: u64,
}

// ============================================================================
// Source Status
// ============================================================================

/// 可信度等级
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfidenceLevel {
    /// 最终总量较可靠
    High,
    /// 部分数据可能缺失
    Medium,
    /// 数据源不可读或为空
    Low,
}

/// 实时性等级
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RealtimeLevel {
    /// 基于文件写入，延迟秒级到分钟级
    Delayed,
    /// 未来可能的改进
    NearRealtime,
    /// 未来 API Proxy 才可能实现
    Realtime,
}

/// 数据源状态
#[derive(Debug, Clone, Serialize)]
pub struct UsageSourceStatus {
    /// 来源类型
    pub source_type: String,
    /// 所有已识别目录
    pub config_dirs: Vec<PathBuf>,
    /// 可读目录
    pub readable_dirs: Vec<PathBuf>,
    /// 不可读/无效目录
    pub unreadable_dirs: Vec<UnreadableDir>,
    /// 最近一次扫描时间
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_scan_at: Option<DateTime<Utc>>,
    /// 最近一次发现 usage 记录的时间
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_usage_at: Option<DateTime<Utc>>,
    /// 可信度
    pub confidence: ConfidenceLevel,
    /// 实时性
    pub realtime_level: RealtimeLevel,
    /// 用户可见的提示文本
    pub notes: Vec<String>,
}

// ============================================================================
// Aggregation Models
// ============================================================================

/// 时间范围
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TimeRange {
    /// 今日（本地时区 00:00:00 至今）
    Today,
    /// 最近 7 天
    Last7Days,
}

/// 分组维度
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GroupBy {
    /// 按项目
    Project,
    /// 按模型
    Model,
    /// 按会话
    Session,
}

/// 分组明细
#[derive(Debug, Clone, Serialize)]
pub struct UsageGroup {
    /// 分组键
    pub group_key: String,
    /// 展示标签
    pub group_label: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_create_tokens: u64,
    pub total_tokens: u64,
    pub session_count: usize,
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
}

/// 聚合结果
#[derive(Debug, Clone, Serialize)]
pub struct UsageAggregate {
    /// 时间范围
    pub time_range: TimeRange,
    /// 分组维度
    pub group_by: GroupBy,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_create_tokens: u64,
    pub total_tokens: u64,
    pub session_count: usize,
    pub project_count: usize,
    pub model_count: usize,
    /// 各分组明细
    pub groups: Vec<UsageGroup>,
}

// ============================================================================
// Scan Result
// ============================================================================

/// 扫描结果
#[derive(Debug, Clone)]
pub struct UsageScanResult {
    /// 解析出的 usage 记录
    pub records: Vec<UsageRecord>,
    /// 数据源状态
    pub source_status: UsageSourceStatus,
    /// 扫描的文件数
    pub scanned_files: usize,
    /// 扫描的行数
    pub scanned_lines: usize,
    /// 扫描过程中的非致命错误
    pub errors: Vec<String>,
}

/// 解析上下文
#[derive(Debug, Clone)]
pub struct UsageParseContext {
    pub config_dir: PathBuf,
    pub raw_file_path: PathBuf,
    pub line_no: u64,
    pub session_id_from_file: Option<String>,
    pub project_from_path: Option<String>,
    pub source: UsageSource,
}

/// 解析错误
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UsageParseError {
    /// JSON 解析失败
    InvalidJson(String),
    /// 缺少必要字段
    MissingField(String),
    /// 字段类型不匹配
    InvalidFieldType(String),
}

impl std::fmt::Display for UsageParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UsageParseError::InvalidJson(msg) => write!(f, "JSON 解析失败: {}", msg),
            UsageParseError::MissingField(field) => write!(f, "缺少字段: {}", field),
            UsageParseError::InvalidFieldType(field) => write!(f, "字段类型错误: {}", field),
        }
    }
}

impl std::error::Error for UsageParseError {}
