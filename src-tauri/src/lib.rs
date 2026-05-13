pub mod collectors;
pub mod commands;
pub mod registry;
pub mod watcher;

use commands::{
    add_project, delete_claude_session_cmd, export_claude_session_cmd,
    get_claude_session_detail_cmd, preview_claude_session_cmd,
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

            // 数据迁移：从旧目录 ptv 迁移到新目录 agent-scope
            let old_data_dir = dirs::data_local_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join("ptv");
            if old_data_dir.exists() && !data_dir.exists() {
                if let Err(e) = std::fs::create_dir_all(&data_dir) {
                    eprintln!("[migrate] 无法创建新数据目录: {}", e);
                } else {
                    let files_to_migrate = ["projects.json", "settings.json"];
                    for file in &files_to_migrate {
                        let old_file = old_data_dir.join(file);
                        let new_file = data_dir.join(file);
                        if old_file.exists() && !new_file.exists() {
                            if let Err(e) = std::fs::copy(&old_file, &new_file) {
                                eprintln!("[migrate] 无法复制 {}: {}", file, e);
                            } else {
                                println!("[migrate] 成功迁移 {}", file);
                            }
                        }
                    }
                }
            }

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
            export_claude_session_cmd,
            preview_claude_session_cmd,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
