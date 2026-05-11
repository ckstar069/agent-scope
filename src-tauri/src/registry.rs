//! # ProjectRegistry — 项目注册表
//!
//! 管理已注册的模板项目，支持添加/移除/列出/获取操作。
//! 数据持久化到本地 JSON 文件（app_data_dir/projects.json）。
//!
//! ## 路径去重
//!
//! 使用 `canonicalize()` 获取规范化绝对路径作为唯一标识，
//! 避免 `/tmp/foo/../bar` 和 `/tmp/bar` 被视为不同项目。
//!
//! ## 设计决策
//!
//! - 使用 `serde_json` 序列化，而非 SQLite 或复杂存储
//! - 不自动扫描文件系统（交给上层调用者决定发现策略）
//! - 不验证路径是否为有效的模板项目（交给上层）

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

// ============================================================================
// 公开类型
// ============================================================================

/// 项目条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectEntry {
    /// 规范化后的绝对路径
    pub path: String,
    /// 添加时间（Unix 时间戳，秒）
    pub added_at: u64,
}

/// 项目注册表错误
#[derive(Debug)]
pub enum RegistryError {
    /// 项目已存在（路径重复）
    AlreadyExists(String),
    /// 项目不存在（移除/获取时未找到）
    NotFound(String),
    /// 路径规范化失败（路径不存在或无权限读取）
    CanonicalizeFailed(String, std::io::Error),
    /// 持久化到 JSON 文件失败
    PersistFailed(String),
    /// 从 JSON 文件加载失败
    LoadFailed(String),
}

impl fmt::Display for RegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RegistryError::AlreadyExists(p) => write!(f, "项目已存在: {}", p),
            RegistryError::NotFound(p) => write!(f, "项目不存在: {}", p),
            RegistryError::CanonicalizeFailed(p, e) => {
                write!(f, "路径规范化失败: {} ({})", p, e)
            }
            RegistryError::PersistFailed(e) => write!(f, "持久化失败: {}", e),
            RegistryError::LoadFailed(e) => write!(f, "加载失败: {}", e),
        }
    }
}

impl std::error::Error for RegistryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            RegistryError::CanonicalizeFailed(_, e) => Some(e),
            _ => None,
        }
    }
}

// ============================================================================
// 内部数据模型（直接对应 JSON 结构）
// ============================================================================

/// 注册表持久化数据结构
///
/// 直接对应 `projects.json` 的内容。
/// 使用 `HashMap<String, ProjectEntry>` 实现 O(1) 查找，
/// Key 是规范化后的绝对路径。
#[derive(Debug, Serialize, Deserialize)]
struct RegistryData {
    projects: HashMap<String, ProjectEntry>,
}

// ============================================================================
// ProjectRegistry 主结构
// ============================================================================

/// 项目注册表
///
/// 管理已注册项目集合，提供增删查改操作。
/// 每个操作（添加/移除）自动触发持久化到 JSON 文件。
///
/// # 示例
///
/// ```ignore
/// use ptv::registry::ProjectRegistry;
/// use std::path::PathBuf;
///
/// let storage = PathBuf::from("/tmp/projects.json");
/// let mut registry = ProjectRegistry::new(storage);
///
/// registry.add(Path::new("/some/project")).unwrap();
/// let all = registry.list();
/// registry.remove(Path::new("/some/project")).unwrap();
/// ```
pub struct ProjectRegistry {
    /// JSON 持久化文件路径
    storage_path: PathBuf,
    /// 内存中的项目映射
    data: RegistryData,
}

impl ProjectRegistry {
    /// 创建一个新的空注册表并指定存储路径
    pub fn new(storage_path: PathBuf) -> Self {
        Self {
            storage_path,
            data: RegistryData {
                projects: HashMap::new(),
            },
        }
    }

    /// 从文件加载注册表，文件不存在或格式错误时返回空注册表
    ///
    /// 静默处理加载失败：文件不存在、权限不足、JSON 格式错误等情况
    /// 都不影响程序的正常启动，空注册表是安全的默认状态。
    pub fn load_or_default(storage_path: PathBuf) -> Self {
        if storage_path.exists() {
            match fs::read_to_string(&storage_path) {
                Ok(content) => {
                    if let Ok(mut data) = serde_json::from_str::<RegistryData>(&content) {
                        // 迁移：移除 Windows 路径的 \\?\ 前缀
                        let cleaned: HashMap<String, ProjectEntry> = data
                            .projects
                            .drain()
                            .map(|(path, entry)| {
                                let cleaned_path = if path.starts_with(r"\\?\") {
                                    path[4..].to_string()
                                } else {
                                    path
                                };
                                (cleaned_path.clone(), ProjectEntry { path: cleaned_path, added_at: entry.added_at })
                            })
                            .collect();
                        data.projects = cleaned;
                        return Self {
                            storage_path,
                            data,
                        };
                    }
                    // JSON 格式错误：打印警告但不阻止启动
                    eprintln!(
                        "[registry:warn] 无法解析项目注册表文件 '{}'，将使用空注册表",
                        storage_path.display()
                    );
                }
                Err(e) => {
                    eprintln!(
                        "[registry:warn] 无法读取项目注册表文件 '{}': {}，将使用空注册表",
                        storage_path.display(),
                        e
                    );
                }
            }
        }
        Self::new(storage_path)
    }

    /// 返回默认数据目录：`{data_local_dir}/ptv`
    ///
    /// 使用 `dirs` crate 获取平台对应的数据目录：
    /// - macOS: `~/Library/Application Support/ptv`
    /// - Linux: `~/.local/share/ptv`
    pub fn default_data_dir() -> PathBuf {
        dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("ptv")
    }

    // ---------------------------------------------------------------
    // 核心 API
    // ---------------------------------------------------------------

    /// 添加一个项目到注册表
    ///
    /// 1. 规范化路径（解析符号链接，转为绝对路径）
    /// 2. 检查去重（规范化后的路径不可重复）
    /// 3. 写入内存
    /// 4. 持久化到 JSON 文件
    ///
    /// # 错误
    ///
    /// - `CanonicalizeFailed`: 路径不存在或无权限
    /// - `AlreadyExists`: 规范化后的路径已注册
    /// - `PersistFailed`: 写入 JSON 文件失败
    pub fn add(&mut self, path: &Path) -> Result<ProjectEntry, RegistryError> {
        let canonical = Self::canonicalize(path)?;

        if self.data.projects.contains_key(&canonical) {
            return Err(RegistryError::AlreadyExists(canonical));
        }

        let entry = ProjectEntry {
            path: canonical.clone(),
            added_at: unix_now(),
        };

        self.data.projects.insert(canonical, entry.clone());
        self.save()?;

        Ok(entry)
    }

    /// 从注册表中移除一个项目
    ///
    /// 1. 规范化路径（如果目录已被删除则回退到使用原始路径）
    /// 2. 从内存中移除
    /// 3. 持久化到 JSON 文件
    ///
    /// # 错误
    ///
    /// - `NotFound`: 路径未注册
    /// - `PersistFailed`: 写入 JSON 文件失败
    pub fn remove(&mut self, path: &Path) -> Result<(), RegistryError> {
        let key = Self::canonicalize(path)
            .unwrap_or_else(|_| path.to_string_lossy().into_owned());

        if self.data.projects.remove(&key).is_none() {
            return Err(RegistryError::NotFound(
                path.to_string_lossy().into_owned(),
            ));
        }

        self.save()?;
        Ok(())
    }

    /// 列出所有已注册项目（按路径字母序排序）
    pub fn list(&self) -> Vec<ProjectEntry> {
        let mut entries: Vec<ProjectEntry> = self.data.projects.values().cloned().collect();
        entries.sort_by(|a, b| a.path.cmp(&b.path));
        entries
    }

    /// 获取单个项目条目
    ///
    /// # 错误
    ///
    /// - `NotFound`: 路径未注册
    pub fn get(&self, path: &Path) -> Result<ProjectEntry, RegistryError> {
        let key = Self::canonicalize(path)
            .unwrap_or_else(|_| path.to_string_lossy().into_owned());

        self.data
            .projects
            .get(&key)
            .cloned()
            .ok_or_else(|| RegistryError::NotFound(path.to_string_lossy().into_owned()))
    }

    // ---------------------------------------------------------------
    // 内部辅助方法
    // ---------------------------------------------------------------

    /// 规范化路径：解析符号链接并返回绝对路径的字符串形式
    fn canonicalize(path: &Path) -> Result<String, RegistryError> {
        match path.canonicalize() {
            Ok(p) => {
                let mut s = p.to_string_lossy().into_owned();
                // Windows 上 canonicalize() 会添加 \\?\ 前缀，移除它以获得可读路径
                if s.starts_with(r"\\?\") {
                    s.drain(..4);
                }
                Ok(s)
            }
            Err(e) => Err(RegistryError::CanonicalizeFailed(
                path.to_string_lossy().into_owned(),
                e,
            )),
        }
    }

    /// 持久化当前数据到 JSON 文件
    fn save(&self) -> Result<(), RegistryError> {
        // 确保父目录存在
        if let Some(parent) = self.storage_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| RegistryError::PersistFailed(e.to_string()))?;
        }

        let json = serde_json::to_string_pretty(&self.data)
            .map_err(|e| RegistryError::PersistFailed(e.to_string()))?;
        fs::write(&self.storage_path, &json)
            .map_err(|e| RegistryError::PersistFailed(e.to_string()))
    }
}

// ============================================================================
// 辅助函数
// ============================================================================

/// 获取当前 Unix 时间戳（秒）
fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    /// 创建临时测试目录，返回 (base_dir, json_path)
    fn test_env() -> (PathBuf, PathBuf) {
        let id = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("ptv-registry-test-{}", id));
        let _ = fs::create_dir_all(&dir);
        let json_path = dir.join("projects.json");
        (dir, json_path)
    }

    /// 在 `base` 目录下创建一个测试项目目录
    fn create_project(base: &Path, name: &str) -> PathBuf {
        let path = base.join(name);
        fs::create_dir_all(&path).unwrap();
        path
    }

    // ---------------------------------------------------------------
    // 添加操作
    // ---------------------------------------------------------------

    /// 测试：添加项目成功
    #[test]
    fn test_add_project() {
        let (base, json_path) = test_env();
        let mut registry = ProjectRegistry::new(json_path);
        let proj = create_project(&base, "my-project");

        let entry = registry.add(&proj).unwrap();
        assert_eq!(
            entry.path,
            proj.canonicalize().unwrap().to_string_lossy()
        );
        assert!(entry.added_at > 0, "时间戳应为正数");
    }

    /// 测试：重复路径被拒绝
    #[test]
    fn test_add_duplicate_rejected() {
        let (base, json_path) = test_env();
        let mut registry = ProjectRegistry::new(json_path);
        let proj = create_project(&base, "my-project");

        registry.add(&proj).unwrap();
        let result = registry.add(&proj);
        assert!(
            matches!(result, Err(RegistryError::AlreadyExists(_))),
            "重复添加应返回 AlreadyExists"
        );
    }

    /// 测试：通过 `./` 和 `..` 构造的相同路径应被识别为重复
    #[test]
    fn test_add_duplicate_normalized() {
        let (base, json_path) = test_env();
        let mut registry = ProjectRegistry::new(json_path);
        let proj = create_project(&base, "my-project");

        registry.add(&proj).unwrap();

        // 使用 ./ 变体
        let variant = base.join("my-project/./.");
        let result = registry.add(&variant);
        assert!(
            matches!(result, Err(RegistryError::AlreadyExists(_))),
            "规范化后的相同路径应被识别为重复"
        );
    }

    /// 测试：添加不存在的路径应失败
    #[test]
    fn test_add_nonexistent_path_fails() {
        let (_, json_path) = test_env();
        let mut registry = ProjectRegistry::new(json_path);
        let non_existent = std::env::temp_dir().join("__ptv_nonexistent_test_xyz__");

        let result = registry.add(&non_existent);
        assert!(
            matches!(result, Err(RegistryError::CanonicalizeFailed(_, _))),
            "不存在的路径应返回 CanonicalizeFailed"
        );
    }

    // ---------------------------------------------------------------
    // 移除操作
    // ---------------------------------------------------------------

    /// 测试：移除已注册项目成功
    #[test]
    fn test_remove_project() {
        let (base, json_path) = test_env();
        let mut registry = ProjectRegistry::new(json_path);
        let proj = create_project(&base, "my-project");

        registry.add(&proj).unwrap();
        registry.remove(&proj).unwrap();
        assert!(registry.list().is_empty(), "移除后列表应为空");
    }

    /// 测试：移除未注册项目应返回 NotFound
    #[test]
    fn test_remove_nonexistent_returns_not_found() {
        let (base, json_path) = test_env();
        let mut registry = ProjectRegistry::new(json_path);
        let proj_a = create_project(&base, "project-a");
        let proj_b = create_project(&base, "project-b");

        registry.add(&proj_a).unwrap();
        let result = registry.remove(&proj_b);
        assert!(
            matches!(result, Err(RegistryError::NotFound(_))),
            "移除未注册项目应返回 NotFound"
        );
    }

    // ---------------------------------------------------------------
    // 列出操作
    // ---------------------------------------------------------------

    /// 测试：空注册表列出为空
    #[test]
    fn test_list_empty() {
        let (_, json_path) = test_env();
        let registry = ProjectRegistry::new(json_path);
        assert!(registry.list().is_empty(), "新注册表应为空");
    }

    /// 测试：列出多个项目，按字母序排序
    #[test]
    fn test_list_sorted() {
        let (base, json_path) = test_env();
        let mut registry = ProjectRegistry::new(json_path);

        let proj_b = create_project(&base, "b-project");
        let proj_a = create_project(&base, "a-project");

        registry.add(&proj_b).unwrap();
        registry.add(&proj_a).unwrap();

        let entries = registry.list();
        assert_eq!(entries.len(), 2);
        assert!(
            entries[0].path.ends_with("a-project"),
            "应按字母序排序，第一个应为 a-project"
        );
        assert!(
            entries[1].path.ends_with("b-project"),
            "应按字母序排序，第二个应为 b-project"
        );
    }

    /// 测试：添加后 list 包含新建项目
    #[test]
    fn test_list_after_add() {
        let (base, json_path) = test_env();
        let mut registry = ProjectRegistry::new(json_path);
        let proj = create_project(&base, "test");

        assert_eq!(registry.list().len(), 0);
        registry.add(&proj).unwrap();
        assert_eq!(registry.list().len(), 1);
    }

    // ---------------------------------------------------------------
    // 获取操作
    // ---------------------------------------------------------------

    /// 测试：获取已注册项目
    #[test]
    fn test_get_project() {
        let (base, json_path) = test_env();
        let mut registry = ProjectRegistry::new(json_path);
        let proj = create_project(&base, "my-project");

        registry.add(&proj).unwrap();
        let entry = registry.get(&proj).unwrap();
        assert_eq!(
            entry.path,
            proj.canonicalize().unwrap().to_string_lossy()
        );
    }

    /// 测试：获取未注册项目应返回 NotFound
    #[test]
    fn test_get_nonexistent_returns_not_found() {
        let (base, json_path) = test_env();
        let registry = ProjectRegistry::new(json_path);
        let proj = create_project(&base, "unregistered");

        let result = registry.get(&proj);
        assert!(
            matches!(result, Err(RegistryError::NotFound(_))),
            "获取未注册项目应返回 NotFound"
        );
    }

    // ---------------------------------------------------------------
    // 持久化测试
    // ---------------------------------------------------------------

    /// 测试：添加项目后数据被持久化到 JSON 文件
    #[test]
    fn test_persistence() {
        let (base, json_path) = test_env();
        let proj = create_project(&base, "persist-test");

        // 第一个实例：添加项目后释放
        {
            let mut registry = ProjectRegistry::new(json_path.clone());
            registry.add(&proj).unwrap();
        }

        // 验证 JSON 文件存在
        assert!(json_path.exists(), "持久化文件应存在");

        // 第二个实例：从文件加载
        let registry = ProjectRegistry::load_or_default(json_path);
        assert_eq!(registry.list().len(), 1, "应有一个已持久化的项目");
        assert!(registry.get(&proj).is_ok(), "持久化的项目应能通过 get 获取");
    }

    /// 测试：移除项目后数据同步从持久化文件中删除
    #[test]
    fn test_persistence_after_remove() {
        let (base, json_path) = test_env();
        let proj_a = create_project(&base, "project-a");
        let proj_b = create_project(&base, "project-b");

        // 添加两个项目，移除一个
        {
            let mut registry = ProjectRegistry::new(json_path.clone());
            registry.add(&proj_a).unwrap();
            registry.add(&proj_b).unwrap();
            registry.remove(&proj_a).unwrap();
        }

        // 重新加载，验证只剩一个
        let registry = ProjectRegistry::load_or_default(json_path);
        assert_eq!(registry.list().len(), 1, "移除后只应剩一个项目");
        assert!(registry.get(&proj_b).is_ok(), "project-b 应存在");
        assert!(
            registry.get(&proj_a).is_err(),
            "被移除的 project-a 应不存在"
        );
    }

    /// 测试：不存在的文件加载为默认空注册表
    #[test]
    fn test_load_or_default_nonexistent_file() {
        let (_, json_path) = test_env();
        // JSON 文件还不存在
        let registry = ProjectRegistry::load_or_default(json_path);
        assert!(registry.list().is_empty(), "空文件应加载为空注册表");
    }

    /// 测试：损坏的 JSON 文件加载为默认空注册表（静默处理）
    #[test]
    fn test_load_or_default_corrupted_file() {
        let (_, json_path) = test_env();
        // 写入无效内容
        fs::write(&json_path, "这不是合法 JSON").unwrap();

        // 应返回空注册表而不是 panic
        let registry = ProjectRegistry::load_or_default(json_path);
        assert!(registry.list().is_empty(), "损坏的 JSON 应加载为空注册表");
    }

    // ---------------------------------------------------------------
    // 辅助功能测试
    // ---------------------------------------------------------------

    /// 测试：默认数据目录以 "ptv" 结尾
    #[test]
    fn test_default_data_dir() {
        let dir = ProjectRegistry::default_data_dir();
        assert!(dir.ends_with("ptv"), "默认数据目录应以 ptv 结尾");
        // 应包含平台对应的数据目录前缀
        let name = dir.to_string_lossy();
        assert!(
            name.contains("ptv"),
            "默认数据目录路径应包含 ptv"
        );
    }

    // ---------------------------------------------------------------
    // 集成场景测试
    // ---------------------------------------------------------------

    /// 测试：添加 → 列出 → 获取 → 移除 → 列出 的完整流程
    #[test]
    fn test_full_lifecycle() {
        let (base, json_path) = test_env();
        let mut registry = ProjectRegistry::new(json_path);

        let proj = create_project(&base, "lifecycle-test");

        // 添加
        let entry = registry.add(&proj).unwrap();
        assert!(!entry.path.is_empty());

        // 列出
        assert_eq!(registry.list().len(), 1);

        // 获取
        let fetched = registry.get(&proj).unwrap();
        assert_eq!(fetched.path, entry.path);

        // 移除
        registry.remove(&proj).unwrap();

        // 再次列出
        assert!(registry.list().is_empty());

        // 再次获取 → 失败
        assert!(
            matches!(registry.get(&proj), Err(RegistryError::NotFound(_))),
            "移除后获取应返回 NotFound"
        );
    }

    /// 测试：多个项目同时存在
    #[test]
    fn test_multiple_projects() {
        let (base, json_path) = test_env();
        let mut registry = ProjectRegistry::new(json_path);

        let names = ["alpha", "beta", "gamma", "delta"];
        let paths: Vec<_> = names
            .iter()
            .map(|n| create_project(&base, n))
            .collect();

        for p in &paths {
            registry.add(p).unwrap();
        }

        assert_eq!(registry.list().len(), names.len());

        // 每个项目都能被获取
        for p in &paths {
            assert!(registry.get(p).is_ok(), "已添加的项目应能被获取");
        }
    }
}
