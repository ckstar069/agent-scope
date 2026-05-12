use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
pub struct SerClaudeSession {
    pub session_id: String,
    pub name: Option<String>,
    pub cwd: String,
    pub status: SerSessionStatus,
    pub started_at: Option<u64>,
    pub updated_at: Option<u64>,
    pub turn_count: Option<usize>,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize)]
pub enum SerSessionStatus {
    Active,
    Idle,
    Exited,
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
pub struct SerProjectSessionGroup {
    pub project_path: String,
    pub project_name: String,
    pub sessions: Vec<SerClaudeSession>,
    pub session_count: usize,
    pub is_orphaned: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct SerHistoryEntry {
    pub display: String,
    pub timestamp: u64,
    pub session_id: String,
    pub project_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExportFormat {
    Jsonl,
    Markdown,
}

#[derive(Debug, Clone, Serialize)]
pub struct SerPreviewMessage {
    pub role: String,
    pub content: String,
    pub timestamp: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SerSessionPreview {
    pub session_id: String,
    pub messages: Vec<SerPreviewMessage>,
    pub total_turns: usize,
}
