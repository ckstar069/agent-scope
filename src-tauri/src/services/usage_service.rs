use chrono::{DateTime, Utc};

use crate::collectors::usage::{
    aggregate_usage, discover_claude_config_dirs, scan_usage_data,
};
use crate::collectors::usage::models::{
    GroupBy, TimeRange, UsageAggregate, UsageScanResult, UsageSourceStatus,
};

/// Usage 扫描摘要（轻量 DTO，避免返回大量原始 records）
#[derive(Debug, Clone, serde::Serialize)]
pub struct UsageScanSummary {
    pub source_status: UsageSourceStatus,
    pub scanned_files: usize,
    pub scanned_lines: usize,
    pub record_count: usize,
    pub error_count: usize,
    /// 最多保留前 20 条错误，避免 payload 过大
    pub errors: Vec<String>,
}

impl From<&UsageScanResult> for UsageScanSummary {
    fn from(result: &UsageScanResult) -> Self {
        let mut errors = result.errors.clone();
        errors.truncate(20);
        Self {
            source_status: result.source_status.clone(),
            scanned_files: result.scanned_files,
            scanned_lines: result.scanned_lines,
            record_count: result.records.len(),
            error_count: result.errors.len(),
            errors,
        }
    }
}

/// Usage 数据服务
///
/// 负责扫描 Claude Code 本地 usage 数据、维护缓存、提供聚合分析。
pub struct UsageService {
    last_result: Option<UsageScanResult>,
    last_scan_at: Option<DateTime<Utc>>,
}

impl UsageService {
    pub fn new() -> Self {
        Self {
            last_result: None,
            last_scan_at: None,
        }
    }

    /// 执行完整扫描，更新缓存并返回结果
    pub fn scan(&mut self) -> &UsageScanResult {
        let dirs = discover_claude_config_dirs();
        let result = scan_usage_data(&dirs);
        self.last_scan_at = Some(Utc::now());
        self.last_result = Some(result);
        self.last_result.as_ref().unwrap()
    }

    /// 确保已有缓存；如果没有则触发扫描
    fn ensure_scanned(&mut self) -> &UsageScanResult {
        match self.last_result {
            None => self.scan(),
            Some(_) => self.last_result.as_ref().unwrap(),
        }
    }

    /// 返回数据源状态
    ///
    /// 如果没有缓存，会触发一次扫描以确保状态可用。
    pub fn source_status(&mut self) -> UsageSourceStatus {
        self.ensure_scanned().source_status.clone()
    }

    /// 返回扫描摘要（轻量 DTO）
    ///
    /// 如果没有缓存，会触发一次扫描。
    pub fn scan_summary(&mut self) -> UsageScanSummary {
        UsageScanSummary::from(self.ensure_scanned())
    }

    /// 返回 usage 分析聚合结果
    ///
    /// 如果没有缓存，会触发一次扫描。
    pub fn analytics(&mut self, time_range: TimeRange, group_by: GroupBy) -> UsageAggregate {
        let records = &self.ensure_scanned().records;
        aggregate_usage(records, time_range, group_by, Utc::now())
    }

    /// 最后一次扫描时间
    pub fn last_scan_at(&self) -> Option<DateTime<Utc>> {
        self.last_scan_at
    }
}

impl Default for UsageService {
    fn default() -> Self {
        Self::new()
    }
}

/// 解析 time_range 字符串参数
pub fn parse_time_range(s: &str) -> Result<TimeRange, String> {
    match s {
        "today" => Ok(TimeRange::Today),
        "last7days" => Ok(TimeRange::Last7Days),
        _ => Err(format!(
            "无效 time_range: '{}', 允许值: 'today', 'last7days'",
            s
        )),
    }
}

/// 解析 group_by 字符串参数
pub fn parse_group_by(s: &str) -> Result<GroupBy, String> {
    match s {
        "project" => Ok(GroupBy::Project),
        "model" => Ok(GroupBy::Model),
        "session" => Ok(GroupBy::Session),
        _ => Err(format!(
            "无效 group_by: '{}', 允许值: 'project', 'model', 'session'",
            s
        )),
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn create_test_config_dir() -> (tempfile::TempDir, std::path::PathBuf) {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().to_path_buf();
        let projects_dir = path.join("projects").join("test-project");
        std::fs::create_dir_all(&projects_dir).unwrap();

        let jsonl_path = projects_dir.join("550e8400-e29b-41d4-a716-446655440000.jsonl");
        let mut file = std::fs::File::create(&jsonl_path).unwrap();
        writeln!(
            file,
            r#"{{"type":"assistant","timestamp":"2026-05-27T01:40:41.560Z","sessionId":"550e8400-e29b-41d4-a716-446655440000","message":{{"model":"claude-sonnet-4-6","usage":{{"input_tokens":100,"output_tokens":50,"cache_read_input_tokens":20,"cache_creation_input_tokens":10}},"stop_reason":"end_turn"}}}}"#
        ).unwrap();

        (temp_dir, path)
    }

    #[test]
    fn test_usage_service_initial_state() {
        let service = UsageService::new();
        assert!(service.last_scan_at().is_none());
        assert!(service.last_result.is_none());
    }

    #[test]
    fn test_usage_service_scan_creates_cache() {
        let _guard = crate::collectors::usage::scanner::ENV_LOCK.lock().unwrap();
        let (_temp_dir, config_path) = create_test_config_dir();

        std::env::set_var("CLAUDE_CONFIG_DIR", config_path.to_string_lossy().to_string());

        let mut service = UsageService::new();
        let result = service.scan();

        assert!(!result.records.is_empty());
        assert!(service.last_scan_at().is_some());

        std::env::remove_var("CLAUDE_CONFIG_DIR");
    }

    #[test]
    fn test_usage_service_analytics_triggers_scan_when_empty() {
        let _guard = crate::collectors::usage::scanner::ENV_LOCK.lock().unwrap();
        let (_temp_dir, config_path) = create_test_config_dir();

        std::env::set_var("CLAUDE_CONFIG_DIR", config_path.to_string_lossy().to_string());

        let mut service = UsageService::new();
        // 不手动 scan，直接调用 analytics
        let agg = service.analytics(TimeRange::Today, GroupBy::Project);

        assert!(service.last_scan_at().is_some());
        // 测试目录数据应被扫描到；真实 ~/.claude 数据也可能被包含
        assert!(!agg.groups.is_empty(), "应至少有一个聚合组");

        std::env::remove_var("CLAUDE_CONFIG_DIR");
    }

    #[test]
    fn test_parse_time_range_valid() {
        assert_eq!(parse_time_range("today").unwrap(), TimeRange::Today);
        assert_eq!(parse_time_range("last7days").unwrap(), TimeRange::Last7Days);
    }

    #[test]
    fn test_parse_time_range_invalid() {
        assert!(parse_time_range("invalid").is_err());
        assert!(parse_time_range("").is_err());
    }

    #[test]
    fn test_parse_group_by_valid() {
        assert_eq!(parse_group_by("project").unwrap(), GroupBy::Project);
        assert_eq!(parse_group_by("model").unwrap(), GroupBy::Model);
        assert_eq!(parse_group_by("session").unwrap(), GroupBy::Session);
    }

    #[test]
    fn test_parse_group_by_invalid() {
        assert!(parse_group_by("invalid").is_err());
        assert!(parse_group_by("").is_err());
    }

    #[test]
    fn test_scan_summary_limits_errors() {
        let result = UsageScanResult {
            records: vec![],
            source_status: UsageSourceStatus {
                source_type: "test".to_string(),
                config_dirs: vec![],
                readable_dirs: vec![],
                unreadable_dirs: vec![],
                last_scan_at: None,
                last_usage_at: None,
                confidence: crate::collectors::usage::models::ConfidenceLevel::Low,
                realtime_level: crate::collectors::usage::models::RealtimeLevel::Delayed,
                notes: vec![],
            },
            scanned_files: 0,
            scanned_lines: 0,
            errors: (0..30).map(|i| format!("error {}", i)).collect(),
        };

        let summary = UsageScanSummary::from(&result);
        assert_eq!(summary.errors.len(), 20);
        assert_eq!(summary.error_count, 30); // 总数仍然是 30
    }
}
