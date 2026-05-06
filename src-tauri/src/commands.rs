use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::collectors::agent::AgentCollector;
use crate::collectors::template::{
    SourceLayout, TemplateData, TemplateDataCollector, WatchedCollector,
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
pub struct SerMemoryEntry {
    pub filename: String,
    pub frontmatter: HashMap<String, String>,
    pub content_preview: String,
}

impl From<crate::collectors::template::MemoryEntry> for SerMemoryEntry {
    fn from(entry: crate::collectors::template::MemoryEntry) -> Self {
        let content_preview = if entry.content.len() > 500 {
            format!("{}...", &entry.content[..500])
        } else {
            entry.content.clone()
        };
        Self {
            filename: entry.filename,
            frontmatter: entry.frontmatter,
            content_preview,
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
pub struct TemplateDataPayload {
    pub project_path: String,
    pub stage: Option<SerStage>,
    pub stage_error: Option<String>,
    pub config: Option<SerProjectConfig>,
    pub config_error: Option<String>,
    pub memories: Vec<SerMemoryEntry>,
    pub memory_error: Option<String>,
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

        let (memories, memory_error) = match data.memories {
            Ok(entries) => {
                let ser: Vec<SerMemoryEntry> = entries.into_iter().map(SerMemoryEntry::from).collect();
                (ser, None)
            }
            Err(e) => (Vec::new(), Some(e.to_string())),
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
            memories,
            memory_error,
            git,
            git_error,
            layout,
            timestamp_ms,
        }
    }
}

pub struct AppState {
    pub registry: Mutex<ProjectRegistry>,
    pub watchers: Mutex<HashMap<String, Arc<AtomicBool>>>,
    pub agent_collector: Mutex<AgentCollector>,
}

impl AppState {
    pub fn new(registry: ProjectRegistry, agent_collector: AgentCollector) -> Self {
        Self {
            registry: Mutex::new(registry),
            watchers: Mutex::new(HashMap::new()),
            agent_collector: Mutex::new(agent_collector),
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

pub fn init_app_state(
    app: &tauri::App,
    registry: ProjectRegistry,
    agent_collector: AgentCollector,
) {
    let state = AppState::new(registry, agent_collector);
    app.manage(state);
}
