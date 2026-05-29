use chrono::{DateTime, Datelike, Local, TimeZone, Utc};
use std::collections::HashMap;

use super::metadata::short_session_id;
use super::models::{
    GroupBy, TimeRange, UsageAggregate, UsageGroup, UsageRecord, UsageTotals,
};

/// 聚合 usage 记录
///
/// `now` 用于计算时间范围边界。建议传入当前时间，测试时可传入固定时间。
pub fn aggregate_usage(
    records: &[UsageRecord],
    time_range: TimeRange,
    group_by: GroupBy,
    now: DateTime<Utc>,
) -> UsageAggregate {
    // 1. 根据时间范围过滤记录
    let filtered: Vec<&UsageRecord> = records
        .iter()
        .filter(|r| is_in_time_range(r.timestamp, time_range, now))
        .collect();

    // 2. 分组聚合
    let mut groups_map: HashMap<String, UsageGroupAccumulator> = HashMap::new();
    let mut session_ids = HashMap::new();
    let mut project_keys = HashMap::new();
    let mut model_keys = HashMap::new();

    for record in &filtered {
        let (group_key, group_label, group_detail) = match group_by {
            GroupBy::Project => {
                let key = record
                    .project_path
                    .clone()
                    .or(record.project_name.clone())
                    .unwrap_or_else(|| "未关联项目".to_string());
                let label = record
                    .project_name
                    .clone()
                    .unwrap_or_else(|| key.clone());
                let detail = record.project_path.clone();
                (key, label, detail)
            }
            GroupBy::Model => {
                let key = record.model.clone().unwrap_or_else(|| "unknown".to_string());
                (key.clone(), key, None)
            }
            GroupBy::Session => {
                let key = record.session_id.clone();
                let label = record
                    .session_title
                    .clone()
                    .unwrap_or_else(|| "(未命名)".to_string());
                let short = short_session_id(&record.session_id);
                let detail = if let Some(ref name) = record.project_name {
                    format!("{} · {}", name, short)
                } else {
                    short
                };
                (key, label, Some(detail))
            }
        };

        let acc = groups_map.entry(group_key.clone()).or_insert_with(|| {
            UsageGroupAccumulator {
                group_key: group_key.clone(),
                group_label,
                group_detail,
                input_tokens: 0,
                output_tokens: 0,
                cache_read_tokens: 0,
                cache_create_tokens: 0,
                total_tokens: 0,
                session_ids: HashMap::new(),
                first_seen: record.timestamp,
                last_seen: record.timestamp,
            }
        });

        acc.input_tokens += record.input_tokens;
        acc.output_tokens += record.output_tokens;
        acc.cache_read_tokens += record.cache_read_tokens;
        acc.cache_create_tokens += record.cache_create_tokens;
        acc.total_tokens += record.total_tokens;
        acc.session_ids.insert(record.session_id.clone(), ());
        if record.timestamp < acc.first_seen {
            acc.first_seen = record.timestamp;
        }
        if record.timestamp > acc.last_seen {
            acc.last_seen = record.timestamp;
        }

        // 统计唯一值
        session_ids.insert(record.session_id.clone(), ());
        if let Some(ref path) = record.project_path {
            project_keys.insert(path.clone(), ());
        }
        if let Some(ref model) = record.model {
            model_keys.insert(model.clone(), ());
        }
    }

    // 3. 构建 groups 列表并排序（按 total_tokens 降序）
    let mut groups: Vec<UsageGroup> = groups_map
        .into_values()
        .map(|acc| UsageGroup {
            group_key: acc.group_key,
            group_label: acc.group_label,
            group_detail: acc.group_detail,
            input_tokens: acc.input_tokens,
            output_tokens: acc.output_tokens,
            cache_read_tokens: acc.cache_read_tokens,
            cache_create_tokens: acc.cache_create_tokens,
            total_tokens: acc.total_tokens,
            session_count: acc.session_ids.len(),
            first_seen: acc.first_seen,
            last_seen: acc.last_seen,
        })
        .collect();

    groups.sort_by(|a, b| b.total_tokens.cmp(&a.total_tokens));

    // 4. 计算总计
    let total_input: u64 = groups.iter().map(|g| g.input_tokens).sum();
    let total_output: u64 = groups.iter().map(|g| g.output_tokens).sum();
    let total_cache_read: u64 = groups.iter().map(|g| g.cache_read_tokens).sum();
    let total_cache_create: u64 = groups.iter().map(|g| g.cache_create_tokens).sum();
    let total_tokens: u64 = groups.iter().map(|g| g.total_tokens).sum();

    UsageAggregate {
        time_range,
        group_by,
        input_tokens: total_input,
        output_tokens: total_output,
        cache_read_tokens: total_cache_read,
        cache_create_tokens: total_cache_create,
        total_tokens,
        session_count: session_ids.len(),
        project_count: project_keys.len(),
        model_count: model_keys.len(),
        groups,
    }
}

/// 判断记录时间是否在指定时间范围内
///
/// P0 实现说明：
/// - Today 使用本地时区（Local），与用户使用习惯一致
/// - 具体为本地日期当天 00:00:00 到当前时间
/// - Last7Days 为当前时间往前 7 天（含今天）
fn is_in_time_range(
    timestamp: DateTime<Utc>,
    time_range: TimeRange,
    now: DateTime<Utc>,
) -> bool {
    match time_range {
        TimeRange::Today => {
            // 使用本地时区计算今日范围
            let local_now = now.with_timezone(&Local);
            let local_today_start = Local
                .with_ymd_and_hms(local_now.year(), local_now.month(), local_now.day(), 0, 0, 0)
                .single()
                .unwrap_or(local_now);
            let today_start_utc = local_today_start.with_timezone(&Utc);
            timestamp >= today_start_utc && timestamp <= now
        }
        TimeRange::Last7Days => {
            let seven_days_ago = now - chrono::Duration::days(7);
            timestamp >= seven_days_ago && timestamp <= now
        }
        TimeRange::All => true,
    }
}

/// 分组聚合累加器（内部使用）
struct UsageGroupAccumulator {
    group_key: String,
    group_label: String,
    group_detail: Option<String>,
    input_tokens: u64,
    output_tokens: u64,
    cache_read_tokens: u64,
    cache_create_tokens: u64,
    total_tokens: u64,
    session_ids: HashMap<String, ()>,
    first_seen: DateTime<Utc>,
    last_seen: DateTime<Utc>,
}

/// 计算 UsageTotals（全量汇总，不考虑时间范围）
pub fn calculate_totals(records: &[UsageRecord]) -> UsageTotals {
    let mut totals = UsageTotals::default();
    for record in records {
        totals.input_tokens += record.input_tokens;
        totals.output_tokens += record.output_tokens;
        totals.cache_read_tokens += record.cache_read_tokens;
        totals.cache_create_tokens += record.cache_create_tokens;
    }
    totals.total_tokens = totals.input_tokens
        + totals.output_tokens
        + totals.cache_read_tokens
        + totals.cache_create_tokens;
    totals
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_record(
        timestamp: DateTime<Utc>,
        session_id: &str,
        model: Option<&str>,
        project: Option<&str>,
        input: u64,
        output: u64,
        cache_read: u64,
        cache_create: u64,
    ) -> UsageRecord {
        UsageRecord {
            source: super::super::models::UsageSource::ClaudeJsonl,
            config_dir: std::path::PathBuf::from("/home/user/.claude"),
            project_path: project.map(|s| s.to_string()),
            project_name: None,
            session_title: None,
            session_id: session_id.to_string(),
            model: model.map(|s| s.to_string()),
            timestamp,
            input_tokens: input,
            output_tokens: output,
            cache_read_tokens: cache_read,
            cache_create_tokens: cache_create,
            total_tokens: input + output + cache_read + cache_create,
            raw_file_path: std::path::PathBuf::from("/test.jsonl"),
            line_no: 1,
        }
    }

    #[test]
    fn test_aggregate_by_project() {
        let now = Utc::now();
        let records = vec![
            make_record(now, "s1", Some("model-a"), Some("project-a"), 100, 50, 10, 5),
            make_record(now, "s2", Some("model-b"), Some("project-a"), 400, 200, 40, 20),
            make_record(now, "s3", Some("model-a"), Some("project-b"), 50, 25, 5, 2),
        ];

        let agg = aggregate_usage(&records, TimeRange::Today, GroupBy::Project, now);

        assert_eq!(agg.groups.len(), 2);
        assert_eq!(agg.input_tokens, 550);
        assert_eq!(agg.output_tokens, 275);
        assert_eq!(agg.session_count, 3);
        assert_eq!(agg.project_count, 2);
        assert_eq!(agg.model_count, 2);

        // 按 total_tokens 降序: project-a (825) > project-b (82)
        assert_eq!(agg.groups[0].group_key, "project-a");
        assert_eq!(agg.groups[0].total_tokens, 825); // 165 + 660
        assert_eq!(agg.groups[1].group_key, "project-b");
        assert_eq!(agg.groups[1].total_tokens, 82);
    }

    #[test]
    fn test_aggregate_by_model() {
        let now = Utc::now();
        let records = vec![
            make_record(now, "s1", Some("model-a"), Some("project-a"), 100, 50, 10, 5),
            make_record(now, "s2", Some("model-a"), Some("project-b"), 200, 100, 20, 10),
            make_record(now, "s3", Some("model-b"), Some("project-a"), 600, 300, 60, 30),
        ];

        let agg = aggregate_usage(&records, TimeRange::Today, GroupBy::Model, now);

        assert_eq!(agg.groups.len(), 2);
        // model-b total = 990, model-a total = 495
        assert_eq!(agg.groups[0].group_key, "model-b");
        assert_eq!(agg.groups[0].total_tokens, 990);
        assert_eq!(agg.groups[1].group_key, "model-a");
        assert_eq!(agg.groups[1].total_tokens, 495);
    }

    #[test]
    fn test_aggregate_by_session() {
        let now = Utc::now();
        let records = vec![
            make_record(now, "s1", Some("model-a"), Some("project-a"), 100, 50, 10, 5),
            make_record(now, "s1", Some("model-a"), Some("project-a"), 200, 100, 20, 10),
            make_record(now, "s2", Some("model-b"), Some("project-b"), 500, 250, 50, 25),
        ];

        let agg = aggregate_usage(&records, TimeRange::Today, GroupBy::Session, now);

        assert_eq!(agg.groups.len(), 2);
        // s2 total = 825, s1 total = 495
        assert_eq!(agg.groups[0].group_key, "s2");
        assert_eq!(agg.groups[0].total_tokens, 825);
        assert_eq!(agg.groups[0].session_count, 1); // 按 session 分组，每个组只有一个 session
        assert_eq!(agg.groups[1].group_key, "s1");
        assert_eq!(agg.groups[1].total_tokens, 495);
    }

    #[test]
    fn test_today_filter() {
        let now = Utc::now();
        let today = now;
        let yesterday = now - chrono::Duration::days(1);

        let records = vec![
            make_record(today, "s1", Some("model-a"), Some("project-a"), 100, 50, 10, 5),
            make_record(yesterday, "s2", Some("model-b"), Some("project-b"), 200, 100, 20, 10),
        ];

        let agg = aggregate_usage(&records, TimeRange::Today, GroupBy::Project, now);

        assert_eq!(agg.groups.len(), 1);
        assert_eq!(agg.groups[0].group_key, "project-a");
        assert_eq!(agg.input_tokens, 100);
    }

    #[test]
    fn test_last7days_filter() {
        let now = Utc::now();
        let six_days_ago = now - chrono::Duration::days(6);
        let eight_days_ago = now - chrono::Duration::days(8);

        let records = vec![
            make_record(six_days_ago, "s1", Some("model-a"), Some("project-a"), 100, 50, 10, 5),
            make_record(eight_days_ago, "s2", Some("model-b"), Some("project-b"), 200, 100, 20, 10),
        ];

        let agg = aggregate_usage(&records, TimeRange::Last7Days, GroupBy::Project, now);

        assert_eq!(agg.groups.len(), 1);
        assert_eq!(agg.groups[0].group_key, "project-a");
    }

    #[test]
    fn test_all_range_no_filter() {
        let now = Utc::now();
        let today = now;
        let yesterday = now - chrono::Duration::days(1);
        let ten_days_ago = now - chrono::Duration::days(10);

        let records = vec![
            make_record(today, "s1", Some("model-a"), Some("project-a"), 100, 50, 10, 5),
            make_record(yesterday, "s2", Some("model-b"), Some("project-b"), 200, 100, 20, 10),
            make_record(ten_days_ago, "s3", Some("model-c"), Some("project-c"), 50, 25, 5, 2),
        ];

        let agg = aggregate_usage(&records, TimeRange::All, GroupBy::Project, now);

        // All 不过滤时间范围，应包含所有 3 条记录
        assert_eq!(agg.groups.len(), 3);
        assert_eq!(agg.input_tokens, 350);
        assert_eq!(agg.session_count, 3);
    }

    #[test]
    fn test_aggregate_unknown_project() {
        let now = Utc::now();
        let records = vec![
            make_record(now, "s1", Some("model-a"), Some("project-a"), 100, 50, 10, 5),
            make_record(now, "s2", Some("model-b"), None, 200, 100, 20, 10),
        ];

        let agg = aggregate_usage(&records, TimeRange::Today, GroupBy::Project, now);

        assert_eq!(agg.groups.len(), 2);
        let unknown = agg
            .groups
            .iter()
            .find(|g| g.group_key == "未关联项目")
            .expect("应有未关联项目组");
        assert_eq!(unknown.total_tokens, 330);
    }

    #[test]
    fn test_aggregate_unknown_model() {
        let now = Utc::now();
        let records = vec![
            make_record(now, "s1", Some("model-a"), Some("project-a"), 100, 50, 10, 5),
            make_record(now, "s2", None, Some("project-b"), 200, 100, 20, 10),
        ];

        let agg = aggregate_usage(&records, TimeRange::Today, GroupBy::Model, now);

        let unknown = agg
            .groups
            .iter()
            .find(|g| g.group_key == "unknown")
            .expect("应有 unknown 模型组");
        assert_eq!(unknown.total_tokens, 330);
    }

    #[test]
    fn test_calculate_totals() {
        let now = Utc::now();
        let records = vec![
            make_record(now, "s1", Some("model-a"), Some("project-a"), 100, 50, 10, 5),
            make_record(now, "s2", Some("model-b"), Some("project-b"), 200, 100, 20, 10),
        ];

        let totals = calculate_totals(&records);
        assert_eq!(totals.input_tokens, 300);
        assert_eq!(totals.output_tokens, 150);
        assert_eq!(totals.cache_read_tokens, 30);
        assert_eq!(totals.cache_create_tokens, 15);
        assert_eq!(totals.total_tokens, 495);
    }

    #[test]
    fn test_aggregate_empty_records() {
        let now = Utc::now();
        let records: Vec<UsageRecord> = vec![];

        let agg = aggregate_usage(&records, TimeRange::Today, GroupBy::Project, now);

        assert_eq!(agg.groups.len(), 0);
        assert_eq!(agg.input_tokens, 0);
        assert_eq!(agg.session_count, 0);
        assert_eq!(agg.project_count, 0);
        assert_eq!(agg.model_count, 0);
    }

    #[test]
    fn test_groups_sorted_by_total_tokens_desc() {
        let now = Utc::now();
        let records = vec![
            make_record(now, "s1", Some("model-a"), Some("small-project"), 10, 5, 1, 1),
            make_record(now, "s2", Some("model-b"), Some("big-project"), 1000, 500, 100, 50),
            make_record(now, "s3", Some("model-c"), Some("medium-project"), 100, 50, 10, 5),
        ];

        let agg = aggregate_usage(&records, TimeRange::Today, GroupBy::Project, now);

        assert_eq!(agg.groups[0].group_key, "big-project");
        assert_eq!(agg.groups[1].group_key, "medium-project");
        assert_eq!(agg.groups[2].group_key, "small-project");
    }
}
