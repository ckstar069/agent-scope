use std::fs;
use std::io::Read;
use std::path::Path;
use std::time::UNIX_EPOCH;

// ============================================================================
// 常量
// ============================================================================

/// 单文件大小上限：1 MiB
const MAX_FILE_SIZE: u64 = 1_048_576;

/// 累计采集大小上限：50 MiB
const MAX_TOTAL_SIZE: u64 = 52_428_800;

// ============================================================================
// WhitelistEntry — 白名单目录条目
// ============================================================================

/// 描述一个允许扫描的目录及其采集规则
struct WhitelistEntry {
    /// 相对于项目根目录的路径
    relative_dir: &'static str,
    /// 是否递归扫描子目录
    recursive: bool,
    /// 来源分组标签
    source_group: &'static str,
    /// 若为 Some，则只采集指定名称的文件；若为 None，则采集所有 *.md
    specific_files: Option<&'static [&'static str]>,
}

/// 所有白名单条目
fn whitelist_entries() -> Vec<WhitelistEntry> {
    vec![
        // 根目录：CLAUDE.md、AGENTS.md
        WhitelistEntry {
            relative_dir: ".",
            recursive: false,
            source_group: "root",
            specific_files: Some(&["CLAUDE.md", "AGENTS.md"]),
        },
        // .claude/rules/*.md（递归 1 层）
        WhitelistEntry {
            relative_dir: ".claude/rules",
            recursive: true,
            source_group: "rules",
            specific_files: None,
        },
        // .sisyphus/notepads/**/*.md（递归）
        WhitelistEntry {
            relative_dir: ".sisyphus/notepads",
            recursive: true,
            source_group: "notepads",
            specific_files: None,
        },
        // .sisyphus/plans/*.md
        WhitelistEntry {
            relative_dir: ".sisyphus/plans",
            recursive: false,
            source_group: "plans",
            specific_files: None,
        },
        // .sisyphus/drafts/*.md
        WhitelistEntry {
            relative_dir: ".sisyphus/drafts",
            recursive: false,
            source_group: "drafts",
            specific_files: None,
        },
        // docs/design/*.md
        WhitelistEntry {
            relative_dir: "docs/design",
            recursive: false,
            source_group: "design",
            specific_files: None,
        },
        // docs/specs/**/*.md（递归）
        WhitelistEntry {
            relative_dir: "docs/specs",
            recursive: true,
            source_group: "specs",
            specific_files: None,
        },
    ]
}

// ============================================================================
// ProjectFile — 采集到的项目文件
// ============================================================================

/// 从监控项目中采集的单个 Markdown 文件
#[derive(Debug, Clone, PartialEq)]
pub struct ProjectFile {
    /// 相对于项目根目录的路径
    pub relative_path: String,
    /// 文件内容（UTF-8），非 UTF-8 文件内容为 "(binary/encoding error)"
    pub content: String,
    /// 内容是否因超过单文件上限而被截断
    pub content_truncated: bool,
    /// 来源分组（root, rules, notepads, plans, drafts, design, specs）
    pub source_group: String,
    /// 文件最后修改时间（Unix 毫秒时间戳）
    pub mtime_ms: u64,
    pub origin: String,
}

// ============================================================================
// ProjectFilesCollector — 项目文件采集器
// ============================================================================

/// 采集 FPGA 项目中的静态 Markdown 文件
///
/// 按白名单目录扫描，仅采集白名单内目录的 `.md` 文件。
/// 不会扫描 `.git/`、`node_modules/`、`target/`、`dist/` 等构建产物目录。
pub struct ProjectFilesCollector;

impl ProjectFilesCollector {
    /// 采集指定路径项目的白名单 Markdown 文件
    ///
    /// # 参数
    /// - `path`: 项目根目录路径
    ///
    /// # 返回
    /// - `Ok(Vec<ProjectFile>)`: 采集到的所有文件
    /// - `Err(ProjectFilesError)`: 采集过程中遇到的不可恢复错误
    ///
    /// # 行为
    /// - 目录不存在时静默跳过（返回空向量，不报错）
    /// - 单文件超过 1 MiB 时截断读取并标记 `content_truncated`
    /// - 累计超过 50 MiB 时停止采集并返回 `FileTooLarge` 错误
    /// - 非 UTF-8 文件内容替换为 "(binary/encoding error)"
    /// - 跳过无法读取的单个文件，继续处理其他文件
    pub fn collect(path: &Path) -> Result<Vec<ProjectFile>, ProjectFilesError> {
        let mut results = Vec::new();
        let mut total_bytes: u64 = 0;

        for entry in whitelist_entries() {
            let dir = path.join(entry.relative_dir);
            if !dir.exists() {
                continue;
            }
            if !dir.is_dir() {
                continue;
            }

            Self::collect_from_dir(
                path,
                &dir,
                entry.recursive,
                entry.source_group,
                entry.specific_files,
                &mut results,
                &mut total_bytes,
            )?;

            // 累计超过上限则停止
            if total_bytes >= MAX_TOTAL_SIZE {
                return Err(ProjectFilesError::FileTooLarge(format!(
                    "累计文件大小超过 {} MiB 上限",
                    MAX_TOTAL_SIZE / 1_048_576
                )));
            }
        }

        Ok(results)
    }

    /// 从指定目录采集文件
    fn collect_from_dir(
        project_root: &Path,
        dir: &Path,
        recursive: bool,
        source_group: &str,
        specific_files: Option<&[&str]>,
        results: &mut Vec<ProjectFile>,
        total_bytes: &mut u64,
    ) -> Result<(), ProjectFilesError> {
        let dir_entries = match fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(e) => {
                // 权限不足：如果目录是明确的白名单目录，报告错误；否则静默跳过
                if e.kind() == std::io::ErrorKind::PermissionDenied {
                    eprintln!(
                        "[project_files] 权限不足，跳过目录 '{}': {}",
                        dir.display(),
                        e
                    );
                    return Ok(());
                }
                eprintln!(
                    "[project_files] 读取目录失败 '{}': {}",
                    dir.display(),
                    e
                );
                return Ok(());
            }
        };

        for entry in dir_entries {
            // 检查累计上限
            if *total_bytes >= MAX_TOTAL_SIZE {
                return Ok(());
            }

            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    eprintln!("[project_files] 跳过无法读取的目录项: {}", e);
                    continue;
                }
            };

            let entry_path = entry.path();
            let file_name = entry.file_name();
            let file_name_str = file_name.to_string_lossy();

            // 跳过隐藏文件和目录（除了白名单中明确指定的目录）
            if file_name_str.starts_with('.') {
                continue;
            }

            if entry_path.is_dir() {
                // 跳过构建产物目录
                if matches!(
                    file_name_str.as_ref(),
                    "node_modules" | "target" | "dist" | ".git"
                ) {
                    continue;
                }
                // 递归扫描子目录
                if recursive {
                    Self::collect_from_dir(
                        project_root,
                        &entry_path,
                        true, // 子目录继续递归
                        source_group,
                        specific_files,
                        results,
                        total_bytes,
                    )?;
                }
                continue;
            }

            // 检查是否为 .md 文件（specific_files 模式下只检查指定文件）
            if let Some(specific) = specific_files {
                if !specific.iter().any(|s| *s == file_name_str.as_ref()) {
                    continue;
                }
            } else if entry_path.extension().and_then(|s| s.to_str()) != Some("md") {
                continue;
            }

            // 处理单个文件
            match Self::read_file(project_root, &entry_path, source_group) {
                Ok(Some(file)) => {
                    let content_len = file.content.len() as u64;
                    *total_bytes += content_len;
                    results.push(file);
                }
                Ok(None) => {} // 文件无法读取，已日志记录
                Err(e) => {
                    eprintln!(
                        "[project_files] 读取文件失败 '{}': {:?}",
                        entry_path.display(),
                        e
                    );
                }
            }
        }

        Ok(())
    }

    /// 读取单个文件并构造 ProjectFile
    ///
    /// 返回 `Ok(None)` 表示文件被跳过（如元数据读取失败），调用方应继续处理。
    fn read_file(
        project_root: &Path,
        file_path: &Path,
        source_group: &str,
    ) -> Result<Option<ProjectFile>, ProjectFilesError> {
        // 获取文件元数据
        let metadata = match fs::metadata(file_path) {
            Ok(m) => m,
            Err(e) => {
                eprintln!(
                    "[project_files] 无法获取文件元数据 '{}': {}",
                    file_path.display(),
                    e
                );
                return Ok(None);
            }
        };

        if !metadata.is_file() {
            return Ok(None);
        }

        let file_size = metadata.len();

        // 获取修改时间
        let mtime_ms = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        // 计算相对路径
        let relative_path = file_path
            .strip_prefix(project_root)
            .unwrap_or(file_path)
            .to_string_lossy()
            .to_string();

        // 读取文件内容
        let (content, content_truncated) = if file_size > MAX_FILE_SIZE {
            // 大文件：只读取前 1 MiB
            match Self::read_file_head(file_path, MAX_FILE_SIZE as usize) {
                Ok(s) => (s, true),
                Err(_) => return Ok(None),
            }
        } else {
            match fs::read_to_string(file_path) {
                Ok(s) => (s, false),
                Err(_) => {
                    // UTF-8 读取失败，尝试按字节读取后转换
                    match Self::read_file_as_string_lossy(file_path) {
                        Some(s) => (s, false),
                        None => {
                            // 完全无法读取，使用占位内容
                            ("(binary/encoding error)".to_string(), false)
                        }
                    }
                }
            }
        };

        Ok(Some(ProjectFile {
            relative_path,
            content,
            content_truncated,
            source_group: source_group.to_string(),
            mtime_ms,
            origin: "unknown".to_string(),
        }))
    }

    /// 读取文件头部指定字节数并尝试转换为 UTF-8
    fn read_file_head(path: &Path, max_bytes: usize) -> Result<String, std::io::Error> {
        let mut file = fs::File::open(path)?;
        let mut buffer = vec![0u8; max_bytes.min(MAX_FILE_SIZE as usize)];
        let bytes_read = file.read(&mut buffer)?;
        buffer.truncate(bytes_read);
        Ok(String::from_utf8_lossy(&buffer).to_string())
    }

    /// 尝试将文件内容读取为 UTF-8 字符串（lossy 模式）
    fn read_file_as_string_lossy(path: &Path) -> Option<String> {
        fs::read(path)
            .ok()
            .map(|bytes| String::from_utf8_lossy(&bytes).to_string())
    }
}

// ============================================================================
// ProjectFilesError — 项目文件采集错误
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectFilesError {
    /// I/O 错误
    Io(String),
    /// 权限不足
    PermissionDenied(String),
    /// 文件过大（单文件超过 1 MiB 或累计超过 50 MiB）
    FileTooLarge(String),
}

impl std::fmt::Display for ProjectFilesError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProjectFilesError::Io(msg) => write!(f, "I/O 错误: {}", msg),
            ProjectFilesError::PermissionDenied(msg) => write!(f, "权限不足: {}", msg),
            ProjectFilesError::FileTooLarge(msg) => write!(f, "文件过大: {}", msg),
        }
    }
}

impl std::error::Error for ProjectFilesError {}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::path::PathBuf;

    /// 辅助函数：在指定目录下创建 Markdown 文件
    fn write_md_file(dir: &Path, name: &str, content: &str) -> PathBuf {
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let mut file = fs::File::create(&path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
        file.sync_all().unwrap();
        path
    }

    /// 辅助函数：创建指定大小的文件
    fn write_sized_file(dir: &Path, name: &str, size: usize) -> PathBuf {
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let mut file = fs::File::create(&path).unwrap();
        let content = vec![b'a'; size];
        file.write_all(&content).unwrap();
        file.sync_all().unwrap();
        path
    }

    // --------------------------------------------------------------------------
    // 测试 1：空目录（无任何白名单目录存在）
    // --------------------------------------------------------------------------

    #[test]
    fn test_collect_empty_project() {
        let dir = tempfile::tempdir().unwrap();
        let files = ProjectFilesCollector::collect(dir.path()).unwrap();
        assert!(files.is_empty());
    }

    // --------------------------------------------------------------------------
    // 测试 2：根目录 CLAUDE.md + AGENTS.md
    // --------------------------------------------------------------------------

    #[test]
    fn test_collect_root_files() {
        let dir = tempfile::tempdir().unwrap();

        write_md_file(dir.path(), "CLAUDE.md", "# CLAUDE\n\nMemory content.");
        write_md_file(dir.path(), "AGENTS.md", "# AGENTS\n\nAgent rules.");
        // 不在白名单中的根目录文件，应被忽略
        write_md_file(dir.path(), "README.md", "# README\n\nNot collected.");

        let files = ProjectFilesCollector::collect(dir.path()).unwrap();
        assert_eq!(files.len(), 2);

        let claude = files.iter().find(|f| f.relative_path == "CLAUDE.md").unwrap();
        assert_eq!(claude.source_group, "root");
        assert!(claude.content.contains("CLAUDE"));
        assert!(!claude.content_truncated);
        assert!(claude.mtime_ms > 0);

        let agents = files.iter().find(|f| f.relative_path == "AGENTS.md").unwrap();
        assert_eq!(agents.source_group, "root");
        assert!(agents.content.contains("AGENTS"));
    }

    // --------------------------------------------------------------------------
    // 测试 3：嵌套 .sisyphus 目录（notepads 递归 + plans + drafts）
    // --------------------------------------------------------------------------

    #[test]
    fn test_collect_sisyphus_nested() {
        let dir = tempfile::tempdir().unwrap();

        // .sisyphus/notepads/ 递归
        write_md_file(
            dir.path(),
            ".sisyphus/notepads/my-plan/learnings.md",
            "# Learnings\n\nWhat I learned.",
        );
        write_md_file(
            dir.path(),
            ".sisyphus/notepads/my-plan/issues.md",
            "# Issues\n\nKnown problems.",
        );
        // 嵌套子目录
        write_md_file(
            dir.path(),
            ".sisyphus/notepads/my-plan/nested/deep.md",
            "# Deep\n\nNested file.",
        );

        // .sisyphus/plans/（非递归）
        write_md_file(
            dir.path(),
            ".sisyphus/plans/my-plan.md",
            "# Plan\n\nPlan content.",
        );

        // .sisyphus/drafts/（非递归）
        write_md_file(
            dir.path(),
            ".sisyphus/drafts/draft-1.md",
            "# Draft\n\nDraft content.",
        );

        let files = ProjectFilesCollector::collect(dir.path()).unwrap();
        // 期望：4 个 notepads 文件(3 + 1 nested) + 1 plan + 1 draft = 6
        // 实际上 notepads 下是 my-plan/learnings.md, my-plan/issues.md, my-plan/nested/deep.md 共 3 个
        assert_eq!(files.len(), 5);

        let notepads: Vec<_> = files
            .iter()
            .filter(|f| f.source_group == "notepads")
            .collect();
        assert_eq!(notepads.len(), 3);

        let plans: Vec<_> = files.iter().filter(|f| f.source_group == "plans").collect();
        assert_eq!(plans.len(), 1);
        assert!(plans[0].relative_path.contains("my-plan.md"));

        let drafts: Vec<_> = files.iter().filter(|f| f.source_group == "drafts").collect();
        assert_eq!(drafts.len(), 1);
    }

    // --------------------------------------------------------------------------
    // 测试 4：大文件截断（>1 MiB）
    // --------------------------------------------------------------------------

    #[test]
    fn test_collect_large_file_truncation() {
        let dir = tempfile::tempdir().unwrap();

        // 创建 CLAUDE.md 正常文件
        write_md_file(dir.path(), "CLAUDE.md", "# Normal\n\nSmall file.");

        // 创建超过 1 MiB 的 .sisyphus/notepads 大文件
        let large_size = (MAX_FILE_SIZE as usize) + 1024; // 1 MiB + 1 KiB
        write_sized_file(
            dir.path(),
            ".sisyphus/notepads/huge.md",
            large_size,
        );

        let files = ProjectFilesCollector::collect(dir.path()).unwrap();
        assert_eq!(files.len(), 2);

        let huge = files.iter().find(|f| f.relative_path.contains("huge.md")).unwrap();
        assert!(huge.content_truncated);
        // 内容应被截断为 MAX_FILE_SIZE 字节（UTF-8 lossy 后可能略有不同）
        assert!(huge.content.len() <= MAX_FILE_SIZE as usize);

        let normal = files.iter().find(|f| f.relative_path == "CLAUDE.md").unwrap();
        assert!(!normal.content_truncated);
    }

    // --------------------------------------------------------------------------
    // 测试 5：混合场景 — 根文件 + rules + docs
    // --------------------------------------------------------------------------

    #[test]
    fn test_collect_mixed_directories() {
        let dir = tempfile::tempdir().unwrap();

        // 根目录
        write_md_file(dir.path(), "CLAUDE.md", "# Root\n\nRoot file.");

        // .claude/rules/
        write_md_file(
            dir.path(),
            ".claude/rules/coding.md",
            "# Coding\n\nCoding rules.",
        );
        write_md_file(
            dir.path(),
            ".claude/rules/stage/l1.md",
            "# L1 Rules\n\nStage L1.",
        );

        // docs/design/
        write_md_file(
            dir.path(),
            "docs/design/architecture.md",
            "# Architecture\n\nDesign doc.",
        );

        // docs/specs/（递归）
        write_md_file(
            dir.path(),
            "docs/specs/overview.md",
            "# Overview\n\nSpec overview.",
        );
        write_md_file(
            dir.path(),
            "docs/specs/modules/module-01.md",
            "# Module 01\n\nModule spec.",
        );

        let files = ProjectFilesCollector::collect(dir.path()).unwrap();
        assert_eq!(files.len(), 6);

        // 验证各来源分组都有文件：root(1) + rules(2) + design(1) + specs(2) = 6
        let groups: Vec<&str> = files.iter().map(|f| f.source_group.as_str()).collect();
        assert!(groups.contains(&"root"));
        assert!(groups.contains(&"rules"));
        assert!(groups.contains(&"design"));
        assert!(groups.contains(&"specs"));
    }

    // --------------------------------------------------------------------------
    // 测试 6：目录不存在时静默跳过
    // --------------------------------------------------------------------------

    #[test]
    fn test_collect_missing_directories_silent() {
        let dir = tempfile::tempdir().unwrap();

        // 只创建根目录文件，其他白名单目录都不存在
        write_md_file(dir.path(), "CLAUDE.md", "# Only root");

        let files = ProjectFilesCollector::collect(dir.path()).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].source_group, "root");
    }
}
