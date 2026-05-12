use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;

use serde_json::Value;

use super::models::{ExportFormat, SerClaudeSession, SerHistoryEntry, SerPreviewMessage, SerProjectSessionGroup, SerSessionPreview, SerSessionStatus};
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

/// 导出会话内容
pub fn export_claude_session(session_id: &str, format: ExportFormat) -> Result<String, String> {
    let config_dir = claude_config_dir().ok_or("无法获取用户主目录")?;

    // 1. 查找 .jsonl 文件
    let projects_dir = config_dir.join("projects");
    if !projects_dir.is_dir() {
        return Err("会话文件不存在或已被删除".to_string());
    }

    let mut jsonl_path: Option<std::path::PathBuf> = None;

    for entry in fs::read_dir(&projects_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if !path.is_dir() { continue; }

        let candidate = path.join(format!("{}.jsonl", session_id));
        if candidate.exists() {
            jsonl_path = Some(candidate);
            break;
        }
    }

    let jsonl_path = jsonl_path.ok_or("会话文件不存在或已被删除")?;

    // 2. 根据格式处理
    match format {
        ExportFormat::Jsonl => {
            fs::read_to_string(&jsonl_path).map_err(|e| e.to_string())
        }
        ExportFormat::Markdown => {
            jsonl_to_markdown(&jsonl_path, session_id)
        }
    }
}

/// 预览会话内容（提取前 N 条消息）
pub fn preview_claude_session(session_id: &str, limit: usize) -> Result<SerSessionPreview, String> {
    let config_dir = claude_config_dir().ok_or("无法获取用户主目录")?;

    // 1. 查找 .jsonl 文件
    let projects_dir = config_dir.join("projects");
    if !projects_dir.is_dir() {
        return Err("会话文件不存在或已被删除".to_string());
    }

    let mut jsonl_path: Option<std::path::PathBuf> = None;

    for entry in fs::read_dir(&projects_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if !path.is_dir() { continue; }

        let candidate = path.join(format!("{}.jsonl", session_id));
        if candidate.exists() {
            jsonl_path = Some(candidate);
            break;
        }
    }

    let jsonl_path = jsonl_path.ok_or("会话文件不存在或已被删除")?;

    // 2. 解析 JSONL 提取消息
    let file = fs::File::open(&jsonl_path).map_err(|e| e.to_string())?;
    let reader = BufReader::new(file);
    let mut messages: Vec<SerPreviewMessage> = Vec::new();
    let mut total_turns = 0;

    for line in reader.lines() {
        let line = match line { Ok(l) => l, Err(_) => continue };
        if line.trim().is_empty() { continue; }

        let value: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let msg_type = value.get("type").and_then(|v| v.as_str()).unwrap_or("");

        match msg_type {
            "last-prompt" => {
                if let Some(prompt) = value.get("lastPrompt").and_then(|v| v.as_str()) {
                    total_turns += 1;
                    if messages.len() < limit {
                        messages.push(SerPreviewMessage {
                            role: "user".to_string(),
                            content: prompt.to_string(),
                            timestamp: None,
                        });
                    }
                }
            }
            "text" => {
                if let Some(message) = value.get("message") {
                    let role = message.get("role").and_then(|v| v.as_str()).unwrap_or("assistant");
                    if let Some(content) = message.get("content").and_then(|c| c.as_array()) {
                        for item in content {
                            if let Some(text) = item.get("text").and_then(|v| v.as_str()) {
                                if messages.len() < limit {
                                    messages.push(SerPreviewMessage {
                                        role: role.to_string(),
                                        content: text.to_string(),
                                        timestamp: None,
                                    });
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }

        if messages.len() >= limit {
            break;
        }
    }

    Ok(SerSessionPreview {
        session_id: session_id.to_string(),
        messages,
        total_turns,
    })
}

fn jsonl_to_markdown(path: &std::path::Path, session_id: &str) -> Result<String, String> {
    let file = fs::File::open(path).map_err(|e| e.to_string())?;
    let reader = BufReader::new(file);

    let mut md = format!("# Claude Code Session: `{}`\n\n", session_id);
    let mut turn_number = 0;

    for line in reader.lines() {
        let line = match line { Ok(l) => l, Err(_) => continue };
        if line.trim().is_empty() { continue; }

        let value: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let msg_type = value.get("type").and_then(|v| v.as_str()).unwrap_or("");

        match msg_type {
            "custom-title" => {
                if let Some(title) = value.get("customTitle").and_then(|v| v.as_str()) {
                    md.push_str(&format!("\n## {}\n\n", title));
                }
            }
            "last-prompt" => {
                if let Some(prompt) = value.get("lastPrompt").and_then(|v| v.as_str()) {
                    turn_number += 1;
                    md.push_str(&format!("### Turn {}\n\n", turn_number));
                    md.push_str(&format!("**User**: {}\n\n", prompt));
                }
            }
            "text" => {
                if let Some(message) = value.get("message") {
                    let role = message.get("role").and_then(|v| v.as_str()).unwrap_or("assistant");
                    if let Some(content) = message.get("content").and_then(|c| c.as_array()) {
                        for item in content {
                            if let Some(text) = item.get("text").and_then(|v| v.as_str()) {
                                let label = match role {
                                    "user" => "**User**",
                                    _ => "**Assistant**",
                                };
                                md.push_str(&format!("{}: {}\n\n", label, text));
                            }
                        }
                    }
                }
            }
            "tool_use" => {
                if let Some(message) = value.get("message") {
                    if let Some(content) = message.get("content").and_then(|c| c.as_array()) {
                        for item in content {
                            let name = item.get("name").and_then(|v| v.as_str()).unwrap_or("");
                            if !name.is_empty() {
                                md.push_str(&format!("> \u{1f527} Tool: `{}`\n\n", name));
                            }
                        }
                    }
                }
            }
            "tool_result" => {
                if let Some(message) = value.get("message") {
                    if let Some(content) = message.get("content").and_then(|c| c.as_array()) {
                        for item in content {
                            let is_error = item.get("is_error").and_then(|v| v.as_bool()).unwrap_or(false);
                            if let Some(text) = item.get("content").and_then(|v| v.as_str()) {
                                let status = if is_error { "error" } else { "ok" };
                                let preview = if text.len() > 200 {
                                    format!("{}... (truncated)", &text[..200])
                                } else {
                                    text.to_string()
                                };
                                md.push_str(&format!("> Tool result ({}): `{}`\n\n", status, preview.replace('\n', " ")));
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    Ok(md)
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

    // =========================================================================
    // 导出功能测试
    // =========================================================================

    #[test]
    fn test_export_nonexistent_session() {
        let result = export_claude_session("nonexistent-session-id-12345", ExportFormat::Jsonl);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("不存在"));
    }

    #[test]
    fn test_jsonl_to_markdown_empty_file() {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("test-empty.jsonl");
        std::fs::write(&path, "").unwrap();

        let result = jsonl_to_markdown(&path, "test-session");
        assert!(result.is_ok());
        let md = result.unwrap();
        assert!(md.contains("# Claude Code Session: `test-session`"));

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_jsonl_to_markdown_text_messages() {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("test-text.jsonl");

        let lines = vec![
            r#"{"type":"text","message":{"role":"user","content":[{"type":"text","text":"Hello"}]}}"#,
            r#"{"type":"text","message":{"role":"assistant","content":[{"type":"text","text":"Hi there"}]}}"#,
        ];
        std::fs::write(&path, lines.join("\n")).unwrap();

        let result = jsonl_to_markdown(&path, "text-session");
        assert!(result.is_ok());
        let md = result.unwrap();
        assert!(md.contains("**User**: Hello"));
        assert!(md.contains("**Assistant**: Hi there"));

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_jsonl_to_markdown_last_prompt() {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("test-prompt.jsonl");

        let line = r#"{"type":"last-prompt","lastPrompt":"请分析这段代码","sessionId":"s1"}"#;
        std::fs::write(&path, line).unwrap();

        let result = jsonl_to_markdown(&path, "prompt-session");
        assert!(result.is_ok());
        let md = result.unwrap();
        assert!(md.contains("### Turn 1"));
        assert!(md.contains("**User**: 请分析这段代码"));

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_jsonl_to_markdown_custom_title() {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("test-title.jsonl");

        let line = r#"{"type":"custom-title","customTitle":"架构设计阶段","sessionId":"s1"}"#;
        std::fs::write(&path, line).unwrap();

        let result = jsonl_to_markdown(&path, "title-session");
        assert!(result.is_ok());
        let md = result.unwrap();
        assert!(md.contains("## 架构设计阶段"));

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_jsonl_to_markdown_tool_use() {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("test-tool.jsonl");

        let line = r#"{"type":"tool_use","message":{"role":"assistant","content":[{"type":"tool_use","name":"Read","input":{"file_path":"/tmp/test.txt"}}]}}"#;
        std::fs::write(&path, line).unwrap();

        let result = jsonl_to_markdown(&path, "tool-session");
        assert!(result.is_ok());
        let md = result.unwrap();
        assert!(md.contains("🔧 Tool: `Read`"));

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_jsonl_to_markdown_tool_result() {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("test-result.jsonl");

        let line = r#"{"type":"tool_result","message":{"role":"user","content":[{"type":"tool_result","content":"File content here","is_error":false,"tool_use_id":"t1"}]}}"#;
        std::fs::write(&path, line).unwrap();

        let result = jsonl_to_markdown(&path, "result-session");
        assert!(result.is_ok());
        let md = result.unwrap();
        assert!(md.contains("Tool result (ok)"));
        assert!(md.contains("File content here"));

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_jsonl_to_markdown_tool_result_error() {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("test-result-err.jsonl");

        let line = r#"{"type":"tool_result","message":{"role":"user","content":[{"type":"tool_result","content":"Error occurred","is_error":true,"tool_use_id":"t1"}]}}"#;
        std::fs::write(&path, line).unwrap();

        let result = jsonl_to_markdown(&path, "result-err-session");
        assert!(result.is_ok());
        let md = result.unwrap();
        assert!(md.contains("Tool result (error)"));

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_jsonl_to_markdown_mixed_content() {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("test-mixed.jsonl");

        let lines = vec![
            r#"{"type":"custom-title","customTitle":"设计评审","sessionId":"s1"}"#,
            r#"{"type":"last-prompt","lastPrompt":"请评审这个设计","sessionId":"s1"}"#,
            r#"{"type":"text","message":{"role":"assistant","content":[{"type":"text","text":"设计整体合理"}]}}"#,
            r#"{"type":"tool_use","message":{"role":"assistant","content":[{"type":"tool_use","name":"Read","input":{}}]}}"#,
        ];
        std::fs::write(&path, lines.join("\n")).unwrap();

        let result = jsonl_to_markdown(&path, "mixed-session");
        assert!(result.is_ok());
        let md = result.unwrap();

        assert!(md.contains("# Claude Code Session: `mixed-session`"));
        assert!(md.contains("## 设计评审"));
        assert!(md.contains("### Turn 1"));
        assert!(md.contains("**User**: 请评审这个设计"));
        assert!(md.contains("**Assistant**: 设计整体合理"));
        assert!(md.contains("🔧 Tool: `Read`"));

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_jsonl_to_markdown_truncates_long_tool_result() {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("test-truncate.jsonl");

        let long_content = "a".repeat(500);
        let line = format!(
            r#"{{"type":"tool_result","message":{{"role":"user","content":[{{"type":"tool_result","content":"{}","is_error":false,"tool_use_id":"t1"}}]}}}}"#,
            long_content
        );
        std::fs::write(&path, line).unwrap();

        let result = jsonl_to_markdown(&path, "truncate-session");
        assert!(result.is_ok());
        let md = result.unwrap();
        assert!(md.contains("truncated"));

        let _ = std::fs::remove_file(&path);
    }
}
