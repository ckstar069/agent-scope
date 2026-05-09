use std::collections::HashSet;
use std::fmt;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde_json::Value;

// ============================================================================
// encode_cwd_path — 路径编码
// ============================================================================

/// 将项目路径编码为 Claude Code 项目目录名格式
///
/// 规则：去除首 `/`，将剩余 `/` 替换为 `-`
///
/// # 示例
///
/// ```
/// use ptv_lib::collectors::template::session_transcript::encode_cwd_path;
/// assert_eq!(encode_cwd_path("/Users/ckstar/Repo/my_project"), "-Users-ckstar-Repo-my-project");
/// assert_eq!(encode_cwd_path("/home/user/project"), "-home-user-project");
/// assert_eq!(encode_cwd_path("relative/path"), "relative-path");
/// ```
#[cfg(not(windows))]
pub fn encode_cwd_path(cwd: &str) -> String {
    let without_leading = cwd.strip_prefix('/').unwrap_or(cwd);
    let encoded = without_leading.replace("/", "-").replace("_", "-");
    if cwd.starts_with('/') {
        format!("-{}", encoded)
    } else {
        encoded
    }
}

// ============================================================================
// TranscriptError — 错误类型
// ============================================================================

/// 会话转录采集过程中可能发生的错误
#[derive(Debug)]
pub enum TranscriptError {
    /// 项目目录不存在或无法访问
    ProjectNotFound(String),
    /// 会话目录不存在
    SessionsDirNotFound(String),
    /// I/O 错误
    Io(std::io::Error),
    /// JSONL 解析失败
    ParseError(String),
}

impl fmt::Display for TranscriptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TranscriptError::ProjectNotFound(path) => write!(f, "项目目录未找到：{}", path),
            TranscriptError::SessionsDirNotFound(path) => write!(f, "会话目录未找到：{}", path),
            TranscriptError::Io(e) => write!(f, "I/O 错误：{}", e),
            TranscriptError::ParseError(msg) => write!(f, "解析错误：{}", msg),
        }
    }
}

impl std::error::Error for TranscriptError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            TranscriptError::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for TranscriptError {
    fn from(e: std::io::Error) -> Self {
        TranscriptError::Io(e)
    }
}

// ============================================================================
// 数据结构
// ============================================================================

/// 会话中的单轮消息
#[derive(Debug, Clone, PartialEq)]
pub struct SessionTurn {
    /// 角色："user" 或 "assistant"
    pub role: String,
    /// 合并后的文本（截断至 1KB）
    pub text: String,
    /// 去重后的工具名列表
    pub tools: Vec<String>,
    /// Unix 时间戳（毫秒），解析自 JSONL 的 timestamp 字段
    pub timestamp: Option<u64>,
}

/// 完整会话转录
#[derive(Debug, Clone, PartialEq)]
pub struct SessionTranscript {
    /// 会话 ID（文件名去扩展名）
    pub session_id: String,
    /// 初始用户提示（首个 user 消息文本）
    pub initial_prompt: String,
    /// 自定义标题（来自 custom-title 类型条目）
    pub custom_title: Option<String>,
    /// 使用的模型（来自 assistant 消息）
    pub model: Option<String>,
    /// 所有轮次
    pub turns: Vec<SessionTurn>,
    /// 涉及的文件路径列表
    pub modified_files: Vec<String>,
    /// 会话创建时间（Unix 秒，取自文件修改时间）
    pub created_at: u64,
}

/// 会话元数据摘要（不含完整轮次）
#[derive(Debug, Clone, PartialEq)]
pub struct SessionSummary {
    /// 会话 ID
    pub session_id: String,
    /// 初始用户提示
    pub initial_prompt: String,
    /// 自定义标题
    pub custom_title: Option<String>,
    /// 使用的模型
    pub model: Option<String>,
    /// 轮次数
    pub turn_count: usize,
    /// 涉及的文件列表
    pub modified_files: Vec<String>,
    /// 会话创建时间
    pub created_at: u64,
}

// ============================================================================
// 内部共享解析上下文
// ============================================================================

/// 在全文扫描和元数据提取间共享的可变解析状态
struct ParseContext {
    initial_prompt: String,
    custom_title: Option<String>,
    model: Option<String>,
    turn_count: usize,
    modified_files: Vec<String>,
    turns: Vec<SessionTurn>,
    /// 是否构建完整轮次列表（元数据模式为 false）
    build_turns: bool,
    /// 用于合并连续同角色轮次的缓冲区
    pending_role: Option<String>,
    pending_text: String,
    pending_tools: HashSet<String>,
    pending_ts: Option<u64>,
}

impl ParseContext {
    fn new(build_turns: bool) -> Self {
        Self {
            initial_prompt: String::new(),
            custom_title: None,
            model: None,
            turn_count: 0,
            modified_files: Vec::new(),
            turns: Vec::new(),
            build_turns,
            pending_role: None,
            pending_text: String::new(),
            pending_tools: HashSet::new(),
            pending_ts: None,
        }
    }

    /// 提交当前 pending 的轮次（如果有的话）
    fn flush_pending_turn(&mut self) {
        if let Some(role) = self.pending_role.take() {
            let text = truncate_text(&self.pending_text, 1024);
            let mut tools: Vec<String> = self.pending_tools.drain().collect();
            tools.sort();
            let turn = SessionTurn {
                role,
                text,
                tools,
                timestamp: self.pending_ts,
            };
            self.turns.push(turn);
            self.pending_text.clear();
            self.pending_ts = None;
        }
    }

    /// 注册一个新的轮次（合并或新建）
    fn register_turn(&mut self, role: &str, text: &str, tools: &[String], ts: Option<u64>) {
        self.turn_count += 1;

        if !self.build_turns {
            return;
        }

        // 检查是否与 pending 同角色
        match &self.pending_role {
            Some(pending_role) if pending_role == role => {
                // 合并同角色轮次
                if !text.is_empty() {
                    if !self.pending_text.is_empty() {
                        self.pending_text.push('\n');
                    }
                    self.pending_text.push_str(text);
                }
                for t in tools {
                    self.pending_tools.insert(t.clone());
                }
                // 保留最早的 timestamp
                if self.pending_ts.is_none() {
                    self.pending_ts = ts;
                }
            }
            _ => {
                // 不同角色 → 先提交 pending，再创建新的
                self.flush_pending_turn();
                self.pending_role = Some(role.to_string());
                self.pending_text = text.to_string();
                self.pending_tools = tools.iter().cloned().collect();
                self.pending_ts = ts;
            }
        }
    }
}

// ============================================================================
// JSONL 解析辅助函数
// ============================================================================

/// 从 JSONL 条目中提取文本内容
///
/// 支持两种格式：
/// - 字符串: `"content": "hello"`
/// - blocks 数组: `"content": [{"type":"text","text":"hello"}, ...]`
fn extract_text_from_content(content: &Value) -> String {
    let raw_text = match content {
        Value::String(s) => s.clone(),
        Value::Array(arr) => {
            arr.iter()
                .filter_map(|block| {
                    if block.get("type").and_then(|t| t.as_str()) == Some("text") {
                        block.get("text").and_then(|t| t.as_str())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
                .join("\n")
        }
        _ => String::new(),
    };

    clean_transcript_text(&raw_text)
}

/// 清理转录文本中的系统 XML 包装与不可见噪声。
///
/// Claude/OpenCode 的本地命令会把真实用户输入包在 `<command-message>` 中，
/// 同时夹带 `<local-command-caveat>`、`<command-name>` 等系统说明。
/// 这里移除系统块，保留普通文本与 `<command-message>` 内的真实请求。
fn clean_transcript_text(text: &str) -> String {
    let mut cleaned = text.replace("\r\n", "\n").replace('\r', "\n");

    for tag in ["local-command-caveat", "command-name", "system-reminder", "work_context"] {
        cleaned = remove_tagged_block(&cleaned, tag);
    }

    cleaned = strip_xml_like_tags(&cleaned);

    cleaned
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// 移除指定 XML-like 标签及其内部内容，大小写不敏感。
fn remove_tagged_block(input: &str, tag: &str) -> String {
    let mut output = input.to_string();
    let open_prefix = format!("<{}", tag.to_lowercase());
    let close_tag = format!("</{}>", tag.to_lowercase());

    loop {
        let lower = output.to_lowercase();
        let Some(start) = lower.find(&open_prefix) else {
            break;
        };

        let Some(open_end_rel) = output[start..].find('>') else {
            break;
        };
        let open_end = start + open_end_rel + 1;

        let end = lower[open_end..]
            .find(&close_tag)
            .map(|close_rel| open_end + close_rel + close_tag.len())
            .unwrap_or(open_end);

        output.replace_range(start..end, "");
    }

    output
}

/// 移除 `<...>` 形式的标签/注释，只保留标签外文本。
fn strip_xml_like_tags(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '<' {
            for inner in chars.by_ref() {
                if inner == '>' {
                    break;
                }
            }
        } else {
            output.push(ch);
        }
    }

    output
}

/// 判断文本是否为噪声/占位内容，不应作为 initial_prompt 或 custom_title
///
/// 过滤规则：
/// - 空字符串或仅空白字符
/// - 精确等于 `$@`
/// - 以 `[SYSTEM DIRECTIVE:` 开头
/// - 包含 title-generator boilerplate（如 "You are a conversation title generator"）
fn is_noisy_text(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return true;
    }
    if trimmed == "$@" {
        return true;
    }
    if trimmed.starts_with("[SYSTEM DIRECTIVE:") {
        return true;
    }
    if trimmed.starts_with("<local-command-caveat>") {
        return true;
    }
    if trimmed.starts_with("[Request interrupted by user]") {
        return true;
    }
    if trimmed.contains("<!-- OMO_INTERNAL_INITIATOR -->") {
        return true;
    }
    if trimmed.contains("You are a conversation title generator") {
        return true;
    }
    false
}

/// 从 content blocks 中提取工具名列表
fn extract_tools_from_content(content: &Value) -> Vec<String> {
    let mut tools = Vec::new();
    if let Value::Array(arr) = content {
        for block in arr {
            if block.get("type").and_then(|t| t.as_str()) == Some("tool_use") {
                if let Some(name) = block.get("name").and_then(|n| n.as_str()) {
                    tools.push(name.to_string());
                }
            }
        }
    }
    tools
}

/// 从 tool_use blocks 中提取文件路径
fn extract_files_from_tool_use(block: &Value) -> Vec<String> {
    let mut files = Vec::new();
    let tool_name = block.get("name").and_then(|n| n.as_str()).unwrap_or("");

    // 常见文件操作工具及其参数键名
    let file_keys: &[&str] = match tool_name {
        "read" | "write" | "edit" | "Bash" | "Glob" | "Grep" | "Read" => &["file_path", "path", "filePath", "command"],
        _ => &[],
    };

    if let Some(input) = block.get("input") {
        for key in file_keys {
            if let Some(path) = input.get(key).and_then(|v| v.as_str()) {
                files.push(path.to_string());
            }
        }
    }

    files
}

/// 提取所有 tool_use 块涉及的文件路径
fn extract_all_files(content: &Value) -> Vec<String> {
    let mut files = Vec::new();
    if let Value::Array(arr) = content {
        for block in arr {
            if block.get("type").and_then(|t| t.as_str()) == Some("tool_use") {
                files.extend(extract_files_from_tool_use(block));
            }
        }
    }
    files
}

/// 截断文本至指定字节长度（UTF-8 安全）
fn truncate_text(text: &str, max_bytes: usize) -> String {
    if text.len() <= max_bytes {
        return text.to_string();
    }
    // 找到最近的合法 UTF-8 字符边界
    let mut end = max_bytes;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    text[..end].to_string()
}

/// 从 ISO 8601 时间戳字符串解析为 Unix 毫秒
fn parse_timestamp_ms(ts_str: &str) -> Option<u64> {
    let s = ts_str.trim();

    // 尝试作为整数毫秒解析
    if let Ok(ms) = s.parse::<u64>() {
        return Some(ms);
    }
    if let Ok(ms) = s.parse::<i64>() {
        if ms > 0 {
            return Some(ms as u64);
        }
    }

    // 简单 ISO 8601 解析: "2024-05-07T10:30:00.000Z" 或 "2024-05-07T10:30:00Z"
    // 使用启发式方法：将日期部分转为天数，时间部分转为秒数
    // 这不需要 chrono 依赖
    let (date_part, time_part) = if let Some(pos) = s.find('T') {
        (&s[..pos], &s[pos + 1..])
    } else {
        return None;
    };

    // 解析日期: "2024-05-07"
    let date_parts: Vec<&str> = date_part.split('-').collect();
    if date_parts.len() != 3 {
        return None;
    }
    let year: i64 = date_parts[0].parse().ok()?;
    let month: i64 = date_parts[1].parse().ok()?;
    let day: i64 = date_parts[2].parse().ok()?;

    // 去掉时区后缀
    let time_clean = time_part
        .strip_suffix('Z')
        .unwrap_or(time_part);
    let time_clean = if let Some(pos) = time_clean.find(|c: char| c == '+' || c == '-') {
        // 注意：可能匹配到负数的数字部分，需要检查上下文
        // 对于 ISO 8601，时区偏移出现在时间部分末尾，如 "10:30:00+08:00"
        // 简单处理：从后往前找 `+` 或 `-`，但只检查倒数几个字符
        &time_clean[..pos]
    } else {
        time_clean
    };

    // 解析时间: "10:30:00" 或 "10:30:00.123"
    let time_parts: Vec<&str> = time_clean.split(':').collect();
    if time_parts.len() < 2 {
        return None;
    }
    let hour: i64 = time_parts[0].parse().ok()?;
    let minute: i64 = time_parts[1].parse().ok()?;

    // 秒可能包含毫秒小数
    let sec_str = if time_parts.len() > 2 { time_parts[2] } else { "0" };
    let (sec_str, ms_str) = if let Some(dot_pos) = sec_str.find('.') {
        (&sec_str[..dot_pos], &sec_str[dot_pos + 1..])
    } else {
        (sec_str, "0")
    };
    let second: i64 = sec_str.parse().ok()?;
    let millis: i64 = if ms_str.len() > 3 {
        ms_str[..3].parse().ok()?
    } else {
        let padded = format!("{:0<3}", ms_str.parse::<i64>().ok()?);
        padded.parse().ok()?
    };

    // 计算自 epoch 以来的天数（简化算法，对于大多数日期够用）
    let days = days_since_epoch(year, month, day)?;
    let total_secs = days as i64 * 86400 + hour * 3600 + minute * 60 + second;

    if total_secs < 0 {
        return None;
    }
    Some(total_secs as u64 * 1000 + millis as u64)
}

/// 计算自 Unix epoch (1970-01-01) 以来的天数（简化算法）
fn days_since_epoch(year: i64, month: i64, day: i64) -> Option<i64> {
    if year < 1970 || month < 1 || month > 12 || day < 1 || day > 31 {
        return None;
    }
    let mut total_days = 0i64;
    // 计算完整年份的天数
    for y in 1970..year {
        total_days += if is_leap_year(y) { 366 } else { 365 };
    }
    // 添加当年过去月份的天数
    let month_days_normal = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let month_days_leap = [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let md = if is_leap_year(year) { &month_days_leap } else { &month_days_normal };
    for m in 1..month {
        total_days += md[(m - 1) as usize] as i64;
    }
    total_days += day - 1;
    Some(total_days)
}

fn is_leap_year(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

/// 处理单个 JSONL 条目，更新解析上下文
fn process_jsonl_entry(ctx: &mut ParseContext, val: &Value, file_seen: &mut HashSet<String>) {
    let entry_type = val.get("type").and_then(|t| t.as_str()).unwrap_or("");

    match entry_type {
        "user" => {
            let top_level_content = val.get("content");
            let msg = val.get("message");
            let msg_content = msg.and_then(|m| m.get("content"));
            let content = top_level_content.or(msg_content);
            let text = content.map(extract_text_from_content).unwrap_or_default();
            let tools: Vec<String> = content.map(extract_tools_from_content).unwrap_or_default();
            let ts = val
                .get("timestamp")
                .and_then(|t| t.as_str())
                .and_then(parse_timestamp_ms);

            if ctx.initial_prompt.is_empty() && !is_noisy_text(&text) {
                ctx.initial_prompt = truncate_text(&text, 1024);
            }

            if !is_noisy_text(&text) {
                ctx.register_turn("user", &text, &tools, ts);
            }
        }

        "assistant" => {
            let msg = val.get("message");

            // 提取模型名
            if ctx.model.is_none() {
                if let Some(m) = msg.and_then(|m| m.get("model")).and_then(|v| v.as_str()) {
                    ctx.model = Some(m.to_string());
                }
            }

            let content = msg.and_then(|m| m.get("content"));
            let text = content.map(extract_text_from_content).unwrap_or_default();
            let tools: Vec<String> = content.map(extract_tools_from_content).unwrap_or_default();
            let ts = val
                .get("timestamp")
                .and_then(|t| t.as_str())
                .and_then(parse_timestamp_ms);

            // 提取工具操作涉及的文件
            if let Some(cnt) = content {
                for f in extract_all_files(cnt) {
                    if file_seen.insert(f.clone()) {
                        ctx.modified_files.push(f);
                    }
                }
            }

            if !is_noisy_text(&text) {
                ctx.register_turn("assistant", &text, &tools, ts);
            }
        }

        "custom-title" => {
            if ctx.custom_title.is_none() {
                let title = val
                    .get("title")
                    .and_then(|v| v.as_str())
                    .or_else(|| val.get("content").and_then(|v| v.as_str()));
                if let Some(t) = title {
                    let title = clean_transcript_text(t);
                    if !is_noisy_text(&title) {
                        ctx.custom_title = Some(title);
                    }
                }
            }
        }

        // 跳过 tool_use、tool_result、system 等类型
        _ => {}
    }
}

// ============================================================================
// 核心解析函数
// ============================================================================

/// 解析单个 JSONL 文件
///
/// - `build_turns=true`: 构建完整轮次列表
/// - `build_turns=false`: 仅提取元数据
fn parse_jsonl_file(file_path: &Path, build_turns: bool) -> Result<ParseContext, TranscriptError> {
    let file = fs::File::open(file_path)?;
    let reader = BufReader::new(file);
    let mut ctx = ParseContext::new(build_turns);
    let mut file_seen: HashSet<String> = HashSet::new();

    for line_result in reader.lines() {
        let line = line_result?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let val = match serde_json::from_str::<Value>(trimmed) {
            Ok(v) => v,
            Err(e) => {
                // 跳过无法解析的行
                eprintln!(
                    "JSONL 解析警告 {}: {} — 跳过该行",
                    file_path.display(),
                    e
                );
                continue;
            }
        };

        process_jsonl_entry(&mut ctx, &val, &mut file_seen);
    }

    // 提交最后的 pending 轮次
    if build_turns {
        ctx.flush_pending_turn();
    }

    Ok(ctx)
}

/// 获取目录下所有 .jsonl 文件路径，按修改时间降序排列
fn list_jsonl_files(dir: &Path) -> Result<Vec<(PathBuf, u64)>, TranscriptError> {
    let mut files: Vec<(PathBuf, u64)> = Vec::new();

    if !dir.is_dir() {
        return Ok(files);
    }

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
            continue;
        }

        let mtime = entry
            .metadata()
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);

        files.push((path, mtime));
    }

    // 按修改时间降序（最新的在前）
    files.sort_by(|a, b| b.1.cmp(&a.1));
    Ok(files)
}

/// 获取项目的会话目录路径
fn sessions_dir(project_path: &Path) -> PathBuf {
    #[cfg(not(windows))]
    {
        let encoded = encode_cwd_path(&project_path.to_string_lossy());
        let home = dirs::home_dir().unwrap_or_default();
        home.join(".claude").join("projects").join(encoded)
    }
    #[cfg(windows)]
    {
        // Claude Code 不在 Windows 上运行，返回默认路径
        let home = dirs::home_dir().unwrap_or_default();
        home.join(".claude").join("projects")
    }
}

/// 从文件路径提取 session_id（文件名去 .jsonl 扩展名）
fn session_id_from_path(path: &Path) -> String {
    path.file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default()
}

// ============================================================================
// SessionTranscriptCollector
// ============================================================================

/// 会话转录采集器
///
/// 扫描 `~/.claude/projects/{encoded_cwd}/` 目录下的 `.jsonl` 文件，
/// 解析会话转录内容并提供搜索功能。
pub struct SessionTranscriptCollector;

impl SessionTranscriptCollector {
    /// 列出所有会话（仅元数据）
    ///
    /// 返回按创建时间降序排列的会话摘要列表。
    pub fn list_sessions(project_path: &Path) -> Result<Vec<SessionSummary>, TranscriptError> {
        let dir = sessions_dir(project_path);

        if !dir.is_dir() {
            return Ok(Vec::new());
        }

        let jsonl_files = list_jsonl_files(&dir)?;
        let mut summaries = Vec::with_capacity(jsonl_files.len());

        for (file_path, created_at) in &jsonl_files {
            match parse_jsonl_file(file_path, false) {
                Ok(ctx) => {
                    summaries.push(SessionSummary {
                        session_id: session_id_from_path(file_path),
                        initial_prompt: ctx.initial_prompt,
                        custom_title: ctx.custom_title,
                        model: ctx.model,
                        turn_count: ctx.turn_count,
                        modified_files: ctx.modified_files,
                        created_at: *created_at,
                    });
                }
                Err(e) => {
                    eprintln!(
                        "解析会话失败 {}: {} — 跳过",
                        file_path.display(),
                        e
                    );
                }
            }
        }

        Ok(summaries)
    }

    /// 获取最新会话的完整内容
    ///
    /// 返回修改时间最新的会话的完整转录内容。
    /// 如果没有找到任何会话，返回 `Ok(None)`。
    pub fn get_latest_session(
        project_path: &Path,
    ) -> Result<Option<SessionTranscript>, TranscriptError> {
        let dir = sessions_dir(project_path);

        if !dir.is_dir() {
            return Ok(None);
        }

        let jsonl_files = list_jsonl_files(&dir)?;
        if jsonl_files.is_empty() {
            return Ok(None);
        }

        let (latest_path, created_at) = &jsonl_files[0];
        let ctx = parse_jsonl_file(latest_path, true)?;

        Ok(Some(SessionTranscript {
            session_id: session_id_from_path(latest_path),
            initial_prompt: ctx.initial_prompt,
            custom_title: ctx.custom_title,
            model: ctx.model,
            turns: ctx.turns,
            modified_files: ctx.modified_files,
            created_at: *created_at,
        }))
    }

    /// 获取指定会话的完整内容
    ///
    /// # 参数
    ///
    /// - `project_path`: 项目根目录路径
    /// - `session_id`: 会话 ID（文件名去扩展名）
    ///
    /// 如果找不到指定会话，返回 `Ok(None)`。
    pub fn get_session(
        project_path: &Path,
        session_id: &str,
    ) -> Result<Option<SessionTranscript>, TranscriptError> {
        let dir = sessions_dir(project_path);

        if !dir.is_dir() {
            return Ok(None);
        }

        let file_path = dir.join(format!("{}.jsonl", session_id));
        if !file_path.exists() {
            return Ok(None);
        }

        let created_at = fs::metadata(&file_path)
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let ctx = parse_jsonl_file(&file_path, true)?;

        Ok(Some(SessionTranscript {
            session_id: session_id.to_string(),
            initial_prompt: ctx.initial_prompt,
            custom_title: ctx.custom_title,
            model: ctx.model,
            turns: ctx.turns,
            modified_files: ctx.modified_files,
            created_at,
        }))
    }

    /// 搜索会话（对 initial_prompt + custom_title 做字符串包含匹配）
    ///
    /// 返回所有匹配的会话摘要，按创建时间降序排列。
    ///
    /// # 参数
    ///
    /// - `project_path`: 项目根目录路径
    /// - `query`: 搜索关键词（大小写不敏感）
    pub fn search_sessions(
        project_path: &Path,
        query: &str,
    ) -> Result<Vec<SessionSummary>, TranscriptError> {
        let all = Self::list_sessions(project_path)?;
        let query_lower = query.to_lowercase();

        let matched: Vec<SessionSummary> = all
            .into_iter()
            .filter(|s| {
                s.initial_prompt.to_lowercase().contains(&query_lower)
                    || s.custom_title
                        .as_ref()
                        .map(|t| t.to_lowercase().contains(&query_lower))
                        .unwrap_or(false)
            })
            .collect();

        Ok(matched)
    }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    // -----------------------------------------------------------------------
    // 测试辅助函数
    // -----------------------------------------------------------------------

    /// 创建模拟的会话 JSONL 文件
    ///
    /// 返回临时目录路径（调用方需保持 `_dir` 存活以维持目录存在）。
    fn create_mock_jsonl(
        filename: &str,
        lines: &[&str],
    ) -> (PathBuf, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(filename);
        let mut file = fs::File::create(&path).unwrap();
        for line in lines {
            writeln!(file, "{}", line).unwrap();
        }
        file.sync_all().unwrap();
        (dir.path().to_path_buf(), dir)
    }

    // -----------------------------------------------------------------------
    // encode_cwd_path 测试
    // -----------------------------------------------------------------------

    #[cfg(not(windows))]
    #[test]
    fn test_encode_cwd_path_absolute() {
        assert_eq!(
            encode_cwd_path("/Users/ckstar/Repo/my_project"),
            "-Users-ckstar-Repo-my-project"
        );
    }

    #[cfg(not(windows))]
    #[test]
    fn test_encode_cwd_path_without_leading_slash() {
        assert_eq!(encode_cwd_path("home/user/project"), "home-user-project");
    }

    #[cfg(not(windows))]
    #[test]
    fn test_encode_cwd_path_single_dir() {
        assert_eq!(encode_cwd_path("/root"), "-root");
    }

    #[cfg(not(windows))]
    #[test]
    fn test_encode_cwd_path_empty() {
        assert_eq!(encode_cwd_path(""), "");
    }

    // -----------------------------------------------------------------------
    // 两种 content 格式解析测试（string + blocks）
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_string_content() {
        let (path, _dir) = create_mock_jsonl(
            "ses_test.jsonl",
            &[
                r#"{"type":"user","message":{"content":"你好，帮我写代码"},"timestamp":"2024-05-07T10:30:00.000Z"}"#,
            ],
        );

        let ctx = parse_jsonl_file(&path.join("ses_test.jsonl"), true).unwrap();
        assert_eq!(ctx.initial_prompt, "你好，帮我写代码");
        assert_eq!(ctx.turn_count, 1);
        assert_eq!(ctx.turns.len(), 1);
        assert_eq!(ctx.turns[0].role, "user");
        assert_eq!(ctx.turns[0].text, "你好，帮我写代码");
    }

    #[test]
    fn test_parse_blocks_content() {
        let (path, _dir) = create_mock_jsonl(
            "ses_test.jsonl",
            &[
                r#"{"type":"user","message":{"content":[{"type":"text","text":"帮我写一个排序函数"}]},"timestamp":"2024-05-07T10:30:00.000Z"}"#,
            ],
        );

        let ctx = parse_jsonl_file(&path.join("ses_test.jsonl"), true).unwrap();
        assert_eq!(ctx.initial_prompt, "帮我写一个排序函数");
    }

    #[test]
    fn test_parse_blocks_with_tools() {
        let (path, _dir) = create_mock_jsonl(
            "ses_test.jsonl",
            &[
                r#"{"type":"assistant","message":{"model":"claude-sonnet","content":[{"type":"text","text":"好的，我来写排序函数"},{"type":"tool_use","name":"write","input":{"file_path":"sort.py"}},{"type":"tool_use","name":"read","input":{"path":"test.py"}}]},"timestamp":"2024-05-07T10:31:00.000Z"}"#,
            ],
        );

        let ctx = parse_jsonl_file(&path.join("ses_test.jsonl"), true).unwrap();
        assert_eq!(ctx.turns.len(), 1);
        assert_eq!(ctx.turns[0].role, "assistant");
        assert_eq!(ctx.turns[0].text, "好的，我来写排序函数");
        // 工具名去重并排序
        assert_eq!(ctx.turns[0].tools, vec!["read", "write"]);
        // 文件路径提取
        assert!(ctx.modified_files.contains(&"sort.py".to_string()));
        assert!(ctx.modified_files.contains(&"test.py".to_string()));
    }

    // -----------------------------------------------------------------------
    // 连续同角色轮次合并测试
    // -----------------------------------------------------------------------

    #[test]
    fn test_merge_consecutive_same_role() {
        let (path, _dir) = create_mock_jsonl(
            "ses_test.jsonl",
            &[
                r#"{"type":"user","message":{"content":"第一段话"},"timestamp":"2024-05-07T10:30:00.000Z"}"#,
                r#"{"type":"user","message":{"content":"第二段话"},"timestamp":"2024-05-07T10:30:10.000Z"}"#,
                r#"{"type":"assistant","message":{"content":"回复来了"},"timestamp":"2024-05-07T10:31:00.000Z"}"#,
            ],
        );

        let ctx = parse_jsonl_file(&path.join("ses_test.jsonl"), true).unwrap();
        // user 连续两条应合并为一条
        assert_eq!(ctx.turns.len(), 2);
        assert_eq!(ctx.turns[0].role, "user");
        assert_eq!(ctx.turns[0].text, "第一段话\n第二段话");
        assert_eq!(ctx.turns[1].role, "assistant");
        assert_eq!(ctx.turns[1].text, "回复来了");
    }

    #[test]
    fn test_merge_preserves_earliest_timestamp() {
        let (path, _dir) = create_mock_jsonl(
            "ses_test.jsonl",
            &[
                r#"{"type":"user","message":{"content":"msg1"},"timestamp":"2024-05-07T10:30:00.000Z"}"#,
                r#"{"type":"user","message":{"content":"msg2"},"timestamp":"2024-05-07T10:30:10.000Z"}"#,
            ],
        );

        let ctx = parse_jsonl_file(&path.join("ses_test.jsonl"), true).unwrap();
        assert_eq!(ctx.turns.len(), 1);
        // 时间戳应为最早的那条（合并时保留第一个 pending_ts）
        // 2024-05-07T10:30:00Z = from epoch days
        let expected_ts = parse_timestamp_ms("2024-05-07T10:30:00.000Z");
        assert_eq!(ctx.turns[0].timestamp, expected_ts);
    }

    // -----------------------------------------------------------------------
    // 空目录返回空结果测试
    // -----------------------------------------------------------------------

    #[test]
    fn test_empty_dir_returns_empty() {
        let dir = tempfile::tempdir().unwrap();

        // 使用 dir 的路径作为 project_path，但不存在对应的 sessions 子目录
        let sessions = SessionTranscriptCollector::list_sessions(dir.path()).unwrap();
        assert!(sessions.is_empty());

        let latest = SessionTranscriptCollector::get_latest_session(dir.path()).unwrap();
        assert!(latest.is_none());
    }

    // -----------------------------------------------------------------------
    // 元数据模式 vs 完整模式测试
    // -----------------------------------------------------------------------

    #[test]
    fn test_metadata_only_parsing() {
        let (path, _dir) = create_mock_jsonl(
            "ses_test.jsonl",
            &[
                r#"{"type":"user","message":{"content":"帮我写排序"},"timestamp":"2024-05-07T10:30:00.000Z"}"#,
                r#"{"type":"assistant","message":{"model":"claude-sonnet","content":[{"type":"text","text":"好的"}]},"timestamp":"2024-05-07T10:31:00.000Z"}"#,
                r#"{"type":"custom-title","title":"排序助手"}"#,
            ],
        );

        // 元数据模式
        let ctx_meta = parse_jsonl_file(&path.join("ses_test.jsonl"), false).unwrap();
        assert_eq!(ctx_meta.initial_prompt, "帮我写排序");
        assert_eq!(ctx_meta.model.as_deref(), Some("claude-sonnet"));
        assert_eq!(ctx_meta.custom_title.as_deref(), Some("排序助手"));
        assert_eq!(ctx_meta.turn_count, 2);
        assert!(ctx_meta.turns.is_empty());

        // 完整模式
        let ctx_full = parse_jsonl_file(&path.join("ses_test.jsonl"), true).unwrap();
        assert_eq!(ctx_full.initial_prompt, "帮我写排序");
        assert_eq!(ctx_full.turn_count, 2);
        assert_eq!(ctx_full.turns.len(), 2);
    }

    // -----------------------------------------------------------------------
    // 搜索测试
    // -----------------------------------------------------------------------

    #[test]
    fn test_search_by_initial_prompt() {
        // 需要在真实的 ~/.claude/projects/ 路径下测试
        // 这里只测试过滤逻辑
        let summaries = vec![
            SessionSummary {
                session_id: "s1".into(),
                initial_prompt: "帮我写排序算法".into(),
                custom_title: None,
                model: None,
                turn_count: 2,
                modified_files: vec![],
                created_at: 100,
            },
            SessionSummary {
                session_id: "s2".into(),
                initial_prompt: "解释二叉树".into(),
                custom_title: Some("数据结构".into()),
                model: None,
                turn_count: 1,
                modified_files: vec![],
                created_at: 200,
            },
            SessionSummary {
                session_id: "s3".into(),
                initial_prompt: "写一个 HTTP 客户端".into(),
                custom_title: None,
                model: None,
                turn_count: 3,
                modified_files: vec![],
                created_at: 300,
            },
        ];

        // 按 initial_prompt 匹配
        let matched: Vec<_> = summaries
            .iter()
            .filter(|s| s.initial_prompt.to_lowercase().contains("排序"))
            .collect();
        assert_eq!(matched.len(), 1);
        assert_eq!(matched[0].session_id, "s1");

        // 按 custom_title 匹配
        let matched: Vec<_> = summaries
            .iter()
            .filter(|s| {
                s.custom_title
                    .as_ref()
                    .map(|t| t.to_lowercase().contains("数据结构"))
                    .unwrap_or(false)
            })
            .collect();
        assert_eq!(matched.len(), 1);
        assert_eq!(matched[0].session_id, "s2");

        // 无匹配
        let matched: Vec<_> = summaries
            .iter()
            .filter(|s| s.initial_prompt.to_lowercase().contains("不存在"))
            .collect();
        assert!(matched.is_empty());
    }

    // -----------------------------------------------------------------------
    // 特殊格式处理测试
    // -----------------------------------------------------------------------

    #[test]
    fn test_skip_system_and_tool_types() {
        let (path, _dir) = create_mock_jsonl(
            "ses_test.jsonl",
            &[
                r#"{"type":"system","message":{"content":"系统提示"},"timestamp":"2024-05-07T10:30:00.000Z"}"#,
                r#"{"type":"user","message":{"content":"用户消息"},"timestamp":"2024-05-07T10:31:00.000Z"}"#,
                r#"{"type":"tool_use","name":"read","input":{"file_path":"test.py"},"timestamp":"2024-05-07T10:32:00.000Z"}"#,
                r#"{"type":"tool_result","content":"文件内容","timestamp":"2024-05-07T10:33:00.000Z"}"#,
                r#"{"type":"assistant","message":{"content":"回复"},"timestamp":"2024-05-07T10:34:00.000Z"}"#,
            ],
        );

        let ctx = parse_jsonl_file(&path.join("ses_test.jsonl"), true).unwrap();
        assert_eq!(ctx.turn_count, 2);
        assert_eq!(ctx.turns.len(), 2);
        assert_eq!(ctx.turns[0].role, "user");
        assert_eq!(ctx.turns[0].text, "用户消息");
        assert_eq!(ctx.turns[1].role, "assistant");
        assert_eq!(ctx.turns[1].text, "回复");
    }

    #[test]
    fn test_top_level_user_content() {
        let (path, _dir) = create_mock_jsonl(
            "ses_test.jsonl",
            &[
                r#"{"type":"user","content":"分析当前项目","timestamp":"2024-05-07T10:30:00.000Z"}"#,
            ],
        );

        let ctx = parse_jsonl_file(&path.join("ses_test.jsonl"), true).unwrap();
        assert_eq!(ctx.turn_count, 1);
        assert_eq!(ctx.initial_prompt, "分析当前项目");
        assert_eq!(ctx.turns.len(), 1);
        assert_eq!(ctx.turns[0].role, "user");
        assert_eq!(ctx.turns[0].text, "分析当前项目");
    }

    /// 测试：用户消息包含 `$@` 特殊模式（真实 JSONL line 4 匹配此模式）
    #[test]
    fn test_user_dollar_at_pattern() {
        let (path, _dir) = create_mock_jsonl(
            "ses_test.jsonl",
            &[
                r#"{"type":"user","message":{"content":"$@"},"timestamp":"2024-05-07T10:30:00.000Z"}"#,
                r#"{"type":"assistant","message":{"content":[{"type":"text","text":"收到命令"}]},"timestamp":"2024-05-07T10:31:00.000Z"}"#,
            ],
        );

        let ctx = parse_jsonl_file(&path.join("ses_test.jsonl"), true).unwrap();
        assert_eq!(ctx.turn_count, 1);
        assert_eq!(ctx.turns.len(), 1);
        assert_eq!(ctx.turns[0].role, "assistant");
        assert!(ctx.initial_prompt.is_empty());
    }

    /// 测试：用户消息包含 `[Request interrupted by user]`（真实 JSONL line 9 匹配此模式）
    #[test]
    fn test_user_interrupted_request() {
        let (path, _dir) = create_mock_jsonl(
            "ses_test.jsonl",
            &[
                r#"{"type":"user","message":{"content":[{"type":"text","text":"[Request interrupted by user]"}]},"timestamp":"2024-05-07T10:30:00.000Z"}"#,
                r#"{"type":"assistant","message":{"content":[{"type":"text","text":"好的，已中断"}]},"timestamp":"2024-05-07T10:31:00.000Z"}"#,
            ],
        );

        let ctx = parse_jsonl_file(&path.join("ses_test.jsonl"), true).unwrap();
        assert_eq!(ctx.turn_count, 1);
        assert_eq!(ctx.turns.len(), 1);
        assert_eq!(ctx.turns[0].role, "assistant");
        assert!(ctx.initial_prompt.is_empty());
    }

    /// 测试：本地命令 XML 包装只保留真实 command-message 内容。
    #[test]
    fn test_xml_system_tags_are_stripped_from_turn_text() {
        let (path, _dir) = create_mock_jsonl(
            "ses_test.jsonl",
            &[
                r#"{"type":"user","message":{"content":"<local-command-caveat>内部命令说明</local-command-caveat>\n<command-name>/clear</command-name>\n<command-message>当前项目缺少 pytest 环境，帮我配置</command-message>"},"timestamp":"2024-05-07T10:30:00.000Z"}"#,
                r#"{"type":"user","message":{"content":"<local-command-caveat>只有系统说明</local-command-caveat><command-name>/clear</command-name>"},"timestamp":"2024-05-07T10:31:00.000Z"}"#,
                r#"{"type":"assistant","message":{"content":[{"type":"text","text":"可以，先检查依赖"}]},"timestamp":"2024-05-07T10:32:00.000Z"}"#,
            ],
        );

        let ctx = parse_jsonl_file(&path.join("ses_test.jsonl"), true).unwrap();
        assert_eq!(ctx.initial_prompt, "当前项目缺少 pytest 环境，帮我配置");
        assert_eq!(ctx.turn_count, 2);
        assert_eq!(ctx.turns.len(), 2);
        assert_eq!(ctx.turns[0].role, "user");
        assert_eq!(ctx.turns[0].text, "当前项目缺少 pytest 环境，帮我配置");
        assert_eq!(ctx.turns[1].role, "assistant");
    }

    /// 测试：用户消息携带 toolUseResult（真实 JSONL lines 18-20 匹配此模式）
    #[test]
    fn test_user_with_tool_use_result() {
        let (path, _dir) = create_mock_jsonl(
            "ses_test.jsonl",
            &[
                r#"{"type":"user","message":{"content":[{"type":"text","text":"工具结果"}]},"toolUseResult":{"status":"success","output":"done"},"sourceToolAssistantUUID":"abc-123","timestamp":"2024-05-07T10:30:00.000Z"}"#,
                r#"{"type":"assistant","message":{"content":[{"type":"text","text":"收到结果"}]},"timestamp":"2024-05-07T10:31:00.000Z"}"#,
            ],
        );

        let ctx = parse_jsonl_file(&path.join("ses_test.jsonl"), true).unwrap();
        assert_eq!(ctx.turns.len(), 2);
        assert_eq!(ctx.turns[0].role, "user");
        assert_eq!(ctx.turns[0].text, "工具结果");
    }

    /// 测试：assistant 消息包含多个 tool_use 块（真实 JSONL 中常见）
    #[test]
    fn test_assistant_multiple_tool_use_blocks() {
        let (path, _dir) = create_mock_jsonl(
            "ses_test.jsonl",
            &[
                r#"{"type":"user","message":{"content":"分析项目"},"timestamp":"2024-05-07T10:30:00.000Z"}"#,
                r#"{"type":"assistant","message":{"model":"claude-sonnet","content":[{"type":"text","text":"我来分析"},{"type":"tool_use","name":"Read","input":{"file_path":"src/main.py"}},{"type":"tool_use","name":"Bash","input":{"command":"ls -la"}},{"type":"tool_use","name":"Read","input":{"path":"README.md"}}]},"timestamp":"2024-05-07T10:31:00.000Z"}"#,
            ],
        );

        let ctx = parse_jsonl_file(&path.join("ses_test.jsonl"), true).unwrap();
        assert_eq!(ctx.turns.len(), 2);
        assert_eq!(ctx.turns[1].role, "assistant");
        assert_eq!(ctx.turns[1].text, "我来分析");
        assert_eq!(ctx.turns[1].tools, vec!["Bash", "Read"]);
        assert!(ctx.modified_files.contains(&"src/main.py".to_string()));
        assert!(ctx.modified_files.contains(&"README.md".to_string()));
    }

    /// 测试：custom-title 条目在真实 JSONL 中不存在，但 parser 已做防御性处理
    #[test]
    fn test_custom_title_not_found_in_real_data_but_parsed() {
        let (path, _dir) = create_mock_jsonl(
            "ses_test.jsonl",
            &[
                r#"{"type":"user","message":{"content":"测试"},"timestamp":"2024-05-07T10:30:00.000Z"}"#,
                r#"{"type":"custom-title","title":"真实标题"}"#,
            ],
        );

        let ctx = parse_jsonl_file(&path.join("ses_test.jsonl"), false).unwrap();
        assert_eq!(ctx.custom_title.as_deref(), Some("真实标题"));
        assert_eq!(ctx.turn_count, 1);
    }

    #[test]
    fn test_transcript_noise_filters() {
        let (path, _dir) = create_mock_jsonl(
            "ses_test.jsonl",
            &[
                r#"{"type":"user","message":{"content":"$@"},"timestamp":"2024-05-07T10:30:00.000Z"}"#,
                r#"{"type":"user","message":{"content":"[SYSTEM DIRECTIVE: internal placeholder]"},"timestamp":"2024-05-07T10:31:00.000Z"}"#,
                r#"{"type":"user","message":{"content":"<!-- OMO_INTERNAL_INITIATOR -->"},"timestamp":"2024-05-07T10:32:00.000Z"}"#,
                r#"{"type":"user","message":{"content":"  "},"timestamp":"2024-05-07T10:33:00.000Z"}"#,
                r#"{"type":"user","message":{"content":"You are a conversation title generator"},"timestamp":"2024-05-07T10:34:00.000Z"}"#,
                r#"{"type":"user","message":{"content":"真实用户请求"},"timestamp":"2024-05-07T10:35:00.000Z"}"#,
                r#"{"type":"assistant","message":{"content":"收到"},"timestamp":"2024-05-07T10:36:00.000Z"}"#,
            ],
        );

        let ctx = parse_jsonl_file(&path.join("ses_test.jsonl"), true).unwrap();
        assert_eq!(ctx.initial_prompt, "真实用户请求");
        assert_eq!(ctx.turn_count, 2);
        assert_eq!(ctx.turns.len(), 2);
        assert_eq!(ctx.turns[0].role, "user");
        assert_eq!(ctx.turns[0].text, "真实用户请求");
        assert_eq!(ctx.turns[1].role, "assistant");
    }

    #[test]
    fn test_custom_title_noise_filtered() {
        let (path, _dir) = create_mock_jsonl(
            "ses_test.jsonl",
            &[
                r#"{"type":"user","message":{"content":"测试"},"timestamp":"2024-05-07T10:30:00.000Z"}"#,
                r#"{"type":"custom-title","title":"$@"}"#,
                r#"{"type":"custom-title","title":"[SYSTEM DIRECTIVE: ignore]"}"#,
                r#"{"type":"custom-title","title":"<!-- OMO_INTERNAL_INITIATOR -->"}"#,
                r#"{"type":"custom-title","title":"  "}"#,
                r#"{"type":"custom-title","title":"有效标题"}"#,
            ],
        );

        let ctx = parse_jsonl_file(&path.join("ses_test.jsonl"), false).unwrap();
        assert_eq!(ctx.custom_title.as_deref(), Some("有效标题"));
    }
}
