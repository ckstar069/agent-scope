use std::path::PathBuf;
use std::sync::atomic::Ordering;
use tauri::State;

use crate::app_state::AppState;
use crate::collectors::template::{ProjectFilesCollector, TemplateDataCollector};
use crate::models::project::{SerProjectFile, TemplateDataPayload};
use crate::registry::ProjectEntry;
use crate::utils::describe_path_error;

fn is_whitelisted_path(relative_path: &str) -> bool {
    let path = std::path::Path::new(relative_path);
    let parent = path
        .parent()
        .map(|p| p.to_string_lossy())
        .unwrap_or_default();
    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy())
        .unwrap_or_default();

    if parent.is_empty() && matches!(file_name.as_ref(), "CLAUDE.md" | "AGENTS.md") {
        return true;
    }

    let whitelist_dirs = [
        ".claude/rules",
        ".sisyphus/notepads",
        ".sisyphus/plans",
        ".sisyphus/drafts",
        "docs/design",
        "docs/specs",
    ];

    whitelist_dirs.iter().any(|&dir| parent.starts_with(dir))
}

/// get_project_files 内部实现，支持测试直接调用（无需 Tauri State）
pub fn get_project_files_impl(
    path: String,
    template_paths: Option<std::collections::HashSet<String>>,
) -> Result<Vec<SerProjectFile>, String> {
    let path_buf = PathBuf::from(&path);
    if !path_buf.exists() || !path_buf.is_dir() {
        return Err(describe_path_error(&path));
    }

    match ProjectFilesCollector::collect(&path_buf) {
        Ok(mut files) => {
            if let Some(ref paths) = template_paths {
                for file in &mut files {
                    file.origin = if paths.contains(&file.relative_path) {
                        "template".to_string()
                    } else {
                        "project".to_string()
                    };
                }
            }
            Ok(files.into_iter().map(SerProjectFile::from).collect())
        }
        Err(e) => Err(e.to_string()),
    }
}

pub fn get_project_files(
    path: String,
    state: State<'_, AppState>,
) -> Result<Vec<SerProjectFile>, String> {
    let template_paths: Option<std::collections::HashSet<String>> = {
        let fp = state
            .template_fingerprint
            .lock()
            .map_err(|e| format!("锁获取失败: {}", e))?;
        fp.as_ref().map(|cache| cache.paths.clone())
    };

    get_project_files_impl(path, template_paths)
}

pub fn get_project_file_content(path: String, relative_path: String) -> Result<String, String> {
    let path_buf = PathBuf::from(&path);
    if !path_buf.exists() || !path_buf.is_dir() {
        return Err(describe_path_error(&path));
    }

    if !is_whitelisted_path(&relative_path) {
        return Err("文件不在白名单内".to_string());
    }

    let file_path = path_buf.join(&relative_path);
    match std::fs::read_to_string(&file_path) {
        Ok(content) => Ok(content),
        Err(e) => Err(format!("读取文件失败: {}", e)),
    }
}

pub fn add_project(path: String, state: State<'_, AppState>) -> Result<ProjectEntry, String> {
    let path_buf = PathBuf::from(&path);
    let mut registry = state
        .registry
        .lock()
        .map_err(|e| format!("锁获取失败: {}", e))?;

    let entry = registry.add(&path_buf).map_err(|e| e.to_string())?;

    if let Ok(agent_collector) = state.agent_collector.lock() {
        agent_collector.register_project(entry.path.clone());
    }

    Ok(entry)
}

pub fn remove_project(path: String, state: State<'_, AppState>) -> Result<(), String> {
    let path_buf = PathBuf::from(&path);
    let mut registry = state
        .registry
        .lock()
        .map_err(|e| format!("锁获取失败: {}", e))?;

    let canonical_path = path_buf
        .canonicalize()
        .unwrap_or_else(|_| path_buf.clone())
        .to_string_lossy()
        .to_string();

    registry.remove(&path_buf).map_err(|e| e.to_string())?;

    if let Ok(agent_collector) = state.agent_collector.lock() {
        agent_collector.unregister_project(&canonical_path);
    }

    drop(registry);
    if let Ok(watchers) = state.watchers.lock() {
        if let Some(stop_signal) = watchers.get(&canonical_path) {
            stop_signal.store(false, Ordering::SeqCst);
        }
    }

    Ok(())
}

pub fn list_projects(state: State<'_, AppState>) -> Vec<ProjectEntry> {
    match state.registry.lock() {
        Ok(registry) => registry.list(),
        Err(_) => Vec::new(),
    }
}

pub fn get_project_data(path: String) -> Result<TemplateDataPayload, String> {
    let path_buf = PathBuf::from(&path);

    if !path_buf.exists() {
        return Err(describe_path_error(&path));
    }

    if !path_buf.is_dir() || std::fs::read_dir(&path_buf).is_err() {
        return Err(describe_path_error(&path));
    }

    let collector = TemplateDataCollector::new(path_buf.clone());
    let data = collector.collect();

    Ok(TemplateDataPayload::from_data(path, data))
}
