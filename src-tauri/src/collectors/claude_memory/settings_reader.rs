use std::fs;
use std::path::{Path, PathBuf};

use super::models::{ClaudeMdExcludesConfig, ExcludePattern};
use super::path_resolver::{find_git_repo_root, resolve_managed_dir};

/// 读取多层 claudeMdExcludes 并合并
///
/// 合并规则：
/// 1. managed 层：先读 managed-settings.json，再读 managed-settings.d/*.json
///    - 各 drop-in 文件按文件名排序后合并
/// 2. user / project / local 层：读对应 settings.json
/// 3. 每层取其 claudeMdExcludes 字符串数组
/// 4. 数组跨层合并（concat），不是覆盖：所有层的 excludes 都生效
/// 5. 每个 pattern 保留来源标注（managed / user / project / local）
/// 6. 返回合并后的排除模式列表
///
/// 从指定 managed 目录读取 excludes（file-based managed policy）
///
/// 返回：(patterns, managed_accessible)
/// - managed_accessible = Some(true)  : 至少成功读取了一个文件
/// - managed_accessible = Some(false) : 文件存在但不可读
/// - managed_accessible = None        : managed 目录不存在或无任何配置
fn read_managed_excludes_from_dir(managed_dir: &Path) -> (Vec<ExcludePattern>, Option<bool>) {
    let mut patterns: Vec<ExcludePattern> = Vec::new();
    let mut managed_accessible: Option<bool> = None;

    let managed_settings = managed_dir.join("managed-settings.json");
    let managed_dropin_dir = managed_dir.join("managed-settings.d");

    let managed_exists = managed_settings.exists();
    let dropin_exists = managed_dropin_dir.exists() && managed_dropin_dir.is_dir();

    if !managed_exists && !dropin_exists {
        return (patterns, managed_accessible);
    }

    // 尝试读取基础文件
    let base_patterns = if managed_exists {
        match read_settings_excludes(&managed_settings) {
            Ok(p) => {
                managed_accessible = Some(true);
                p
            }
            Err(_e) => {
                managed_accessible = Some(false);
                Vec::new()
            }
        }
    } else {
        Vec::new()
    };

    // 读取 drop-in 文件
    let mut dropin_patterns: Vec<String> = Vec::new();
    if dropin_exists {
        let mut dropin_files: Vec<PathBuf> = Vec::new();
        if let Ok(entries) = fs::read_dir(&managed_dropin_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("json") {
                    dropin_files.push(path);
                }
            }
        }
        dropin_files.sort();
        for path in dropin_files {
            if let Ok(p) = read_settings_excludes(&path) {
                dropin_patterns.extend(p);
            }
        }
    }

    for p in base_patterns {
        patterns.push(ExcludePattern {
            pattern: p,
            source: "managed".to_string(),
        });
    }
    for p in dropin_patterns {
        patterns.push(ExcludePattern {
            pattern: p,
            source: "managed".to_string(),
        });
    }

    (patterns, managed_accessible)
}

pub fn read_claude_md_excludes(cwd: &Path) -> Result<ClaudeMdExcludesConfig, String> {
    let mut patterns: Vec<ExcludePattern> = Vec::new();
    let mut managed_accessible: Option<bool> = None;

    // 1. managed 层（file-based）
    if let Some(managed_dir) = resolve_managed_dir() {
        let (managed_patterns, accessible) = read_managed_excludes_from_dir(&managed_dir);
        patterns.extend(managed_patterns);
        managed_accessible = accessible;
    }

    // 2. user 层
    if let Ok(claude_dir) = super::path_resolver::resolve_claude_config_dir() {
        let user_settings = claude_dir.join("settings.json");
        if let Ok(p) = read_settings_excludes(&user_settings) {
            for pattern in p {
                patterns.push(ExcludePattern {
                    pattern,
                    source: "user".to_string(),
                });
            }
        }
    }

    // 3. project 层（git repo 子目录应使用 repo root 的 .claude/settings.json）
    let project_base = find_git_repo_root(cwd).unwrap_or_else(|| cwd.to_path_buf());
    let project_settings = project_base.join(".claude").join("settings.json");
    if let Ok(p) = read_settings_excludes(&project_settings) {
        for pattern in p {
            patterns.push(ExcludePattern {
                pattern,
                source: "project".to_string(),
            });
        }
    }

    // 4. local 层（git repo 子目录应使用 repo root 的 .claude/settings.local.json）
    let local_settings = project_base.join(".claude").join("settings.local.json");
    if let Ok(p) = read_settings_excludes(&local_settings) {
        for pattern in p {
            patterns.push(ExcludePattern {
                pattern,
                source: "local".to_string(),
            });
        }
    }

    Ok(ClaudeMdExcludesConfig {
        patterns,
        managed_accessible,
    })
}

/// 读取单个 settings.json 的 claudeMdExcludes 字段
fn read_settings_excludes(path: &Path) -> Result<Vec<String>, String> {
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let json: serde_json::Value = serde_json::from_str(&content).map_err(|e| e.to_string())?;

    let excludes = json
        .get("claudeMdExcludes")
        .and_then(|v| v.as_array())
        .unwrap_or(&Vec::new())
        .iter()
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .collect();

    Ok(excludes)
}

/// 检查文件是否被 claudeMdExcludes 排除
/// 返回匹配的 pattern 和来源（若被排除）
pub fn is_excluded<'a>(
    file_path: &'a Path,
    config: &'a ClaudeMdExcludesConfig,
) -> Option<(&'a str, &'a str)> {
    // 1. 先尝试用原始路径匹配（保留符号链接，如 /tmp/...）
    let original_str = file_path.to_string_lossy();
    for ep in &config.patterns {
        if glob_match(&ep.pattern, &original_str) {
            return Some((&ep.pattern, &ep.source));
        }
    }

    // 2. 再尝试用 canonical 路径匹配（解析符号链接，如 /private/tmp/...）
    let canonical = match fs::canonicalize(file_path) {
        Ok(p) if p != file_path => p,
        _ => return None,
    };
    let canonical_str = canonical.to_string_lossy();

    for ep in &config.patterns {
        if glob_match(&ep.pattern, &canonical_str) {
            return Some((&ep.pattern, &ep.source));
        }
    }

    None
}

/// 轻量 glob 匹配
/// 使用简单的通配符匹配（* 和 **），不引入完整 glob crate
fn glob_match(pattern: &str, path: &str) -> bool {
    // 简单实现：将 glob 模式转换为正则表达式风格的匹配
    // 处理 **（任意层级）和 *（单层级任意字符）
    let mut regex_str = String::new();
    let mut chars = pattern.chars().peekable();

    regex_str.push('^');
    while let Some(c) = chars.next() {
        match c {
            '*' => {
                if chars.peek() == Some(&'*') {
                    chars.next(); // 消费第二个 *
                    regex_str.push_str(".*");
                } else {
                    regex_str.push_str("[^/]*");
                }
            }
            '?' => regex_str.push('.'),
            '.' => regex_str.push_str("\\."),
            '+' => regex_str.push_str("\\+"),
            '(' => regex_str.push_str("\\("),
            ')' => regex_str.push_str("\\)"),
            '[' => regex_str.push_str("\\["),
            ']' => regex_str.push_str("\\]"),
            '{' => regex_str.push_str("\\{"),
            '}' => regex_str.push_str("\\}"),
            '^' => regex_str.push_str("\\^"),
            '$' => regex_str.push_str("\\$"),
            '\\' => regex_str.push_str("\\\\"),
            '/' => regex_str.push('/'),
            other => regex_str.push(other),
        }
    }
    regex_str.push('$');

    regex::Regex::new(&regex_str)
        .map(|re| re.is_match(path))
        .unwrap_or(false)
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::super::path_resolver::test_helpers::with_env_var;
    use super::*;
    use std::fs;

    /// 测试：读取 user settings.json 的 excludes
    #[test]
    fn test_read_user_settings_excludes() {
        let tmp_dir = std::env::temp_dir().join("agent-scope-settings-test");
        let _ = fs::remove_dir_all(&tmp_dir);
        fs::create_dir_all(&tmp_dir).unwrap();

        let settings_path = tmp_dir.join("settings.json");
        fs::write(
            &settings_path,
            r#"{"claudeMdExcludes": ["/tmp/test/*.md", "/secret/**"]}
"#,
        )
        .unwrap();

        let result = read_settings_excludes(&settings_path).unwrap();
        assert_eq!(result, vec!["/tmp/test/*.md", "/secret/**"]);

        let _ = fs::remove_dir_all(&tmp_dir);
    }

    /// 测试：没有 claudeMdExcludes 字段时返回空
    #[test]
    fn test_read_settings_no_excludes() {
        let tmp_dir = std::env::temp_dir().join("agent-scope-settings-empty");
        let _ = fs::remove_dir_all(&tmp_dir);
        fs::create_dir_all(&tmp_dir).unwrap();

        let settings_path = tmp_dir.join("settings.json");
        fs::write(
            &settings_path,
            r#"{"otherField": "value"}
"#,
        )
        .unwrap();

        let result = read_settings_excludes(&settings_path).unwrap();
        assert!(result.is_empty());

        let _ = fs::remove_dir_all(&tmp_dir);
    }

    /// 测试：glob 匹配 *（单层通配符）
    #[test]
    fn test_glob_match_star() {
        assert!(glob_match("/tmp/test/*.md", "/tmp/test/hello.md"));
        assert!(!glob_match("/tmp/test/*.md", "/tmp/test/sub/hello.md"));
        assert!(!glob_match("/tmp/test/*.md", "/tmp/test/hello.txt"));
    }

    /// 测试：glob 匹配 **（多层通配符）
    #[test]
    fn test_glob_match_double_star() {
        assert!(glob_match("/tmp/test/**", "/tmp/test/a.md"));
        assert!(glob_match("/tmp/test/**", "/tmp/test/sub/a.md"));
        assert!(glob_match("/tmp/test/**", "/tmp/test/deep/nested/a.md"));
        assert!(!glob_match("/tmp/test/**", "/tmp/other/a.md"));
    }

    /// 测试：多层 excludes concat 合并（使用临时 CLAUDE_CONFIG_DIR，绝不触碰真实 ~/.claude）
    #[test]
    fn test_concat_merge() {
        let tmp_dir = std::env::temp_dir().join(format!(
            "agent-scope-merge-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&tmp_dir);
        fs::create_dir_all(&tmp_dir.join(".claude")).unwrap();

        // 创建临时 user settings（通过环境变量隔离）
        let fake_claude_dir = tmp_dir.join("fake-claude");
        fs::create_dir_all(&fake_claude_dir).unwrap();
        fs::write(
            fake_claude_dir.join("settings.json"),
            r#"{"claudeMdExcludes": ["/user-pattern/**"]}"#,
        )
        .unwrap();

        fs::write(
            tmp_dir.join(".claude").join("settings.json"),
            r#"{"claudeMdExcludes": ["/project-pattern/**"]}"#,
        )
        .unwrap();

        fs::write(
            tmp_dir.join(".claude").join("settings.local.json"),
            r#"{"claudeMdExcludes": ["/local-pattern/**"]}"#,
        )
        .unwrap();

        let config = with_env_var(
            "CLAUDE_CONFIG_DIR",
            fake_claude_dir.to_str().unwrap(),
            || read_claude_md_excludes(&tmp_dir).unwrap(),
        );

        let sources: Vec<_> = config.patterns.iter().map(|p| p.source.as_str()).collect();
        assert!(sources.contains(&"user"), "应包含 user 层 excludes");
        assert!(sources.contains(&"project"), "应包含 project 层 excludes");
        assert!(sources.contains(&"local"), "应包含 local 层 excludes");

        // 清理
        let _ = fs::remove_dir_all(&tmp_dir);
    }

    /// 测试：managed-settings.json + managed-settings.d/*.json 合并 excludes
    #[test]
    fn test_managed_drop_in() {
        let tmp_dir = std::env::temp_dir().join(format!(
            "agent-scope-managed-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&tmp_dir);
        fs::create_dir_all(&tmp_dir.join("managed-settings.d")).unwrap();

        // 基础 managed-settings.json
        fs::write(
            tmp_dir.join("managed-settings.json"),
            r#"{"claudeMdExcludes": ["/base-pattern/**"]}"#,
        )
        .unwrap();

        // drop-in 文件（按字母序排在 base 之后）
        fs::write(
            tmp_dir.join("managed-settings.d").join("10-extra.json"),
            r#"{"claudeMdExcludes": ["/dropin-a/**", "/dropin-b/*.md"]}"#,
        )
        .unwrap();
        fs::write(
            tmp_dir.join("managed-settings.d").join("20-more.json"),
            r#"{"claudeMdExcludes": ["/dropin-c/**"]}"#,
        )
        .unwrap();

        let (patterns, accessible) = read_managed_excludes_from_dir(&tmp_dir);

        // 验证所有层的 excludes 都被合并
        assert_eq!(
            patterns.len(),
            4,
            "应合并 base + 2 个 drop-in 共 4 个 pattern"
        );
        assert!(accessible.unwrap_or(false), "managed 应可读");

        let pattern_strs: Vec<_> = patterns.iter().map(|p| p.pattern.as_str()).collect();
        assert!(
            pattern_strs.contains(&"/base-pattern/**"),
            "应包含 base pattern"
        );
        assert!(
            pattern_strs.contains(&"/dropin-a/**"),
            "应包含 dropin-a pattern"
        );
        assert!(
            pattern_strs.contains(&"/dropin-b/*.md"),
            "应包含 dropin-b pattern"
        );
        assert!(
            pattern_strs.contains(&"/dropin-c/**"),
            "应包含 dropin-c pattern"
        );

        // 所有 pattern 来源都应为 managed
        assert!(
            patterns.iter().all(|p| p.source == "managed"),
            "所有 pattern 来源应为 managed"
        );

        let _ = fs::remove_dir_all(&tmp_dir);
    }
}
