pub mod collectors;
pub mod commands;
pub mod registry;
pub mod watcher;

use commands::{
    add_project, delete_claude_session_cmd, get_claude_session_detail_cmd,
    get_latest_session, get_project_data, get_project_files, get_project_file_content,
    get_session_transcript, get_template_path, list_claude_sessions_cmd,
    list_project_sessions, list_projects, remove_project, save_candidate_memory,
    search_claude_history_cmd, search_sessions, set_template_path, start_watching,
    stop_watching,
};
use collectors::agent::AgentCollector;
use registry::ProjectRegistry;

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            let data_dir = ProjectRegistry::default_data_dir();
            let storage_path = data_dir.join("projects.json");
            let registry = ProjectRegistry::load_or_default(storage_path);

            let agent_collector = AgentCollector::new();

            for entry in registry.list() {
                agent_collector.register_project(entry.path);
            }

            let agent_handle = app.handle().clone();
            let _join_handle = agent_collector.start(agent_handle);

            commands::init_app_state(app, registry, agent_collector);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            add_project,
            remove_project,
            list_projects,
            get_project_data,
            get_project_files,
            get_project_file_content,
            start_watching,
            stop_watching,
            get_latest_session,
            list_project_sessions,
            search_sessions,
            get_session_transcript,
            save_candidate_memory,
            set_template_path,
            get_template_path,
            list_claude_sessions_cmd,
            get_claude_session_detail_cmd,
            search_claude_history_cmd,
            delete_claude_session_cmd,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
