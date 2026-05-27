use super::models::{
    SerClaudeMemoryAsset, SerContextPressure, SerPressureAlert, SerPressureHeavyAsset,
};

/// 预估 token 上限（Claude 上下文窗口参考值）
const TOKEN_BUDGET: f64 = 200_000.0;
/// 重资产行数阈值
const HEAVY_LINES_THRESHOLD: usize = 200;
/// 重资产字节阈值
const HEAVY_BYTES_THRESHOLD: u64 = 100_000;
/// warning 级别总代码行数阈值
const WARNING_LINES_THRESHOLD: usize = 5_000;
/// critical 级别总代码行数阈值
const CRITICAL_LINES_THRESHOLD: usize = 10_000;
/// warning 级别重资产数量阈值
const WARNING_HEAVY_ASSETS_THRESHOLD: usize = 3;
/// critical 级别重资产数量阈值
const CRITICAL_HEAVY_ASSETS_THRESHOLD: usize = 10;
/// warning 级别压力比例阈值
const WARNING_RATIO_THRESHOLD: f64 = 0.3;
/// critical 级别压力比例阈值
const CRITICAL_RATIO_THRESHOLD: f64 = 0.6;

/// 计算上下文压力报告
pub fn compute_context_pressure(assets: &[SerClaudeMemoryAsset]) -> SerContextPressure {
    let total_assets = assets.len();
    let existing_assets: Vec<&SerClaudeMemoryAsset> = assets.iter().filter(|a| a.exists).collect();
    let existing_count = existing_assets.len();

    let total_lines: usize = existing_assets.iter().filter_map(|a| a.line_count).sum();

    let total_bytes: u64 = existing_assets.iter().filter_map(|a| a.byte_size).sum();

    // 保守估算：每字节约 0.5 个 token
    let estimated_tokens = ((total_bytes as f64) / 2.0).ceil() as usize;
    let pressure_ratio = estimated_tokens as f64 / TOKEN_BUDGET;

    // 收集重资产
    let heavy_assets: Vec<SerPressureHeavyAsset> = existing_assets
        .iter()
        .filter(|a| {
            let is_heavy_lines = a.line_count.is_some_and(|l| l >= HEAVY_LINES_THRESHOLD);
            let is_heavy_bytes = a.byte_size.is_some_and(|b| b >= HEAVY_BYTES_THRESHOLD);
            is_heavy_lines || is_heavy_bytes
        })
        .map(|a| SerPressureHeavyAsset {
            asset_id: a.id.clone(),
            asset_type: a.asset_type.clone(),
            logical_path: a.logical_path.clone(),
            line_count: a.line_count,
            byte_size: a.byte_size,
        })
        .collect();

    // 生成 alerts
    let mut alerts: Vec<SerPressureAlert> = Vec::new();

    // pressure_ratio alert
    if pressure_ratio >= CRITICAL_RATIO_THRESHOLD {
        alerts.push(SerPressureAlert {
            metric: "pressure_ratio".to_string(),
            current: pressure_ratio,
            threshold: CRITICAL_RATIO_THRESHOLD,
            severity: "critical".to_string(),
            message: format!(
                "估算 token 占用 {:.0}% ({:.0}K / {:.0}K)，建议立即审查记忆文件",
                pressure_ratio * 100.0,
                estimated_tokens as f64 / 1000.0,
                TOKEN_BUDGET / 1000.0
            ),
        });
    } else if pressure_ratio >= WARNING_RATIO_THRESHOLD {
        alerts.push(SerPressureAlert {
            metric: "pressure_ratio".to_string(),
            current: pressure_ratio,
            threshold: WARNING_RATIO_THRESHOLD,
            severity: "warning".to_string(),
            message: format!(
                "估算 token 占用 {:.0}% ({:.0}K / {:.0}K)，建议审查是否有冗余",
                pressure_ratio * 100.0,
                estimated_tokens as f64 / 1000.0,
                TOKEN_BUDGET / 1000.0
            ),
        });
    }

    // total_lines alert
    if total_lines >= CRITICAL_LINES_THRESHOLD {
        alerts.push(SerPressureAlert {
            metric: "total_lines".to_string(),
            current: total_lines as f64,
            threshold: CRITICAL_LINES_THRESHOLD as f64,
            severity: "critical".to_string(),
            message: format!(
                "记忆文件总行数 {}，超过 {} 行建议拆分",
                total_lines, CRITICAL_LINES_THRESHOLD
            ),
        });
    } else if total_lines >= WARNING_LINES_THRESHOLD {
        alerts.push(SerPressureAlert {
            metric: "total_lines".to_string(),
            current: total_lines as f64,
            threshold: WARNING_LINES_THRESHOLD as f64,
            severity: "warning".to_string(),
            message: format!(
                "记忆文件总行数 {}，超过 {} 行建议审查",
                total_lines, WARNING_LINES_THRESHOLD
            ),
        });
    }

    // heavy_assets alert
    if heavy_assets.len() >= CRITICAL_HEAVY_ASSETS_THRESHOLD {
        alerts.push(SerPressureAlert {
            metric: "heavy_assets".to_string(),
            current: heavy_assets.len() as f64,
            threshold: CRITICAL_HEAVY_ASSETS_THRESHOLD as f64,
            severity: "critical".to_string(),
            message: format!(
                "发现 {} 个重资产（≥{} 行或 ≥{}KB），建议拆分或归档",
                heavy_assets.len(),
                HEAVY_LINES_THRESHOLD,
                HEAVY_BYTES_THRESHOLD / 1000
            ),
        });
    } else if heavy_assets.len() >= WARNING_HEAVY_ASSETS_THRESHOLD {
        alerts.push(SerPressureAlert {
            metric: "heavy_assets".to_string(),
            current: heavy_assets.len() as f64,
            threshold: WARNING_HEAVY_ASSETS_THRESHOLD as f64,
            severity: "warning".to_string(),
            message: format!(
                "发现 {} 个重资产（≥{} 行或 ≥{}KB），建议审查",
                heavy_assets.len(),
                HEAVY_LINES_THRESHOLD,
                HEAVY_BYTES_THRESHOLD / 1000
            ),
        });
    }

    // 判定 level
    let level = if pressure_ratio >= CRITICAL_RATIO_THRESHOLD
        || total_lines >= CRITICAL_LINES_THRESHOLD
        || heavy_assets.len() >= CRITICAL_HEAVY_ASSETS_THRESHOLD
    {
        "critical"
    } else if pressure_ratio >= WARNING_RATIO_THRESHOLD
        || total_lines >= WARNING_LINES_THRESHOLD
        || heavy_assets.len() >= WARNING_HEAVY_ASSETS_THRESHOLD
    {
        "warning"
    } else {
        "normal"
    }
    .to_string();

    SerContextPressure {
        total_assets,
        existing_assets: existing_count,
        total_lines,
        total_bytes,
        estimated_tokens,
        pressure_ratio,
        level,
        heavy_assets,
        alerts,
    }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::super::models::SerClaudeMemoryAsset;
    use super::*;

    fn make_asset(
        id: &str,
        asset_type: &str,
        exists: bool,
        line_count: Option<usize>,
        byte_size: Option<u64>,
    ) -> SerClaudeMemoryAsset {
        SerClaudeMemoryAsset {
            id: id.to_string(),
            scope: "project".to_string(),
            asset_type: asset_type.to_string(),
            logical_path: format!("/test/{}", id),
            native_path: format!("/test/{}", id),
            content_hash: None,
            content_preview: None,
            content_truncated: false,
            line_count,
            byte_size,
            mtime_ms: None,
            frontmatter: None,
            secret_issues: Vec::new(),
            exists,
        }
    }

    #[test]
    fn test_empty_assets_normal() {
        let pressure = compute_context_pressure(&[]);
        assert_eq!(pressure.total_assets, 0);
        assert_eq!(pressure.existing_assets, 0);
        assert_eq!(pressure.total_lines, 0);
        assert_eq!(pressure.total_bytes, 0);
        assert_eq!(pressure.estimated_tokens, 0);
        assert_eq!(pressure.pressure_ratio, 0.0);
        assert_eq!(pressure.level, "normal");
        assert!(pressure.alerts.is_empty());
        assert!(pressure.heavy_assets.is_empty());
    }

    #[test]
    fn test_normal_small_project() {
        let assets = vec![
            make_asset("a1", "project_claude_md", true, Some(50), Some(5_000)),
            make_asset("a2", "global_rule", true, Some(30), Some(3_000)),
            make_asset("a3", "auto_memory_index", true, Some(20), Some(2_000)),
        ];
        let pressure = compute_context_pressure(&assets);
        assert_eq!(pressure.total_assets, 3);
        assert_eq!(pressure.existing_assets, 3);
        assert_eq!(pressure.total_lines, 100);
        assert_eq!(pressure.total_bytes, 10_000);
        assert_eq!(pressure.level, "normal");
        assert!(pressure.alerts.is_empty());
    }

    #[test]
    fn test_warning_by_total_lines() {
        // 60 个资产，每个 100 行 = 6000 行，超过 5000 警告线
        // bytes 设小一点避免触发 pressure_ratio critical
        let assets: Vec<SerClaudeMemoryAsset> = (0..60)
            .map(|i| {
                make_asset(
                    &format!("a{}", i),
                    "project_rule",
                    true,
                    Some(100),
                    Some(1_000),
                )
            })
            .collect();
        let pressure = compute_context_pressure(&assets);
        assert_eq!(pressure.total_lines, 6_000);
        assert_eq!(pressure.level, "warning");
        let line_alert = pressure.alerts.iter().find(|a| a.metric == "total_lines");
        assert!(line_alert.is_some(), "应有 total_lines warning alert");
        assert_eq!(line_alert.unwrap().severity, "warning");
    }

    #[test]
    fn test_critical_by_pressure_ratio() {
        // 每个 200KB，120 个资产 = 24MB = 24000K bytes
        // estimated_tokens = 24000K / 2 = 12000K
        // pressure_ratio = 12000K / 200K = 60.0 → 远远超过 0.6 critical 线
        let assets: Vec<SerClaudeMemoryAsset> = (0..120)
            .map(|i| {
                make_asset(
                    &format!("a{}", i),
                    "project_rule",
                    true,
                    Some(100),
                    Some(200_000),
                )
            })
            .collect();
        let pressure = compute_context_pressure(&assets);
        assert!(
            pressure.pressure_ratio >= CRITICAL_RATIO_THRESHOLD,
            "pressure_ratio 应达到 critical: {}",
            pressure.pressure_ratio
        );
        assert_eq!(pressure.level, "critical");
        let ratio_alert = pressure
            .alerts
            .iter()
            .find(|a| a.metric == "pressure_ratio");
        assert!(ratio_alert.is_some(), "应有 pressure_ratio critical alert");
        assert_eq!(ratio_alert.unwrap().severity, "critical");
    }

    #[test]
    fn test_heavy_assets_detection() {
        let assets = vec![
            make_asset("small", "project_claude_md", true, Some(50), Some(5_000)),
            make_asset("heavy_lines", "project_rule", true, Some(250), Some(50_000)),
            make_asset(
                "heavy_bytes",
                "global_skill",
                true,
                Some(100),
                Some(150_000),
            ),
            make_asset(
                "both_heavy",
                "auto_memory_index",
                true,
                Some(300),
                Some(200_000),
            ),
            make_asset(
                "missing",
                "project_claude_md",
                false,
                Some(500),
                Some(500_000),
            ),
        ];
        let pressure = compute_context_pressure(&assets);
        // 不存在的资产不应计入
        assert_eq!(pressure.heavy_assets.len(), 3);
        let ids: Vec<&str> = pressure
            .heavy_assets
            .iter()
            .map(|h| h.asset_id.as_str())
            .collect();
        assert!(ids.contains(&"heavy_lines"));
        assert!(ids.contains(&"heavy_bytes"));
        assert!(ids.contains(&"both_heavy"));
        assert!(!ids.contains(&"small"));
        assert!(!ids.contains(&"missing"));
    }

    #[test]
    fn test_warning_by_heavy_assets_count() {
        // 5 个重资产（行数 ≥200 触发 heavy），bytes 设小避免触发 pressure_ratio critical
        let assets: Vec<SerClaudeMemoryAsset> = (0..5)
            .map(|i| {
                make_asset(
                    &format!("heavy{}", i),
                    "project_rule",
                    true,
                    Some(250),
                    Some(1_000),
                )
            })
            .collect();
        let pressure = compute_context_pressure(&assets);
        assert_eq!(pressure.heavy_assets.len(), 5);
        assert_eq!(pressure.level, "warning");
        let heavy_alert = pressure.alerts.iter().find(|a| a.metric == "heavy_assets");
        assert!(heavy_alert.is_some(), "应有 heavy_assets warning alert");
        assert_eq!(heavy_alert.unwrap().severity, "warning");
    }

    #[test]
    fn test_nonexistent_assets_excluded() {
        let assets = vec![
            make_asset("exists", "project_claude_md", true, Some(100), Some(10_000)),
            make_asset(
                "missing",
                "project_claude_md",
                false,
                Some(500),
                Some(500_000),
            ),
        ];
        let pressure = compute_context_pressure(&assets);
        assert_eq!(pressure.existing_assets, 1);
        assert_eq!(pressure.total_lines, 100);
        assert_eq!(pressure.total_bytes, 10_000);
        assert_eq!(pressure.heavy_assets.len(), 0);
    }

    #[test]
    fn test_critical_lines_and_ratio_combined() {
        // 150 个资产，每个 100 行 200KB → 15000 行 + pressure_ratio > 0.6
        let assets: Vec<SerClaudeMemoryAsset> = (0..150)
            .map(|i| {
                make_asset(
                    &format!("a{}", i),
                    "project_rule",
                    true,
                    Some(100),
                    Some(200_000),
                )
            })
            .collect();
        let pressure = compute_context_pressure(&assets);
        assert_eq!(pressure.total_lines, 15_000);
        assert!(pressure.pressure_ratio >= CRITICAL_RATIO_THRESHOLD);
        assert_eq!(pressure.level, "critical");
        // 应同时有 lines 和 ratio 两个 critical alerts
        let critical_alerts: Vec<&SerPressureAlert> = pressure
            .alerts
            .iter()
            .filter(|a| a.severity == "critical")
            .collect();
        assert!(
            critical_alerts.len() >= 2,
            "应至少有 2 个 critical alert: {:?}",
            critical_alerts
        );
    }
}
