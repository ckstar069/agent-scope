use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

// ============================================================================
// WhitelistEntry — 白名单目录条目（与 project_files.rs 保持一致）
// ============================================================================

/// 描述一个允许扫描的目录及其规则
struct WhitelistEntry {
    /// 相对于模板根目录的路径
    relative_dir: &'static str,
    /// 是否递归扫描子目录
    recursive: bool,
    /// 若为 Some，则只采集指定名称的文件；若为 None，则采集所有 *.md
    specific_files: Option<&'static [&'static str]>,
}

/// 所有白名单条目（与 ProjectFilesCollector 的 whitelist 一致）
fn whitelist_entries() -> Vec<WhitelistEntry> {
    vec![
        // 根目录：CLAUDE.md、AGENTS.md
        WhitelistEntry {
            relative_dir: ".",
            recursive: false,
            specific_files: Some(&["CLAUDE.md", "AGENTS.md"]),
        },
        // .claude/rules/*.md（递归 1 层）
        WhitelistEntry {
            relative_dir: ".claude/rules",
            recursive: true,
            specific_files: None,
        },
        // .sisyphus/notepads/**/*.md（递归）
        WhitelistEntry {
            relative_dir: ".sisyphus/notepads",
            recursive: true,
            specific_files: None,
        },
        // .sisyphus/plans/*.md
        WhitelistEntry {
            relative_dir: ".sisyphus/plans",
            recursive: false,
            specific_files: None,
        },
        // .sisyphus/drafts/*.md
        WhitelistEntry {
            relative_dir: ".sisyphus/drafts",
            recursive: false,
            specific_files: None,
        },
        // docs/design/*.md
        WhitelistEntry {
            relative_dir: "docs/design",
            recursive: false,
            specific_files: None,
        },
        // docs/specs/**/*.md（递归）
        WhitelistEntry {
            relative_dir: "docs/specs",
            recursive: true,
            specific_files: None,
        },
    ]
}

// ============================================================================
// TemplateFingerprint — 模板项目文件指纹
// ============================================================================

/// 模板项目的文件指纹
///
/// 扫描模板项目的白名单目录，收集所有 .md 文件的相对路径（不含内容）。
/// 用于后续比较以区分某个文件是模板原生的还是项目新增/修改的。
#[derive(Debug, Clone)]
pub struct TemplateFingerprint {
    /// 模板项目中所有白名单文件的相对路径集合
    pub paths: HashSet<String>,
}

impl TemplateFingerprint {
    /// 构建模板文件指纹
    ///
    /// 扫描 `template_path` 下的白名单目录，收集所有匹配文件的相对路径。
    /// 使用与 `ProjectFilesCollector` 相同的白名单目录。
    pub fn build(template_path: &Path) -> Result<Self, String> {
        let mut paths = HashSet::new();

        for entry in whitelist_entries() {
            let dir_path = if entry.relative_dir == "." {
                template_path.to_path_buf()
            } else {
                template_path.join(entry.relative_dir)
            };

            if !dir_path.exists() || !dir_path.is_dir() {
                continue;
            }

            let _ = Self::collect_from_dir(
                template_path,
                &dir_path,
                entry.recursive,
                entry.specific_files,
                &mut paths,
            );
        }

        Ok(Self { paths })
    }

    /// 从指定目录递归/非递归收集文件路径
    fn collect_from_dir(
        template_root: &Path,
        dir: &Path,
        recursive: bool,
        specific_files: Option<&'static [&'static str]>,
        paths: &mut HashSet<String>,
    ) -> Result<(), String> {
        let dir_entries = match fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(e) => {
                eprintln!(
                    "[template_fingerprint] 读取目录失败 '{}': {}",
                    dir.display(),
                    e
                );
                return Ok(());
            }
        };

        for entry in dir_entries {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    eprintln!("[template_fingerprint] 跳过无法读取的目录项: {}", e);
                    continue;
                }
            };

            let entry_path = entry.path();
            let file_name = entry.file_name();
            let file_name_str = file_name.to_string_lossy();

            // 跳过隐藏文件和目录
            if file_name_str.starts_with('.') {
                continue;
            }

            if entry_path.is_dir() {
                // 递归扫描子目录
                if recursive {
                    let _ = Self::collect_from_dir(
                        template_root,
                        &entry_path,
                        true,
                        specific_files,
                        paths,
                    );
                }
                continue;
            }

            // 检查文件是否匹配
            if let Some(specific) = specific_files {
                if !specific.iter().any(|s| *s == file_name_str.as_ref()) {
                    continue;
                }
            } else if entry_path.extension().and_then(|s| s.to_str()) != Some("md") {
                continue;
            }

            // 计算相对路径并加入集合
            if let Ok(relative) = entry_path.strip_prefix(template_root) {
                paths.insert(relative.to_string_lossy().to_string());
            }
        }

        Ok(())
    }
}

// ============================================================================
// settings.json 持久化
// ============================================================================

/// settings.json 顶层结构
#[derive(serde::Serialize, serde::Deserialize)]
struct Settings {
    /// 模板项目路径
    template_path: Option<String>,
}

/// 构建 settings.json 的完整路径：`{data_dir}/agent-scope/settings.json`
fn settings_path(data_dir: &Path) -> PathBuf {
    data_dir.join("agent-scope").join("settings.json")
}

/// 从 settings.json 加载模板路径
///
/// 返回 `None` 的情况（不视为错误）：
/// - settings.json 文件不存在
/// - JSON 格式错误或字段缺失
/// - 文件读取失败（如权限不足）
pub fn load_template_path(data_dir: &Path) -> Option<PathBuf> {
    let path = settings_path(data_dir);
    if !path.exists() {
        return None;
    }

    match fs::read_to_string(&path) {
        Ok(content) => match serde_json::from_str::<Settings>(&content) {
            Ok(settings) => settings.template_path.map(PathBuf::from),
            Err(e) => {
                eprintln!(
                    "[template_fingerprint:warn] settings.json 格式错误 '{}': {}，将忽略",
                    path.display(),
                    e
                );
                None
            }
        },
        Err(e) => {
            eprintln!(
                "[template_fingerprint:warn] 无法读取 settings.json '{}': {}",
                path.display(),
                e
            );
            None
        }
    }
}

/// 保存模板路径到 settings.json
///
/// 自动创建父目录（如果不存在）。
pub fn save_template_path(data_dir: &Path, template_path: &Path) -> Result<(), String> {
    let path = settings_path(data_dir);

    // 确保目录存在
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("无法创建 settings 目录 '{}': {}", parent.display(), e))?;
    }

    let settings = Settings {
        template_path: Some(template_path.to_string_lossy().to_string()),
    };

    let json = serde_json::to_string_pretty(&settings)
        .map_err(|e| format!("序列化 settings.json 失败: {}", e))?;

    fs::write(&path, &json)
        .map_err(|e| format!("写入 settings.json '{}' 失败: {}", path.display(), e))?;

    Ok(())
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 创建迷你模板目录结构
    fn create_mini_template(dir: &Path) {
        // 根目录文件
        fs::write(dir.join("CLAUDE.md"), "# CLAUDE").unwrap();
        fs::write(dir.join("AGENTS.md"), "# AGENTS").unwrap();
        fs::write(dir.join("README.md"), "# README").unwrap(); // 不在白名单中

        // .claude/rules/
        fs::create_dir_all(dir.join(".claude").join("rules")).unwrap();
        fs::write(
            dir.join(".claude").join("rules").join("00-core.md"),
            "# Core",
        )
        .unwrap();
        fs::write(
            dir.join(".claude").join("rules").join("01-security.md"),
            "# Security",
        )
        .unwrap();

        // .sisyphus/notepads/
        fs::create_dir_all(dir.join(".sisyphus").join("notepads")).unwrap();
        fs::write(
            dir.join(".sisyphus")
                .join("notepads")
                .join("design-note.md"),
            "# Design",
        )
        .unwrap();

        // .sisyphus/plans/
        fs::create_dir_all(dir.join(".sisyphus").join("plans")).unwrap();
        fs::write(
            dir.join(".sisyphus").join("plans").join("sprint-1.md"),
            "# Sprint 1",
        )
        .unwrap();

        // docs/specs/
        fs::create_dir_all(dir.join("docs").join("specs")).unwrap();
        fs::write(
            dir.join("docs").join("specs").join("module-01.md"),
            "# Module 1",
        )
        .unwrap();
        fs::create_dir_all(dir.join("docs").join("specs").join("sub")).unwrap();
        fs::write(
            dir.join("docs").join("specs").join("sub").join("nested.md"),
            "# Nested",
        )
        .unwrap();

        // docs/design/
        fs::create_dir_all(dir.join("docs").join("design")).unwrap();
        fs::write(
            dir.join("docs").join("design").join("ui-flow.md"),
            "# UI Flow",
        )
        .unwrap();
    }

    #[test]
    fn test_build_fingerprint() {
        let dir = tempfile::tempdir().unwrap();
        create_mini_template(dir.path());

        let fp = TemplateFingerprint::build(dir.path()).unwrap();

        // 白名单内的文件都应该被收录
        assert!(fp.paths.contains("CLAUDE.md"));
        assert!(fp.paths.contains("AGENTS.md"));
        assert!(fp.paths.contains(".claude/rules/00-core.md"));
        assert!(fp.paths.contains(".claude/rules/01-security.md"));
        assert!(fp.paths.contains(".sisyphus/notepads/design-note.md"));
        assert!(fp.paths.contains(".sisyphus/plans/sprint-1.md"));
        assert!(fp.paths.contains("docs/specs/module-01.md"));
        assert!(fp.paths.contains("docs/specs/sub/nested.md"));
        assert!(fp.paths.contains("docs/design/ui-flow.md"));

        // README.md 不在白名单中，不应被收录
        assert!(!fp.paths.contains("README.md"));

        // 验证总数
        assert_eq!(fp.paths.len(), 9);
    }

    #[test]
    fn test_build_empty_path() {
        let dir = tempfile::tempdir().unwrap();
        // 空目录：没有白名单文件，路径集合为空
        let fp = TemplateFingerprint::build(dir.path()).unwrap();
        assert!(fp.paths.is_empty());
    }

    #[test]
    fn test_build_nonexistent_path() {
        let fp = TemplateFingerprint::build(
            &std::env::temp_dir().join("nonexistent-agent-scope-test-path"),
        );
        assert!(fp.is_ok());
        assert!(fp.unwrap().paths.is_empty());
    }

    #[test]
    fn test_save_and_load_template_path() {
        let dir = tempfile::tempdir().unwrap();
        let data_dir = dir.path();

        // 初始状态：无配置文件
        assert!(load_template_path(data_dir).is_none());

        // 保存路径
        let template_path = std::env::temp_dir().join("my-template");
        save_template_path(data_dir, &template_path).unwrap();

        // 加载验证
        let loaded = load_template_path(data_dir);
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap(), template_path);
    }

    #[test]
    fn test_load_corrupt_settings() {
        let dir = tempfile::tempdir().unwrap();
        let data_dir = dir.path();

        // 写入无效 JSON
        let settings_file = data_dir.join("agent-scope").join("settings.json");
        fs::create_dir_all(settings_file.parent().unwrap()).unwrap();
        fs::write(&settings_file, "not valid json").unwrap();

        // 格式错误时应返回 None（不 panic）
        assert!(load_template_path(data_dir).is_none());
    }

    #[test]
    fn test_load_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        // 文件不存在时应返回 None
        assert!(load_template_path(dir.path()).is_none());
    }

    #[test]
    fn test_fingerprint_skips_non_md() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("CLAUDE.md"), "# CLAUDE").unwrap();
        fs::write(dir.path().join("AGENTS.md"), "# AGENTS").unwrap();
        fs::write(dir.path().join("some_file.txt"), "not md").unwrap();
        fs::write(dir.path().join("data.json"), "{}").unwrap();

        let fp = TemplateFingerprint::build(dir.path()).unwrap();
        assert!(fp.paths.contains("CLAUDE.md"));
        assert!(fp.paths.contains("AGENTS.md"));
        assert!(!fp.paths.contains("some_file.txt"));
        assert!(!fp.paths.contains("data.json"));
        assert_eq!(fp.paths.len(), 2);
    }
}
