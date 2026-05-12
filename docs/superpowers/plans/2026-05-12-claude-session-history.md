# Claude Code 会话管理模块实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 为 PTV 新增独立的 Claude Code 会话管理模块，扫描本地 `~/.claude/` 并按项目组织展示历史会话，支持删除非活跃会话。

**Architecture:** 后端新建 `collectors/claude_history/` 模块负责数据扫描与解析，提取共享 `path_codec.rs` 替换 `session_transcript.rs` 中内联的 Unix-only 实现；前端新增 `"claude-history"` 路由及左右分栏页面。

**Tech Stack:** Tauri v2 (Rust), React 19 + TypeScript, Tailwind CSS, shadcn/ui

---

## 文件映射

### 后端（Rust）

| 文件 | 操作 | 职责 |
|------|------|------|
| `src-tauri/src/collectors/mod.rs` | 修改 | 新增 `pub mod claude_history;` |
| `src-tauri/src/collectors/claude_history/mod.rs` | 创建 | 模块入口，暴露 `Scanner` 和 `path_codec` |
| `src-tauri/src/collectors/claude_history/path_codec.rs` | 创建 | 跨平台路径编解码 + `claude_config_dir()` |
| `src-tauri/src/collectors/claude_history/models.rs` | 创建 | 数据结构：`SerClaudeSession`, `SerProjectSessionGroup`, `SerHistoryEntry` |
| `src-tauri/src/collectors/claude_history/scanner.rs` | 创建 | 目录扫描、数据聚合、删除逻辑 |
| `src-tauri/src/collectors/template/session_transcript.rs` | 修改 | 复用 `path_codec.rs`，移除内联 `encode_cwd_path` |
| `src-tauri/src/commands.rs` | 修改 | 新增 4 个 Tauri 命令 |
| `src-tauri/src/lib.rs` | 修改 | 注册新命令到 `invoke_handler` |

### 前端（React/TypeScript）

| 文件 | 操作 | 职责 |
|------|------|------|
| `src/App.tsx` | 修改 | 扩展 `AppRoute` 类型，新增 `"claude-history"` 路由 |
| `src/components/Sidebar.tsx` | 修改 | 新增导航项 |
| `src/hooks/useClaudeHistory.ts` | 创建 | 数据获取 hook（invoke + 缓存） |
| `src/pages/ClaudeHistory.tsx` | 创建 | 主页面，左右分栏布局 |
| `src/components/claude-history/ProjectList.tsx` | 创建 | 左侧项目列表 |
| `src/components/claude-history/SessionTimeline.tsx` | 创建 | 右侧会话时间线 |
| `src/components/claude-history/SearchBar.tsx` | 创建 | 顶部搜索过滤 |

---

## Task 1: 共享路径编解码模块 + 修复 session_transcript.rs

**Files:**
- Create: `src-tauri/src/collectors/claude_history/mod.rs`
- Create: `src-tauri/src/collectors/claude_history/path_codec.rs`
- Modify: `src-tauri/src/collectors/mod.rs`
- Modify: `src-tauri/src/collectors/template/session_transcript.rs:10-35`

---

- [ ] **Step 1: 注册新模块**

修改 `src-tauri/src/collectors/mod.rs`，新增一行：

```rust
pub mod agent;
pub mod claude_history;
pub mod template;
```

- [ ] **Step 2: 创建 path_codec.rs**

创建 `src-tauri/src/collectors/claude_history/path_codec.rs`：

```rust
use std::path::PathBuf;

/// 获取 Claude Code 配置目录（跨平台）
/// macOS/Linux: ~/.claude
/// Windows: %USERPROFILE%\.claude
pub fn claude_config_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(".claude"))
}

/// 将项目路径编码为 Claude Code 项目目录名格式
///
/// macOS/Linux: /Users/name/Repo → -Users-name-Repo
/// Windows: C:\Repo → C--Repo
pub fn encode_cwd_path(cwd: &str) -> String {
    if cfg!(target_os = "windows") {
        // Windows: \ 替换为 --
        cwd.replace("\\", "--")
    } else {
        // Unix: 首 / 保留为前缀 -，其余 / 替换为 -
        let without_leading = cwd.strip_prefix('/').unwrap_or(cwd);
        let encoded = without_leading.replace("/", "-").replace("_", "-");
        if cwd.starts_with('/') {
            format!("-{}", encoded)
        } else {
            encoded
        }
    }
}

/// 将编码目录名还原为原始项目路径
pub fn decode_project_dir(encoded: &str) -> String {
    if cfg!(target_os = "windows") {
        // Windows: -- 替换为 \，剩余 - 替换为 \
        // 注意：这种解码在极端情况下可能有歧义（如 C:\-Repo），
        // 但与 Claude Code 自身行为一致
        encoded.replace("--", "\\").replace('-', "\\")
    } else {
        // Unix: 首 - 替换为 /，其余 - 替换为 /
        if let Some(stripped) = encoded.strip_prefix('-') {
            format!("/{}", stripped.replace('-', "/"))
        } else {
            encoded.replace('-', "/")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_claude_config_dir() {
        let dir = claude_config_dir();
        assert!(dir.is_some());
        let path = dir.unwrap();
        assert!(path.to_string_lossy().ends_with(".claude"));
    }

    #[test]
    fn test_encode_cwd_path_unix() {
        assert_eq!(encode_cwd_path("/Users/ckstar/Repo/my_project"), "-Users-ckstar-Repo-my-project");
        assert_eq!(encode_cwd_path("/home/user/project"), "-home-user-project");
    }

    #[test]
    fn test_decode_project_dir_unix() {
        assert_eq!(decode_project_dir("-Users-ckstar-Repo"), "/Users/ckstar/Repo");
        assert_eq!(decode_project_dir("home-user-project"), "home/user/project");
    }

    #[test]
    fn test_encode_decode_roundtrip_unix() {
        let original = "/Users/name/project";
        let encoded = encode_cwd_path(original);
        let decoded = decode_project_dir(&encoded);
        assert_eq!(decoded, original);
    }
}
```

- [ ] **Step 3: 创建 mod.rs**

创建 `src-tauri/src/collectors/claude_history/mod.rs`：

```rust
pub mod models;
pub mod path_codec;
pub mod scanner;
```

- [ ] **Step 4: 修改 session_transcript.rs**

修改 `src-tauri/src/collectors/template/session_transcript.rs`：

1. 在文件顶部 `use` 语句后添加：

```rust
use crate::collectors::claude_history::path_codec::{claude_config_dir, encode_cwd_path};
```

2. 删除原 `encode_cwd_path` 函数（第 10-35 行）：

```rust
// 删除整个 encode_cwd_path 函数块（含 #[cfg(not(windows))] 和 doc comment）
```

3. 修改 `sessions_dir` 函数（约第 683-696 行）：

替换为：

```rust
fn sessions_dir(project_path: &Path) -> PathBuf {
    let encoded = encode_cwd_path(&project_path.to_string_lossy());
    claude_config_dir()
        .unwrap_or_default()
        .join("projects")
        .join(encoded)
}
```

- [ ] **Step 5: 运行 Rust 测试**

```bash
cd src-tauri && cargo test path_codec::tests --lib
```

Expected: 4 tests PASS

- [ ] **Step 6: 运行完整 Rust 测试**

```bash
cd src-tauri && cargo test
```

Expected: 全部通过（无回归）

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/collectors/claude_history/ src-tauri/src/collectors/mod.rs src-tauri/src/collectors/template/session_transcript.rs
git commit -m "feat(backend): 提取共享 path_codec.rs 模块，修复 Windows 路径编码支持

- 新建 collectors/claude_history/path_codec.rs：encode_cwd_path、decode_project_dir、claude_config_dir
- 新增 collectors/claude_history/mod.rs 模块入口
- session_transcript.rs 复用 path_codec.rs，移除内联 Unix-only encode_cwd_path
- session_transcript.rs 的 sessions_dir 移除 #[cfg(windows)] 过时分支
- 新增 path_codec 单元测试（macOS/Linux/Windows 场景）

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>"
```

---

## Task 2: claude_history 模块（models + scanner）

**Files:**
- Create: `src-tauri/src/collectors/claude_history/models.rs`
- Create: `src-tauri/src/collectors/claude_history/scanner.rs`

---

- [ ] **Step 1: 创建 models.rs**

创建 `src-tauri/src/collectors/claude_history/models.rs`：

```rust
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct SerClaudeSession {
    pub session_id: String,
    pub name: Option<String>,
    pub cwd: String,
    pub status: SerSessionStatus,
    pub started_at: Option<u64>,
    pub updated_at: Option<u64>,
    pub turn_count: Option<usize>,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize)]
pub enum SerSessionStatus {
    Active,
    Idle,
    Exited,
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
pub struct SerProjectSessionGroup {
    pub project_path: String,
    pub project_name: String,
    pub sessions: Vec<SerClaudeSession>,
    pub session_count: usize,
    pub is_orphaned: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct SerHistoryEntry {
    pub display: String,
    pub timestamp: u64,
    pub session_id: String,
    pub project_path: String,
}
```

- [ ] **Step 2: 创建 scanner.rs**

创建 `src-tauri/src/collectors/claude_history/scanner.rs`：

```rust
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
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };

        if line.trim().is_empty() {
            continue;
        }

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

    // 按时间倒序
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
        if !path.is_dir() {
            continue;
        }

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

/// 扫描活跃会话（sessions/ 目录下的 {pid}.json）
fn scan_active_sessions(config_dir: &Path) -> Result<HashMap<String, ActiveSessionInfo>, String> {
    let sessions_dir = config_dir.join("sessions");
    let mut active = HashMap::new();

    if !sessions_dir.is_dir() {
        return Ok(active);
    }

    for entry in fs::read_dir(&sessions_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();

        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }

        match fs::read_to_string(&path) {
            Ok(content) => {
                if let Ok(value) = serde_json::from_str::<Value>(&content) {
                    let session_id = value
                        .get("sessionId")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let name = value.get("name").and_then(|v| v.as_str()).map(|s| s.to_string());
                    let cwd = value.get("cwd").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let status = value.get("status").and_then(|v| v.as_str()).unwrap_or("unknown");
                    let started_at = value.get("startedAt").and_then(|v| v.as_u64());
                    let updated_at = value.get("updatedAt").and_then(|v| v.as_u64());

                    active.insert(
                        session_id.clone(),
                        ActiveSessionInfo {
                            session_id,
                            name,
                            cwd,
                            status: status.to_string(),
                            started_at,
                            updated_at,
                        },
                    );
                }
            }
            Err(_) => continue,
        }
    }

    Ok(active)
}

struct ActiveSessionInfo {
    session_id: String,
    name: Option<String>,
    cwd: String,
    status: String,
    started_at: Option<u64>,
    updated_at: Option<u64>,
}

/// 扫描 projects/ 目录
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

        if !encoded_dir.is_dir() {
            continue;
        }

        let dir_name = encoded_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        let project_path = decode_project_dir(&dir_name);
        let project_name = Path::new(&project_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();
        let is_orphaned = !Path::new(&project_path).exists();

        let mut sessions = Vec::new();

        for file_entry in fs::read_dir(&encoded_dir).map_err(|e| e.to_string())? {
            let file_entry = file_entry.map_err(|e| e.to_string())?;
            let file_path = file_entry.path();

            if file_path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                continue;
            }

            let session_id = file_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();

            // 获取文件修改时间作为 started_at 的备选
            let mtime = file_entry
                .metadata()
                .ok()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(std::time::SystemTime::UNIX_EPOCH).ok())
                .map(|d| d.as_secs() * 1000);

            // 统计 turn_count（.jsonl 文件行数）
            let turn_count = count_jsonl_lines(&file_path).unwrap_or(None);

            // 合并活跃会话数据
            let (name, status, started_at, updated_at, is_active) =
                if let Some(active) = active_sessions.get(&session_id) {
                    (
                        active.name.clone().or_else(|| {
                            // 活跃会话 name 为 None 时，尝试从 .jsonl 第一行提取
                            extract_session_name(&file_path)
                        }),
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
                project_path: project_path.clone(),
                project_name,
                sessions,
                session_count: 0, // 将在下面计算
                is_orphaned,
            });
        }
    }

    // 计算 session_count
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

/// 从 .jsonl 文件中提取 session 名称（从第一条 user message 中解析）
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

/// 统计 .jsonl 文件行数
fn count_jsonl_lines(path: &Path) -> Result<Option<usize>, String> {
    let file = fs::File::open(path).map_err(|e| e.to_string())?;
    let reader = BufReader::new(file);
    let mut count = 0;

    for line in reader.lines() {
        if line.is_ok() {
            count += 1;
        }
    }

    Ok(Some(count))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_list_claude_sessions_empty() {
        // 测试无 .claude 目录时返回空列表
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
```

- [ ] **Step 3: 运行 Rust 测试**

```bash
cd src-tauri && cargo test claude_history --lib
```

Expected: 3 tests PASS

- [ ] **Step 4: 运行完整 Rust 测试**

```bash
cd src-tauri && cargo test
```

Expected: 全部通过（无回归）

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/collectors/claude_history/
git commit -m "feat(backend): 新增 claude_history scanner 和 models

- models.rs: SerClaudeSession, SerProjectSessionGroup, SerHistoryEntry, SerSessionStatus
- scanner.rs: list_claude_sessions, get_session_detail, search_claude_history, delete_claude_session
- 包含活跃会话扫描、projects/ 目录扫描、history.jsonl 流式过滤
- 支持 orphaned 检测、turn_count 统计、并发竞态保护

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>"
```

---

## Task 3: 新增 Tauri 命令 + 注册

**Files:**
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`

---

- [ ] **Step 1: 在 commands.rs 中新增命令**

在 `src-tauri/src/commands.rs` 中，在现有 `use` 语句后添加：

```rust
use crate::collectors::claude_history::{
    models::{SerClaudeSession, SerHistoryEntry, SerProjectSessionGroup},
    scanner::{delete_claude_session, get_session_detail, list_claude_sessions, search_claude_history},
};
```

然后在文件末尾（`save_candidate_memory` 函数之后）添加：

```rust
#[tauri::command]
pub fn list_claude_sessions_cmd() -> Result<Vec<SerProjectSessionGroup>, String> {
    list_claude_sessions()
}

#[tauri::command]
pub fn get_claude_session_detail_cmd(session_id: String) -> Result<Option<SerClaudeSession>, String> {
    get_session_detail(&session_id)
}

#[tauri::command]
pub fn search_claude_history_cmd(query: String) -> Result<Vec<SerHistoryEntry>, String> {
    search_claude_history(&query)
}

#[tauri::command]
pub fn delete_claude_session_cmd(session_id: String) -> Result<(), String> {
    delete_claude_session(&session_id)
}
```

- [ ] **Step 2: 在 lib.rs 中导入并注册命令**

修改 `src-tauri/src/lib.rs`：

1. 在 `use commands::{...}` 中添加新命令：

```rust
use commands::{
    add_project, delete_claude_session_cmd, get_claude_session_detail_cmd,
    get_latest_session, get_project_data, get_project_files, get_project_file_content,
    get_session_transcript, get_template_path, list_claude_sessions_cmd,
    list_project_sessions, list_projects, remove_project, save_candidate_memory,
    search_claude_history_cmd, search_sessions, set_template_path, start_watching,
    stop_watching,
};
```

2. 在 `.invoke_handler(tauri::generate_handler![...])` 中添加：

```rust
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
```

- [ ] **Step 3: 编译检查**

```bash
cd src-tauri && cargo check
```

Expected: 编译通过，无错误

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "feat(backend): 注册 claude_history Tauri 命令

- commands.rs 新增 list_claude_sessions_cmd, get_claude_session_detail_cmd,
  search_claude_history_cmd, delete_claude_session_cmd
- lib.rs 导入并注册到 invoke_handler

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>"
```

---

## Task 4: 前端页面组件

**Files:**
- Create: `src/hooks/useClaudeHistory.ts`
- Create: `src/components/claude-history/SearchBar.tsx`
- Create: `src/components/claude-history/ProjectList.tsx`
- Create: `src/components/claude-history/SessionTimeline.tsx`
- Create: `src/pages/ClaudeHistory.tsx`

---

- [ ] **Step 1: 创建 useClaudeHistory.ts**

创建 `src/hooks/useClaudeHistory.ts`：

```typescript
import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

export interface ClaudeSession {
  session_id: string;
  name: string | null;
  cwd: string;
  status: "Active" | "Idle" | "Exited" | "Unknown";
  started_at: number | null;
  updated_at: number | null;
  turn_count: number | null;
  is_active: boolean;
}

export interface ProjectSessionGroup {
  project_path: string;
  project_name: string;
  sessions: ClaudeSession[];
  session_count: number;
  is_orphaned: boolean;
}

export function useClaudeHistory() {
  const [projectGroups, setProjectGroups] = useState<ProjectSessionGroup[]>([]);
  const [selectedProject, setSelectedProject] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState("");
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const fetchSessions = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    try {
      const groups = await invoke<ProjectSessionGroup[]>("list_claude_sessions_cmd");
      setProjectGroups(groups);
      if (groups.length > 0 && !selectedProject) {
        setSelectedProject(groups[0].project_path);
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setIsLoading(false);
    }
  }, [selectedProject]);

  const deleteSession = useCallback(async (sessionId: string) => {
    if (!confirm("此操作不可逆，删除后无法通过 /resume 恢复该会话。确定删除吗？")) {
      return;
    }
    try {
      await invoke("delete_claude_session_cmd", { sessionId });
      await fetchSessions();
    } catch (e) {
      alert(`删除失败: ${e}`);
    }
  }, [fetchSessions]);

  useEffect(() => {
    fetchSessions();
  }, [fetchSessions]);

  const filteredGroups = projectGroups.filter((group) => {
    const query = searchQuery.toLowerCase();
    const matchProject =
      group.project_name.toLowerCase().includes(query) ||
      group.project_path.toLowerCase().includes(query);
    const matchSession = group.sessions.some(
      (s) => s.name?.toLowerCase().includes(query)
    );
    return matchProject || matchSession;
  });

  const selectedGroup = projectGroups.find(
    (g) => g.project_path === selectedProject
  );

  return {
    projectGroups,
    filteredGroups,
    selectedGroup,
    selectedProject,
    setSelectedProject,
    searchQuery,
    setSearchQuery,
    isLoading,
    error,
    fetchSessions,
    deleteSession,
  };
}
```

- [ ] **Step 2: 创建 SearchBar.tsx**

创建 `src/components/claude-history/SearchBar.tsx`：

```typescript
import { Search } from "lucide-react";

import { Input } from "@/components/ui/input";

interface SearchBarProps {
  value: string;
  onChange: (value: string) => void;
}

export function SearchBar({ value, onChange }: SearchBarProps) {
  return (
    <div className="relative">
      <Search className="absolute left-3 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
      <Input
        type="text"
        placeholder="搜索会话或项目..."
        value={value}
        onChange={(e) => onChange(e.target.value)}
        className="pl-9"
      />
    </div>
  );
}
```

- [ ] **Step 3: 创建 ProjectList.tsx**

创建 `src/components/claude-history/ProjectList.tsx`：

```typescript
import { Folder, FolderOpen } from "lucide-react";

import { cn } from "@/lib/utils";

import type { ProjectSessionGroup } from "@/hooks/useClaudeHistory";

interface ProjectListProps {
  groups: ProjectSessionGroup[];
  selectedPath: string | null;
  onSelect: (path: string) => void;
}

export function ProjectList({ groups, selectedPath, onSelect }: ProjectListProps) {
  return (
    <div className="flex flex-col gap-1">
      {groups.map((group) => {
        const isSelected = group.project_path === selectedPath;
        const activeCount = group.sessions.filter((s) => s.is_active).length;

        return (
          <button
            key={group.project_path}
            type="button"
            onClick={() => onSelect(group.project_path)}
            className={cn(
              "flex items-center gap-2 rounded-md px-3 py-2 text-left text-sm transition-colors",
              "hover:bg-accent hover:text-accent-foreground",
              isSelected && "bg-accent text-accent-foreground"
            )}
            title={group.project_path}
          >
            {isSelected ? (
              <FolderOpen className="size-4 shrink-0 text-primary" />
            ) : (
              <Folder className="size-4 shrink-0 text-muted-foreground" />
            )}
            <span className="min-w-0 flex-1 truncate">{group.project_name}</span>
            {activeCount > 0 && (
              <span className="flex size-5 shrink-0 items-center justify-center rounded-full bg-primary text-[10px] text-primary-foreground">
                {activeCount}
              </span>
            )}
            {group.is_orphaned && (
              <span className="text-xs text-muted-foreground">🚫</span>
            )}
          </button>
        );
      })}
    </div>
  );
}
```

- [ ] **Step 4: 创建 SessionTimeline.tsx**

创建 `src/components/claude-history/SessionTimeline.tsx`：

```typescript
import { Circle, Trash2 } from "lucide-react";

import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

import type { ClaudeSession } from "@/hooks/useClaudeHistory";

interface SessionTimelineProps {
  sessions: ClaudeSession[];
  onDelete: (sessionId: string) => void;
}

function formatDate(timestamp: number | null): string {
  if (!timestamp) return "未知时间";
  return new Date(timestamp).toLocaleString("zh-CN");
}

export function SessionTimeline({ sessions, onDelete }: SessionTimelineProps) {
  return (
    <div className="flex flex-col gap-2">
      {sessions.map((session) => (
        <div
          key={session.session_id}
          className={cn(
            "flex items-start gap-3 rounded-lg border p-3 transition-colors",
            session.is_active
              ? "border-primary/30 bg-primary/5"
              : "border-border bg-card hover:bg-accent/50"
          )}
        >
          <div className="mt-1 shrink-0">
            <Circle
              className={cn(
                "size-3",
                session.is_active ? "fill-green-500 text-green-500" : "fill-muted text-muted"
              )}
            />
          </div>
          <div className="min-w-0 flex-1">
            <p className="text-sm font-medium">
              {session.name || "未命名会话"}
            </p>
            <p className="text-xs text-muted-foreground">
              {formatDate(session.started_at)}
              {session.is_active && (
                <span className="ml-2 text-green-600">运行中</span>
              )}
              {!session.is_active && session.turn_count !== null && (
                <span className="ml-2">{session.turn_count} 轮对话</span>
              )}
            </p>
            <p className="mt-1 truncate text-xs text-muted-foreground">
              {session.cwd}
            </p>
          </div>
          {!session.is_active && (
            <Button
              type="button"
              variant="ghost"
              size="icon"
              className="size-8 shrink-0 text-muted-foreground hover:text-destructive"
              title="删除会话"
              onClick={() => onDelete(session.session_id)}
            >
              <Trash2 className="size-4" />
            </Button>
          )}
        </div>
      ))}
    </div>
  );
}
```

- [ ] **Step 5: 创建 ClaudeHistory.tsx**

创建 `src/pages/ClaudeHistory.tsx`：

```typescript
import { RefreshCw } from "lucide-react";

import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import { ProjectList } from "@/components/claude-history/ProjectList";
import { SearchBar } from "@/components/claude-history/SearchBar";
import { SessionTimeline } from "@/components/claude-history/SessionTimeline";
import { useClaudeHistory } from "@/hooks/useClaudeHistory";

export function ClaudeHistory() {
  const {
    filteredGroups,
    selectedGroup,
    selectedProject,
    setSelectedProject,
    searchQuery,
    setSearchQuery,
    isLoading,
    error,
    fetchSessions,
    deleteSession,
  } = useClaudeHistory();

  return (
    <div className="flex h-full flex-col gap-4">
      {/* 顶部工具栏 */}
      <div className="flex items-center gap-4">
        <h1 className="text-xl font-semibold">会话历史</h1>
        <div className="flex-1">
          <SearchBar value={searchQuery} onChange={setSearchQuery} />
        </div>
        <Button
          type="button"
          variant="outline"
          size="sm"
          onClick={fetchSessions}
          disabled={isLoading}
        >
          <RefreshCw className={cn("mr-1 size-4", isLoading && "animate-spin")} />
          刷新
        </Button>
      </div>

      {/* 错误状态 */}
      {error && (
        <div className="rounded-md border border-destructive bg-destructive/10 px-4 py-3 text-sm text-destructive">
          {error}
        </div>
      )}

      {/* 空状态 */}
      {!isLoading && filteredGroups.length === 0 && !error && (
        <div className="flex flex-1 flex-col items-center justify-center text-muted-foreground">
          <p>未找到 Claude Code 会话</p>
          <p className="text-sm">请确认已安装 Claude Code 且有过历史会话</p>
        </div>
      )}

      {/* 左右分栏 */}
      {filteredGroups.length > 0 && (
        <div className="flex flex-1 gap-4 overflow-hidden">
          {/* 左侧项目列表 */}
          <div className="flex w-64 shrink-0 flex-col gap-2">
            <p className="px-3 text-xs font-medium text-muted-foreground">
              项目 ({filteredGroups.length})
            </p>
            <ScrollArea className="flex-1">
              <ProjectList
                groups={filteredGroups}
                selectedPath={selectedProject}
                onSelect={setSelectedProject}
              />
            </ScrollArea>
          </div>

          {/* 右侧会话时间线 */}
          <div className="flex min-w-0 flex-1 flex-col gap-2">
            <p className="px-1 text-xs font-medium text-muted-foreground">
              {selectedGroup
                ? `${selectedGroup.project_name} (${selectedGroup.sessions.length} 个会话)`
                : "选择项目查看会话"}
            </p>
            <ScrollArea className="flex-1">
              {selectedGroup ? (
                <SessionTimeline
                  sessions={selectedGroup.sessions}
                  onDelete={deleteSession}
                />
              ) : (
                <p className="text-sm text-muted-foreground">请从左侧选择一个项目</p>
              )}
            </ScrollArea>
          </div>
        </div>
      )}
    </div>
  );
}

function cn(...inputs: (string | false | null | undefined)[]) {
  return inputs.filter(Boolean).join(" ");
}
```

注意：ClaudeHistory.tsx 底部定义了局部 `cn` 函数。如果项目已有 `@/lib/utils` 的 `cn`，请改为 `import { cn } from "@/lib/utils";` 并删除底部定义。

- [ ] **Step 6: 验证 TypeScript 编译**

```bash
npm run build
```

Expected: 编译通过，无 TypeScript 错误

- [ ] **Step 7: Commit**

```bash
git add src/hooks/useClaudeHistory.ts src/components/claude-history/ src/pages/ClaudeHistory.tsx
git commit -m "feat(frontend): 新增 Claude History 页面组件

- useClaudeHistory hook: 数据获取、缓存、搜索过滤、删除
- SearchBar: 搜索输入组件
- ProjectList: 左侧项目列表（活跃计数、orphaned 标记）
- SessionTimeline: 右侧会话时间线（活跃状态、删除按钮）
- ClaudeHistory: 主页面，左右分栏布局

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>"
```

---

## Task 5: 路由扩展 + 导航 + 集成验证

**Files:**
- Modify: `src/App.tsx`
- Modify: `src/components/Sidebar.tsx`

---

- [ ] **Step 1: 扩展 AppRoute 类型并新增路由**

修改 `src/App.tsx`：

1. 将 `AppRoute` 类型从：

```typescript
export type AppRoute = "dashboard" | "agents" | "settings";
```

改为：

```typescript
export type AppRoute = "dashboard" | "agents" | "settings" | "claude-history";
```

2. 在 imports 中添加：

```typescript
import { ClaudeHistory } from "@/pages/ClaudeHistory";
```

3. 在 `page` 的 `switch` 语句中，在 `case "settings":` 之前添加：

```typescript
case "claude-history":
  return <ClaudeHistory />;
```

- [ ] **Step 2: 新增 Sidebar 导航项**

修改 `src/components/Sidebar.tsx`：

1. 在 imports 中添加 `History` 图标：

```typescript
import { Bot, History, LayoutDashboard, PanelLeftClose, PanelLeftOpen, Settings } from "lucide-react";
```

2. 在 `navigationItems` 数组中添加：

```typescript
const navigationItems: Array<{
  icon: typeof LayoutDashboard;
  label: string;
  route: AppRoute;
}> = [
  { icon: LayoutDashboard, label: "仪表盘", route: "dashboard" },
  { icon: Bot, label: "代理监控", route: "agents" },
  { icon: History, label: "会话历史", route: "claude-history" },
  { icon: Settings, label: "设置", route: "settings" },
];
```

- [ ] **Step 3: 验证前端构建**

```bash
npm run build
```

Expected: 编译通过

- [ ] **Step 4: 验证 Rust 编译**

```bash
cd src-tauri && cargo check
```

Expected: 编译通过

- [ ] **Step 5: 运行 Playwright E2E 测试**

```bash
npm test
```

Expected: 现有测试全部通过（无回归）

- [ ] **Step 6: Commit**

```bash
git add src/App.tsx src/components/Sidebar.tsx
git commit -m "feat(ui): 新增 claude-history 路由和导航

- App.tsx: 扩展 AppRoute 类型，新增 claude-history 路由
- Sidebar.tsx: 新增"会话历史"导航项（History 图标）

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>"
```

---

## Self-Review

### Spec Coverage Check

| 需求 | 实现任务 |
|------|----------|
| 扫描 sessions/ | Task 2 scanner.rs: scan_active_sessions |
| 扫描 history.jsonl | Task 2 scanner.rs: search_claude_history |
| 扫描 projects/ | Task 2 scanner.rs: scan_projects |
| 按项目分组 | Task 2 scanner.rs: list_claude_sessions |
| 搜索过滤 | Task 2 scanner.rs: search_claude_history + Task 4 useClaudeHistory |
| 区分活跃/非活跃 | Task 2 models.rs: is_active + SerSessionStatus |
| 删除非活跃会话 | Task 2 scanner.rs: delete_claude_session |
| 路径编解码 | Task 1 path_codec.rs |
| 修复 session_transcript.rs | Task 1 Step 4 |
| 左右分栏布局 | Task 4 ClaudeHistory.tsx |
| 前端确认对话框 | Task 4 useClaudeHistory.ts |
| orphaned 检测 | Task 2 scanner.rs: is_orphaned |
| 权限/不存在区分 | Task 2 scanner.rs: 错误处理 |

**覆盖完整，无遗漏。**

### Placeholder Scan

- 无 TBD/TODO/placeholder
- 所有步骤包含具体代码
- 所有步骤包含具体命令

### Type Consistency

- `SerClaudeSession`, `SerProjectSessionGroup`, `SerHistoryEntry` 在 Task 2 中定义，Task 3 中复用
- `ClaudeSession`, `ProjectSessionGroup` 在 Task 4 hook 中定义，与前端的 `Ser*` 类型对应
- `session_id` 字段命名在所有文件中一致

---

## 执行方式

**Plan saved to `docs/superpowers/plans/2026-05-12-claude-session-history.md`**

两种执行选项：

**1. Subagent-Driven (recommended)** — 每个 Task 分配独立子代理执行，我在每 Task 完成后审查

**2. Inline Execution** — 在当前会话中顺序执行所有 Task

选择哪种方式？