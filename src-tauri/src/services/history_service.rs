use std::path::Path;

use crate::collectors::claude_history::{
    models::{
        ExportFormat, SerClaudeSession, SerHistoryEntry, SerProjectSessionGroup, SerSessionPreview,
    },
    scanner::{
        delete_claude_session, export_claude_session, get_session_detail, list_claude_sessions,
        preview_claude_session, search_claude_history,
    },
};

pub fn list_claude_sessions_service() -> Result<Vec<SerProjectSessionGroup>, String> {
    list_claude_sessions()
}

pub fn get_claude_session_detail_service(
    session_id: String,
) -> Result<Option<SerClaudeSession>, String> {
    get_session_detail(&session_id)
}

pub fn search_claude_history_service(query: String) -> Result<Vec<SerHistoryEntry>, String> {
    search_claude_history(&query)
}

pub fn delete_claude_session_service(session_id: String) -> Result<(), String> {
    delete_claude_session(&session_id)
}

pub fn export_claude_session_service(
    session_id: String,
    format: ExportFormat,
    output_path: String,
) -> Result<String, String> {
    let path = Path::new(&output_path);
    export_claude_session(&session_id, format, path)
}

pub fn preview_claude_session_service(session_id: String) -> Result<SerSessionPreview, String> {
    preview_claude_session(&session_id)
}
