use tauri::{AppHandle, State};

use crate::app_state::AppState;
use crate::models::project::{SerSessionSummary, SerTranscript};
use crate::services::agent_service;

#[tauri::command(rename = "start_watching")]
pub fn start_watching_cmd(
    path: String,
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    agent_service::start_watching(path, app_handle, &state)
}

#[tauri::command(rename = "stop_watching")]
pub fn stop_watching_cmd(path: String, state: State<'_, AppState>) -> Result<(), String> {
    agent_service::stop_watching(path, &state)
}

#[tauri::command(rename = "get_latest_session")]
pub fn get_latest_session_cmd(path: String) -> Result<Option<SerTranscript>, String> {
    agent_service::get_latest_session(path)
}

#[tauri::command(rename = "list_project_sessions")]
pub fn list_project_sessions_cmd(path: String) -> Result<Vec<SerSessionSummary>, String> {
    agent_service::list_project_sessions(path)
}

#[tauri::command(rename = "search_sessions")]
pub fn search_sessions_cmd(path: String, query: String) -> Result<Vec<SerSessionSummary>, String> {
    agent_service::search_sessions(path, query)
}

#[tauri::command(rename = "get_session_transcript")]
pub fn get_session_transcript_cmd(
    path: String,
    session_id: String,
) -> Result<SerTranscript, String> {
    agent_service::get_session_transcript(path, session_id)
}
