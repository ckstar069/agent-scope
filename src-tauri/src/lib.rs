pub mod app_state;
pub mod collectors;
pub mod models;
pub mod registry;
pub mod routes;
pub mod services;
pub mod utils;
pub mod watcher;

use app_state::init_app_state;
use collectors::agent::AgentCollector;
use registry::ProjectRegistry;
use routes::{
    add_project_cmd, delete_claude_session_cmd, export_claude_session_cmd,
    get_claude_memory_file_content_cmd, get_claude_memory_overview_cmd,
    get_claude_session_detail_cmd, get_context_pressure_cmd, get_latest_session_cmd,
    get_memory_health_report_cmd, get_project_data_cmd, get_project_file_content_cmd,
    get_project_files_cmd, get_session_transcript_cmd, get_template_path_cmd,
    list_claude_sessions_cmd, list_project_sessions_cmd, list_projects_cmd,
    preview_claude_session_cmd, remove_project_cmd, save_candidate_memory_cmd,
    search_claude_history_cmd, search_sessions_cmd, set_template_path_cmd,
    simulate_claude_memory_load_chain_cmd, start_watching_cmd, stop_watching_cmd,
};

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

            init_app_state(app, registry, agent_collector);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            add_project_cmd,
            remove_project_cmd,
            list_projects_cmd,
            get_project_data_cmd,
            get_project_files_cmd,
            get_project_file_content_cmd,
            start_watching_cmd,
            stop_watching_cmd,
            get_latest_session_cmd,
            list_project_sessions_cmd,
            search_sessions_cmd,
            get_session_transcript_cmd,
            save_candidate_memory_cmd,
            set_template_path_cmd,
            get_template_path_cmd,
            list_claude_sessions_cmd,
            get_claude_session_detail_cmd,
            search_claude_history_cmd,
            delete_claude_session_cmd,
            export_claude_session_cmd,
            preview_claude_session_cmd,
            get_claude_memory_overview_cmd,
            get_claude_memory_file_content_cmd,
            simulate_claude_memory_load_chain_cmd,
            get_memory_health_report_cmd,
            get_context_pressure_cmd,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
