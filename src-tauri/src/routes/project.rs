use tauri::State;

use crate::app_state::AppState;
use crate::models::project::{SerProjectFile, TemplateDataPayload};
use crate::registry::ProjectEntry;
use crate::services::project_service::{
    add_project as add_project_service, get_project_data as get_project_data_service,
    get_project_file_content as get_project_file_content_service,
    get_project_files as get_project_files_service, list_projects as list_projects_service,
    remove_project as remove_project_service,
};

#[tauri::command(rename = "add_project")]
pub fn add_project_cmd(path: String, state: State<'_, AppState>) -> Result<ProjectEntry, String> {
    add_project_service(path, state)
}

#[tauri::command(rename = "remove_project")]
pub fn remove_project_cmd(path: String, state: State<'_, AppState>) -> Result<(), String> {
    remove_project_service(path, state)
}

#[tauri::command(rename = "list_projects")]
pub fn list_projects_cmd(state: State<'_, AppState>) -> Vec<ProjectEntry> {
    list_projects_service(state)
}

#[tauri::command(rename = "get_project_data")]
pub fn get_project_data_cmd(
    path: String,
    _state: State<'_, AppState>,
) -> Result<TemplateDataPayload, String> {
    get_project_data_service(path)
}

#[tauri::command(rename = "get_project_files")]
pub fn get_project_files_cmd(
    path: String,
    state: State<'_, AppState>,
) -> Result<Vec<SerProjectFile>, String> {
    get_project_files_service(path, state)
}

#[tauri::command(rename = "get_project_file_content")]
pub fn get_project_file_content_cmd(path: String, relative_path: String) -> Result<String, String> {
    get_project_file_content_service(path, relative_path)
}
