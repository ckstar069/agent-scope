use tauri::State;

use crate::app_state::AppState;
use crate::services::settings_service::{get_template_path, set_template_path};

#[tauri::command]
pub fn get_template_path_cmd(state: State<'_, AppState>) -> Result<Option<String>, String> {
    get_template_path(state)
}

#[tauri::command]
pub fn set_template_path_cmd(
    path: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    set_template_path(path, state)
}
