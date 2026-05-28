use tauri::State;

use crate::app_state::AppState;
use crate::collectors::claude_memory::models::{
    SerClaudeMemoryDashboard, SerClaudeMemoryScanResult, SerContextPressure, SerLoadChain,
    SerMemoryHealthReport, SerReviewItem, SerReviewQueue, SerReviewQueueCounts,
    SerReviewQueueSyncResult,
};
use crate::services::claude_memory_service::{
    get_claude_memory_dashboard_service, get_claude_memory_file_content_service,
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
pub fn get_claude_memory_dashboard_cmd(
    project_path: Option<String>,
    force: bool,
    state: State<'_, AppState>,
) -> Result<SerClaudeMemoryDashboard, String> {
    get_claude_memory_dashboard_service(project_path, force, state.inner())
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
