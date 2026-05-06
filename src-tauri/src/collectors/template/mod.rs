use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};
use std::{fs, thread};

use crate::watcher::FileWatcher;

pub use config::{parse_parameters_py, ConfigCollector, ParameterError, ProjectConfig};
pub use git::{GitCollector, GitError, GitStatus};
pub use memory::{MemoryCollector, MemoryEntry, MemoryError};
pub use stage::{Stage, StageCollector, StageError};

pub mod config;
pub mod git;
pub mod memory;
pub mod stage;

// ============================================================================
// SourceLayout — 源码布局类型
// ============================================================================

/// 模板项目的源码布局类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceLayout {
    /// 扁平布局：`src/python_model/L*/`
    Flat,
    /// 命名空间布局：`src/<module_name>/python_model/L*/`
    Namespaced(String),
    /// 无法识别的布局
    Unknown,
}

impl SourceLayout {
    /// 检测指定路径的源码布局
    pub fn detect(path: &Path) -> Self {
        // 检查扁平布局
        let flat_path = path.join("src").join("python_model");
        if flat_path.is_dir() {
            return SourceLayout::Flat;
        }

        // 检查命名空间布局：src/*/python_model/
        let src_dir = path.join("src");
        if let Ok(entries) = fs::read_dir(&src_dir) {
            for entry in entries.flatten() {
                let entry_path = entry.path();
                if entry_path.is_dir() {
                    let python_model = entry_path.join("python_model");
                    if python_model.is_dir() {
                        let module_name = entry_path
                            .file_name()
                            .map(|s| s.to_string_lossy().to_string())
                            .unwrap_or_default();
                        return SourceLayout::Namespaced(module_name);
                    }
                }
            }
        }

        SourceLayout::Unknown
    }
}

// ============================================================================
// TemplateData — 统一采集结果
// ============================================================================

/// 模板项目的完整采集数据
#[derive(Debug, Clone)]
pub struct TemplateData {
    /// 当前阶段
    pub stage: Result<Stage, StageError>,
    /// 项目配置
    pub config: Result<ProjectConfig, ParameterError>,
    /// 记忆条目列表
    pub memories: Result<Vec<MemoryEntry>, MemoryError>,
    /// Git 状态
    pub git: Result<GitStatus, GitError>,
    /// 源码布局
    pub layout: SourceLayout,
}

impl TemplateData {
    /// 创建包含所有空/默认值的 TemplateData
    pub fn empty(path: &Path) -> Self {
        Self {
            stage: Err(StageError::FileNotFound(
                path.join(".current_stage").to_string_lossy().to_string(),
            )),
            config: Err(ParameterError::FileNotFound(
                path.join("config/parameters.py").to_string_lossy().to_string(),
            )),
            memories: Ok(Vec::new()),
            git: Ok(GitStatus::no_repo()),
            layout: SourceLayout::Unknown,
        }
    }

    /// 返回是否所有采集器都成功
    pub fn is_complete(&self) -> bool {
        self.stage.is_ok()
            && self.config.is_ok()
            && self.memories.is_ok()
            && self.git.is_ok()
    }
}

// ============================================================================
// TemplateDataCollector — 编排层
// ============================================================================

/// 模板项目数据统一采集器
///
/// 协调 [`StageCollector`]、[`ConfigCollector`]、[`MemoryCollector`]、[`GitCollector`]
/// 四个采集器，提供单次采集和持续监听两种模式。
pub struct TemplateDataCollector {
    project_path: PathBuf,
}

impl TemplateDataCollector {
    /// 创建新的采集器实例
    pub fn new(project_path: PathBuf) -> Self {
        Self { project_path }
    }

    /// 执行一次性完整采集
    pub fn collect(&self) -> TemplateData {
        let layout = SourceLayout::detect(&self.project_path);

        TemplateData {
            stage: StageCollector::collect(&self.project_path),
            config: ConfigCollector::collect(&self.project_path),
            memories: MemoryCollector::collect(&self.project_path),
            git: GitCollector::collect(&self.project_path),
            layout,
        }
    }

    /// 获取项目路径
    pub fn path(&self) -> &Path {
        &self.project_path
    }
}

// ============================================================================
// WatchedCollector — 带文件监听的持续采集
// ============================================================================

/// 使用 [`FileWatcher`] 监听项目文件变化，自动触发重新采集
///
/// 监听路径：
/// - `.current_stage` — 阶段变化
/// - `config/parameters.py` — 配置变化
/// - `.claude/memory/` — 记忆文件变化
///
/// 变化后 5 秒内触发重新采集（防抖）。
pub struct WatchedCollector {
    collector: TemplateDataCollector,
    debounce_ms: u64,
}

/// 采集更新事件
#[derive(Debug, Clone)]
pub struct CollectEvent {
    pub data: TemplateData,
    pub triggered_by: Vec<PathBuf>,
    pub timestamp: Instant,
}

impl WatchedCollector {
    /// 创建带监听功能的采集器
    pub fn new(project_path: PathBuf) -> Self {
        Self {
            collector: TemplateDataCollector::new(project_path),
            debounce_ms: 5000,
        }
    }

    /// 设置防抖间隔（毫秒），默认 5000ms
    pub fn with_debounce(mut self, ms: u64) -> Self {
        self.debounce_ms = ms;
        self
    }

    /// 启动监听并持续采集
    ///
    /// 返回一个接收器，每次采集完成后发送 [`CollectEvent`]。
    /// 调用方可以通过返回的停止信号终止监听。
    pub fn start(self) -> (mpsc::Receiver<CollectEvent>, Arc<AtomicBool>) {
        let (tx, rx) = mpsc::channel();
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = running.clone();

        let collector = self.collector;
        let debounce = Duration::from_millis(self.debounce_ms);

        // 执行初始采集
        let initial_data = collector.collect();
        let _ = tx.send(CollectEvent {
            triggered_by: vec![collector.path().to_path_buf()],
            timestamp: Instant::now(),
            data: initial_data,
        });

        // 设置文件监听
        let path = collector.path().to_path_buf();
        thread::spawn(move || {
            let mut watcher = FileWatcher::with_interval(Duration::from_millis(500));
            watcher.add(path.join(".current_stage"), false);
            watcher.add(path.join("config").join("parameters.py"), false);
            watcher.add(path.join(".claude").join("memory"), true);

            let pending: Arc<Mutex<Option<(Instant, Vec<PathBuf>)>>> =
                Arc::new(Mutex::new(None));
            let pending_for_callback = pending.clone();
            let tx_for_callback = tx.clone();
            let path_for_callback = path.clone();

            let handles = watcher
                .start(move |event| {
                    let changed_path = event.path().to_path_buf();
                    let mut guard = pending_for_callback.lock().unwrap();
                    match guard.as_mut() {
                        Some((_, paths)) => {
                            if !paths.contains(&changed_path) {
                                paths.push(changed_path);
                            }
                        }
                        None => {
                            *guard = Some((Instant::now(), vec![changed_path]));
                        }
                    }
                })
                .expect("Failed to start file watcher");

            // 防抖检查循环
            while running_clone.load(Ordering::SeqCst) {
                thread::sleep(Duration::from_millis(100));

                let should_collect = {
                    let mut guard = pending.lock().unwrap();
                    if let Some((time, _)) = guard.as_ref() {
                        if time.elapsed() >= debounce {
                            let paths = guard.take().unwrap().1;
                            Some(paths)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                };

                if let Some(triggered_by) = should_collect {
                    let data = TemplateDataCollector::new(path_for_callback.clone()).collect();
                    let _ = tx_for_callback.send(CollectEvent {
                        data,
                        triggered_by,
                        timestamp: Instant::now(),
                    });
                }
            }

            handles.stop_and_join().ok();
        });

        (rx, running)
    }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_source_layout_flat() {
        let dir = tempfile::tempdir().unwrap();
        let python_model = dir.path().join("src").join("python_model");
        fs::create_dir_all(&python_model).unwrap();

        assert_eq!(SourceLayout::detect(dir.path()), SourceLayout::Flat);
    }

    #[test]
    fn test_source_layout_namespaced() {
        let dir = tempfile::tempdir().unwrap();
        let python_model = dir.path().join("src").join("my_module").join("python_model");
        fs::create_dir_all(&python_model).unwrap();

        assert_eq!(
            SourceLayout::detect(dir.path()),
            SourceLayout::Namespaced("my_module".to_string())
        );
    }

    #[test]
    fn test_source_layout_unknown() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("src")).unwrap();

        assert_eq!(SourceLayout::detect(dir.path()), SourceLayout::Unknown);
    }

    #[test]
    fn test_template_data_collector_empty_project() {
        let dir = tempfile::tempdir().unwrap();
        let collector = TemplateDataCollector::new(dir.path().to_path_buf());
        let data = collector.collect();

        assert!(data.stage.is_err());
        assert!(data.config.is_err());
        assert!(data.memories.is_ok());
        assert!(data.git.is_ok());
        assert_eq!(data.layout, SourceLayout::Unknown);
        assert!(!data.is_complete());
    }

    #[test]
    fn test_template_data_collector_with_stage() {
        let dir = tempfile::tempdir().unwrap();

        // 创建 .current_stage
        let stage_file = dir.path().join(".current_stage");
        let mut file = fs::File::create(&stage_file).unwrap();
        writeln!(file, "l3").unwrap();
        drop(file);

        let collector = TemplateDataCollector::new(dir.path().to_path_buf());
        let data = collector.collect();

        assert_eq!(data.stage.unwrap(), Stage::L3);
    }

    #[test]
    fn test_template_data_collector_with_flat_layout() {
        let dir = tempfile::tempdir().unwrap();

        fs::create_dir_all(dir.path().join("src").join("python_model").join("L1_prototype")).unwrap();
        fs::create_dir_all(dir.path().join("src").join("verilog_model")).unwrap();

        let stage_file = dir.path().join(".current_stage");
        let mut file = fs::File::create(&stage_file).unwrap();
        writeln!(file, "l1").unwrap();
        drop(file);

        let collector = TemplateDataCollector::new(dir.path().to_path_buf());
        let data = collector.collect();

        assert_eq!(data.layout, SourceLayout::Flat);
    }

    #[test]
    fn test_watched_collector_start_stop() {
        let dir = tempfile::tempdir().unwrap();

        // 创建初始文件
        let stage_file = dir.path().join(".current_stage");
        let mut file = fs::File::create(&stage_file).unwrap();
        writeln!(file, "l0").unwrap();
        drop(file);

        // 确保文件系统时间戳更新（macOS 需要）
        thread::sleep(Duration::from_millis(100));

        let watched = WatchedCollector::new(dir.path().to_path_buf()).with_debounce(300);
        let (rx, stop_signal) = watched.start();

        // 等待初始采集
        let event = rx.recv_timeout(Duration::from_secs(3)).unwrap();
        assert_eq!(event.data.stage.unwrap(), Stage::L0);

        // 给 FileWatcher 足够时间完成初始快照
        thread::sleep(Duration::from_millis(600));

        // 修改文件触发重新采集
        let mut file = fs::File::create(&stage_file).unwrap();
        writeln!(file, "l2").unwrap();
        drop(file);

        // 等待防抖后的事件（考虑 500ms 轮询 + 300ms 防抖 + 缓冲）
        let event = rx.recv_timeout(Duration::from_secs(10)).unwrap();
        assert_eq!(event.data.stage.unwrap(), Stage::L2);
        assert!(!event.triggered_by.is_empty());

        // 停止监听
        stop_signal.store(false, Ordering::SeqCst);
    }
}
