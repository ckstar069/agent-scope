use super::dedup::find_duplicates;
use super::models::*;

/// 默认过期阈值（毫秒）
const DEFAULT_STALE_THRESHOLD_MS: u64 = 30 * 24 * 60 * 60 * 1000; // 30 天
/// Auto Memory 专项过期阈值（毫秒）
const AUTO_MEMORY_STALE_THRESHOLD_MS: u64 = 14 * 24 * 60 * 60 * 1000; // 14 天

/// 计算记忆健康报告
pub fn compute_health_report(assets: &[SerClaudeMemoryAsset]) -> SerMemoryHealthReport {
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    // 过期检测
    let stale_assets = compute_staleness(assets, now_ms);

    // 重复检测
    let duplicate_groups = find_duplicates(assets);

    // 五维度评分
    let freshness = compute_freshness(assets, &stale_assets);
    let quality = compute_quality(assets);
    let coverage = compute_coverage(assets);
    let cleanliness = compute_cleanliness(assets, &duplicate_groups);
    let safety = compute_safety(assets);

    // 加权汇总
    let overall_score = (freshness.score as f64 * 0.2
        + quality.score as f64 * 0.2
        + coverage.score as f64 * 0.2
        + cleanliness.score as f64 * 0.25
        + safety.score as f64 * 0.15)
        .round()
        .clamp(0.0, 100.0) as u8;

    // Top issues
    let top_issues = collect_top_issues(assets, &stale_assets, &duplicate_groups);

    SerMemoryHealthReport {
        overall_score,
        freshness,
        quality,
        coverage,
        cleanliness,
        safety,
        top_issues,
        stale_assets,
        duplicate_groups,
    }
}

/// 计算过期资产列表
pub fn compute_staleness(assets: &[SerClaudeMemoryAsset], now_ms: u64) -> Vec<SerMemoryStaleness> {
    let mut stale_list = Vec::new();

    for asset in assets {
        if !asset.exists {
            continue;
        }

        let threshold_ms = if asset.asset_type.starts_with("auto_memory") {
            AUTO_MEMORY_STALE_THRESHOLD_MS
        } else {
            DEFAULT_STALE_THRESHOLD_MS
        };
        let threshold_days = threshold_ms / (24 * 60 * 60 * 1000);

        match asset.mtime_ms {
            Some(mtime) => {
                if mtime > now_ms {
                    // mtime 在未来，跳过
                    continue;
                }
                let age_ms = now_ms - mtime;
                if age_ms > threshold_ms {
                    let stale_days = age_ms / (24 * 60 * 60 * 1000);
                    stale_list.push(SerMemoryStaleness {
                        asset_id: asset.id.clone(),
                        asset_type: asset.asset_type.clone(),
                        scope: asset.scope.clone(),
                        logical_path: asset.logical_path.clone(),
                        mtime_ms: Some(mtime),
                        stale_days: Some(stale_days),
                        threshold_days,
                    });
                }
            }
            None => {
                // mtime 不可用，不标记为 stale
            }
        }
    }

    // 按 stale_days 降序排列
    stale_list.sort_by(|a, b| b.stale_days.cmp(&a.stale_days));
    stale_list
}

// ============================================================================
// 维度评分
// ============================================================================

/// Freshness（新鲜度）
fn compute_freshness(
    assets: &[SerClaudeMemoryAsset],
    stale_assets: &[SerMemoryStaleness],
) -> SerHealthDimension {
    let existing: Vec<&SerClaudeMemoryAsset> = assets.iter().filter(|a| a.exists).collect();
    // 只计算有 mtime 的资产
    let eligible: Vec<&SerClaudeMemoryAsset> = existing
        .iter()
        .filter(|a| a.mtime_ms.is_some())
        .copied()
        .collect();

    if eligible.is_empty() {
        return SerHealthDimension {
            name: "freshness".to_string(),
            score: 100,
            reason: "无可计算新鲜度的资产".to_string(),
            contributing_assets: Vec::new(),
        };
    }

    let stale_count = stale_assets.len();
    let stale_ratio = stale_count as f64 / eligible.len() as f64;
    let score = ((1.0 - stale_ratio) * 100.0).round() as u8;

    let reason = if stale_count == 0 {
        "所有资产均在有效期内".to_string()
    } else {
        format!(
            "{}/{} 资产已过期（阈值 {} 天）",
            stale_count,
            eligible.len(),
            stale_assets.first().map(|s| s.threshold_days).unwrap_or(30)
        )
    };

    // contributing: stale_days 最长的 3 个
    let contributing: Vec<String> = stale_assets
        .iter()
        .take(3)
        .map(|s| s.asset_id.clone())
        .collect();

    SerHealthDimension {
        name: "freshness".to_string(),
        score: score.min(100),
        reason,
        contributing_assets: contributing,
    }
}

/// Quality（质量）
fn compute_quality(assets: &[SerClaudeMemoryAsset]) -> SerHealthDimension {
    let existing: Vec<&SerClaudeMemoryAsset> = assets.iter().filter(|a| a.exists).collect();

    if existing.is_empty() {
        return SerHealthDimension {
            name: "quality".to_string(),
            score: 100,
            reason: "无现有资产".to_string(),
            contributing_assets: Vec::new(),
        };
    }

    // 过长资产
    let too_long_count = existing
        .iter()
        .filter(|a| {
            let is_instruction = matches!(
                a.asset_type.as_str(),
                "user_claude_md" | "project_claude_md" | "project_dot_claude_md" | "local_md"
            );
            let is_auto_index = a.asset_type == "auto_memory_index";
            if is_instruction || is_auto_index {
                a.line_count.unwrap_or(0) > 200
            } else {
                false
            }
        })
        .count();

    // frontmatter 缺失（仅 rule/skill/agent 需要检查）
    let need_frontmatter: Vec<&SerClaudeMemoryAsset> = existing
        .iter()
        .copied()
        .filter(|a| {
            matches!(
                a.asset_type.as_str(),
                "global_rule"
                    | "project_rule"
                    | "global_skill"
                    | "project_skill"
                    | "global_agent"
                    | "project_agent"
            )
        })
        .collect();
    let no_frontmatter_count = need_frontmatter
        .iter()
        .filter(|a| a.frontmatter.is_none())
        .count();

    // 大文件（> 100KB）
    let large_file_count = existing
        .iter()
        .filter(|a| a.byte_size.unwrap_or(0) > 100_000)
        .count();

    let total = existing.len() as f64;
    let too_long_ratio = too_long_count as f64 / total;
    let no_fm_ratio = if need_frontmatter.is_empty() {
        0.0
    } else {
        no_frontmatter_count as f64 / need_frontmatter.len() as f64
    };
    let large_ratio = large_file_count as f64 / total;

    let score = ((1.0 - (too_long_ratio * 0.5 + no_fm_ratio * 0.3 + large_ratio * 0.2)) * 100.0)
        .round()
        .clamp(0.0, 100.0) as u8;

    let reason = format!(
        "过长 {}/{}，缺 frontmatter {}/{}，大文件 {}/{}",
        too_long_count,
        existing.len(),
        no_frontmatter_count,
        need_frontmatter.len(),
        large_file_count,
        existing.len()
    );

    // contributing: line_count 最长的 3 个
    let mut by_lines: Vec<&SerClaudeMemoryAsset> = existing.clone();
    by_lines.sort_by(|a, b| b.line_count.cmp(&a.line_count));
    let contributing: Vec<String> = by_lines.iter().take(3).map(|a| a.id.clone()).collect();

    SerHealthDimension {
        name: "quality".to_string(),
        score: score.min(100),
        reason,
        contributing_assets: contributing,
    }
}

/// Coverage（覆盖度）
fn compute_coverage(assets: &[SerClaudeMemoryAsset]) -> SerHealthDimension {
    let mut penalty = 0u8;
    let mut contributing = Vec::new();

    // instruction 文件覆盖
    let has_user_md = assets
        .iter()
        .any(|a| a.asset_type == "user_claude_md" && a.exists);
    let has_project_md = assets
        .iter()
        .any(|a| a.asset_type == "project_claude_md" && a.exists);
    let has_local_md = assets
        .iter()
        .any(|a| a.asset_type == "local_md" && a.exists);

    if !has_user_md {
        penalty += 20;
        if let Some(a) = assets.iter().find(|a| a.asset_type == "user_claude_md") {
            contributing.push(a.id.clone());
        }
    }
    if !has_project_md {
        penalty += 20;
        if let Some(a) = assets.iter().find(|a| a.asset_type == "project_claude_md") {
            contributing.push(a.id.clone());
        }
    }
    if !has_local_md {
        penalty += 10;
    }

    // rules 覆盖
    let has_unconditional_rules = assets
        .iter()
        .any(|a| matches!(a.asset_type.as_str(), "global_rule" | "project_rule") && a.exists);
    if !has_unconditional_rules {
        penalty += 30;
    }

    // auto memory 覆盖
    let has_auto_memory = assets
        .iter()
        .any(|a| a.asset_type == "auto_memory_index" && a.exists);
    if !has_auto_memory {
        penalty += 20;
        if let Some(a) = assets.iter().find(|a| a.asset_type == "auto_memory_index") {
            contributing.push(a.id.clone());
        }
    }

    let score = 100u8.saturating_sub(penalty);

    let reason = format!(
        "user_md={} project_md={} local_md={} rules={} auto_mem={}",
        has_user_md, has_project_md, has_local_md, has_unconditional_rules, has_auto_memory
    );

    SerHealthDimension {
        name: "coverage".to_string(),
        score,
        reason,
        contributing_assets: contributing,
    }
}

/// Cleanliness（清洁度）
fn compute_cleanliness(
    assets: &[SerClaudeMemoryAsset],
    duplicate_groups: &[SerMemoryDuplicateGroup],
) -> SerHealthDimension {
    let existing_count = assets.iter().filter(|a| a.exists).count();

    if existing_count == 0 {
        return SerHealthDimension {
            name: "cleanliness".to_string(),
            score: 100,
            reason: "无现有资产".to_string(),
            contributing_assets: Vec::new(),
        };
    }

    // 重复组数惩罚
    let dup_group_count = duplicate_groups.len();
    let dup_group_penalty = if dup_group_count == 0 {
        0.0
    } else if dup_group_count <= 5 {
        dup_group_count as f64 / 10.0
    } else {
        1.0
    };

    // 重复资产占比
    let dup_asset_ids: std::collections::HashSet<String> = duplicate_groups
        .iter()
        .flat_map(|g| g.asset_ids.clone())
        .collect();
    let dup_ratio = dup_asset_ids.len() as f64 / existing_count as f64;

    let score = ((1.0 - (dup_ratio * 0.6 + dup_group_penalty * 0.4)) * 100.0)
        .round()
        .clamp(0.0, 100.0) as u8;

    let reason = if dup_group_count == 0 {
        "未检测到重复".to_string()
    } else {
        format!(
            "{} 组重复，涉及 {} 个资产",
            dup_group_count,
            dup_asset_ids.len()
        )
    };

    // contributing: 相似度最高的 3 个组中的 asset id
    let contributing: Vec<String> = duplicate_groups
        .iter()
        .take(3)
        .flat_map(|g| g.asset_ids.clone())
        .collect();

    SerHealthDimension {
        name: "cleanliness".to_string(),
        score: score.min(100),
        reason,
        contributing_assets: contributing,
    }
}

/// Safety（安全性）
fn compute_safety(assets: &[SerClaudeMemoryAsset]) -> SerHealthDimension {
    let total_secrets: usize = assets.iter().map(|a| a.secret_issues.len()).sum();
    let critical_secrets: usize = assets
        .iter()
        .flat_map(|a| &a.secret_issues)
        .filter(|s| matches!(s.issue_type.as_str(), "env_content" | "private_url"))
        .count();

    let raw = (total_secrets as f64 * 0.7 + critical_secrets as f64 * 0.3) / 10.0;
    let score = ((1.0 - raw.min(1.0)) * 100.0).round().clamp(0.0, 100.0) as u8;

    let reason = if total_secrets == 0 {
        "未检测到敏感信息".to_string()
    } else {
        format!(
            "{} 个风险项（其中 {} 个高危）",
            total_secrets, critical_secrets
        )
    };

    // contributing: secret_issues 最多的 3 个 asset
    let mut by_secrets: Vec<&SerClaudeMemoryAsset> = assets
        .iter()
        .filter(|a| !a.secret_issues.is_empty())
        .collect();
    by_secrets.sort_by(|a, b| b.secret_issues.len().cmp(&a.secret_issues.len()));
    let contributing: Vec<String> = by_secrets.iter().take(3).map(|a| a.id.clone()).collect();

    SerHealthDimension {
        name: "safety".to_string(),
        score: score.min(100),
        reason,
        contributing_assets: contributing,
    }
}

/// 收集 Top 5 健康问题
fn collect_top_issues(
    assets: &[SerClaudeMemoryAsset],
    stale_assets: &[SerMemoryStaleness],
    duplicate_groups: &[SerMemoryDuplicateGroup],
) -> Vec<SerMemoryHealthIssue> {
    let mut issues: Vec<SerMemoryHealthIssue> = Vec::new();

    // 过期问题
    for stale in stale_assets.iter().take(3) {
        let days = stale.stale_days.unwrap_or(0);
        issues.push(SerMemoryHealthIssue {
            issue_type: "stale".to_string(),
            severity: if days > 60 { "warning" } else { "info" }.to_string(),
            asset_ids: vec![stale.asset_id.clone()],
            message: format!(
                "{} 已 {} 天未更新（阈值 {} 天）",
                stale.logical_path, days, stale.threshold_days
            ),
            suggestion: "复查内容是否仍然有效，考虑归档或更新".to_string(),
        });
    }

    // 过长问题
    for asset in assets.iter().filter(|a| a.exists) {
        let is_instruction = matches!(
            asset.asset_type.as_str(),
            "user_claude_md" | "project_claude_md" | "project_dot_claude_md" | "local_md"
        );
        let is_auto_index = asset.asset_type == "auto_memory_index";
        if (is_instruction || is_auto_index) && asset.line_count.unwrap_or(0) > 200 {
            issues.push(SerMemoryHealthIssue {
                issue_type: "too_long".to_string(),
                severity: "warning".to_string(),
                asset_ids: vec![asset.id.clone()],
                message: format!(
                    "{} 超过 200 行（{} 行）",
                    asset.logical_path,
                    asset.line_count.unwrap_or(0)
                ),
                suggestion: "考虑拆分为 rules 或 skills".to_string(),
            });
        }
    }

    // Secret 风险
    for asset in assets.iter().filter(|a| !a.secret_issues.is_empty()) {
        let has_critical = asset
            .secret_issues
            .iter()
            .any(|s| matches!(s.issue_type.as_str(), "env_content" | "private_url"));
        issues.push(SerMemoryHealthIssue {
            issue_type: "secret_risk".to_string(),
            severity: if has_critical { "critical" } else { "warning" }.to_string(),
            asset_ids: vec![asset.id.clone()],
            message: format!(
                "{} 包含 {} 个敏感信息",
                asset.logical_path,
                asset.secret_issues.len()
            ),
            suggestion: "移除敏感信息，改用环境变量或 secrets manager".to_string(),
        });
    }

    // 重复问题
    for group in duplicate_groups.iter().take(3) {
        issues.push(SerMemoryHealthIssue {
            issue_type: "duplicate".to_string(),
            severity: "info".to_string(),
            asset_ids: group.asset_ids.clone(),
            message: format!(
                "检测到 {} 个相似资产（相似度 {:.0}%）",
                group.asset_ids.len(),
                group.similarity * 100.0
            ),
            suggestion: if group.suggestion == "merge" {
                "内容完全相同，建议合并".to_string()
            } else {
                "内容近似，建议人工审查".to_string()
            },
        });
    }

    // 缺失 instruction
    let has_user_md = assets
        .iter()
        .any(|a| a.asset_type == "user_claude_md" && a.exists);
    let has_project_md = assets
        .iter()
        .any(|a| a.asset_type == "project_claude_md" && a.exists);
    if !has_user_md && !has_project_md {
        issues.push(SerMemoryHealthIssue {
            issue_type: "missing_instruction".to_string(),
            severity: "warning".to_string(),
            asset_ids: Vec::new(),
            message: "未找到任何 instruction 文件（CLAUDE.md）".to_string(),
            suggestion: "创建 CLAUDE.md 为项目提供基础指令".to_string(),
        });
    }

    // 按严重程度排序：critical > warning > info
    issues.sort_by(|a, b| {
        let order = |s: &str| match s {
            "critical" => 0,
            "warning" => 1,
            _ => 2,
        };
        order(&a.severity).cmp(&order(&b.severity))
    });

    issues.truncate(5);
    issues
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_asset(
        id: &str,
        asset_type: &str,
        exists: bool,
        mtime_ms: Option<u64>,
        line_count: Option<usize>,
        content_preview: Option<&str>,
    ) -> SerClaudeMemoryAsset {
        SerClaudeMemoryAsset {
            id: id.to_string(),
            scope: "project".to_string(),
            asset_type: asset_type.to_string(),
            logical_path: format!("/test/{}", id),
            native_path: format!("/test/{}", id),
            content_hash: None,
            content_preview: content_preview.map(|s| s.to_string()),
            content_truncated: false,
            line_count,
            byte_size: Some(100),
            mtime_ms,
            frontmatter: None,
            secret_issues: Vec::new(),
            exists,
        }
    }

    fn now_ms() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    }

    #[test]
    fn test_staleness_fresh_asset() {
        let now = now_ms();
        let assets = vec![make_asset(
            "a1",
            "project_claude_md",
            true,
            Some(now),
            None,
            None,
        )];
        let stale = compute_staleness(&assets, now);
        assert!(stale.is_empty(), "刚修改的资产不应过期");
    }

    #[test]
    fn test_staleness_stale_asset() {
        let now = now_ms();
        let thirty_one_days_ago = now - 31 * 24 * 60 * 60 * 1000;
        let assets = vec![make_asset(
            "a1",
            "project_claude_md",
            true,
            Some(thirty_one_days_ago),
            None,
            None,
        )];
        let stale = compute_staleness(&assets, now);
        assert_eq!(stale.len(), 1, "31 天前的资产应过期");
        assert!(stale[0].stale_days.unwrap_or(0) >= 31);
    }

    #[test]
    fn test_staleness_auto_memory_threshold() {
        let now = now_ms();
        let fifteen_days_ago = now - 15 * 24 * 60 * 60 * 1000;
        let assets = vec![make_asset(
            "a1",
            "auto_memory_index",
            true,
            Some(fifteen_days_ago),
            None,
            None,
        )];
        let stale = compute_staleness(&assets, now);
        assert_eq!(stale.len(), 1, "Auto Memory 15 天前应过期（阈值 14 天）");
        assert_eq!(stale[0].threshold_days, 14);
    }

    #[test]
    fn test_staleness_no_mtime() {
        let now = now_ms();
        let assets = vec![make_asset(
            "a1",
            "project_claude_md",
            true,
            None,
            None,
            None,
        )];
        let stale = compute_staleness(&assets, now);
        assert!(stale.is_empty(), "mtime 不可用不应标记为过期");
    }

    #[test]
    fn test_staleness_nonexistent() {
        let now = now_ms();
        let assets = vec![make_asset(
            "a1",
            "project_claude_md",
            false,
            None,
            None,
            None,
        )];
        let stale = compute_staleness(&assets, now);
        assert!(stale.is_empty(), "不存在的资产不应参与过期检测");
    }

    #[test]
    fn test_health_report_empty() {
        let report = compute_health_report(&[]);
        assert_eq!(report.overall_score, 80);
        assert!(report.stale_assets.is_empty());
        assert!(report.duplicate_groups.is_empty());
    }

    #[test]
    fn test_health_report_fresh_assets() {
        let now = now_ms();
        let assets = vec![
            make_asset(
                "a1",
                "user_claude_md",
                true,
                Some(now),
                Some(50),
                Some("Use pnpm"),
            ),
            make_asset(
                "a2",
                "project_claude_md",
                true,
                Some(now),
                Some(30),
                Some("Test first"),
            ),
            make_asset(
                "a3",
                "auto_memory_index",
                true,
                Some(now),
                Some(10),
                Some("Lesson 1"),
            ),
        ];
        let report = compute_health_report(&assets);
        assert!(
            report.overall_score > 50,
            "新鲜资产应得分较高: {}",
            report.overall_score
        );
        assert!(report.stale_assets.is_empty());
    }

    #[test]
    fn test_health_report_stale_assets() {
        let now = now_ms();
        let forty_days_ago = now - 40 * 24 * 60 * 60 * 1000;
        let assets = vec![make_asset(
            "a1",
            "project_claude_md",
            true,
            Some(forty_days_ago),
            Some(50),
            Some("Old content"),
        )];
        let report = compute_health_report(&assets);
        assert!(!report.stale_assets.is_empty(), "应检测到过期资产");
        assert!(report.freshness.score < 100, "有过期资产时新鲜度应 < 100");
    }

    #[test]
    fn test_health_report_with_secrets() {
        let now = now_ms();
        let mut asset = make_asset(
            "a1",
            "project_claude_md",
            true,
            Some(now),
            Some(10),
            Some("API key here"),
        );
        asset.secret_issues.push(SerSecretIssue {
            issue_type: "api_key".to_string(),
            line_number: 1,
            column_start: 0,
            column_end: 10,
            matched_text: "sk-***".to_string(),
        });
        let report = compute_health_report(&[asset]);
        assert!(report.safety.score < 100, "有 secret 时安全分应 < 100");
    }

    #[test]
    fn test_health_dimensions_range() {
        let now = now_ms();
        let assets = vec![make_asset(
            "a1",
            "project_claude_md",
            true,
            Some(now),
            Some(50),
            Some("Content"),
        )];
        let report = compute_health_report(&assets);
        for dim in &[
            &report.freshness,
            &report.quality,
            &report.coverage,
            &report.cleanliness,
            &report.safety,
        ] {
            assert!(
                dim.score <= 100,
                "维度 {} 得分 {} 超过 100",
                dim.name,
                dim.score
            );
        }
    }

    #[test]
    fn test_top_issues_limited() {
        let report = compute_health_report(&[]);
        assert!(report.top_issues.len() <= 5, "Top issues 不应超过 5 个");
    }

    #[test]
    fn test_coverage_missing_instruction() {
        let assets = vec![make_asset(
            "a1",
            "global_rule",
            true,
            None,
            Some(5),
            Some("Rule content"),
        )];
        let report = compute_health_report(&assets);
        assert!(
            report.coverage.score < 100,
            "缺少 instruction 文件时覆盖度应 < 100"
        );
    }

    /// 测试：默认 30 天阈值边界（刚好 30 天不应过期）
    #[test]
    fn test_staleness_default_threshold_boundary() {
        let now = now_ms();
        let exactly_30_days = now - 30 * 24 * 60 * 60 * 1000;
        let assets = vec![make_asset(
            "a1",
            "project_claude_md",
            true,
            Some(exactly_30_days),
            None,
            None,
        )];
        let stale = compute_staleness(&assets, now);
        assert!(stale.is_empty(), "刚好 30 天不应过期（需 > 阈值）");
    }

    /// 测试：auto_memory 14 天阈值边界
    #[test]
    fn test_staleness_auto_memory_14d_boundary() {
        let now = now_ms();
        let exactly_14_days = now - 14 * 24 * 60 * 60 * 1000;
        let assets = vec![make_asset(
            "a1",
            "auto_memory_index",
            true,
            Some(exactly_14_days),
            None,
            None,
        )];
        let stale = compute_staleness(&assets, now);
        assert!(stale.is_empty(), "刚好 14 天不应过期（需 > 阈值）");
    }

    /// 测试：safety score 遇到 critical secret 正确扣分
    #[test]
    fn test_safety_critical_secret_scoring() {
        let now = now_ms();
        let mut asset = make_asset(
            "a1",
            "project_claude_md",
            true,
            Some(now),
            Some(10),
            Some("env content"),
        );
        // 3 个 critical secret
        for _ in 0..3 {
            asset.secret_issues.push(SerSecretIssue {
                issue_type: "env_content".to_string(),
                line_number: 1,
                column_start: 0,
                column_end: 10,
                matched_text: "AWS_KEY".to_string(),
            });
        }
        // 2 个非 critical
        for _ in 0..2 {
            asset.secret_issues.push(SerSecretIssue {
                issue_type: "api_key".to_string(),
                line_number: 1,
                column_start: 0,
                column_end: 10,
                matched_text: "sk-***".to_string(),
            });
        }
        let report = compute_health_report(&[asset]);
        // raw = (5*0.7 + 3*0.3)/10 = (3.5+0.9)/10 = 0.44
        // score = (1-0.44)*100 = 56
        assert!(
            report.safety.score < 70,
            "有 critical secret 时安全分应大幅扣分: {}",
            report.safety.score
        );
        assert!(
            report.safety.score > 0,
            "少量 secret 不应使安全分归零: {}",
            report.safety.score
        );
    }

    /// 测试：overall_score 始终在 0..=100 范围内（全 stale + 全 secret）
    #[test]
    fn test_overall_score_bounds_under_stress() {
        let now = now_ms();
        let forty_days_ago = now - 40 * 24 * 60 * 60 * 1000;
        let mut assets = Vec::new();
        for i in 0..10 {
            let id = format!("a{}", i);
            let mut asset = make_asset(
                &id,
                "project_claude_md",
                true,
                Some(forty_days_ago),
                Some(300), // 过长
                Some("Old content"),
            );
            // 每个 asset 加 5 个 critical secret
            for j in 0..5 {
                asset.secret_issues.push(SerSecretIssue {
                    issue_type: "env_content".to_string(),
                    line_number: j,
                    column_start: 0,
                    column_end: 10,
                    matched_text: "SECRET".to_string(),
                });
            }
            assets.push(asset);
        }
        let report = compute_health_report(&assets);
        assert!(
            report.overall_score <= 100,
            "overall_score 不应超过 100: {}",
            report.overall_score
        );
        // u8 类型保证 >= 0，无需断言；只验证上界
        assert!(
            report.overall_score <= 100,
            "overall_score 不应超过 100: {}",
            report.overall_score
        );
    }

    /// 测试：空资产列表所有维度有合理值
    #[test]
    fn test_empty_assets_all_dimensions() {
        let report = compute_health_report(&[]);
        // 空：freshness=100（无 eligible），quality=100（无 existing）
        // coverage=0（全部缺失），cleanliness=100（无 existing），safety=100（无 secrets）
        // overall = 100*0.2 + 100*0.2 + 0*0.2 + 100*0.25 + 100*0.15 = 80
        assert_eq!(report.freshness.score, 100);
        assert_eq!(report.quality.score, 100);
        assert_eq!(report.coverage.score, 0);
        assert_eq!(report.cleanliness.score, 100);
        assert_eq!(report.safety.score, 100);
    }

    /// 测试：mtime 在未来的资产不应过期
    #[test]
    fn test_staleness_future_mtime() {
        let now = now_ms();
        let future = now + 1000;
        let assets = vec![make_asset(
            "a1",
            "project_claude_md",
            true,
            Some(future),
            None,
            None,
        )];
        let stale = compute_staleness(&assets, now);
        assert!(stale.is_empty(), "mtime 在未来不应标记为过期");
    }
}
