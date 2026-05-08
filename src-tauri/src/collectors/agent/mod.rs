//! # Agent 运行时采集器
//!
//! 集成 `abtop-collector` crate，每 2 秒轮询一次系统中活跃的 Agent 会话状态，
//! 通过 Tauri event (`agent-update`) 将数据推送到前端。
//!
//! ## 设计说明
//!
//! - 轮询间隔固定为 2 秒，使用独立后台线程
//! - 结果按 `cwd` 关联到已注册的项目路径（前缀匹配）
//! - 错误处理：采集失败时记录日志，不 panic、不中断轮询
//! - 优雅降级：当 abtop-collector 无可采集数据时发送空列表

use abtop_collector::collector::MultiCollector;
use abtop_collector::model::{AgentSession, SessionStatus};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

// ============================================================================
// 可序列化的数据结构（用于 Tauri event）
// ============================================================================

/// Agent 状态的可序列化表示
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub enum SerializableStatus {
    /// Agent 正在生成（无活跃工具，CPU 活跃）
    Thinking,
    /// 正在运行工具
    Executing,
    /// 空闲，等待用户输入
    Waiting,
    /// 因速率限制等待
    RateLimited,
    /// 会话已结束
    Done,
}

impl From<&SessionStatus> for SerializableStatus {
    fn from(status: &SessionStatus) -> Self {
        match status {
            SessionStatus::Thinking => SerializableStatus::Thinking,
            SessionStatus::Executing => SerializableStatus::Executing,
            SessionStatus::Waiting => SerializableStatus::Waiting,
            SessionStatus::RateLimited => SerializableStatus::RateLimited,
            SessionStatus::Done => SerializableStatus::Done,
        }
    }
}

/// 单个 Agent Session 的精简信息（推送给前端）
#[derive(Debug, Clone, Serialize)]
pub struct AgentInfo {
    /// Agent CLI 类型："claude"、"codex" 等
    pub agent_type: String,
    /// 会话唯一标识
    pub session_id: String,
    /// 工作目录
    pub cwd: String,
    /// 项目名称
    pub project_name: String,
    /// 当前状态
    pub status: SerializableStatus,
    /// 使用的模型
    pub model: String,
    /// 上下文使用百分比 (0-100)
    pub context_percent: f64,
    /// 上下文窗口大小（token 数）
    pub context_window: u64,
    /// 总输入 token 数
    pub total_input_tokens: u64,
    /// 总输出 token 数
    pub total_output_tokens: u64,
    /// 缓存读取 token 数
    pub total_cache_read: u64,
    /// 缓存创建 token 数
    pub total_cache_create: u64,
    /// 当前轮次
    pub turn_count: u32,
    /// 当前任务列表
    pub current_tasks: Vec<String>,
    /// 内存使用（MB）
    pub mem_mb: u64,
    /// Git 分支
    pub git_branch: String,
    /// Git 新增文件数
    pub git_added: u32,
    /// Git 修改文件数
    pub git_modified: u32,
    /// Token 历史（每轮总 token 数）
    pub token_history: Vec<u64>,
    /// 上下文历史（每轮输入 token 数）
    pub context_history: Vec<u64>,
    /// 压缩事件次数
    pub compaction_count: u32,
    /// 子进程列表
    pub children: Vec<ChildProcessInfo>,
    /// 初始提示词（截断）
    pub initial_prompt: String,
    /// 首次助手回复（截断）
    pub first_assistant_text: String,
    /// Token 速率（token/秒），基于最近 2 秒采样计算
    pub token_rate: f64,
    /// 最近 1 分钟平均 Token 速率（token/秒）
    pub token_rate_1m: f64,
    /// 会话开始至今平均 Token 速率（token/秒）
    pub token_rate_total: f64,
    /// 进程 PID
    pub pid: u32,
    /// 版本号
    pub version: String,
    /// 推理 effort（Codex CLI）
    pub effort: String,
    /// 工具调用记录
    pub tool_calls: Vec<SerToolCall>,
    /// 子 Agent 信息
    pub subagents: Vec<SerSubAgent>,
    /// 文件访问记录
    pub file_accesses: Vec<SerFileAccess>,
    /// 待处理工具调用的起始时间戳（毫秒），0 表示无待处理工具
    pub pending_since_ms: u64,
    /// 最近一次用户消息的时间戳，用于渲染"思考中"虚拟行
    pub thinking_since_ms: u64,
}

/// 子进程信息
#[derive(Debug, Clone, Serialize)]
pub struct ChildProcessInfo {
    pub pid: u32,
    pub command: String,
    pub mem_kb: u64,
    pub port: Option<u16>,
}

/// 工具调用记录
#[derive(Debug, Clone, Serialize)]
pub struct SerToolCall {
    pub name: String,
    pub arg: String,
    pub duration_ms: u64,
}

/// 子 Agent 信息
#[derive(Debug, Clone, Serialize)]
pub struct SerSubAgent {
    pub name: String,
    pub status: String,
    pub tokens: u64,
}

/// 文件访问记录
#[derive(Debug, Clone, Serialize)]
pub struct SerFileAccess {
    pub path: String,
    /// "R" | "W" | "E"（映射自 FileOp）
    pub operation: String,
    pub turn_index: u32,
}

/// 单个项目的 Agent 聚合数据
#[derive(Debug, Clone, Serialize)]
pub struct ProjectAgents {
    /// 项目路径
    pub project_path: String,
    /// 该项目下的活跃 sessions
    pub agents: Vec<AgentInfo>,
    /// 该项目下的 session 数量
    pub count: usize,
}

/// Agent 更新事件的完整 Payload
#[derive(Debug, Clone, Serialize)]
pub struct AgentUpdatePayload {
    /// 按项目分组的 Agent 数据
    pub projects: Vec<ProjectAgents>,
    /// 未匹配到任何注册项目的 orphan sessions
    pub unmapped: Vec<AgentInfo>,
    /// 采集时间戳（Unix 毫秒）
    pub timestamp_ms: u64,
    /// 总 session 数
    pub total_sessions: usize,
}

// ============================================================================
// AgentCollector 实现
// ============================================================================

/// Agent 运行时采集器
///
/// 每 2 秒调用 `MultiCollector::collect()` 采集系统中活跃的 Agent 会话，
/// 按 cwd 匹配到已注册项目，通过 Tauri event 推送到前端。
pub struct AgentCollector {
    /// 已注册的项目路径列表（前缀匹配用）
    registered_paths: Arc<Mutex<Vec<String>>>,
    /// 运行状态信号
    running: Arc<AtomicBool>,
    /// 上一次各 session 的 active_tokens，用于计算 token_rate
    last_tokens: Arc<Mutex<HashMap<String, (u64, Instant)>>>,
    /// 每个 session 的 token 历史采样：[(timestamp, total_tokens), ...]
    /// 保留最近 1 分钟的采样（30 个点，每 2 秒一个）
    token_history_samples: Arc<Mutex<HashMap<String, Vec<(Instant, u64)>>>>,
}

impl AgentCollector {
    /// 创建新的 AgentCollector
    pub fn new() -> Self {
        Self {
            registered_paths: Arc::new(Mutex::new(Vec::new())),
            running: Arc::new(AtomicBool::new(false)),
            last_tokens: Arc::new(Mutex::new(HashMap::new())),
            token_history_samples: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// 注册一个项目路径
    ///
    /// Agent session 的 cwd 若以该路径为前缀，则关联到此项目。
    pub fn register_project(&self, path: String) {
        let mut paths = self.registered_paths.lock().unwrap();
        // 规范化：去除末尾斜杠，确保前缀匹配正确
        let normalized = path.trim_end_matches('/').to_string();
        if !paths.contains(&normalized) {
            paths.push(normalized);
        }
    }

    /// 取消注册一个项目路径
    pub fn unregister_project(&self, path: &str) {
        let mut paths = self.registered_paths.lock().unwrap();
        let normalized = path.trim_end_matches('/');
        paths.retain(|p| p.as_str() != normalized);
    }

    /// 获取当前已注册的项目路径
    pub fn registered_projects(&self) -> Vec<String> {
        self.registered_paths.lock().unwrap().clone()
    }

    /// 启动采集循环
    ///
    /// 在独立后台线程中每 2 秒执行一次采集，结果通过 Tauri event 推送。
    /// 返回线程 JoinHandle，可用于等待线程结束。
    pub fn start(&self, app_handle: AppHandle) -> thread::JoinHandle<()> {
        self.running.store(true, Ordering::SeqCst);

        let running = self.running.clone();
        let registered_paths = self.registered_paths.clone();
        let last_tokens = self.last_tokens.clone();
        let token_history_samples = self.token_history_samples.clone();

        thread::Builder::new()
            .name("ptv-agent-collector".into())
            .spawn(move || {
                let mut collector = MultiCollector::with_hidden(&[]);
                let poll_interval = Duration::from_secs(2);

                while running.load(Ordering::SeqCst) {
                    let tick_start = Instant::now();

                    // 使用 catch_unwind 防止 abtop-collector 内部 panic 导致线程崩溃
                    let sessions = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        collector.collect()
                    }));

                    match sessions {
                        Ok(sessions) => {
                            let payload = build_payload(
                                sessions,
                                &registered_paths,
                                &last_tokens,
                                &token_history_samples,
                            );

                            if let Err(e) = app_handle.emit("agent-update", &payload) {
                                eprintln!("[agent-collector] 发送 Tauri event 失败: {}", e);
                            }
                        }
                        Err(panic_info) => {
                            let msg = if let Some(s) = panic_info.downcast_ref::<&str>() {
                                s.to_string()
                            } else if let Some(s) = panic_info.downcast_ref::<String>() {
                                s.clone()
                            } else {
                                "未知 panic".to_string()
                            };
                            eprintln!("[agent-collector] MultiCollector::collect() panic: {}", msg);
                        }
                    }

                    // 精确控制 2 秒间隔
                    let elapsed = tick_start.elapsed();
                    if elapsed < poll_interval {
                        thread::sleep(poll_interval - elapsed);
                    }
                }

                println!("[agent-collector] 采集线程已退出");
            })
            .expect("failed to spawn agent collector thread")
    }

    /// 发送停止信号
    ///
    /// 采集线程会在下一次轮询时检测到信号并退出。
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    /// 检查是否正在运行
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }
}

impl Default for AgentCollector {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 辅助函数
// ============================================================================

/// 计算 1 分钟速率和全程速率
fn compute_token_rates(samples: &[(Instant, u64)]) -> (f64, f64) {
    if samples.len() < 2 {
        return (0.0, 0.0);
    }

    let first = samples[0];
    let last = samples[samples.len() - 1];
    let delta_tokens = last.1.saturating_sub(first.1);
    let delta_secs = last.0.duration_since(first.0).as_secs_f64();
    let rate_1m = if delta_secs > 0.0 {
        delta_tokens as f64 / delta_secs
    } else {
        0.0
    };

    let now = Instant::now();
    let delta_secs_total = now.duration_since(first.0).as_secs_f64();
    let rate_total = if delta_secs_total > 0.0 {
        delta_tokens as f64 / delta_secs_total
    } else {
        0.0
    };

    (rate_1m, rate_total)
}

/// 将 AgentSession 转换为可序列化的 AgentInfo
fn session_to_info(
    session: &AgentSession,
    last_tokens: &Arc<Mutex<HashMap<String, (u64, Instant)>>>,
    token_history_samples: &Arc<Mutex<HashMap<String, Vec<(Instant, u64)>>>>,
) -> AgentInfo {
    let now = Instant::now();
    let active_tokens = session.active_tokens();

    // 计算 token_rate（token/秒）
    let token_rate = {
        let mut map = last_tokens.lock().unwrap();
        match map.get(&session.session_id) {
            Some((last_count, last_time)) => {
                let delta_tokens = active_tokens.saturating_sub(*last_count);
                let delta_secs = last_time.elapsed().as_secs_f64();
                let rate = if delta_secs > 0.0 {
                    delta_tokens as f64 / delta_secs
                } else {
                    0.0
                };
                map.insert(session.session_id.clone(), (active_tokens, now));
                rate
            }
            None => {
                map.insert(session.session_id.clone(), (active_tokens, now));
                0.0
            }
        }
    };

    // 更新 token 历史采样，计算 1 分钟速率
    let token_rate_1m = {
        let mut history = token_history_samples.lock().unwrap();
        let samples = history.entry(session.session_id.clone()).or_default();
        samples.push((now, active_tokens));

        // 清理超过 1 分钟的旧采样点
        let one_minute = Duration::from_secs(60);
        samples.retain(|(t, _)| now.duration_since(*t) <= one_minute);

        // 计算 1 分钟速率：窗口内 token 增量 / 时间差
        if samples.len() >= 2 {
            let first = samples[0];
            let last = samples[samples.len() - 1];
            let delta_tokens = last.1.saturating_sub(first.1);
            let delta_secs = last.0.duration_since(first.0).as_secs_f64();
            if delta_secs > 0.0 {
                delta_tokens as f64 / delta_secs
            } else {
                0.0
            }
        } else {
            0.0
        }
    };

    // 全程速率：基于会话总 token 数 / 会话持续时间
    let elapsed_secs = session.elapsed().as_secs_f64();
    let token_rate_total = if elapsed_secs > 0.0 {
        active_tokens as f64 / elapsed_secs
    } else {
        0.0
    };

    AgentInfo {
        agent_type: session.agent_cli.to_string(),
        session_id: session.session_id.clone(),
        cwd: session.cwd.clone(),
        project_name: session.project_name.clone(),
        status: SerializableStatus::from(&session.status),
        model: session.model.clone(),
        context_percent: session.context_percent,
        context_window: session.context_window,
        total_input_tokens: session.total_input_tokens,
        total_output_tokens: session.total_output_tokens,
        total_cache_read: session.total_cache_read,
        total_cache_create: session.total_cache_create,
        turn_count: session.turn_count,
        current_tasks: session.current_tasks.clone(),
        mem_mb: session.mem_mb,
        git_branch: session.git_branch.clone(),
        git_added: session.git_added,
        git_modified: session.git_modified,
        token_history: session.token_history.clone(),
        context_history: session.context_history.clone(),
        compaction_count: session.compaction_count,
        children: session
            .children
            .iter()
            .map(|c| ChildProcessInfo {
                pid: c.pid,
                command: c.command.clone(),
                mem_kb: c.mem_kb,
                port: c.port,
            })
            .collect(),
        initial_prompt: session.initial_prompt.clone(),
        first_assistant_text: session.first_assistant_text.clone(),
        token_rate,
        token_rate_1m,
        token_rate_total,
        pid: session.pid,
        version: session.version.clone(),
        effort: session.effort.clone(),
        tool_calls: session.tool_calls.iter().map(|tc| SerToolCall {
            name: tc.name.clone(),
            arg: tc.arg.clone(),
            duration_ms: tc.duration_ms,
        }).collect(),
        subagents: session.subagents.iter().map(|sa| SerSubAgent {
            name: sa.name.clone(),
            status: sa.status.clone(),
            tokens: sa.tokens,
        }).collect(),
        file_accesses: session.file_accesses.iter().map(|fa| SerFileAccess {
            path: fa.path.clone(),
            operation: fa.operation.to_string(),
            turn_index: fa.turn_index,
        }).collect(),
        pending_since_ms: session.pending_since_ms,
        thinking_since_ms: session.thinking_since_ms,
    }
}

/// 构建 AgentUpdatePayload
fn build_payload(
    sessions: Vec<AgentSession>,
    registered_paths: &Arc<Mutex<Vec<String>>>,
    last_tokens: &Arc<Mutex<HashMap<String, (u64, Instant)>>>,
    token_history_samples: &Arc<Mutex<HashMap<String, Vec<(Instant, u64)>>>>,
) -> AgentUpdatePayload {
    let paths = registered_paths.lock().unwrap().clone();
    let timestamp_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    let mut projects_map: HashMap<String, Vec<AgentInfo>> = HashMap::new();
    let mut unmapped: Vec<AgentInfo> = Vec::new();

    for session in &sessions {
        let info = session_to_info(session, last_tokens, token_history_samples);

        // 按 cwd 前缀匹配注册项目
        let matched_project = paths.iter().find(|p| {
            let session_cwd = session.cwd.trim_end_matches('/');
            let project_path = p.trim_end_matches('/');
            session_cwd == project_path || session_cwd.starts_with(&format!("{}/", project_path))
        });

        if let Some(project_path) = matched_project {
            projects_map
                .entry(project_path.clone())
                .or_default()
                .push(info);
        } else {
            unmapped.push(info);
        }
    }

    // 确保所有注册项目都出现在结果中（即使没有 agent）
    let mut projects: Vec<ProjectAgents> = paths
        .iter()
        .map(|p| {
            let agents = projects_map.remove(p).unwrap_or_default();
            let count = agents.len();
            ProjectAgents {
                project_path: p.clone(),
                agents,
                count,
            }
        })
        .collect();

    // 补充未在 paths 中但仍然有数据的 project（来自路径的父目录匹配等边缘情况）
    for (path, agents) in projects_map {
        let count = agents.len();
        projects.push(ProjectAgents {
            project_path: path,
            agents,
            count,
        });
    }

    let total_sessions = sessions.len();

    // 清理已结束 session 的历史采样数据
    let active_session_ids: std::collections::HashSet<String> = sessions
        .iter()
        .map(|s| s.session_id.clone())
        .collect();
    {
        let mut history = token_history_samples.lock().unwrap();
        history.retain(|session_id, _| active_session_ids.contains(session_id));
    }

    AgentUpdatePayload {
        projects,
        unmapped,
        timestamp_ms,
        total_sessions,
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_collector_new() {
        let collector = AgentCollector::new();
        assert!(!collector.is_running());
        assert!(collector.registered_projects().is_empty());
    }

    #[test]
    fn test_register_unregister_project() {
        let collector = AgentCollector::new();
        collector.register_project("/home/user/project-a".to_string());
        collector.register_project("/home/user/project-b".to_string());
        collector.register_project("/home/user/project-a".to_string()); // duplicate

        let paths = collector.registered_projects();
        assert_eq!(paths.len(), 2);
        assert!(paths.contains(&"/home/user/project-a".to_string()));
        assert!(paths.contains(&"/home/user/project-b".to_string()));

        collector.unregister_project("/home/user/project-a");
        let paths = collector.registered_projects();
        assert_eq!(paths.len(), 1);
        assert!(!paths.contains(&"/home/user/project-a".to_string()));
    }

    #[test]
    fn test_serializable_status_from() {
        assert!(matches!(
            SerializableStatus::from(&SessionStatus::Thinking),
            SerializableStatus::Thinking
        ));
        assert!(matches!(
            SerializableStatus::from(&SessionStatus::Executing),
            SerializableStatus::Executing
        ));
        assert!(matches!(
            SerializableStatus::from(&SessionStatus::Waiting),
            SerializableStatus::Waiting
        ));
        assert!(matches!(
            SerializableStatus::from(&SessionStatus::RateLimited),
            SerializableStatus::RateLimited
        ));
        assert!(matches!(
            SerializableStatus::from(&SessionStatus::Done),
            SerializableStatus::Done
        ));
    }

    #[test]
    fn test_session_to_info_basic() {
        let last_tokens = Arc::new(Mutex::new(HashMap::new()));
        let token_history_samples = Arc::new(Mutex::new(HashMap::new()));
        let session = AgentSession {
            agent_cli: "claude",
            pid: 1234,
            session_id: "test-session-1".to_string(),
            cwd: "/home/user/project-a".to_string(),
            project_name: "project-a".to_string(),
            started_at: 0,
            status: SessionStatus::Thinking,
            model: "claude-sonnet-4".to_string(),
            effort: "medium".to_string(),
            context_percent: 45.5,
            total_input_tokens: 1000,
            total_output_tokens: 500,
            total_cache_read: 200,
            total_cache_create: 50,
            turn_count: 3,
            current_tasks: vec!["reading file".to_string()],
            mem_mb: 128,
            version: "1.0.0".to_string(),
            git_branch: "main".to_string(),
            git_added: 2,
            git_modified: 5,
            token_history: vec![100, 300, 1550],
            context_history: vec![100, 200, 1000],
            compaction_count: 0,
            context_window: 200_000,
            subagents: vec![],
            mem_file_count: 10,
            mem_line_count: 500,
            children: vec![abtop_collector::model::ChildProcess {
                pid: 1235,
                command: "node".to_string(),
                mem_kb: 64_000,
                port: Some(3000),
            }],
            initial_prompt: "Help me refactor".to_string(),
            first_assistant_text: "Sure!".to_string(),
            tool_calls: vec![],
            pending_since_ms: 0,
            thinking_since_ms: 0,
            file_accesses: vec![],
        };

        let info = session_to_info(&session, &last_tokens, &token_history_samples);
        assert_eq!(info.agent_type, "claude");
        assert_eq!(info.session_id, "test-session-1");
        assert_eq!(info.pid, 1234);
        assert_eq!(info.status, SerializableStatus::Thinking);
        assert_eq!(info.token_rate, 0.0); // 首次采集，无历史记录
        assert_eq!(info.token_rate_1m, 0.0);
        assert_eq!(info.token_rate_total, 0.0);
        assert_eq!(info.children.len(), 1);
        assert_eq!(info.children[0].port, Some(3000));
        // 新增字段验证
        assert!(info.tool_calls.is_empty());
        assert!(info.subagents.is_empty());
        assert!(info.file_accesses.is_empty());
        assert_eq!(info.pending_since_ms, 0);
        assert_eq!(info.thinking_since_ms, 0);
    }

    #[test]
    fn test_build_payload_mapping() {
        let last_tokens = Arc::new(Mutex::new(HashMap::new()));
        let token_history_samples = Arc::new(Mutex::new(HashMap::new()));
        let registered_paths = Arc::new(Mutex::new(vec![
            "/home/user/project-a".to_string(),
            "/home/user/project-b".to_string(),
        ]));

        let sessions = vec![
            AgentSession {
                agent_cli: "claude",
                pid: 1,
                session_id: "s1".to_string(),
                cwd: "/home/user/project-a/src".to_string(),
                project_name: "project-a".to_string(),
                started_at: 0,
                status: SessionStatus::Thinking,
                model: "claude".to_string(),
                effort: "".to_string(),
                context_percent: 0.0,
                total_input_tokens: 100,
                total_output_tokens: 50,
                total_cache_read: 0,
                total_cache_create: 0,
                turn_count: 1,
                current_tasks: vec![],
                mem_mb: 64,
                version: "".to_string(),
                git_branch: "".to_string(),
                git_added: 0,
                git_modified: 0,
                token_history: vec![],
                context_history: vec![],
                compaction_count: 0,
                context_window: 0,
                subagents: vec![],
                mem_file_count: 0,
                mem_line_count: 0,
                children: vec![],
                initial_prompt: "".to_string(),
                first_assistant_text: "".to_string(),
                tool_calls: vec![],
                pending_since_ms: 0,
                thinking_since_ms: 0,
                file_accesses: vec![],
            },
            AgentSession {
                agent_cli: "codex",
                pid: 2,
                session_id: "s2".to_string(),
                cwd: "/home/user/project-b".to_string(),
                project_name: "project-b".to_string(),
                started_at: 0,
                status: SessionStatus::Waiting,
                model: "codex".to_string(),
                effort: "".to_string(),
                context_percent: 0.0,
                total_input_tokens: 200,
                total_output_tokens: 100,
                total_cache_read: 0,
                total_cache_create: 0,
                turn_count: 2,
                current_tasks: vec![],
                mem_mb: 128,
                version: "".to_string(),
                git_branch: "".to_string(),
                git_added: 0,
                git_modified: 0,
                token_history: vec![],
                context_history: vec![],
                compaction_count: 0,
                context_window: 0,
                subagents: vec![],
                mem_file_count: 0,
                mem_line_count: 0,
                children: vec![],
                initial_prompt: "".to_string(),
                first_assistant_text: "".to_string(),
                tool_calls: vec![],
                pending_since_ms: 0,
                thinking_since_ms: 0,
                file_accesses: vec![],
            },
            AgentSession {
                agent_cli: "claude",
                pid: 3,
                session_id: "s3".to_string(),
                cwd: "/home/user/orphan".to_string(),
                project_name: "orphan".to_string(),
                started_at: 0,
                status: SessionStatus::Executing,
                model: "claude".to_string(),
                effort: "".to_string(),
                context_percent: 0.0,
                total_input_tokens: 50,
                total_output_tokens: 25,
                total_cache_read: 0,
                total_cache_create: 0,
                turn_count: 1,
                current_tasks: vec![],
                mem_mb: 32,
                version: "".to_string(),
                git_branch: "".to_string(),
                git_added: 0,
                git_modified: 0,
                token_history: vec![],
                context_history: vec![],
                compaction_count: 0,
                context_window: 0,
                subagents: vec![],
                mem_file_count: 0,
                mem_line_count: 0,
                children: vec![],
                initial_prompt: "".to_string(),
                first_assistant_text: "".to_string(),
                tool_calls: vec![],
                pending_since_ms: 0,
                thinking_since_ms: 0,
                file_accesses: vec![],
            },
        ];

        let payload = build_payload(sessions, &registered_paths, &last_tokens, &token_history_samples);

        assert_eq!(payload.total_sessions, 3);
        assert_eq!(payload.projects.len(), 2);
        assert_eq!(payload.unmapped.len(), 1);

        // project-a 应该包含 s1
        let proj_a = payload
            .projects
            .iter()
            .find(|p| p.project_path == "/home/user/project-a")
            .expect("project-a should exist");
        assert_eq!(proj_a.count, 1);
        assert_eq!(proj_a.agents[0].session_id, "s1");

        // project-b 应该包含 s2
        let proj_b = payload
            .projects
            .iter()
            .find(|p| p.project_path == "/home/user/project-b")
            .expect("project-b should exist");
        assert_eq!(proj_b.count, 1);
        assert_eq!(proj_b.agents[0].session_id, "s2");

        // orphan 应该在 unmapped 中
        assert_eq!(payload.unmapped[0].session_id, "s3");
    }

    #[test]
    fn test_build_payload_empty_projects() {
        let last_tokens = Arc::new(Mutex::new(HashMap::new()));
        let token_history_samples = Arc::new(Mutex::new(HashMap::new()));
        let registered_paths = Arc::new(Mutex::new(vec![
            "/home/user/project-a".to_string(),
        ]));

        let sessions: Vec<AgentSession> = vec![];
        let payload = build_payload(sessions, &registered_paths, &last_tokens, &token_history_samples);

        assert_eq!(payload.total_sessions, 0);
        assert_eq!(payload.projects.len(), 1);
        assert_eq!(payload.projects[0].count, 0);
        assert!(payload.unmapped.is_empty());
    }

    #[test]
    fn test_session_to_info_with_tool_calls() {
        let last_tokens = Arc::new(Mutex::new(HashMap::new()));
        let token_history_samples = Arc::new(Mutex::new(HashMap::new()));
        let session = AgentSession {
            agent_cli: "claude",
            pid: 1,
            session_id: "s1".to_string(),
            cwd: "/tmp".to_string(),
            project_name: "test".to_string(),
            started_at: 0,
            status: SessionStatus::Executing,
            model: "claude".to_string(),
            effort: "".to_string(),
            context_percent: 0.0,
            total_input_tokens: 0,
            total_output_tokens: 0,
            total_cache_read: 0,
            total_cache_create: 0,
            turn_count: 1,
            current_tasks: vec![],
            mem_mb: 0,
            version: "".to_string(),
            git_branch: "".to_string(),
            git_added: 0,
            git_modified: 0,
            token_history: vec![],
            context_history: vec![],
            compaction_count: 0,
            context_window: 0,
            subagents: vec![],
            mem_file_count: 0,
            mem_line_count: 0,
            children: vec![],
            initial_prompt: "".to_string(),
            first_assistant_text: "".to_string(),
            tool_calls: vec![
                abtop_collector::model::ToolCall {
                    name: "Read".to_string(),
                    arg: "src/main.rs".to_string(),
                    duration_ms: 1500,
                },
                abtop_collector::model::ToolCall {
                    name: "Bash".to_string(),
                    arg: "cargo build".to_string(),
                    duration_ms: 3200,
                },
            ],
            pending_since_ms: 0,
            thinking_since_ms: 0,
            file_accesses: vec![],
        };

        let info = session_to_info(&session, &last_tokens, &token_history_samples);
        assert_eq!(info.tool_calls.len(), 2);
        assert_eq!(info.tool_calls[0].name, "Read");
        assert_eq!(info.tool_calls[0].arg, "src/main.rs");
        assert_eq!(info.tool_calls[0].duration_ms, 1500);
        assert_eq!(info.tool_calls[1].name, "Bash");
        assert_eq!(info.tool_calls[1].duration_ms, 3200);
    }

    #[test]
    fn test_session_to_info_with_subagents() {
        let last_tokens = Arc::new(Mutex::new(HashMap::new()));
        let token_history_samples = Arc::new(Mutex::new(HashMap::new()));
        let session = AgentSession {
            agent_cli: "claude",
            pid: 1,
            session_id: "s1".to_string(),
            cwd: "/tmp".to_string(),
            project_name: "test".to_string(),
            started_at: 0,
            status: SessionStatus::Executing,
            model: "claude".to_string(),
            effort: "".to_string(),
            context_percent: 0.0,
            total_input_tokens: 0,
            total_output_tokens: 0,
            total_cache_read: 0,
            total_cache_create: 0,
            turn_count: 1,
            current_tasks: vec![],
            mem_mb: 0,
            version: "".to_string(),
            git_branch: "".to_string(),
            git_added: 0,
            git_modified: 0,
            token_history: vec![],
            context_history: vec![],
            compaction_count: 0,
            context_window: 0,
            subagents: vec![
                abtop_collector::model::SubAgent {
                    name: "build".to_string(),
                    status: "running".to_string(),
                    tokens: 5000,
                },
                abtop_collector::model::SubAgent {
                    name: "oracle".to_string(),
                    status: "done".to_string(),
                    tokens: 1200,
                },
            ],
            mem_file_count: 0,
            mem_line_count: 0,
            children: vec![],
            initial_prompt: "".to_string(),
            first_assistant_text: "".to_string(),
            tool_calls: vec![],
            pending_since_ms: 0,
            thinking_since_ms: 0,
            file_accesses: vec![],
        };

        let info = session_to_info(&session, &last_tokens, &token_history_samples);
        assert_eq!(info.subagents.len(), 2);
        assert_eq!(info.subagents[0].name, "build");
        assert_eq!(info.subagents[0].status, "running");
        assert_eq!(info.subagents[0].tokens, 5000);
        assert_eq!(info.subagents[1].name, "oracle");
        assert_eq!(info.subagents[1].tokens, 1200);
    }

    #[test]
    fn test_session_to_info_with_file_accesses() {
        let last_tokens = Arc::new(Mutex::new(HashMap::new()));
        let token_history_samples = Arc::new(Mutex::new(HashMap::new()));
        let session = AgentSession {
            agent_cli: "claude",
            pid: 1,
            session_id: "s1".to_string(),
            cwd: "/tmp".to_string(),
            project_name: "test".to_string(),
            started_at: 0,
            status: SessionStatus::Executing,
            model: "claude".to_string(),
            effort: "".to_string(),
            context_percent: 0.0,
            total_input_tokens: 0,
            total_output_tokens: 0,
            total_cache_read: 0,
            total_cache_create: 0,
            turn_count: 1,
            current_tasks: vec![],
            mem_mb: 0,
            version: "".to_string(),
            git_branch: "".to_string(),
            git_added: 0,
            git_modified: 0,
            token_history: vec![],
            context_history: vec![],
            compaction_count: 0,
            context_window: 0,
            subagents: vec![],
            mem_file_count: 0,
            mem_line_count: 0,
            children: vec![],
            initial_prompt: "".to_string(),
            first_assistant_text: "".to_string(),
            tool_calls: vec![],
            pending_since_ms: 0,
            thinking_since_ms: 0,
            file_accesses: vec![
                abtop_collector::model::FileAccess {
                    path: "src/main.rs".to_string(),
                    operation: abtop_collector::model::FileOp::Read,
                    turn_index: 0,
                },
                abtop_collector::model::FileAccess {
                    path: "src/lib.rs".to_string(),
                    operation: abtop_collector::model::FileOp::Write,
                    turn_index: 1,
                },
                abtop_collector::model::FileAccess {
                    path: "Cargo.toml".to_string(),
                    operation: abtop_collector::model::FileOp::Edit,
                    turn_index: 1,
                },
            ],
        };

        let info = session_to_info(&session, &last_tokens, &token_history_samples);
        assert_eq!(info.file_accesses.len(), 3);
        assert_eq!(info.file_accesses[0].path, "src/main.rs");
        assert_eq!(info.file_accesses[0].operation, "R");
        assert_eq!(info.file_accesses[0].turn_index, 0);
        assert_eq!(info.file_accesses[1].operation, "W");
        assert_eq!(info.file_accesses[2].operation, "E");
    }

    #[test]
    fn test_session_to_info_empty_fields() {
        let last_tokens = Arc::new(Mutex::new(HashMap::new()));
        let token_history_samples = Arc::new(Mutex::new(HashMap::new()));
        let session = AgentSession {
            agent_cli: "claude",
            pid: 1,
            session_id: "s1".to_string(),
            cwd: "/tmp".to_string(),
            project_name: "test".to_string(),
            started_at: 0,
            status: SessionStatus::Waiting,
            model: "claude".to_string(),
            effort: "".to_string(),
            context_percent: 0.0,
            total_input_tokens: 0,
            total_output_tokens: 0,
            total_cache_read: 0,
            total_cache_create: 0,
            turn_count: 0,
            current_tasks: vec![],
            mem_mb: 0,
            version: "".to_string(),
            git_branch: "".to_string(),
            git_added: 0,
            git_modified: 0,
            token_history: vec![],
            context_history: vec![],
            compaction_count: 0,
            context_window: 0,
            subagents: vec![],
            mem_file_count: 0,
            mem_line_count: 0,
            children: vec![],
            initial_prompt: "".to_string(),
            first_assistant_text: "".to_string(),
            tool_calls: vec![],
            pending_since_ms: 0,
            thinking_since_ms: 0,
            file_accesses: vec![],
        };

        let info = session_to_info(&session, &last_tokens, &token_history_samples);
        assert!(info.tool_calls.is_empty());
        assert!(info.subagents.is_empty());
        assert!(info.file_accesses.is_empty());
        assert_eq!(info.pending_since_ms, 0);
        assert_eq!(info.thinking_since_ms, 0);
    }
}
