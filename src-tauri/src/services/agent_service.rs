use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::thread;

use tauri::{AppHandle, Emitter};

use crate::app_state::AppState;
use crate::collectors::template::{
    SessionTranscriptCollector, WatchedCollector,
};
use crate::models::project::{SerSessionSummary, SerTranscript, TemplateDataPayload};
use crate::utils::describe_path_error;

pub fn start_watching(
    path: String,
    app_handle: AppHandle,
    state: &AppState,
) -> Result<(), String> {
    let path_buf = PathBuf::from(&path);

    if !path_buf.exists() {
        return Err(describe_path_error(&path));
    }

    if !path_buf.is_dir() || std::fs::read_dir(&path_buf).is_err() {
        return Err(describe_path_error(&path));
    }

    let canonical_path = path_buf
        .canonicalize()
        .unwrap_or_else(|_| path_buf.clone())
        .to_string_lossy()
        .to_string();

    {
        let watchers = state.watchers.lock().map_err(|e| format!("锁获取失败: {}", e))?;
        if watchers.contains_key(&canonical_path) {
            return Err(format!("项目已在监听中: {}", canonical_path));
        }
    }

    let watched = WatchedCollector::new(path_buf.clone());
    let (rx, stop_signal) = watched.start();

    {
        let mut watchers = state.watchers.lock().map_err(|e| format!("锁获取失败: {}", e))?;
        watchers.insert(canonical_path.clone(), stop_signal);
    }

    let project_path = canonical_path.clone();
    thread::Builder::new()
        .name(format!("agent-scope-watcher-{}", project_path))
        .spawn(move || {
            while let Ok(event) = rx.recv() {
                let payload = TemplateDataPayload::from_data(project_path.clone(), event.data);
                if let Err(e) = app_handle.emit("template-update", &payload) {
                    eprintln!("[agent_service] 发送 template-update 失败: {}", e);
                }
            }
            println!("[agent_service] 项目监听线程已退出: {}", project_path);
        })
        .map_err(|e| format!("无法启动监听线程: {}", e))?;

    println!("[agent_service] 开始监听项目: {}", canonical_path);
    Ok(())
}

pub fn stop_watching(path: String, state: &AppState) -> Result<(), String> {
    let path_buf = PathBuf::from(&path);
    let canonical_path = path_buf
        .canonicalize()
        .unwrap_or_else(|_| path_buf.clone())
        .to_string_lossy()
        .to_string();

    let mut watchers = state.watchers.lock().map_err(|e| format!("锁获取失败: {}", e))?;

    if let Some(stop_signal) = watchers.remove(&canonical_path) {
        stop_signal.store(false, Ordering::SeqCst);
        println!("[agent_service] 已停止监听项目: {}", canonical_path);
    }

    Ok(())
}

pub fn get_latest_session(path: String) -> Result<Option<SerTranscript>, String> {
    let path_buf = PathBuf::from(&path);
    if !path_buf.exists() || !path_buf.is_dir() {
        return Err(describe_path_error(&path));
    }

    match SessionTranscriptCollector::get_latest_session(&path_buf) {
        Ok(Some(transcript)) => Ok(Some(SerTranscript::from(transcript))),
        Ok(None) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

pub fn list_project_sessions(path: String) -> Result<Vec<SerSessionSummary>, String> {
    let path_buf = PathBuf::from(&path);
    if !path_buf.exists() || !path_buf.is_dir() {
        return Err(describe_path_error(&path));
    }

    match SessionTranscriptCollector::list_sessions(&path_buf) {
        Ok(sessions) => Ok(sessions.into_iter().map(SerSessionSummary::from).collect()),
        Err(e) => Err(e.to_string()),
    }
}

pub fn search_sessions(
    path: String,
    query: String,
) -> Result<Vec<SerSessionSummary>, String> {
    let path_buf = PathBuf::from(&path);
    if !path_buf.exists() || !path_buf.is_dir() {
        return Err(describe_path_error(&path));
    }

    match SessionTranscriptCollector::search_sessions(&path_buf, &query) {
        Ok(sessions) => Ok(sessions.into_iter().map(SerSessionSummary::from).collect()),
        Err(e) => Err(e.to_string()),
    }
}

pub fn get_session_transcript(
    path: String,
    session_id: String,
) -> Result<SerTranscript, String> {
    let path_buf = PathBuf::from(&path);
    if !path_buf.exists() || !path_buf.is_dir() {
        return Err(describe_path_error(&path));
    }

    match SessionTranscriptCollector::get_session(&path_buf, &session_id) {
        Ok(Some(transcript)) => Ok(SerTranscript::from(transcript)),
        Ok(None) => Err("会话未找到".to_string()),
        Err(e) => Err(e.to_string()),
    }
}
