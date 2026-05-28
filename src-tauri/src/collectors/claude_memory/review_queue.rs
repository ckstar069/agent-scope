use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashSet;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::Path;

use super::models::{
    SerClaudeMemoryAsset, SerMemoryDuplicateGroup, SerMemoryHealthIssue, SerMemoryHealthReport,
    SerMemoryStaleness, SerReviewItem, SerReviewQueue, SerReviewQueueCounts,
    SerReviewQueueSyncResult, SerReviewState,
};
// ============================================================================
// 持久化数据结构
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
struct ReviewQueueData {
    items: Vec<SerReviewItem>,
    last_sync_at: Option<u64>,
}

// ============================================================================
// ReviewQueueStore
// ============================================================================

pub struct ReviewQueueStore {
    storage_path: std::path::PathBuf,
    data: ReviewQueueData,
}

impl ReviewQueueStore {
    /// 创建新的空存储
    pub fn new(storage_path: std::path::PathBuf) -> Self {
        Self {
            storage_path,
            data: ReviewQueueData {
                items: Vec::new(),
                last_sync_at: None,
            },
        }
    }

    /// 从文件加载，不存在或损坏时返回空存储
    pub fn load_or_default(storage_path: std::path::PathBuf) -> Self {
        if storage_path.exists() {
            match fs::read_to_string(&storage_path) {
                Ok(content) => {
                    if let Ok(data) = serde_json::from_str::<ReviewQueueData>(&content) {
                        return Self { storage_path, data };
                    }
                    eprintln!(
                        "[review_queue:warn] 无法解析 review queue 文件 '{}'，将使用空队列",
                        storage_path.display()
                    );
                }
                Err(e) => {
                    eprintln!(
                        "[review_queue:warn] 无法读取 review queue 文件 '{}': {}，将使用空队列",
                        storage_path.display(),
                        e
                    );
                }
            }
        }
        Self::new(storage_path)
    }

    // ------------------------------------------------------------------
    // 查询 API
    // ------------------------------------------------------------------

    pub fn get_queue(&self, project_id: &str, filter: Option<&str>) -> SerReviewQueue {
        let all_items: Vec<SerReviewItem> = self
            .data
            .items
            .iter()
            .filter(|i| i.project_id == project_id)
            .cloned()
            .collect();

        // counts 基于全量 items（不受 filter 影响）
        let pending_count = all_items.iter().filter(|i| i.state == SerReviewState::Pending).count();
        let reviewed_count = all_items.iter().filter(|i| i.state == SerReviewState::Reviewed).count();
        let ignored_count = all_items.iter().filter(|i| i.state == SerReviewState::Ignored).count();
        let snoozed_count = all_items.iter().filter(|i| i.state == SerReviewState::Snoozed).count();

        // filter 只影响返回的 items 列表
        let mut items = all_items;
        if let Some(f) = filter {
            let state_filter = f.to_lowercase();
            if state_filter != "all" {
                items.retain(|i| match i.state {
                    SerReviewState::Pending => state_filter == "pending",
                    SerReviewState::Reviewed => state_filter == "reviewed",
                    SerReviewState::Ignored => state_filter == "ignored",
                    SerReviewState::Snoozed => state_filter == "snoozed",
                });
            }
        }

        // 默认按 updated_at 倒序
        items.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

        SerReviewQueue {
            items,
            pending_count,
            reviewed_count,
            ignored_count,
            snoozed_count,
            last_sync_at: self.data.last_sync_at,
        }
    }

    pub fn get_counts(&self, project_id: &str) -> SerReviewQueueCounts {
        let items: Vec<&SerReviewItem> = self
            .data
            .items
            .iter()
            .filter(|i| i.project_id == project_id)
            .collect();

        let pending = items.iter().filter(|i| i.state == SerReviewState::Pending).count();
        let reviewed = items.iter().filter(|i| i.state == SerReviewState::Reviewed).count();
        let ignored = items.iter().filter(|i| i.state == SerReviewState::Ignored).count();
        let snoozed = items.iter().filter(|i| i.state == SerReviewState::Snoozed).count();

        SerReviewQueueCounts {
            pending,
            reviewed,
            ignored,
            snoozed,
            total: items.len(),
        }
    }

    // ------------------------------------------------------------------
    // 状态更新
    // ------------------------------------------------------------------

    pub fn update_state(
        &mut self,
        item_id: &str,
        new_state: SerReviewState,
        snooze_days: Option<u32>,
        note: Option<String>,
    ) -> Result<SerReviewItem, String> {
        let now = unix_now();
        let item = self
            .data
            .items
            .iter_mut()
            .find(|i| i.id == item_id)
            .ok_or_else(|| "审阅项不存在".to_string())?;

        item.state = new_state.clone();
        item.updated_at = now;

        if let Some(days) = snooze_days {
            item.snooze_until = Some(now + (days as u64) * 86400);
        } else if new_state != SerReviewState::Snoozed {
            item.snooze_until = None;
        }

        if note.is_some() {
            item.review_note = note;
        }

        let result = item.clone();
        self.save()?;
        Ok(result)
    }

    // ------------------------------------------------------------------
    // Sync：从 Health Report 合并
    // ------------------------------------------------------------------

    pub fn sync(
        &mut self,
        project_id: &str,
        health_report: &SerMemoryHealthReport,
        assets: &[SerClaudeMemoryAsset],
    ) -> Result<SerReviewQueueSyncResult, String> {
        let now = unix_now();
        let mut created = 0usize;
        let mut updated = 0usize;
        let mut unchanged = 0usize;
        let mut expired_snoozes = 0usize;

        // 1. 检查过期 snoozed
        for item in &mut self.data.items {
            if item.project_id != project_id {
                continue;
            }
            if item.state == SerReviewState::Snoozed {
                if let Some(until) = item.snooze_until {
                    if now >= until {
                        item.state = SerReviewState::Pending;
                        item.updated_at = now;
                        item.snooze_until = None;
                        expired_snoozes += 1;
                    }
                }
            }
        }

        // 2. 收集已有 source_key 集合（用于快速判断）
        let existing_keys: HashSet<String> = self
            .data
            .items
            .iter()
            .filter(|i| i.project_id == project_id)
            .map(|i| i.source_key.clone())
            .collect();

        // 3. 跟踪本次 sync 处理的 source_key（用于清理 orphan）
        let mut processed_keys: HashSet<String> = HashSet::new();

        // 4. 从 top_issues 生成 candidate items
        for issue in &health_report.top_issues {
            match issue.issue_type.as_str() {
                "duplicate" => {
                    if let Some(group) = find_duplicate_group_for_issue(
                        &health_report.duplicate_groups,
                        &issue.asset_ids,
                    ) {
                        let source_key =
                            make_duplicate_source_key(project_id, &group.group_id);
                        processed_keys.insert(source_key.clone());

                        if existing_keys.contains(&source_key) {
                            let item = self
                                .data
                                .items
                                .iter_mut()
                                .find(|i| i.project_id == project_id && i.source_key == source_key)
                                .unwrap();
                            if update_item_if_mutable(item, issue, now) {
                                updated += 1;
                            } else {
                                unchanged += 1;
                            }
                        } else {
                            let new_item = create_duplicate_item(
                                project_id,
                                &source_key,
                                issue,
                                group,
                                now,
                            );
                            self.data.items.push(new_item);
                            created += 1;
                        }
                    }
                }
                _ => {
                    for asset_id in &issue.asset_ids {
                        let source_key =
                            make_asset_issue_source_key(project_id, asset_id, &issue.issue_type);
                        processed_keys.insert(source_key.clone());

                        if existing_keys.contains(&source_key) {
                            let item = self
                                .data
                                .items
                                .iter_mut()
                                .find(|i| {
                                    i.project_id == project_id && i.source_key == source_key
                                })
                                .unwrap();
                            if update_item_if_mutable(item, issue, now) {
                                updated += 1;
                            } else {
                                unchanged += 1;
                            }
                        } else {
                            let new_item = create_asset_issue_item(
                                project_id,
                                &source_key,
                                issue,
                                asset_id,
                                now,
                            );
                            self.data.items.push(new_item);
                            created += 1;
                        }
                    }
                }
            }
        }

        // 5. 从 stale_assets 生成（仅当 top_issues 未覆盖时）
        for stale in &health_report.stale_assets {
            let source_key =
                make_asset_issue_source_key(project_id, &stale.asset_id, "stale");
            if processed_keys.contains(&source_key) {
                continue;
            }
            processed_keys.insert(source_key.clone());

            if existing_keys.contains(&source_key) {
                let item = self
                    .data
                    .items
                    .iter_mut()
                    .find(|i| i.project_id == project_id && i.source_key == source_key)
                    .unwrap();
                if item.state == SerReviewState::Pending || item.state == SerReviewState::Snoozed
                {
                    let message = format_stale_message(stale);
                    if item.message != message {
                        item.message = message;
                        item.updated_at = now;
                        updated += 1;
                    } else {
                        unchanged += 1;
                    }
                } else {
                    unchanged += 1;
                }
            } else {
                let new_item = create_stale_item(project_id, &source_key, stale, now);
                self.data.items.push(new_item);
                created += 1;
            }
        }

        // 6. 从 duplicate_groups 生成（仅当 top_issues 未覆盖时）
        for group in &health_report.duplicate_groups {
            let source_key = make_duplicate_source_key(project_id, &group.group_id);
            if processed_keys.contains(&source_key) {
                continue;
            }
            processed_keys.insert(source_key.clone());

            if existing_keys.contains(&source_key) {
                unchanged += 1;
            } else {
                let new_item = create_duplicate_group_item(project_id, &source_key, group, now);
                self.data.items.push(new_item);
                created += 1;
            }
        }

        // 7. 按 asset 聚合 secret_issues
        let asset_secret_map = collect_secret_issues_by_asset(assets);
        for (asset_id, _) in asset_secret_map {
            let source_key =
                make_asset_issue_source_key(project_id, &asset_id, "secret");
            if processed_keys.contains(&source_key) {
                continue;
            }
            processed_keys.insert(source_key.clone());

            if existing_keys.contains(&source_key) {
                unchanged += 1;
            } else {
                let new_item = create_secret_item(project_id, &source_key, &asset_id, now);
                self.data.items.push(new_item);
                created += 1;
            }
        }

        // 8. 清理 orphan pending items（仅当 exists=false 时清理）
        let existing_asset_ids: HashSet<String> = assets
            .iter()
            .filter(|a| a.exists)
            .map(|a| a.id.clone())
            .collect();
        self.data.items.retain(|item| {
            if item.project_id != project_id {
                return true;
            }
            if item.state != SerReviewState::Pending {
                return true;
            }
            // 如果不在本次 processed_keys 中，且对应 exists=true 的资产已不存在，则删除
            if !processed_keys.contains(&item.source_key) {
                return existing_asset_ids.contains(&item.primary_asset_id);
            }
            true
        });

        self.data.last_sync_at = Some(now);
        self.save()?;

        let queue = self.get_queue(project_id, None);
        Ok(SerReviewQueueSyncResult {
            created,
            updated,
            unchanged,
            expired_snoozes,
            queue,
        })
    }

    // ------------------------------------------------------------------
    // 内部辅助
    // ------------------------------------------------------------------

    fn save(&self) -> Result<(), String> {
        if let Some(parent) = self.storage_path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let json = serde_json::to_string_pretty(&self.data).map_err(|e| e.to_string())?;
        fs::write(&self.storage_path, &json).map_err(|e| e.to_string())?;
        Ok(())
    }
}

// ============================================================================
// source_key 生成
// ============================================================================

fn make_asset_issue_source_key(project_id: &str, asset_id: &str, issue_type: &str) -> String {
    format!("{}::asset::{}::{}", project_id, asset_id, issue_type)
}

fn make_duplicate_source_key(project_id: &str, group_id: &str) -> String {
    format!("{}::dup::{}", project_id, group_id)
}

// ============================================================================
// id 生成：基于 source_key 的稳定 hash
// ============================================================================

fn stable_id_from_source_key(source_key: &str) -> String {
    let mut hasher = DefaultHasher::new();
    source_key.hash(&mut hasher);
    let hash = hasher.finish();
    format!("review_{:016x}", hash)
}

// ============================================================================
// Item 创建工厂
// ============================================================================

fn create_asset_issue_item(
    project_id: &str,
    source_key: &str,
    issue: &SerMemoryHealthIssue,
    primary_asset_id: &str,
    now: u64,
) -> SerReviewItem {
    SerReviewItem {
        id: stable_id_from_source_key(source_key),
        source_key: source_key.to_string(),
        project_id: project_id.to_string(),
        issue_type: issue.issue_type.clone(),
        severity: issue.severity.clone(),
        message: issue.message.clone(),
        suggestion: issue.suggestion.clone(),
        asset_ids: issue.asset_ids.clone(),
        primary_asset_id: primary_asset_id.to_string(),
        group_id: None,
        state: SerReviewState::Pending,
        created_at: now,
        updated_at: now,
        snooze_until: None,
        review_note: None,
    }
}

fn create_duplicate_item(
    project_id: &str,
    source_key: &str,
    issue: &SerMemoryHealthIssue,
    group: &SerMemoryDuplicateGroup,
    now: u64,
) -> SerReviewItem {
    SerReviewItem {
        id: stable_id_from_source_key(source_key),
        source_key: source_key.to_string(),
        project_id: project_id.to_string(),
        issue_type: issue.issue_type.clone(),
        severity: issue.severity.clone(),
        message: issue.message.clone(),
        suggestion: issue.suggestion.clone(),
        asset_ids: group.asset_ids.clone(),
        primary_asset_id: group.asset_ids.first().cloned().unwrap_or_default(),
        group_id: Some(group.group_id.clone()),
        state: SerReviewState::Pending,
        created_at: now,
        updated_at: now,
        snooze_until: None,
        review_note: None,
    }
}

fn create_stale_item(
    project_id: &str,
    source_key: &str,
    stale: &SerMemoryStaleness,
    now: u64,
) -> SerReviewItem {
    SerReviewItem {
        id: stable_id_from_source_key(source_key),
        source_key: source_key.to_string(),
        project_id: project_id.to_string(),
        issue_type: "stale".to_string(),
        severity: "warning".to_string(),
        message: format_stale_message(stale),
        suggestion: "在 IDE 中打开确认内容是否仍有效；如已废弃，考虑从 load chain 中移除或归档。".to_string(),
        asset_ids: vec![stale.asset_id.clone()],
        primary_asset_id: stale.asset_id.clone(),
        group_id: None,
        state: SerReviewState::Pending,
        created_at: now,
        updated_at: now,
        snooze_until: None,
        review_note: None,
    }
}

fn create_duplicate_group_item(
    project_id: &str,
    source_key: &str,
    group: &SerMemoryDuplicateGroup,
    now: u64,
) -> SerReviewItem {
    SerReviewItem {
        id: stable_id_from_source_key(source_key),
        source_key: source_key.to_string(),
        project_id: project_id.to_string(),
        issue_type: "duplicate".to_string(),
        severity: "warning".to_string(),
        message: format!(
            "发现 {} 个文件存在重复内容（相似度 {:.0}%）",
            group.asset_ids.len(),
            group.similarity * 100.0
        ),
        suggestion: group.suggestion.clone(),
        asset_ids: group.asset_ids.clone(),
        primary_asset_id: group.asset_ids.first().cloned().unwrap_or_default(),
        group_id: Some(group.group_id.clone()),
        state: SerReviewState::Pending,
        created_at: now,
        updated_at: now,
        snooze_until: None,
        review_note: None,
    }
}

fn create_secret_item(
    project_id: &str,
    source_key: &str,
    asset_id: &str,
    now: u64,
) -> SerReviewItem {
    SerReviewItem {
        id: stable_id_from_source_key(source_key),
        source_key: source_key.to_string(),
        project_id: project_id.to_string(),
        issue_type: "secret".to_string(),
        severity: "critical".to_string(),
        message: "检测到疑似凭证的高熵字符串".to_string(),
        suggestion: "确认是否为真实凭证；如是，立即从文件中移除并轮换该凭证；考虑使用环境变量替代硬编码。".to_string(),
        asset_ids: vec![asset_id.to_string()],
        primary_asset_id: asset_id.to_string(),
        group_id: None,
        state: SerReviewState::Pending,
        created_at: now,
        updated_at: now,
        snooze_until: None,
        review_note: None,
    }
}

// ============================================================================
// 更新逻辑
// ============================================================================

/// 如果 item 处于 pending 或 snoozed 状态，更新元数据；返回 true 表示发生了更新
fn update_item_if_mutable(
    item: &mut SerReviewItem,
    issue: &SerMemoryHealthIssue,
    now: u64,
) -> bool {
    if item.state != SerReviewState::Pending && item.state != SerReviewState::Snoozed {
        return false;
    }

    let mut changed = false;
    if item.message != issue.message {
        item.message = issue.message.clone();
        changed = true;
    }
    if item.suggestion != issue.suggestion {
        item.suggestion = issue.suggestion.clone();
        changed = true;
    }
    if item.severity != issue.severity {
        item.severity = issue.severity.clone();
        changed = true;
    }

    if changed {
        item.updated_at = now;
    }
    changed
}

fn format_stale_message(stale: &SerMemoryStaleness) -> String {
    match stale.stale_days {
        Some(days) => format!(
            "{} 已 {} 天未更新",
            stale.logical_path, days
        ),
        None => format!("{} 可能已过期", stale.logical_path),
    }
}

// ============================================================================
// 辅助函数
// ============================================================================

/// 规范化 project_id
/// - path Some → canonicalized path 字符串，Windows 去掉 \\?\ 前缀
/// - path None → "__global__"
pub fn canonicalize_project_id(project_path: Option<&str>) -> String {
    match project_path {
        Some(path) => {
            let p = Path::new(path);
            match p.canonicalize() {
                Ok(cp) => {
                    let mut s = cp.to_string_lossy().into_owned();
                    if s.starts_with(r"\\?\") {
                        s.drain(..4);
                    }
                    s
                }
                Err(_) => path.to_string(),
            }
        }
        None => "__global__".to_string(),
    }
}

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn find_duplicate_group_for_issue<'a>(
    groups: &'a [SerMemoryDuplicateGroup],
    issue_asset_ids: &'a [String],
) -> Option<&'a SerMemoryDuplicateGroup> {
    groups.iter().find(|g| {
        // 判断 asset_ids 是否完全匹配（不考虑顺序）
        let issue_set: HashSet<&str> = issue_asset_ids.iter().map(|s| s.as_str()).collect();
        let group_set: HashSet<&str> = g.asset_ids.iter().map(|s| s.as_str()).collect();
        issue_set == group_set
    })
}

fn collect_secret_issues_by_asset(assets: &[SerClaudeMemoryAsset]) -> Vec<(String, usize)> {
    let mut result = Vec::new();
    for asset in assets {
        if !asset.secret_issues.is_empty() {
            result.push((asset.id.clone(), asset.secret_issues.len()));
        }
    }
    result
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::atomic::{AtomicUsize, Ordering};

    static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

    fn temp_store() -> (ReviewQueueStore, std::path::PathBuf) {
        let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let tmp = std::env::temp_dir().join(format!("agent-scope-rq-test-{}", id));
        let _ = fs::remove_dir_all(&tmp);
        let _ = fs::create_dir_all(&tmp);
        let path = tmp.join("reviews.json");
        let store = ReviewQueueStore::new(path.clone());
        (store, path)
    }

    fn make_health_report_with_issue(
        issue_type: &str,
        asset_ids: Vec<String>,
    ) -> SerMemoryHealthReport {
        SerMemoryHealthReport {
            overall_score: 50,
            freshness: SerHealthDimension {
                name: "freshness".to_string(),
                score: 50,
                reason: "test".to_string(),
                contributing_assets: Vec::new(),
            },
            quality: SerHealthDimension {
                name: "quality".to_string(),
                score: 50,
                reason: "test".to_string(),
                contributing_assets: Vec::new(),
            },
            coverage: SerHealthDimension {
                name: "coverage".to_string(),
                score: 50,
                reason: "test".to_string(),
                contributing_assets: Vec::new(),
            },
            cleanliness: SerHealthDimension {
                name: "cleanliness".to_string(),
                score: 50,
                reason: "test".to_string(),
                contributing_assets: Vec::new(),
            },
            safety: SerHealthDimension {
                name: "safety".to_string(),
                score: 50,
                reason: "test".to_string(),
                contributing_assets: Vec::new(),
            },
            top_issues: vec![SerMemoryHealthIssue {
                issue_type: issue_type.to_string(),
                severity: "warning".to_string(),
                asset_ids,
                message: "test message".to_string(),
                suggestion: "test suggestion".to_string(),
            }],
            stale_assets: Vec::new(),
            duplicate_groups: Vec::new(),
        }
    }

    // 需要导入 SerHealthDimension
    use crate::collectors::claude_memory::models::SerHealthDimension;

    #[test]
    fn test_empty_store() {
        let (store, _) = temp_store();
        let queue = store.get_queue("test-project", None);
        assert_eq!(queue.items.len(), 0);
        assert_eq!(queue.pending_count, 0);
    }

    #[test]
    fn test_sync_creates_new_items() {
        let (mut store, _) = temp_store();
        let report = make_health_report_with_issue("stale", vec!["a1".to_string()]);
        let assets = vec![];
        let result = store.sync("proj1", &report, &assets).unwrap();
        assert_eq!(result.created, 1);
        assert_eq!(result.updated, 0);

        let queue = store.get_queue("proj1", None);
        assert_eq!(queue.items.len(), 1);
        assert_eq!(queue.pending_count, 1);
        assert_eq!(queue.items[0].issue_type, "stale");
        assert_eq!(queue.items[0].state, SerReviewState::Pending);
    }

    #[test]
    fn test_reviewed_state_preserved_on_sync() {
        let (mut store, _) = temp_store();
        let report = make_health_report_with_issue("stale", vec!["a1".to_string()]);
        let assets = vec![];

        // 第一次 sync，创建 pending item
        store.sync("proj1", &report, &assets).unwrap();

        // 标记为 reviewed
        let item_id = store.get_queue("proj1", None).items[0].id.clone();
        store.update_state(&item_id, SerReviewState::Reviewed, None, None).unwrap();

        // 第二次 sync，同一问题
        let result = store.sync("proj1", &report, &assets).unwrap();
        assert_eq!(result.unchanged, 1);

        let queue = store.get_queue("proj1", None);
        assert_eq!(queue.items[0].state, SerReviewState::Reviewed);
    }

    #[test]
    fn test_pending_state_gets_updated_on_sync() {
        let (mut store, _) = temp_store();
        let mut report = make_health_report_with_issue("stale", vec!["a1".to_string()]);
        let assets = vec![];
        store.sync("proj1", &report, &assets).unwrap();

        // 修改 message
        report.top_issues[0].message = "updated message".to_string();
        let result = store.sync("proj1", &report, &assets).unwrap();
        assert_eq!(result.updated, 1);

        let queue = store.get_queue("proj1", None);
        assert_eq!(queue.items[0].message, "updated message");
    }

    #[test]
    fn test_snooze_expiration() {
        let (mut store, _) = temp_store();
        let report = make_health_report_with_issue("stale", vec!["a1".to_string()]);
        let assets = vec![];
        store.sync("proj1", &report, &assets).unwrap();

        let item_id = store.get_queue("proj1", None).items[0].id.clone();
        // 设置 snooze 为 0 天（立即过期）
        store.update_state(&item_id, SerReviewState::Snoozed, Some(0), None).unwrap();

        // sync 应触发过期
        let result = store.sync("proj1", &report, &assets).unwrap();
        assert_eq!(result.expired_snoozes, 1);

        let queue = store.get_queue("proj1", None);
        assert_eq!(queue.items[0].state, SerReviewState::Pending);
        assert!(queue.items[0].snooze_until.is_none());
    }

    #[test]
    fn test_filter_by_state() {
        let (mut store, _) = temp_store();
        let report = make_health_report_with_issue("stale", vec!["a1".to_string()]);
        let assets = vec![];
        store.sync("proj1", &report, &assets).unwrap();

        let item_id = store.get_queue("proj1", None).items[0].id.clone();
        store.update_state(&item_id, SerReviewState::Reviewed, None, None).unwrap();

        let pending = store.get_queue("proj1", Some("pending"));
        assert_eq!(pending.items.len(), 0);

        let reviewed = store.get_queue("proj1", Some("reviewed"));
        assert_eq!(reviewed.items.len(), 1);
    }

    #[test]
    fn test_stable_id_from_source_key() {
        let id1 = stable_id_from_source_key("proj1::asset::a1::stale");
        let id2 = stable_id_from_source_key("proj1::asset::a1::stale");
        assert_eq!(id1, id2);
        assert!(id1.starts_with("review_"));
    }

    #[test]
    fn test_canonicalize_project_id() {
        assert_eq!(
            canonicalize_project_id(None),
            "__global__"
        );
        // 存在的路径
        let tmp = std::env::temp_dir();
        let result = canonicalize_project_id(Some(tmp.to_string_lossy().as_ref()));
        assert!(!result.is_empty());
    }

    #[test]
    fn test_persistence_roundtrip() {
        let (mut store, path) = temp_store();
        let report = make_health_report_with_issue("stale", vec!["a1".to_string()]);
        let assets = vec![];
        store.sync("proj1", &report, &assets).unwrap();

        // 保存并重新加载
        drop(store);
        let store2 = ReviewQueueStore::load_or_default(path);
        let queue = store2.get_queue("proj1", None);
        assert_eq!(queue.items.len(), 1);
        assert_eq!(queue.items[0].issue_type, "stale");
    }

    #[test]
    fn test_counts() {
        let (mut store, _) = temp_store();
        let report = make_health_report_with_issue("stale", vec!["a1".to_string()]);
        let assets = vec![];
        store.sync("proj1", &report, &assets).unwrap();

        let counts = store.get_counts("proj1");
        assert_eq!(counts.pending, 1);
        assert_eq!(counts.total, 1);
    }

    // ─── 补充测试：review 问题修复 ───

    fn make_test_asset(id: &str, exists: bool) -> SerClaudeMemoryAsset {
        SerClaudeMemoryAsset {
            id: id.to_string(),
            scope: "project".to_string(),
            asset_type: "project_rule".to_string(),
            logical_path: format!("/test/{}", id),
            native_path: format!("/test/{}", id),
            content_hash: None,
            content_preview: None,
            content_truncated: false,
            line_count: Some(50),
            byte_size: Some(5000),
            mtime_ms: Some(unix_now() * 1000),
            frontmatter: None,
            secret_issues: Vec::new(),
            exists,
        }
    }

    #[test]
    fn test_get_queue_counts_unchanged_by_filter() {
        let (mut store, _) = temp_store();
        let report = make_health_report_with_issue("stale", vec!["a1".to_string()]);
        let assets = vec![make_test_asset("a1", true), make_test_asset("a2", true)];
        store.sync("proj1", &report, &assets).unwrap();

        // 创建一个 reviewed item（a2 的 quality issue）
        let report2 = make_health_report_with_issue("quality", vec!["a2".to_string()]);
        store.sync("proj1", &report2, &assets).unwrap();
        let item_id = store
            .get_queue("proj1", None)
            .items
            .iter()
            .find(|i| i.primary_asset_id == "a1")
            .unwrap()
            .id
            .clone();
        store.update_state(&item_id, SerReviewState::Reviewed, None, None).unwrap();

        // 全量队列：2 items，1 pending, 1 reviewed
        let all = store.get_queue("proj1", None);
        assert_eq!(all.items.len(), 2);
        assert_eq!(all.pending_count, 1);
        assert_eq!(all.reviewed_count, 1);

        // filter=reviewed 时：items 列表只有 1 个，但 counts 仍为全量
        let filtered = store.get_queue("proj1", Some("reviewed"));
        assert_eq!(filtered.items.len(), 1);
        assert_eq!(filtered.pending_count, 1);    // 全量 pending count
        assert_eq!(filtered.reviewed_count, 1);   // 全量 reviewed count
    }

    #[test]
    fn test_update_state_returns_err_for_missing_item() {
        let (mut store, _) = temp_store();
        let result = store.update_state("nonexistent", SerReviewState::Reviewed, None, None);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("不存在"));
    }

    #[test]
    fn test_orphan_cleanup_only_for_missing_exists() {
        let (mut store, _) = temp_store();
        let report = make_health_report_with_issue("stale", vec!["a1".to_string()]);
        let assets_exists = vec![
            SerClaudeMemoryAsset {
                id: "a1".to_string(),
                scope: "project".to_string(),
                asset_type: "project_rule".to_string(),
                logical_path: "/test/a1".to_string(),
                native_path: "/test/a1".to_string(),
                content_hash: None,
                content_preview: None,
                content_truncated: false,
                line_count: Some(50),
                byte_size: Some(5000),
                mtime_ms: Some(unix_now() * 1000),
                frontmatter: None,
                secret_issues: Vec::new(),
                exists: true,
            },
        ];
        // sync 时 exists=true，创建 pending item
        store.sync("proj1", &report, &assets_exists).unwrap();
        let queue = store.get_queue("proj1", None);
        assert_eq!(queue.items.len(), 1);

        // 再次 sync，但 a1 变为 exists=false，pending item 应被清理
        let assets_missing = vec![
            SerClaudeMemoryAsset {
                id: "a1".to_string(),
                scope: "project".to_string(),
                asset_type: "project_rule".to_string(),
                logical_path: "/test/a1".to_string(),
                native_path: "/test/a1".to_string(),
                content_hash: None,
                content_preview: None,
                content_truncated: false,
                line_count: Some(50),
                byte_size: Some(5000),
                mtime_ms: Some(unix_now() * 1000),
                frontmatter: None,
                secret_issues: Vec::new(),
                exists: false,
            },
        ];
        let empty_report = SerMemoryHealthReport {
            overall_score: 100,
            freshness: SerHealthDimension {
                name: "freshness".to_string(),
                score: 100,
                reason: "ok".to_string(),
                contributing_assets: Vec::new(),
            },
            quality: SerHealthDimension {
                name: "quality".to_string(),
                score: 100,
                reason: "ok".to_string(),
                contributing_assets: Vec::new(),
            },
            coverage: SerHealthDimension {
                name: "coverage".to_string(),
                score: 100,
                reason: "ok".to_string(),
                contributing_assets: Vec::new(),
            },
            cleanliness: SerHealthDimension {
                name: "cleanliness".to_string(),
                score: 100,
                reason: "ok".to_string(),
                contributing_assets: Vec::new(),
            },
            safety: SerHealthDimension {
                name: "safety".to_string(),
                score: 100,
                reason: "ok".to_string(),
                contributing_assets: Vec::new(),
            },
            top_issues: Vec::new(),
            stale_assets: Vec::new(),
            duplicate_groups: Vec::new(),
        };
        store.sync("proj1", &empty_report, &assets_missing).unwrap();
        let queue = store.get_queue("proj1", None);
        assert_eq!(queue.items.len(), 0, "exists=false 的 pending item 应被清理");
    }

    #[test]
    fn test_orphan_retains_reviewed_for_missing_exists() {
        let (mut store, _) = temp_store();
        let report = make_health_report_with_issue("stale", vec!["a1".to_string()]);
        let assets = vec![
            SerClaudeMemoryAsset {
                id: "a1".to_string(),
                scope: "project".to_string(),
                asset_type: "project_rule".to_string(),
                logical_path: "/test/a1".to_string(),
                native_path: "/test/a1".to_string(),
                content_hash: None,
                content_preview: None,
                content_truncated: false,
                line_count: Some(50),
                byte_size: Some(5000),
                mtime_ms: Some(unix_now() * 1000),
                frontmatter: None,
                secret_issues: Vec::new(),
                exists: true,
            },
        ];
        store.sync("proj1", &report, &assets).unwrap();
        let item_id = store.get_queue("proj1", None).items[0].id.clone();
        store.update_state(&item_id, SerReviewState::Reviewed, None, None).unwrap();

        // 变为 exists=false，但 reviewed 应保留
        let assets_missing = vec![
            SerClaudeMemoryAsset {
                id: "a1".to_string(),
                scope: "project".to_string(),
                asset_type: "project_rule".to_string(),
                logical_path: "/test/a1".to_string(),
                native_path: "/test/a1".to_string(),
                content_hash: None,
                content_preview: None,
                content_truncated: false,
                line_count: Some(50),
                byte_size: Some(5000),
                mtime_ms: Some(unix_now() * 1000),
                frontmatter: None,
                secret_issues: Vec::new(),
                exists: false,
            },
        ];
        let empty_report = SerMemoryHealthReport {
            overall_score: 100,
            freshness: SerHealthDimension {
                name: "freshness".to_string(),
                score: 100,
                reason: "ok".to_string(),
                contributing_assets: Vec::new(),
            },
            quality: SerHealthDimension {
                name: "quality".to_string(),
                score: 100,
                reason: "ok".to_string(),
                contributing_assets: Vec::new(),
            },
            coverage: SerHealthDimension {
                name: "coverage".to_string(),
                score: 100,
                reason: "ok".to_string(),
                contributing_assets: Vec::new(),
            },
            cleanliness: SerHealthDimension {
                name: "cleanliness".to_string(),
                score: 100,
                reason: "ok".to_string(),
                contributing_assets: Vec::new(),
            },
            safety: SerHealthDimension {
                name: "safety".to_string(),
                score: 100,
                reason: "ok".to_string(),
                contributing_assets: Vec::new(),
            },
            top_issues: Vec::new(),
            stale_assets: Vec::new(),
            duplicate_groups: Vec::new(),
        };
        store.sync("proj1", &empty_report, &assets_missing).unwrap();
        let queue = store.get_queue("proj1", None);
        assert_eq!(queue.items.len(), 1, "reviewed item 应保留");
        assert_eq!(queue.items[0].state, SerReviewState::Reviewed);
    }
}
