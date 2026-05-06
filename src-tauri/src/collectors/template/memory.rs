use std::collections::HashMap;
use std::fs;
use std::path::Path;

// ============================================================================
// MemoryEntry — 单条记忆记录
// ============================================================================

/// 从 `.claude/memory/*.md` 文件解析出的记忆条目
#[derive(Debug, Clone, PartialEq)]
pub struct MemoryEntry {
    /// 文件名（不含路径）
    pub filename: String,
    /// YAML frontmatter 键值对
    pub frontmatter: HashMap<String, String>,
    /// Markdown 正文内容（不含 frontmatter）
    pub content: String,
}

// ============================================================================
// MemoryCollector — 记忆文件采集器
// ============================================================================

/// 采集 `.claude/memory/` 目录下的所有 Markdown 文件
///
/// 解析每个 `.md` 文件的 YAML frontmatter 和正文内容。
pub struct MemoryCollector;

impl MemoryCollector {
    /// 采集指定路径项目的记忆文件
    ///
    /// # 参数
    /// - `path`: 项目根目录路径
    ///
    /// # 返回
    /// - `Ok(Vec<MemoryEntry>)`: 成功解析的所有记忆条目
    /// - `Err(MemoryError)`: 目录不存在或读取失败
    ///
    /// # 行为
    /// - 如果 `.claude/memory/` 目录不存在，返回空向量（不报错）
    /// - 如果目录存在但为空，返回空向量
    /// - 跳过无法解析的文件，继续处理其他文件
    pub fn collect(path: &Path) -> Result<Vec<MemoryEntry>, MemoryError> {
        let memory_dir = path.join(".claude").join("memory");

        if !memory_dir.exists() {
            return Ok(Vec::new());
        }

        if !memory_dir.is_dir() {
            return Err(MemoryError::NotADirectory(
                memory_dir.to_string_lossy().to_string(),
            ));
        }

        let mut entries = Vec::new();

        let dir_entries = fs::read_dir(&memory_dir).map_err(|e| {
            MemoryError::ReadError(memory_dir.to_string_lossy().to_string(), e.to_string())
        })?;

        for entry in dir_entries {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    eprintln!("[memory] 跳过无法读取的目录项: {}", e);
                    continue;
                }
            };

            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("md") {
                continue;
            }

            match Self::parse_file(&path) {
                Ok(entry) => entries.push(entry),
                Err(e) => {
                    eprintln!(
                        "[memory] 解析失败 '{}': {}",
                        path.display(),
                        e
                    );
                }
            }
        }

        Ok(entries)
    }

    /// 解析单个 Markdown 文件
    fn parse_file(path: &Path) -> Result<MemoryEntry, MemoryError> {
        let content = fs::read_to_string(path).map_err(|e| {
            MemoryError::ReadError(path.to_string_lossy().to_string(), e.to_string())
        })?;

        let filename = path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();

        let (frontmatter, body) = Self::split_frontmatter(&content);

        Ok(MemoryEntry {
            filename,
            frontmatter,
            content: body.trim().to_string(),
        })
    }

    /// 分割 YAML frontmatter 和正文
    ///
    /// 格式：
    /// ```markdown
    /// ---
    /// key: value
    /// ---
    /// 正文内容
    /// ```
    ///
    /// 如果没有 frontmatter，返回空 HashMap 和完整内容。
    fn split_frontmatter(content: &str) -> (HashMap<String, String>, String) {
        let lines: Vec<&str> = content.lines().collect();

        if lines.len() < 3 || lines[0].trim() != "---" {
            return (HashMap::new(), content.to_string());
        }

        let mut end_idx = None;
        for (i, line) in lines.iter().enumerate().skip(1) {
            if line.trim() == "---" {
                end_idx = Some(i);
                break;
            }
        }

        let end_idx = match end_idx {
            Some(idx) => idx,
            None => return (HashMap::new(), content.to_string()),
        };

        let mut frontmatter = HashMap::new();
        for line in &lines[1..end_idx] {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            if let Some(pos) = trimmed.find(':') {
                let key = trimmed[..pos].trim().to_string();
                let value = trimmed[pos + 1..].trim().to_string();
                if !key.is_empty() {
                    frontmatter.insert(key, value);
                }
            }
        }

        let body = lines[end_idx + 1..].join("\n");
        (frontmatter, body)
    }
}

// ============================================================================
// MemoryError — 记忆采集错误
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemoryError {
    NotADirectory(String),
    ReadError(String, String),
}

impl std::fmt::Display for MemoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemoryError::NotADirectory(path) => {
                write!(f, "记忆路径不是目录: {}", path)
            }
            MemoryError::ReadError(path, err) => {
                write!(f, "读取记忆文件失败 ({}): {}", path, err)
            }
        }
    }
}

impl std::error::Error for MemoryError {}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_memory_file(dir: &Path, name: &str, content: &str) -> std::path::PathBuf {
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let mut file = fs::File::create(&path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
        file.sync_all().unwrap();
        path
    }

    #[test]
    fn test_split_frontmatter_with_yaml() {
        let content = "---\ntitle: Test Memory\ndate: 2024-01-01\n---\n\nThis is the body.\n\nMore content.";
        let (fm, body) = MemoryCollector::split_frontmatter(content);
        assert_eq!(fm.get("title"), Some(&"Test Memory".to_string()));
        assert_eq!(fm.get("date"), Some(&"2024-01-01".to_string()));
        assert!(body.contains("This is the body."));
    }

    #[test]
    fn test_split_frontmatter_without_yaml() {
        let content = "Just a plain markdown file.\n\nNo frontmatter here.";
        let (fm, body) = MemoryCollector::split_frontmatter(content);
        assert!(fm.is_empty());
        assert_eq!(body, content);
    }

    #[test]
    fn test_split_frontmatter_empty_yaml() {
        let content = "---\n---\nBody starts here.";
        let (fm, body) = MemoryCollector::split_frontmatter(content);
        assert!(fm.is_empty());
        assert_eq!(body.trim(), "Body starts here.");
    }

    #[test]
    fn test_collect_empty_directory() {
        let dir = tempfile::tempdir().unwrap();
        let memory_dir = dir.path().join(".claude").join("memory");
        fs::create_dir_all(&memory_dir).unwrap();

        let entries = MemoryCollector::collect(dir.path()).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn test_collect_missing_directory() {
        let dir = tempfile::tempdir().unwrap();
        let entries = MemoryCollector::collect(dir.path()).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn test_collect_single_file() {
        let dir = tempfile::tempdir().unwrap();
        let memory_dir = dir.path().join(".claude").join("memory");
        fs::create_dir_all(&memory_dir).unwrap();

        write_memory_file(
            &memory_dir,
            "test.md",
            "---\ntitle: Hello\n---\n\nContent here."
        );

        let entries = MemoryCollector::collect(dir.path()).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].filename, "test.md");
        assert_eq!(entries[0].frontmatter.get("title"), Some(&"Hello".to_string()));
        assert!(entries[0].content.contains("Content here."));
    }

    #[test]
    fn test_collect_multiple_files() {
        let dir = tempfile::tempdir().unwrap();
        let memory_dir = dir.path().join(".claude").join("memory");
        fs::create_dir_all(&memory_dir).unwrap();

        write_memory_file(
            &memory_dir,
            "a.md",
            "---\nkey: value1\n---\nBody A"
        );
        write_memory_file(
            &memory_dir,
            "b.md",
            "---\nkey: value2\n---\nBody B"
        );

        let entries = MemoryCollector::collect(dir.path()).unwrap();
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn test_collect_skips_non_md() {
        let dir = tempfile::tempdir().unwrap();
        let memory_dir = dir.path().join(".claude").join("memory");
        fs::create_dir_all(&memory_dir).unwrap();

        write_memory_file(&memory_dir, "valid.md", "---\ntitle: OK\n---\nBody");
        write_memory_file(&memory_dir, "readme.txt", "Not a markdown");

        let entries = MemoryCollector::collect(dir.path()).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].filename, "valid.md");
    }
}
