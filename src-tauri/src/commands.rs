use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::collectors::agent::AgentCollector;
use crate::collectors::claude_history::{
    models::{ExportFormat, SerClaudeSession, SerHistoryEntry, SerProjectSessionGroup, SerSessionPreview},
    scanner::{delete_claude_session, export_claude_session, get_session_detail, list_claude_sessions, preview_claude_session, search_claude_history},
};
use crate::collectors::template::{
    load_template_path, save_template_path, ProjectFile, ProjectFilesCollector, SessionSummary,
    SessionTranscript, SessionTranscriptCollector, SessionTurn, SourceLayout, TemplateData,
    TemplateDataCollector, TemplateFingerprint, WatchedCollector,
};
use crate::registry::{ProjectEntry, ProjectRegistry};

fn describe_path_error(path: &str) -> String {
    let path_buf = PathBuf::from(path);

    if !path_buf.exists() {
        return "项目路径不存在或已被删除".to_string();
    }

    if std::fs::read_dir(&path_buf).is_err() {
        return "无权访问该项目路径".to_string();
    }

    "无法访问该项目路径".to_string()
}

#[derive(Debug, Clone, Serialize)]
pub struct SerStage {
    pub name: String,
    pub description: String,
    pub ordinal: u8,
}

impl From<crate::collectors::template::Stage> for SerStage {
    fn from(stage: crate::collectors::template::Stage) -> Self {
        Self {
            name: stage.as_str().to_string(),
            description: stage.description().to_string(),
            ordinal: stage.ordinal(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerProjectConfig {
    pub project_name: String,
    pub module_name: String,
    pub interface_type: String,
    #[serde(default)]
    pub reference_project: String,
    #[serde(default)]
    pub use_l0: bool,
    #[serde(default)]
    pub data_width: i64,
    #[serde(default)]
    pub iterations: i64,
    #[serde(default)]
    pub q_int_bits: i64,
    #[serde(default)]
    pub q_frac_bits: i64,
    #[serde(default)]
    pub rounding_mode: String,
    #[serde(default)]
    pub saturation: bool,
    #[serde(default)]
    pub pipeline_stages: i64,
    #[serde(default)]
    pub cycles_per_stage: i64,
    #[serde(default)]
    pub output_register: bool,
    #[serde(default)]
    pub axis_data_width: i64,
    #[serde(default)]
    pub axis_has_tlast: bool,
    #[serde(default)]
    pub axis_has_tkeep: bool,
    #[serde(default)]
    pub handshake_delay: i64,
    #[serde(default)]
    pub axi_lite_addr_width: i64,
    #[serde(default)]
    pub test_data_length: i64,
    #[serde(default)]
    pub random_seed: i64,
    #[serde(default)]
    pub float_tolerance: f64,
    #[serde(default)]
    pub fixed_tolerance: f64,
    #[serde(default)]
    pub clock_frequency: i64,
    #[serde(default)]
    pub reset_sync_stages: i64,
    #[serde(default)]
    pub use_clock_enable: bool,
    #[serde(default)]
    pub debug_mode: bool,
    #[serde(default)]
    pub debug_level: i64,
    #[serde(default)]
    pub total_bits: Option<i64>,
    #[serde(default)]
    pub q_scale: Option<i64>,
    #[serde(default)]
    pub pipeline_latency: Option<i64>,
    #[serde(default)]
    pub max_positive: Option<f64>,
    #[serde(default)]
    pub min_negative: Option<f64>,
}

impl From<crate::collectors::template::ProjectConfig> for SerProjectConfig {
    fn from(cfg: crate::collectors::template::ProjectConfig) -> Self {
        Self {
            project_name: cfg.project_name,
            module_name: cfg.module_name,
            interface_type: cfg.interface_type,
            reference_project: cfg.reference_project,
            use_l0: cfg.use_l0,
            data_width: cfg.data_width,
            iterations: cfg.iterations,
            q_int_bits: cfg.q_int_bits,
            q_frac_bits: cfg.q_frac_bits,
            rounding_mode: cfg.rounding_mode,
            saturation: cfg.saturation,
            pipeline_stages: cfg.pipeline_stages,
            cycles_per_stage: cfg.cycles_per_stage,
            output_register: cfg.output_register,
            axis_data_width: cfg.axis_data_width,
            axis_has_tlast: cfg.axis_has_tlast,
            axis_has_tkeep: cfg.axis_has_tkeep,
            handshake_delay: cfg.handshake_delay,
            axi_lite_addr_width: cfg.axi_lite_addr_width,
            test_data_length: cfg.test_data_length,
            random_seed: cfg.random_seed,
            float_tolerance: cfg.float_tolerance,
            fixed_tolerance: cfg.fixed_tolerance,
            clock_frequency: cfg.clock_frequency,
            reset_sync_stages: cfg.reset_sync_stages,
            use_clock_enable: cfg.use_clock_enable,
            debug_mode: cfg.debug_mode,
            debug_level: cfg.debug_level,
            total_bits: cfg.total_bits,
            q_scale: cfg.q_scale,
            pipeline_latency: cfg.pipeline_latency,
            max_positive: cfg.max_positive,
            min_negative: cfg.min_negative,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SerGitStatus {
    pub branch: String,
    pub modified_count: usize,
    pub staged_count: usize,
    pub untracked_count: usize,
    pub conflict_count: usize,
    pub is_clean: bool,
    pub changed_files: Vec<String>,
}

impl From<crate::collectors::template::GitStatus> for SerGitStatus {
    fn from(status: crate::collectors::template::GitStatus) -> Self {
        Self {
            branch: status.branch,
            modified_count: status.modified_count,
            staged_count: status.staged_count,
            untracked_count: status.untracked_count,
            conflict_count: status.conflict_count,
            is_clean: status.is_clean,
            changed_files: status.changed_files,
        }
    }
}

impl Default for SerGitStatus {
    fn default() -> Self {
        Self {
            branch: String::new(),
            modified_count: 0,
            staged_count: 0,
            untracked_count: 0,
            conflict_count: 0,
            is_clean: true,
            changed_files: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SerProjectFile {
    pub relative_path: String,
    pub source_group: String,
    pub content_preview: String,
    pub mtime_ms: u64,
    pub origin: String,
}

impl From<ProjectFile> for SerProjectFile {
    fn from(f: ProjectFile) -> Self {
        let content_preview = if f.content.chars().count() > 200 {
            let truncated: String = f.content.chars().take(200).collect();
            format!("{}...", truncated)
        } else {
            f.content.clone()
        };
        let source_group = match f.source_group.as_str() {
            "design" | "specs" => "docs".to_string(),
            other => other.to_string(),
        };
        Self {
            relative_path: f.relative_path,
            source_group,
            content_preview,
            mtime_ms: f.mtime_ms,
            origin: f.origin,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SerTurn {
    pub role: String,
    pub text: String,
    pub tools: Vec<String>,
    pub timestamp: Option<u64>,
}

impl From<SessionTurn> for SerTurn {
    fn from(turn: SessionTurn) -> Self {
        Self {
            role: turn.role,
            text: turn.text,
            tools: turn.tools,
            timestamp: turn.timestamp,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SerTranscript {
    pub session_id: String,
    pub initial_prompt: String,
    pub custom_title: Option<String>,
    pub model: Option<String>,
    pub turns: Vec<SerTurn>,
    pub modified_files: Vec<String>,
    pub created_at: u64,
}

impl From<SessionTranscript> for SerTranscript {
    fn from(t: SessionTranscript) -> Self {
        Self {
            session_id: t.session_id,
            initial_prompt: t.initial_prompt,
            custom_title: t.custom_title,
            model: t.model,
            turns: t.turns.into_iter().map(SerTurn::from).collect(),
            modified_files: t.modified_files,
            created_at: t.created_at,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SerSessionSummary {
    pub session_id: String,
    pub initial_prompt: String,
    pub custom_title: Option<String>,
    pub model: Option<String>,
    pub turn_count: usize,
    pub modified_files: Vec<String>,
    pub created_at: u64,
}

impl From<SessionSummary> for SerSessionSummary {
    fn from(s: SessionSummary) -> Self {
        Self {
            session_id: s.session_id,
            initial_prompt: s.initial_prompt,
            custom_title: s.custom_title,
            model: s.model,
            turn_count: s.turn_count,
            modified_files: s.modified_files,
            created_at: s.created_at,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct SerCandidateMemory {
    pub content: String,
    pub source_session_id: String,
    pub source_turn_index: usize,
    pub source_snippet: String,
    pub category: String,
}

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

#[tauri::command]
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

#[tauri::command]
pub fn get_project_file_content(
    path: String,
    relative_path: String,
) -> Result<String, String> {
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

#[derive(Debug, Clone, Serialize)]
pub struct TemplateDataPayload {
    pub project_path: String,
    pub stage: Option<SerStage>,
    pub stage_error: Option<String>,
    pub config: Option<SerProjectConfig>,
    pub config_error: Option<String>,
    pub git: SerGitStatus,
    pub git_error: Option<String>,
    pub layout: String,
    pub timestamp_ms: u64,
}

impl TemplateDataPayload {
    pub fn from_data(project_path: String, data: TemplateData) -> Self {
        let timestamp_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let (stage, stage_error) = match data.stage {
            Ok(s) => (Some(SerStage::from(s)), None),
            Err(e) => (None, Some(e.to_string())),
        };

        let (config, config_error) = match data.config {
            Ok(c) => (Some(SerProjectConfig::from(c)), None),
            Err(e) => (None, Some(e.to_string())),
        };

        let (git, git_error) = match data.git {
            Ok(g) => (SerGitStatus::from(g), None),
            Err(e) => (SerGitStatus::default(), Some(e.to_string())),
        };

        let layout = match data.layout {
            SourceLayout::Flat => "flat".to_string(),
            SourceLayout::Namespaced(name) => format!("namespaced:{}", name),
            SourceLayout::Unknown => "unknown".to_string(),
        };

        Self {
            project_path,
            stage,
            stage_error,
            config,
            config_error,
            git,
            git_error,
            layout,
            timestamp_ms,
        }
    }
}

/// 模板指纹缓存 — 记录模板目录中所有文件路径的快照
#[derive(Debug, Clone)]
pub struct TemplateFingerprintCache {
    pub paths: HashSet<String>,
    pub generated_at: std::time::Instant,
}

pub struct AppState {
    pub registry: Mutex<ProjectRegistry>,
    pub watchers: Mutex<HashMap<String, Arc<AtomicBool>>>,
    pub agent_collector: Mutex<AgentCollector>,
    pub template_path: Mutex<Option<PathBuf>>,
    pub template_fingerprint: Mutex<Option<TemplateFingerprintCache>>,
}

impl AppState {
    pub fn new(registry: ProjectRegistry, agent_collector: AgentCollector) -> Self {
        Self {
            registry: Mutex::new(registry),
            watchers: Mutex::new(HashMap::new()),
            agent_collector: Mutex::new(agent_collector),
            template_path: Mutex::new(None),
            template_fingerprint: Mutex::new(None),
        }
    }
}

#[tauri::command]
pub fn add_project(
    path: String,
    state: State<'_, AppState>,
) -> Result<ProjectEntry, String> {
    let path_buf = PathBuf::from(&path);
    let mut registry = state.registry.lock().map_err(|e| format!("锁获取失败: {}", e))?;

    let entry = registry.add(&path_buf).map_err(|e| e.to_string())?;

    if let Ok(agent_collector) = state.agent_collector.lock() {
        agent_collector.register_project(entry.path.clone());
    }

    Ok(entry)
}

#[tauri::command]
pub fn remove_project(
    path: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let path_buf = PathBuf::from(&path);
    let mut registry = state.registry.lock().map_err(|e| format!("锁获取失败: {}", e))?;

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

#[tauri::command]
pub fn list_projects(state: State<'_, AppState>) -> Vec<ProjectEntry> {
    match state.registry.lock() {
        Ok(registry) => registry.list(),
        Err(_) => Vec::new(),
    }
}

#[tauri::command]
pub fn get_project_data(
    path: String,
    _state: State<'_, AppState>,
) -> Result<TemplateDataPayload, String> {
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

#[tauri::command]
pub fn start_watching(
    path: String,
    app_handle: AppHandle,
    state: State<'_, AppState>,
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
        .name(format!("ptv-watcher-{}", project_path))
        .spawn(move || {
            while let Ok(event) = rx.recv() {
                let payload = TemplateDataPayload::from_data(project_path.clone(), event.data);
                if let Err(e) = app_handle.emit("template-update", &payload) {
                    eprintln!("[commands] 发送 template-update 失败: {}", e);
                }
            }
            println!("[commands] 项目监听线程已退出: {}", project_path);
        })
        .map_err(|e| format!("无法启动监听线程: {}", e))?;

    println!("[commands] 开始监听项目: {}", canonical_path);
    Ok(())
}

#[tauri::command]
pub fn stop_watching(
    path: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let path_buf = PathBuf::from(&path);
    let canonical_path = path_buf
        .canonicalize()
        .unwrap_or_else(|_| path_buf.clone())
        .to_string_lossy()
        .to_string();

    let mut watchers = state.watchers.lock().map_err(|e| format!("锁获取失败: {}", e))?;

    if let Some(stop_signal) = watchers.remove(&canonical_path) {
        stop_signal.store(false, Ordering::SeqCst);
        println!("[commands] 已停止监听项目: {}", canonical_path);
    }

    Ok(())
}

#[tauri::command]
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

#[tauri::command]
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

#[tauri::command]
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

#[tauri::command]
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

#[tauri::command]
pub fn save_candidate_memory(
    path: String,
    memory: SerCandidateMemory,
) -> Result<(), String> {
    use std::io::Write;

    let path_buf = PathBuf::from(&path);
    if !path_buf.exists() || !path_buf.is_dir() {
        return Err(describe_path_error(&path));
    }

    let memory_dir = path_buf
        .join(".sisyphus")
        .join("notepads")
        .join("project-memory");
    let memory_file = memory_dir.join("decisions.md");

    // 确保目录存在
    if let Err(e) = std::fs::create_dir_all(&memory_dir) {
        return Err(format!("创建目录失败: {}", e));
    }

    // 生成时间戳（使用 std::time，无需 chrono 依赖）
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    // 简单日期计算（UTC），近似 YYYY-MM-DD HH:MM:SS
    let days = secs / 86400;
    let time_of_day = secs % 86400;
    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;
    let seconds = time_of_day % 60;

    // 从 epoch 天数计算日期（1970-01-01 = day 0）
    let mut y = 1970i64;
    let mut remaining_days = days as i64;
    loop {
        let year_days = if (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0) {
            366
        } else {
            365
        };
        if remaining_days < year_days {
            break;
        }
        remaining_days -= year_days;
        y += 1;
    }
    let month_days = if (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut m = 1usize;
    for &md in &month_days {
        if remaining_days < md as i64 {
            break;
        }
        remaining_days -= md as i64;
        m += 1;
    }
    let d = remaining_days + 1;

    let timestamp = format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        y, m, d, hours, minutes, seconds
    );

    let entry = format!(
        "\n## [{}] {}\n\n{}\n\n来源: {} / Turn {}\n",
        timestamp,
        memory.category,
        memory.content,
        memory.source_session_id,
        memory.source_turn_index
    );

    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&memory_file)
        .map_err(|e| format!("打开文件失败: {}", e))?;

    file.write_all(entry.as_bytes())
        .map_err(|e| format!("写入文件失败: {}", e))?;

    Ok(())
}

// ============================================================================
// 模板路径管理
// ============================================================================

#[tauri::command]
pub fn set_template_path(
    path: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let path_buf = PathBuf::from(&path);

    if !path_buf.exists() {
        return Err(format!("路径不存在: {}", path));
    }
    if !path_buf.is_dir() {
        return Err(format!("路径不是目录: {}", path));
    }

    let fingerprint = TemplateFingerprint::build(&path_buf)
        .map_err(|e| format!("构建模板指纹失败: {}", e))?;

    {
        let mut tp = state.template_path.lock().map_err(|e| format!("锁获取失败: {}", e))?;
        *tp = Some(path_buf.clone());
    }
    {
        let mut fp = state
            .template_fingerprint
            .lock()
            .map_err(|e| format!("锁获取失败: {}", e))?;
        *fp = Some(TemplateFingerprintCache {
            paths: fingerprint.paths,
            generated_at: std::time::Instant::now(),
        });
    }

    let data_dir = ProjectRegistry::default_data_dir();
    save_template_path(&data_dir, &path_buf)
        .map_err(|e| format!("保存模板路径失败: {}", e))?;

    Ok(())
}

#[tauri::command]
pub fn get_template_path(state: State<'_, AppState>) -> Result<Option<String>, String> {
    let tp = state
        .template_path
        .lock()
        .map_err(|e| format!("锁获取失败: {}", e))?;
    Ok(tp.as_ref().map(|p| p.to_string_lossy().to_string()))
}

pub fn init_app_state(
    app: &tauri::App,
    registry: ProjectRegistry,
    agent_collector: AgentCollector,
) {
    let state = AppState::new(registry, agent_collector);

    let data_dir = ProjectRegistry::default_data_dir();
    if let Some(template_path) = load_template_path(&data_dir) {
        if template_path.exists() && template_path.is_dir() {
            if let Ok(fingerprint) = TemplateFingerprint::build(&template_path) {
                if let Ok(mut tp) = state.template_path.lock() {
                    *tp = Some(template_path);
                }
                if let Ok(mut fp) = state.template_fingerprint.lock() {
                    *fp = Some(TemplateFingerprintCache {
                        paths: fingerprint.paths,
                        generated_at: std::time::Instant::now(),
                    });
                }
            }
        } else {
            eprintln!(
                "[init_app_state] 警告: 已保存的模板路径不存在或不是目录: {}",
                template_path.display()
            );
        }
    }

    app.manage(state);
}

// ============================================================================
// Claude Code 会话管理命令
// ============================================================================

#[tauri::command]
pub fn list_claude_sessions_cmd() -> Result<Vec<SerProjectSessionGroup>, String> {
    list_claude_sessions()
}

#[tauri::command]
pub fn get_claude_session_detail_cmd(
    session_id: String,
) -> Result<Option<SerClaudeSession>, String> {
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

#[tauri::command]
pub fn export_claude_session_cmd(
    session_id: String,
    format: ExportFormat,
) -> Result<String, String> {
    export_claude_session(&session_id, format)
}

#[tauri::command]
pub fn preview_claude_session_cmd(session_id: String) -> Result<SerSessionPreview, String> {
    preview_claude_session(&session_id)
}
