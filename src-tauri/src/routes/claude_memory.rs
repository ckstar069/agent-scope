use std::collections::HashSet;
use std::path::PathBuf;

use tauri::State;

use crate::app_state::AppState;
use crate::collectors::claude_memory::models::{
    SerClaudeMemoryDashboard, SerClaudeMemoryScanResult, SerContextPressure, SerLoadChain,
    SerMemoryHealthReport, SerMemorySummary, SerReviewItem, SerReviewQueue, SerReviewQueueCounts,
    SerReviewQueueSyncResult,
};
use crate::collectors::claude_memory::pressure::compute_context_pressure;
use crate::collectors::claude_memory::review_queue::canonicalize_project_id;
use crate::collectors::claude_memory::scanner::{scan_claude_memory, scan_project_level};
use crate::collectors::claude_memory::secret_scanner::SecretScanner;
use crate::collectors::claude_memory::health_checker::compute_health_report;
use crate::services::claude_memory_service::{
    get_claude_memory_file_content_service,
    get_claude_memory_overview_service, get_context_pressure_service,
    get_memory_health_report_service, get_review_queue_counts_service, get_review_queue_service,
    simulate_load_chain_service, sync_review_queue_service, update_review_item_state_service,
};

#[tauri::command(rename = "get_claude_memory_overview")]
pub fn get_claude_memory_overview_cmd(
    project_path: Option<String>,
    force: bool,
    state: State<'_, AppState>,
) -> Result<SerClaudeMemoryScanResult, String> {
    get_claude_memory_overview_service(project_path, force, state.inner())
}

#[tauri::command(rename = "get_claude_memory_dashboard")]
pub async fn get_claude_memory_dashboard_cmd(
    project_path: Option<String>,
    _force: bool,
    state: State<'_, AppState>,
) -> Result<SerClaudeMemoryDashboard, String> {
    // 1. 先获取 review_queue 和已注册项目路径（轻量操作，快速释放锁）
    let project_id = canonicalize_project_id(project_path.as_deref());
    let review_queue = {
        let store = state.review_queue.lock().map_err(|e| e.to_string())?;
        store.get_queue(&project_id, None)
    };
    let registered_paths: Vec<String> = {
        let registry = state.registry.lock().map_err(|e| e.to_string())?;
        registry.list().into_iter().map(|p| p.path).collect()
    };

    // 2. 使用 spawn_blocking 避免扫描阻塞 Tauri 主事件循环
    let project_path_clone = project_path.clone();
    let spawn_result: Result<(SerClaudeMemoryScanResult, SerMemoryHealthReport, SerContextPressure), String> =
        tauri::async_runtime::spawn_blocking(move || {
            // 2a. 执行基础扫描（用户级 + 可选 project_path）
            let mut result = scan_claude_memory(project_path_clone.clone());

            // 2b. 收集已扫描的 canonical project path（用于去重）
            let mut scanned_paths: HashSet<PathBuf> = HashSet::new();
            if let Some(ref extra) = project_path_clone {
                if let Ok(canonical) = std::fs::canonicalize(extra) {
                    scanned_paths.insert(canonical);
                }
            }

            // 2c. 扫描已注册项目，跳过已扫描的路径
            let scanner = SecretScanner::new();
            for project_path in &registered_paths {
                let path = PathBuf::from(project_path);
                let canonical = match std::fs::canonicalize(&path) {
                    Ok(c) => c,
                    Err(e) => {
                        result.errors.push(
                            crate::collectors::claude_memory::models::SerMemoryScanError {
                                scope: "project".to_string(),
                                path: project_path.clone(),
                                message: format!("canonicalize 失败: {}", e),
                            },
                        );
                        continue;
                    }
                };
                if scanned_paths.contains(&canonical) {
                    continue;
                }
                scanned_paths.insert(canonical);
                scan_project_level(&path, &mut result.assets, &scanner, &mut result.errors);
            }
            result.summary = build_summary(&result.assets);

            let health = compute_health_report(&result.assets);
            let pressure = compute_context_pressure(&result.assets);
            Ok::<_, String>((result, health, pressure))
        })
        .await
        .map_err(|e| format!("Dashboard 扫描任务被取消: {}", e))?;

    let (overview, health_report, context_pressure) = spawn_result?;

    Ok(SerClaudeMemoryDashboard {
        overview,
        health_report,
        context_pressure,
        review_queue,
    })
}

fn build_summary(
    assets: &[crate::collectors::claude_memory::models::SerClaudeMemoryAsset],
) -> SerMemorySummary {
    let total_assets = assets.len();
    let total_existing = assets.iter().filter(|a| a.exists).count();
    let total_secret_issues: usize = assets.iter().map(|a| a.secret_issues.len()).sum();

    let mut by_scope = std::collections::HashMap::new();
    let mut by_type = std::collections::HashMap::new();

    for asset in assets {
        *by_scope.entry(asset.scope.clone()).or_insert(0) += 1;
        *by_type.entry(asset.asset_type.clone()).or_insert(0) += 1;
    }

    SerMemorySummary {
        total_assets,
        total_existing,
        by_scope,
        by_type,
        total_secret_issues,
    }
}

#[tauri::command(rename = "get_claude_memory_file_content")]
pub fn get_claude_memory_file_content_cmd(
    native_path: String,
    project_path: Option<String>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    get_claude_memory_file_content_service(native_path, project_path, state.inner())
}

#[tauri::command(rename = "simulate_claude_memory_load_chain")]
pub fn simulate_claude_memory_load_chain_cmd(
    cwd: String,
    _state: State<'_, AppState>,
) -> Result<SerLoadChain, String> {
    simulate_load_chain_service(cwd)
}

#[tauri::command(rename = "get_memory_health_report")]
pub fn get_memory_health_report_cmd(
    project_path: Option<String>,
    force: bool,
    state: State<'_, AppState>,
) -> Result<SerMemoryHealthReport, String> {
    get_memory_health_report_service(project_path, force, state.inner())
}

#[tauri::command(rename = "get_context_pressure")]
pub fn get_context_pressure_cmd(
    project_path: Option<String>,
    force: bool,
    state: State<'_, AppState>,
) -> Result<SerContextPressure, String> {
    get_context_pressure_service(project_path, force, state.inner())
}

#[tauri::command(rename = "get_review_queue")]
pub fn get_review_queue_cmd(
    project_path: Option<String>,
    filter: Option<String>,
    state: State<'_, AppState>,
) -> Result<SerReviewQueue, String> {
    get_review_queue_service(project_path, filter, state.inner())
}

#[tauri::command(rename = "sync_review_queue")]
pub fn sync_review_queue_cmd(
    project_path: Option<String>,
    force: bool,
    state: State<'_, AppState>,
) -> Result<SerReviewQueueSyncResult, String> {
    sync_review_queue_service(project_path, force, state.inner())
}

#[tauri::command(rename = "update_review_item_state")]
pub fn update_review_item_state_cmd(
    item_id: String,
    new_state: String,
    snooze_days: Option<u32>,
    note: Option<String>,
    state: State<'_, AppState>,
) -> Result<SerReviewItem, String> {
    update_review_item_state_service(item_id, new_state, snooze_days, note, state.inner())
}

#[tauri::command(rename = "get_review_queue_counts")]
pub fn get_review_queue_counts_cmd(
    project_path: Option<String>,
    state: State<'_, AppState>,
) -> Result<SerReviewQueueCounts, String> {
    get_review_queue_counts_service(project_path, state.inner())
}
