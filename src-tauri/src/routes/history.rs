use crate::collectors::claude_history::models::{
    ExportFormat, SerClaudeSession, SerHistoryEntry, SerProjectSessionGroup, SerSessionPreview,
};
use crate::services::history_service::{
    delete_claude_session_service, export_claude_session_service,
    get_claude_session_detail_service, list_claude_sessions_service,
    preview_claude_session_service, search_claude_history_service,
};

#[tauri::command]
pub async fn list_claude_sessions_cmd() -> Result<Vec<SerProjectSessionGroup>, String> {
    // 使用 spawn_blocking 避免全量 JSONL 扫描阻塞 Tauri 主事件循环
    tauri::async_runtime::spawn_blocking(list_claude_sessions_service)
        .await
        .map_err(|e| format!("扫描任务被取消: {}", e))?
}

#[tauri::command]
pub fn get_claude_session_detail_cmd(
    session_id: String,
) -> Result<Option<SerClaudeSession>, String> {
    get_claude_session_detail_service(session_id)
}

#[tauri::command]
pub fn search_claude_history_cmd(query: String) -> Result<Vec<SerHistoryEntry>, String> {
    search_claude_history_service(query)
}

#[tauri::command]
pub fn delete_claude_session_cmd(session_id: String) -> Result<(), String> {
    delete_claude_session_service(session_id)
}

#[tauri::command]
pub fn export_claude_session_cmd(
    session_id: String,
    format: ExportFormat,
    output_path: String,
) -> Result<String, String> {
    export_claude_session_service(session_id, format, output_path)
}

#[tauri::command]
pub fn preview_claude_session_cmd(session_id: String) -> Result<SerSessionPreview, String> {
    preview_claude_session_service(session_id)
}
