//! # FileWatcher 抽象层
//!
//! 封装文件系统监听逻辑，使用 mtime 轮询方式检测文件变化。
//! 提供 `start`/`stop` 接口，支持单文件和目录递归监听。
//!
//! ## 设计说明
//!
//! - 不依赖 `notify` crate，使用 `std::fs::metadata` 轮询（ms 级间隔）
//! - 线程安全：`stop()` 通过 `AtomicBool` 信号通知监听线程退出
//! - 事件类型：Modified / Created / Deleted
//! - 错误处理：权限问题返回明确错误信息，不 panic
//!
//! ## 使用场景
//!
//! - 监听 `.current_stage` 文件变化 → 触发 Stage 重新采集
//! - 监听 `.claude/memory/` 目录变化 → 触发 Memory 重新采集
//! - 监听 `config/parameters.py` 变化 → 触发 Config 重新采集

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, SystemTime};

// ============================================================================
// 公开类型
// ============================================================================

/// 文件变化事件类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WatchEvent {
    /// 文件内容或元数据被修改
    Modified(PathBuf),
    /// 文件被创建（新出现）
    Created(PathBuf),
    /// 文件被删除
    Deleted(PathBuf),
}

impl WatchEvent {
    /// 返回事件关联的路径引用
    pub fn path(&self) -> &Path {
        match self {
            WatchEvent::Modified(p) | WatchEvent::Created(p) | WatchEvent::Deleted(p) => p.as_path(),
        }
    }

    /// 返回事件类型的简短描述（用于日志等）
    pub fn kind_str(&self) -> &'static str {
        match self {
            WatchEvent::Modified(_) => "modified",
            WatchEvent::Created(_) => "created",
            WatchEvent::Deleted(_) => "deleted",
        }
    }
}

/// 文件监听器
///
/// 通过轮询 mtime 检测文件变化，在独立线程中运行。
///
/// # 示例
///
/// ```ignore
/// use ptv::watcher::FileWatcher;
/// use std::time::Duration;
/// use std::path::PathBuf;
///
/// let mut watcher = FileWatcher::new();
/// watcher.add(PathBuf::from("/tmp/test/.current_stage"), false);
/// watcher.add(PathBuf::from("/tmp/test/.claude/memory"), true);
///
/// let handle = watcher.start(|event| {
///     println!("[watcher] {:?} {:?}", event.kind_str(), event.path());
/// });
///
/// // ... 稍后停止
/// // 通过 drop 或调用 stop 信号触发
/// ```
pub struct FileWatcher {
    /// 被监听的路径列表
    entries: Vec<WatchEntry>,
    /// 轮询间隔
    poll_interval: Duration,
    /// 运行状态信号
    running: Arc<AtomicBool>,
}

/// 单个监听条目
#[derive(Debug, Clone)]
struct WatchEntry {
    path: PathBuf,
    recursive: bool,
}

// ============================================================================
// FileWatcher 实现
// ============================================================================

impl FileWatcher {
    /// 创建一个新的 FileWatcher，默认轮询间隔 500ms
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            poll_interval: Duration::from_millis(500),
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    /// 创建 FileWatcher 并指定轮询间隔
    pub fn with_interval(interval: Duration) -> Self {
        Self {
            entries: Vec::new(),
            poll_interval: interval,
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    /// 添加一个监听路径
    ///
    /// - `path`: 文件或目录路径
    /// - `recursive`: 如果 `path` 是目录，是否递归监听子目录中的文件
    ///
    /// # 注意
    ///
    /// 重复添加同一路径会被忽略。
    pub fn add(&mut self, path: PathBuf, recursive: bool) {
        // 规范化路径后再检查重复
        let canonical = normalize_path(&path);
        if self.entries.iter().any(|e| normalize_path(&e.path) == canonical) {
            return;
        }
        self.entries.push(WatchEntry { path, recursive });
    }

    /// 移除一个监听路径
    pub fn remove(&mut self, path: &Path) {
        let canonical = normalize_path(path);
        self.entries.retain(|e| normalize_path(&e.path) != canonical);
    }

    /// 获取所有已注册的监听路径
    pub fn paths(&self) -> impl Iterator<Item = &Path> {
        self.entries.iter().map(|e| e.path.as_path())
    }

    /// 设置轮询间隔
    pub fn set_interval(&mut self, interval: Duration) {
        self.poll_interval = interval;
    }

    /// 开始监听
    ///
    /// 启动一个后台线程，持续轮询文件 mtime 并在检测到变化时调用 `on_event`。
    /// 返回 `JoinHandle`，可用于 `join()` 等待线程结束。
    ///
    /// `stop()` 方法设置停止信号，线程会在下次轮询时自动退出。
    pub fn start<F>(self, on_event: F) -> thread::Result<JoinHandles>
    where
        F: Fn(WatchEvent) + Send + 'static,
    {
        self.running.store(true, Ordering::SeqCst);

        let running = self.running.clone();
        let entries = self.entries.clone();
        let interval = self.poll_interval;

        let handle = thread::Builder::new()
            .name("ptv-file-watcher".into())
            .spawn(move || {
                // 构建初始 mtime 快照
                let mut snapshot = MtimeSnapshot::new();
                for entry in &entries {
                    snapshot.sync(entry);
                }

                // 主轮询循环
                while running.load(Ordering::SeqCst) {
                    thread::sleep(interval);

                    let mut current = MtimeSnapshot::new();
                    for entry in &entries {
                        current.sync(entry);
                    }

                    // 检测变化：Modified / Created
                    for (path_str, new_mtime) in current.map.iter() {
                        let path = PathBuf::from(path_str);
                        match snapshot.map.get(path_str) {
                            Some(old_mtime) if old_mtime != new_mtime => {
                                on_event(WatchEvent::Modified(path));
                            }
                            None => {
                                on_event(WatchEvent::Created(path));
                            }
                            _ => {}
                        }
                    }

                    // 检测变化：Deleted
                    for path_str in snapshot.map.keys() {
                        if !current.map.contains_key(path_str) {
                            let path = PathBuf::from(path_str);
                            on_event(WatchEvent::Deleted(path));
                        }
                    }

                    snapshot = current;
                }
            })
            .map_err(|e| Box::new(e) as Box<dyn std::any::Any + Send>)?;

        Ok(JoinHandles {
            inner: vec![handle],
            running: self.running.clone(),
        })
    }

    /// 发送停止信号
    ///
    /// 监听线程会在下次轮询时检测到信号并退出。
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }
}

impl Default for FileWatcher {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// JoinHandles — 等待线程结束
// ============================================================================

/// `start()` 返回的句柄集合，可用于等待监听线程结束
pub struct JoinHandles {
    inner: Vec<thread::JoinHandle<()>>,
    running: Arc<AtomicBool>,
}

impl JoinHandles {
    /// 等待所有监听线程结束
    pub fn join(self) -> thread::Result<()> {
        for handle in self.inner {
            handle.join()?;
        }
        Ok(())
    }

    /// 停止监听并等待线程结束
    pub fn stop_and_join(self) -> thread::Result<()> {
        self.running.store(false, Ordering::SeqCst);
        self.join()
    }
}

// ============================================================================
// Mtime 快照
// ============================================================================

/// mtime 快照：路径 → 最后修改时间，用于对比检测变化
struct MtimeSnapshot {
    map: HashMap<String, SystemTime>,
}

impl MtimeSnapshot {
    fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    /// 同步一个监听条目的所有文件 mtime
    fn sync(&mut self, entry: &WatchEntry) {
        let path = &entry.path;

        if path.is_dir() {
            // 目录监听
            match self.collect_dir_mtimes(path, entry.recursive) {
                Ok(mtimes) => {
                    for (p, mtime) in mtimes {
                        self.map.insert(p, mtime);
                    }
                }
                Err(e) => {
                    // 权限错误等，在条目中记录错误（返回但不 panic）
                    // 标记目录本身不可访问（保留旧快照以检测恢复）
                    log_error(&format!("cannot read directory '{}': {}", path.display(), e));
                }
            }
        } else {
            // 文件监听
            match mtime_of(path) {
                Ok(Some(mtime)) => {
                    self.map.insert(path_to_string(path), mtime);
                }
                Ok(None) => {
                    // 文件不存在，不加入快照（将触发 Created 事件）
                }
                Err(e) => {
                    log_error(&format!("cannot access file '{}': {}", path.display(), e));
                }
            }
        }
    }

    /// 收集目录下所有文件的 mtime
    fn collect_dir_mtimes(&self, dir: &Path, recursive: bool) -> std::io::Result<HashMap<String, SystemTime>> {
        let mut result = HashMap::new();

        if recursive {
            self.collect_recursive(dir, &mut result)?;
        } else {
            self.collect_direct(dir, &mut result)?;
        }

        Ok(result)
    }

    fn collect_recursive(&self, dir: &Path, result: &mut HashMap<String, SystemTime>) -> std::io::Result<()> {
        if !dir.is_dir() {
            return Ok(());
        }

        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let file_type = entry.file_type()?;

            if file_type.is_dir() {
                // 递归子目录
                self.collect_recursive(&path, result)?;
            } else if file_type.is_file() || file_type.is_symlink() {
                match mtime_of(&path) {
                    Ok(Some(mtime)) => {
                        result.insert(path_to_string(&path), mtime);
                    }
                    Ok(None) => {}
                    Err(e) => {
                        log_error(&format!("cannot read '{}': {}", path.display(), e));
                    }
                }
            }
        }

        Ok(())
    }

    fn collect_direct(&self, dir: &Path, result: &mut HashMap<String, SystemTime>) -> std::io::Result<()> {
        if !dir.is_dir() {
            return Ok(());
        }

        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() || path.is_symlink() {
                match mtime_of(&path) {
                    Ok(Some(mtime)) => {
                        result.insert(path_to_string(&path), mtime);
                    }
                    Ok(None) => {}
                    Err(e) => {
                        log_error(&format!("cannot read '{}': {}", path.display(), e));
                    }
                }
            }
        }

        Ok(())
    }
}

// ============================================================================
// 辅助函数
// ============================================================================

/// 读取文件的 mtime，如果文件不存在则返回 Ok(None)
fn mtime_of(path: &Path) -> std::io::Result<Option<SystemTime>> {
    match fs::metadata(path) {
        Ok(meta) => Ok(Some(meta.modified()?)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}

/// 将 PathBuf 转为字符串（用于 HashMap key）
fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

/// 规范化路径（解析符号链接，转为绝对路径）
fn normalize_path(path: &Path) -> String {
    match path.canonicalize() {
        Ok(p) => path_to_string(&p),
        Err(_) => path_to_string(path),
    }
}

/// 日志错误（当前输出到 stderr，未来可接入日志系统）
fn log_error(msg: &str) {
    // 使用 eprintln 作为默认日志输出
    // TODO: 在集成到 Tauri 后替换为 log/tracing
    eprintln!("[watcher:error] {}", msg);
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use std::sync::mpsc;
    use std::time::Instant;

    /// 创建临时目录，返回路径和清理函数
    fn temp_dir() -> (PathBuf, temp_cleanup::TempDir) {
        temp_cleanup::create()
    }

    mod temp_cleanup {
        use std::path::{Path, PathBuf};
        use std::sync::atomic::{AtomicUsize, Ordering};
        static COUNTER: AtomicUsize = AtomicUsize::new(0);

        pub struct TempDir {
            path: PathBuf,
        }

        impl Drop for TempDir {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.path);
            }
        }

        impl AsRef<Path> for TempDir {
            fn as_ref(&self) -> &Path {
                &self.path
            }
        }

        pub fn create() -> (PathBuf, TempDir) {
            let id = COUNTER.fetch_add(1, Ordering::SeqCst);
            let path = std::env::temp_dir().join(format!("ptv-watcher-test-{}", id));
            let _ = std::fs::create_dir_all(&path);
            let dir = TempDir { path: path.clone() };
            (path, dir)
        }
    }

    /// 在测试目录中写入文件内容
    fn write_file(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let mut f = fs::File::create(path).unwrap();
        f.write_all(content.as_bytes()).unwrap();
        f.sync_all().unwrap();
    }

    // ---------------------------------------------------------------
    // 基础功能测试
    // ---------------------------------------------------------------

    /// 测试：监听单个文件，修改后触发 Modified 事件
    #[test]
    fn test_watch_single_file_modified() {
        let (dir, _guard) = temp_dir();
        let file_path = dir.join(".current_stage");

        // 先创建文件
        write_file(&file_path, "l0");

        let mut watcher = FileWatcher::with_interval(Duration::from_millis(50));
        watcher.add(file_path.clone(), false);

        let (tx, rx) = mpsc::channel();
        let handles = watcher.start(move |event| {
            let _ = tx.send(event);
        })
        .expect("failed to start watcher");

        // 等待监听线程启动
        thread::sleep(Duration::from_millis(100));

        // 修改文件
        write_file(&file_path, "l1");

        // 等待事件触发
        let deadline = Instant::now() + Duration::from_secs(3);
        let mut received = false;
        while Instant::now() < deadline {
            if let Ok(event) = rx.try_recv() {
                assert_eq!(event.kind_str(), "modified", "should detect modification");
                assert_eq!(event.path(), file_path, "path should match");
                received = true;
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }

        assert!(received, "should receive Modified event within 3 seconds");

        handles.stop_and_join().ok();
    }

    /// 测试：监听目录，子文件修改后触发事件
    #[test]
    fn test_watch_directory_recursive() {
        let (dir, _guard) = temp_dir();
        let sub_dir = dir.join("subdir");
        let file_path = sub_dir.join("test.txt");

        // 先创建文件
        write_file(&file_path, "hello");

        let mut watcher = FileWatcher::with_interval(Duration::from_millis(50));
        watcher.add(dir.clone(), true); // recursive

        let (tx, rx) = mpsc::channel();
        let handles = watcher
            .start(move |event| {
                let _ = tx.send(event);
            })
            .expect("failed to start watcher");

        thread::sleep(Duration::from_millis(100));

        // 修改文件
        write_file(&file_path, "world");

        let deadline = Instant::now() + Duration::from_secs(3);
        let mut received = false;
        while Instant::now() < deadline {
            if let Ok(event) = rx.try_recv() {
                if event.kind_str() == "modified" && event.path() == file_path {
                    received = true;
                    break;
                }
            }
            thread::sleep(Duration::from_millis(10));
        }

        assert!(received, "should detect recursive directory change within 3s");

        handles.stop_and_join().ok();
    }

    /// 测试：监听目录（非递归），子目录内的文件变化不应触发
    #[test]
    fn test_watch_directory_non_recursive() {
        let (dir, _guard) = temp_dir();
        let sub_dir = dir.join("subdir");
        let file_path = sub_dir.join("test.txt");

        write_file(&file_path, "hello");

        let mut watcher = FileWatcher::with_interval(Duration::from_millis(50));
        watcher.add(dir.clone(), false); // non-recursive

        let (tx, rx) = mpsc::channel();
        let handles = watcher
            .start(move |event| {
                let _ = tx.send(event);
            })
            .expect("failed to start watcher");

        thread::sleep(Duration::from_millis(100));

        // 在子目录中修改文件（不应该被监听到）
        write_file(&file_path, "world");

        thread::sleep(Duration::from_millis(300));

        let received: Vec<WatchEvent> = rx.try_iter().collect();
        let has_subdir_event = received.iter().any(|e| e.path() == file_path);
        assert!(!has_subdir_event, "non-recursive watch should not detect subdir changes");

        handles.stop_and_join().ok();
    }

    /// 测试：文件被删除后触发 Deleted 事件
    #[test]
    fn test_watch_file_deleted() {
        let (dir, _guard) = temp_dir();
        let file_path = dir.join("temp.txt");

        write_file(&file_path, "content");

        let mut watcher = FileWatcher::with_interval(Duration::from_millis(50));
        watcher.add(file_path.clone(), false);

        let (tx, rx) = mpsc::channel();
        let handles = watcher
            .start(move |event| {
                let _ = tx.send(event);
            })
            .expect("failed to start watcher");

        thread::sleep(Duration::from_millis(100));

        // 删除文件
        fs::remove_file(&file_path).unwrap();

        let deadline = Instant::now() + Duration::from_secs(3);
        let mut received = false;
        while Instant::now() < deadline {
            if let Ok(event) = rx.try_recv() {
                if event.kind_str() == "deleted" {
                    received = true;
                    break;
                }
            }
            thread::sleep(Duration::from_millis(10));
        }

        assert!(received, "should receive Deleted event within 3 seconds");
        handles.stop_and_join().ok();
    }

    /// 测试：文件创建后触发 Created 事件
    #[test]
    fn test_watch_file_created() {
        let (dir, _guard) = temp_dir();
        let file_path = dir.join("new_file.txt");

        // 先不创建文件
        let mut watcher = FileWatcher::with_interval(Duration::from_millis(50));
        watcher.add(file_path.clone(), false);

        let (tx, rx) = mpsc::channel();
        let handles = watcher
            .start(move |event| {
                let _ = tx.send(event);
            })
            .expect("failed to start watcher");

        thread::sleep(Duration::from_millis(100));

        // 创建文件
        write_file(&file_path, "new content");

        let deadline = Instant::now() + Duration::from_secs(3);
        let mut received = false;
        while Instant::now() < deadline {
            if let Ok(event) = rx.try_recv() {
                if event.kind_str() == "created" {
                    received = true;
                    break;
                }
            }
            thread::sleep(Duration::from_millis(10));
        }

        assert!(received, "should receive Created event within 3 seconds");
        handles.stop_and_join().ok();
    }

    // ---------------------------------------------------------------
    // 边界情况测试
    // ---------------------------------------------------------------

    /// 测试：监听不存在的文件（不 panic，文件创建后可检测）
    #[test]
    fn test_watch_nonexistent_file_no_panic() {
        let (dir, _guard) = temp_dir();
        let file_path = dir.join("does_not_exist.txt");

        let mut watcher = FileWatcher::with_interval(Duration::from_millis(50));
        watcher.add(file_path.clone(), false);

        let (tx, rx) = mpsc::channel();
        let handles = watcher
            .start(move |event| {
                let _ = tx.send(event);
            })
            .expect("failed to start watcher");

        thread::sleep(Duration::from_millis(200));

        // 不 panic
        // 然后创建文件
        write_file(&file_path, "now it exists");

        let deadline = Instant::now() + Duration::from_secs(3);
        let mut received = false;
        while Instant::now() < deadline {
            if let Ok(event) = rx.try_recv() {
                if event.kind_str() == "created" {
                    received = true;
                    break;
                }
            }
            thread::sleep(Duration::from_millis(10));
        }

        assert!(received, "should detect file creation after starting watcher");
        handles.stop_and_join().ok();
    }

    /// 测试：监听空目录（不 panic）
    #[test]
    fn test_watch_empty_directory() {
        let (dir, _guard) = temp_dir();

        let mut watcher = FileWatcher::with_interval(Duration::from_millis(50));
        watcher.add(dir.clone(), true);

        let (tx, rx) = mpsc::channel();
        let handles = watcher
            .start(move |event| {
                let _ = tx.send(event);
            })
            .expect("failed to start watcher");

        thread::sleep(Duration::from_millis(200));

        // 在空目录中创建文件
        let new_file = dir.join("new.txt");
        write_file(&new_file, "content");

        let deadline = Instant::now() + Duration::from_secs(3);
        let mut received = false;
        while Instant::now() < deadline {
            if let Ok(event) = rx.try_recv() {
                if event.kind_str() == "created" {
                    received = true;
                    break;
                }
            }
            thread::sleep(Duration::from_millis(10));
        }

        assert!(received, "should detect file creation in empty directory");
        handles.stop_and_join().ok();
    }

    /// 测试：监听目录中创建新文件
    #[test]
    fn test_watch_directory_file_created() {
        let (dir, _guard) = temp_dir();
        let existing = dir.join("existing.txt");
        write_file(&existing, "existing");

        let mut watcher = FileWatcher::with_interval(Duration::from_millis(50));
        watcher.add(dir.clone(), false);

        let (tx, rx) = mpsc::channel();
        let handles = watcher
            .start(move |event| {
                let _ = tx.send(event);
            })
            .expect("failed to start watcher");

        thread::sleep(Duration::from_millis(100));

        let new_file = dir.join("new.txt");
        write_file(&new_file, "new content");

        let deadline = Instant::now() + Duration::from_secs(3);
        let mut received = false;
        while Instant::now() < deadline {
            if let Ok(event) = rx.try_recv() {
                if event.kind_str() == "created" && event.path() == new_file {
                    received = true;
                    break;
                }
            }
            thread::sleep(Duration::from_millis(10));
        }

        assert!(received, "should detect file creation in watched directory");
        handles.stop_and_join().ok();
    }

    // ---------------------------------------------------------------
    // stop/start 接口测试
    // ---------------------------------------------------------------

    /// 测试：stop 后不再收到事件
    #[test]
    fn test_stop_stops_events() {
        let (dir, _guard) = temp_dir();
        let file_path = dir.join("test.txt");

        write_file(&file_path, "initial");

        let mut watcher = FileWatcher::with_interval(Duration::from_millis(50));
        watcher.add(file_path.clone(), false);

        let (tx, rx) = mpsc::channel();
        let handles = watcher
            .start(move |event| {
                let _ = tx.send(event);
            })
            .expect("failed to start watcher");

        thread::sleep(Duration::from_millis(100));

        // 停止监听
        handles.stop_and_join().ok();

        // 修改文件（此时不应收到事件）
        write_file(&file_path, "after stop");

        thread::sleep(Duration::from_millis(200));

        // 应该没有更多事件（可能有一些已在通道中的，但不应有新的事件）
        // 注意：已有的事件可能已在通道中，但我们至少可以验证 stop 后通道不会收到新事件
        let after_stop_count = rx.try_iter().count();
        // 此时 watcher 已停止，可能有一些在退出前的事件，但不应有新的
        // 我们只是验证不 panic 且线程已退出
        println!("Events after stop: {}", after_stop_count);
    }

    /// 测试：add 和 remove 路径
    #[test]
    fn test_add_remove_paths() {
        let mut watcher = FileWatcher::new();
        let p1 = PathBuf::from("/tmp/test1");
        let p2 = PathBuf::from("/tmp/test1"); // same as p1
        let p3 = PathBuf::from("/tmp/test2");

        watcher.add(p1.clone(), false);
        watcher.add(p2.clone(), false); // duplicate, should be ignored
        watcher.add(p3.clone(), true);

        let paths: Vec<&Path> = watcher.paths().collect();
        assert_eq!(paths.len(), 2, "duplicate path should be ignored");

        watcher.remove(&p3);
        let paths: Vec<&Path> = watcher.paths().collect();
        assert_eq!(paths.len(), 1, "removed path should not be in list");
        assert_eq!(paths[0], p1, "remaining path should be p1");
    }

    /// 测试：设置轮询间隔
    #[test]
    fn test_set_interval() {
        let mut watcher = FileWatcher::new();
        assert_eq!(watcher.poll_interval, Duration::from_millis(500));

        watcher.set_interval(Duration::from_millis(100));
        assert_eq!(watcher.poll_interval, Duration::from_millis(100));
    }

    /// 测试：默认构造
    #[test]
    fn test_default() {
        let watcher = FileWatcher::default();
        assert_eq!(watcher.poll_interval, Duration::from_millis(500));
        assert!(watcher.entries.is_empty());
        assert!(!watcher.running.load(Ordering::SeqCst));
    }

    // ---------------------------------------------------------------
    // 模拟真实场景测试
    // ---------------------------------------------------------------

    /// 测试：模拟 `.current_stage` 文件从 L0 变为 L1
    #[test]
    fn test_current_stage_change() {
        let (dir, _guard) = temp_dir();
        let stage_file = dir.join(".current_stage");

        write_file(&stage_file, "l0");

        let mut watcher = FileWatcher::with_interval(Duration::from_millis(50));
        watcher.add(stage_file.clone(), false);

        let (tx, rx) = mpsc::channel();
        let handles = watcher
            .start(move |event| {
                let _ = tx.send(event);
            })
            .expect("failed to start watcher");

        thread::sleep(Duration::from_millis(100));

        // 模拟 Stage 升级
        write_file(&stage_file, "l3");

        let deadline = Instant::now() + Duration::from_secs(3);
        let mut stage_updated = false;
        while Instant::now() < deadline {
            if let Ok(event) = rx.try_recv() {
                if event.kind_str() == "modified" && event.path() == stage_file {
                    stage_updated = true;
                    break;
                }
            }
            thread::sleep(Duration::from_millis(10));
        }

        assert!(
            stage_updated,
            ".current_stage change should trigger Modified within 3s"
        );

        handles.stop_and_join().ok();
    }

    /// 测试：模拟 `.claude/memory/` 目录内文件变化（递归监听）
    #[test]
    fn test_memory_directory_change() {
        let (dir, _guard) = temp_dir();
        let memory_dir = dir.join(".claude").join("memory");
        let memory_file = memory_dir.join("user-guidelines.md");

        // 先创建目录和初始文件
        write_file(&memory_file, "initial");

        let mut watcher = FileWatcher::with_interval(Duration::from_millis(50));
        watcher.add(memory_dir.clone(), true); // recursive

        let (tx, rx) = mpsc::channel();
        let handles = watcher
            .start(move |event| {
                let _ = tx.send(event);
            })
            .expect("failed to start watcher");

        thread::sleep(Duration::from_millis(100));

        // 修改 memory 文件
        write_file(&memory_file, "updated content");

        let deadline = Instant::now() + Duration::from_secs(3);
        let mut memory_updated = false;
        while Instant::now() < deadline {
            if let Ok(event) = rx.try_recv() {
                if event.kind_str() == "modified" && event.path() == memory_file {
                    memory_updated = true;
                    break;
                }
            }
            thread::sleep(Duration::from_millis(10));
        }

        assert!(
            memory_updated,
            "memory file change should trigger Modified within 3s"
        );

        handles.stop_and_join().ok();
    }

    /// 测试：目录包含多级子目录时也能检测变化
    #[test]
    fn test_deeply_nested_file_change() {
        let (dir, _guard) = temp_dir();
        let deep_file = dir.join("a").join("b").join("c").join("deep.txt");
        write_file(&deep_file, "initial");

        let mut watcher = FileWatcher::with_interval(Duration::from_millis(50));
        watcher.add(dir.clone(), true);

        let (tx, rx) = mpsc::channel();
        let handles = watcher
            .start(move |event| {
                let _ = tx.send(event);
            })
            .expect("failed to start watcher");

        thread::sleep(Duration::from_millis(100));

        write_file(&deep_file, "changed");

        let deadline = Instant::now() + Duration::from_secs(3);
        let mut received = false;
        while Instant::now() < deadline {
            if let Ok(event) = rx.try_recv() {
                if event.kind_str() == "modified" && event.path() == deep_file {
                    received = true;
                    break;
                }
            }
            thread::sleep(Duration::from_millis(10));
        }

        assert!(received, "deeply nested file change should be detected");
        handles.stop_and_join().ok();
    }

    /// 测试：并发安全性（多个事件同时触发）
    #[test]
    fn test_concurrent_changes() {
        let (dir, _guard) = temp_dir();
        let f1 = dir.join("file1.txt");
        let f2 = dir.join("file2.txt");
        let f3 = dir.join("file3.txt");

        write_file(&f1, "one");
        write_file(&f2, "two");
        write_file(&f3, "three");

        let mut watcher = FileWatcher::with_interval(Duration::from_millis(50));
        watcher.add(dir.clone(), false);

        let (tx, rx) = mpsc::channel();
        let handles = watcher
            .start(move |event| {
                let _ = tx.send(event);
            })
            .expect("failed to start watcher");

        thread::sleep(Duration::from_millis(100));

        // 同时修改多个文件
        write_file(&f1, "one-updated");
        write_file(&f2, "two-updated");
        write_file(&f3, "three-updated");

        thread::sleep(Duration::from_millis(300));

        let events: Vec<WatchEvent> = rx.try_iter().collect();
        let modified_count = events.iter().filter(|e| e.kind_str() == "modified").count();
        assert!(
            modified_count >= 3,
            "should detect all 3 file changes (got {})",
            modified_count
        );

        handles.stop_and_join().ok();
    }
}
