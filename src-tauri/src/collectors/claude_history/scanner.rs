use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;

use serde_json::Value;

use super::models::{SerClaudeSession, SerHistoryEntry, SerProjectSessionGroup, SerSessionStatus};
use super::path_codec::{claude_config_dir, decode_project_dir, encode_cwd_path};

/// 扫描所有 Claude Code 会话并按项目分组
pub fn list_claude_sessions() -> Result<Vec<SerProjectSessionGroup>, String> {
    let config_dir = claude_config_dir().ok_or("无法获取用户主目录")?;
    if !config_dir.exists() {
        return Ok(Vec::new());
    }

    // 1. 扫描活跃会话
    let active_sessions = scan_active_sessions(&config_dir)?;

    // 2. 扫描 projects/ 目录
    let mut groups = scan_projects(&config_dir, &active_sessions)?;

    // 3. 活跃会话置顶排序
    for group in &mut groups {
        group.sessions.sort_by(|a, b| {
            let a_active = if a.is_active { 1 } else { 0 };
            let b_active = if b.is_active { 1 } else { 0 };
            b_active.cmp(&a_active)
                .then_with(|| b.started_at.unwrap_or(0).cmp(&a.started_at.unwrap_or(0)))
        });
    }

    // 4. 活跃会话数量多的项目排前面
    groups.sort_by(|a, b| {
        let a_active = a.sessions.iter().filter(|s| s.is_active).count();
        let b_active = b.sessions.iter().filter(|s| s.is_active).count();
        b_active.cmp(&a_active)
            .then_with(|| b.sessions.len().cmp(&a.sessions.len()))
    });

    Ok(groups)
}

/// 获取单个会话详情
pub fn get_session_detail(session_id: &str) -> Result<Option<SerClaudeSession>, String> {
    let groups = list_claude_sessions()?;
    for group in groups {
        if let Some(session) = group.sessions.into_iter().find(|s| s.session_id == session_id) {
            return Ok(Some(session));
        }
    }
    Ok(None)
}

/// 搜索历史命令（从 history.jsonl 中过滤）
pub fn search_claude_history(query: &str) -> Result<Vec<SerHistoryEntry>, String> {
    let config_dir = claude_config_dir().ok_or("无法获取用户主目录")?;
    let history_path = config_dir.join("history.jsonl");

    if !history_path.exists() {
        return Ok(Vec::new());
    }

    let file = fs::File::open(&history_path).map_err(|e| e.to_string())?;
    let reader = BufReader::new(file);
    let query_lower = query.to_lowercase();
    let mut results = Vec::new();

    for line in reader.lines() {
        let line = match line { Ok(l) => l, Err(_) => continue };
        if line.trim().is_empty() { continue; }

        match serde_json::from_str::<Value>(&line) {
            Ok(value) => {
                let display = value.get("display").and_then(|v| v.as_str()).unwrap_or("");
                let session_id = value.get("sessionId").and_then(|v| v.as_str()).unwrap_or("");
                let project = value.get("project").and_then(|v| v.as_str()).unwrap_or("");

                if display.to_lowercase().contains(&query_lower)
                    || session_id.to_lowercase().contains(&query_lower)
                    || project.to_lowercase().contains(&query_lower)
                {
                    results.push(SerHistoryEntry {
                        display: display.to_string(),
                        timestamp: value.get("timestamp").and_then(|v| v.as_u64()).unwrap_or(0),
                        session_id: session_id.to_string(),
                        project_path: project.to_string(),
                    });
                }
            }
            Err(_) => continue,
        }
    }

    results.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    Ok(results)
}

/// 删除非活跃会话
pub fn delete_claude_session(session_id: &str) -> Result<(), String> {
    let config_dir = claude_config_dir().ok_or("无法获取用户主目录")?;

    // 1. 检查是否活跃
    let active_sessions = scan_active_sessions(&config_dir)?;
    if active_sessions.contains_key(session_id) {
        return Err("无法删除正在运行的会话".to_string());
    }

    // 2. 查找 .jsonl 文件
    let projects_dir = config_dir.join("projects");
    if !projects_dir.is_dir() {
        return Err("会话文件不存在或已被删除".to_string());
    }

    for entry in fs::read_dir(&projects_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if !path.is_dir() { continue; }

        let jsonl_path = path.join(format!("{}.jsonl", session_id));
        if jsonl_path.exists() {
            // 3. 再次检查活跃状态（缓解竞态）
            let active_sessions = scan_active_sessions(&config_dir)?;
            if active_sessions.contains_key(session_id) {
                return Err("无法删除正在运行的会话".to_string());
            }

            // 4. 删除文件
            fs::remove_file(&jsonl_path).map_err(|e| e.to_string())?;

            // 5. 若目录为空，清理空目录
            if let Ok(mut entries) = fs::read_dir(&path) {
                if entries.next().is_none() {
                    let _ = fs::remove_dir(&path);
                }
            }

            return Ok(());
        }
    }

    Err("会话文件不存在或已被删除".to_string())
}

// ============================================================================
// 内部辅助函数
// ============================================================================

struct ActiveSessionInfo {
    session_id: String,
    name: Option<String>,
    cwd: String,
    status: String,
    started_at: Option<u64>,
    updated_at: Option<u64>,
}

fn scan_active_sessions(config_dir: &Path) -> Result<HashMap<String, ActiveSessionInfo>, String> {
    let sessions_dir = config_dir.join("sessions");
    let mut active = HashMap::new();

    if !sessions_dir.is_dir() {
        return Ok(active);
    }

    for entry in fs::read_dir(&sessions_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") { continue; }

        match fs::read_to_string(&path) {
            Ok(content) => {
                if let Ok(value) = serde_json::from_str::<Value>(&content) {
                    let sid = value.get("sessionId").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    active.insert(sid.clone(), ActiveSessionInfo {
                        session_id: sid,
                        name: value.get("name").and_then(|v| v.as_str()).map(|s| s.to_string()),
                        cwd: value.get("cwd").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        status: value.get("status").and_then(|v| v.as_str()).unwrap_or("unknown").to_string(),
                        started_at: value.get("startedAt").and_then(|v| v.as_u64()),
                        updated_at: value.get("updatedAt").and_then(|v| v.as_u64()),
                    });
                }
            }
            Err(_) => continue,
        }
    }

    Ok(active)
}

fn scan_projects(
    config_dir: &Path,
    active_sessions: &HashMap<String, ActiveSessionInfo>,
) -> Result<Vec<SerProjectSessionGroup>, String> {
    let projects_dir = config_dir.join("projects");
    let mut groups = Vec::new();

    if !projects_dir.is_dir() {
        return Ok(groups);
    }

    for entry in fs::read_dir(&projects_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let encoded_dir = entry.path();
        if !encoded_dir.is_dir() { continue; }

        let dir_name = encoded_dir.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
        let project_path = decode_project_dir(&dir_name);
        let project_name = Path::new(&project_path)
            .file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
        let is_orphaned = !Path::new(&project_path).exists();

        let mut sessions = Vec::new();

        for file_entry in fs::read_dir(&encoded_dir).map_err(|e| e.to_string())? {
            let file_entry = file_entry.map_err(|e| e.to_string())?;
            let file_path = file_entry.path();
            if file_path.extension().and_then(|e| e.to_str()) != Some("jsonl") { continue; }

            let session_id = file_path.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string();

            let mtime = file_entry.metadata().ok()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(std::time::SystemTime::UNIX_EPOCH).ok())
                .map(|d| d.as_secs() * 1000);

            let turn_count = count_jsonl_lines(&file_path).unwrap_or(None);

            let (name, status, started_at, updated_at, is_active) =
                if let Some(active) = active_sessions.get(&session_id) {
                    (
                        active.name.clone().or_else(|| extract_session_name(&file_path)),
                        parse_status(&active.status),
                        active.started_at,
                        active.updated_at,
                        true,
                    )
                } else {
                    (
                        extract_session_name(&file_path),
                        SerSessionStatus::Exited,
                        mtime,
                        None,
                        false,
                    )
                };

            sessions.push(SerClaudeSession {
                session_id,
                name,
                cwd: project_path.clone(),
                status,
                started_at,
                updated_at,
                turn_count,
                is_active,
            });
        }

        if !sessions.is_empty() {
            groups.push(SerProjectSessionGroup {
                project_path,
                project_name,
                sessions,
                session_count: 0,
                is_orphaned,
            });
        }
    }

    for group in &mut groups {
        group.session_count = group.sessions.len();
    }

    Ok(groups)
}

fn parse_status(status: &str) -> SerSessionStatus {
    match status {
        "active" | "busy" => SerSessionStatus::Active,
        "idle" => SerSessionStatus::Idle,
        "exited" => SerSessionStatus::Exited,
        _ => SerSessionStatus::Unknown,
    }
}

fn extract_session_name(jsonl_path: &Path) -> Option<String> {
    let file = fs::File::open(jsonl_path).ok()?;
    let reader = BufReader::new(file);
    for line in reader.lines().take(10) {
        let line = line.ok()?;
        if let Ok(value) = serde_json::from_str::<Value>(&line) {
            if let Some(name) = value.get("name").and_then(|v| v.as_str()) {
                return Some(name.to_string());
            }
        }
    }
    None
}

fn count_jsonl_lines(path: &Path) -> Result<Option<usize>, String> {
    let file = fs::File::open(path).map_err(|e| e.to_string())?;
    let reader = BufReader::new(file);
    let mut count = 0;
    for line in reader.lines() {
        if line.is_ok() { count += 1; }
    }
    Ok(Some(count))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_claude_sessions_empty() {
        let result = list_claude_sessions();
        assert!(result.is_ok());
    }

    #[test]
    fn test_delete_nonexistent_session() {
        let result = delete_claude_session("nonexistent-session-id");
        assert!(result.is_err());
    }

    #[test]
    fn test_search_claude_history_empty() {
        let result = search_claude_history("test");
        assert!(result.is_ok());
    }
}
