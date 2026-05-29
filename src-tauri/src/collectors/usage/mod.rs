//! Usage Analytics Collector
//!
//! 负责扫描 Claude Code 本地 usage 数据、解析 JSONL、聚合统计。
//!
//! 主扫描路径: `{config_dir}/projects/**/*.jsonl`
//! 每个 `.jsonl` 文件对应一个 session 的完整 transcript。
//!
//! 数据源:
//! - Claude Code 本地 JSONL usage 记录 (主)
//! - 可选兼容: `{config_dir}/usage.jsonl` (legacy)

pub mod aggregate;
pub mod models;
pub mod parser;
pub mod scanner;

pub use aggregate::{aggregate_usage, calculate_totals};
pub use models::{
    CandidateConfigDir, ConfigDirSource, ConfidenceLevel, DirErrorReason, GroupBy, RealtimeLevel,
    TimeRange, UnreadableDir, UsageAggregate, UsageGroup, UsageParseContext, UsageParseError,
    UsageRecord, UsageScanResult, UsageSource, UsageSourceStatus, UsageTotals,
};
pub use parser::parse_usage_line;
pub use scanner::{discover_claude_config_dirs, scan_usage_data};
