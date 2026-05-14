use crate::models::project::SerCandidateMemory;
use crate::services::memory_service::save_candidate_memory as save_candidate_memory_service;

#[tauri::command]
pub fn save_candidate_memory_cmd(
    path: String,
    memory: SerCandidateMemory,
) -> Result<(), String> {
    save_candidate_memory_service(path, memory)
}
