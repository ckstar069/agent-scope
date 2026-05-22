use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use super::frontmatter::{extract_frontmatter, parse_frontmatter, parse_list_field};
use super::models::*;
use super::path_resolver::*;
use super::secret_scanner::SecretScanner;

const MAX_PREVIEW_SIZE: usize = 2048; // 2KB
const MAX_FILE_SIZE: u64 = 1_048_576; // 1 MiB

/// 主扫描入口
pub fn scan_claude_memory(project_path: Option<String>) -> SerClaudeMemoryScanResult {
    let mut assets = Vec::new();
    let mut errors = Vec::new();
    let scanner = SecretScanner::new();

    let host_profile = build_host_profile();

    // 1. 扫描用户级资产
    if let Err(e) = scan_user_level(&mut assets, &scanner, &mut errors) {
        errors.push(SerMemoryScanError {
            scope: "user".to_string(),
            path: "~/.claude/".to_string(),
            message: e,
        });
    }

    // 2. 扫描已注册项目（通过 registry 传入的项目列表）
    // 注：registry 列表由调用方提供，避免在此引入 AppState 依赖

    // 3. 扫描额外的 project_path
    if let Some(extra) = project_path {
        let path = PathBuf::from(&extra);
        if !path.exists() {
            errors.push(SerMemoryScanError {
                scope: "project".to_string(),
                path: extra.clone(),
                message: "目录不存在".to_string(),
            });
        } else if !path.is_dir() {
            errors.push(SerMemoryScanError {
                scope: "project".to_string(),
                path: extra.clone(),
                message: "路径不是目录".to_string(),
            });
        } else {
            scan_project_level(&path, &mut assets, &scanner, &mut errors);
        }
    }

    let summary = build_summary(&assets);

    SerClaudeMemoryScanResult {
        scanned_at_ms: now_ms(),
        host_profile,
        assets,
        summary,
        errors,
    }
}

/// 扫描用户级资产（~/.claude/ 下）
fn scan_user_level(
    assets: &mut Vec<SerClaudeMemoryAsset>,
    scanner: &SecretScanner,
    errors: &mut Vec<SerMemoryScanError>,
) -> Result<(), String> {
    let claude_dir = resolve_claude_config_dir()?;

    // ~/.claude/CLAUDE.md
    let user_claude_md = claude_dir.join("CLAUDE.md");
    if user_claude_md.exists() {
        assets.push(scan_single_file(
            &user_claude_md,
            "user",
            "user_claude_md",
            scanner,
        ));
    } else {
        assets.push(missing_asset(&user_claude_md, "user", "user_claude_md"));
    }

    // ~/.claude/rules/*.md
    let rules_dir = claude_dir.join("rules");
    scan_dir_md_files(&rules_dir, "user", "global_rule", assets, scanner, errors);

    // ~/.claude/skills/*/SKILL.md
    let skills_dir = claude_dir.join("skills");
    scan_skills_dir(&skills_dir, "user", "global_skill", assets, scanner, errors);

    // ~/.claude/agents/*.md
    let agents_dir = claude_dir.join("agents");
    scan_dir_md_files(&agents_dir, "user", "global_agent", assets, scanner, errors);

    // ~/.claude/projects/<id>/memory/*.md
    let projects_dir = claude_dir.join("projects");
    scan_auto_memory_dir(&projects_dir, assets, scanner, errors);

    Ok(())
}

/// 扫描单个项目目录
pub fn scan_project_level(
    project_root: &Path,
    assets: &mut Vec<SerClaudeMemoryAsset>,
    scanner: &SecretScanner,
    errors: &mut Vec<SerMemoryScanError>,
) {
    // <repo>/CLAUDE.md
    let claude_md = project_root.join("CLAUDE.md");
    if claude_md.exists() {
        assets.push(scan_single_file(
            &claude_md,
            "project",
            "project_claude_md",
            scanner,
        ));
    } else {
        assets.push(missing_asset(&claude_md, "project", "project_claude_md"));
    }

    // <repo>/CLAUDE.local.md
    let local_md = project_root.join("CLAUDE.local.md");
    if local_md.exists() {
        assets.push(scan_single_file(&local_md, "local", "local_md", scanner));
    } else {
        assets.push(missing_asset(&local_md, "local", "local_md"));
    }

    // <repo>/.claude/CLAUDE.md
    let dot_claude_md = project_root.join(".claude").join("CLAUDE.md");
    if dot_claude_md.exists() {
        assets.push(scan_single_file(
            &dot_claude_md,
            "project",
            "project_dot_claude_md",
            scanner,
        ));
    } else {
        assets.push(missing_asset(
            &dot_claude_md,
            "project",
            "project_dot_claude_md",
        ));
    }

    // <repo>/.claude/rules/*.md
    let rules_dir = project_root.join(".claude").join("rules");
    scan_dir_md_files(
        &rules_dir,
        "project",
        "project_rule",
        assets,
        scanner,
        errors,
    );

    // <repo>/.claude/skills/*/SKILL.md
    let skills_dir = project_root.join(".claude").join("skills");
    scan_skills_dir(
        &skills_dir,
        "project",
        "project_skill",
        assets,
        scanner,
        errors,
    );

    // <repo>/.claude/agents/*.md
    let agents_dir = project_root.join(".claude").join("agents");
    scan_dir_md_files(
        &agents_dir,
        "project",
        "project_agent",
        assets,
        scanner,
        errors,
    );
}

/// 扫描 auto memory 目录 ~/.claude/projects/<id>/memory/
fn scan_auto_memory_dir(
    projects_dir: &Path,
    assets: &mut Vec<SerClaudeMemoryAsset>,
    scanner: &SecretScanner,
    errors: &mut Vec<SerMemoryScanError>,
) {
    let entries = match fs::read_dir(projects_dir) {
        Ok(e) => e,
        Err(e) => {
            // NotFound 时静默返回（没有 auto memory 是空状态，不是错误）
            if e.kind() == std::io::ErrorKind::NotFound {
                return;
            }
            errors.push(SerMemoryScanError {
                scope: "auto".to_string(),
                path: projects_dir.to_string_lossy().to_string(),
                message: format!("读取 projects 目录失败: {}", e),
            });
            return;
        }
    };

    for entry in entries.flatten() {
        let memory_dir = entry.path().join("memory");
        if !memory_dir.exists() || !memory_dir.is_dir() {
            continue;
        }

        let md_entries = match fs::read_dir(&memory_dir) {
            Ok(e) => e,
            Err(e) => {
                errors.push(SerMemoryScanError {
                    scope: "auto".to_string(),
                    path: memory_dir.to_string_lossy().to_string(),
                    message: format!("读取 memory 目录失败: {}", e),
                });
                continue;
            }
        };

        for md_entry in md_entries.flatten() {
            let path = md_entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }

            let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            let asset_type = if file_name == "MEMORY.md" {
                "auto_memory_index"
            } else {
                "auto_memory_topic"
            };

            assets.push(scan_single_file(&path, "auto", asset_type, scanner));
        }
    }
}

/// 扫描目录下所有 .md 文件
fn scan_dir_md_files(
    dir: &Path,
    scope: &str,
    asset_type: &str,
    assets: &mut Vec<SerClaudeMemoryAsset>,
    scanner: &SecretScanner,
    errors: &mut Vec<SerMemoryScanError>,
) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            if e.kind() != std::io::ErrorKind::NotFound {
                errors.push(SerMemoryScanError {
                    scope: scope.to_string(),
                    path: dir.to_string_lossy().to_string(),
                    message: format!("读取目录失败: {}", e),
                });
            }
            return;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        if !path.is_file() {
            continue;
        }
        assets.push(scan_single_file(&path, scope, asset_type, scanner));
    }
}

/// 扫描 skills 目录（skills/<name>/SKILL.md）
fn scan_skills_dir(
    dir: &Path,
    scope: &str,
    asset_type: &str,
    assets: &mut Vec<SerClaudeMemoryAsset>,
    scanner: &SecretScanner,
    errors: &mut Vec<SerMemoryScanError>,
) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            if e.kind() != std::io::ErrorKind::NotFound {
                errors.push(SerMemoryScanError {
                    scope: scope.to_string(),
                    path: dir.to_string_lossy().to_string(),
                    message: format!("读取 skills 目录失败: {}", e),
                });
            }
            return;
        }
    };

    for entry in entries.flatten() {
        let skill_dir = entry.path();
        if !skill_dir.is_dir() {
            continue;
        }
        let skill_md = skill_dir.join("SKILL.md");
        if skill_md.exists() && skill_md.is_file() {
            assets.push(scan_single_file(&skill_md, scope, asset_type, scanner));
        }
    }
}

/// 读取文件前 max_bytes 字节内容（UTF-8 安全截断）
fn read_file_head(path: &Path, max_bytes: usize) -> Option<String> {
    use std::io::Read;

    let mut file = fs::File::open(path).ok()?;
    let mut buf = vec![0u8; max_bytes];
    let n = file.read(&mut buf).ok()?;
    buf.truncate(n);

    Some(String::from_utf8_lossy(&buf).to_string())
}

/// 扫描单个文件，生成 asset
/// 大文件策略：超过 MAX_FILE_SIZE 时只读取前 MAX_PREVIEW_SIZE 字节，
/// secret_scanner 和 frontmatter 只基于 preview 运行，line_count 设为 None。
fn scan_single_file(
    path: &Path,
    scope: &str,
    asset_type: &str,
    scanner: &SecretScanner,
) -> SerClaudeMemoryAsset {
    let id = file_id(path);
    let native_path = path.to_string_lossy().to_string();
    let logical_path = native_path.replace('\\', "/");

    let metadata = fs::metadata(path).ok();
    let byte_size = metadata.as_ref().map(|m| m.len());
    let mtime_ms = metadata
        .as_ref()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as u64);

    let file_len = byte_size.unwrap_or(0);
    let is_large = file_len > MAX_FILE_SIZE;

    let (content_preview, content_truncated, line_count, frontmatter, secret_issues) = if is_large {
        // 大文件：只读 preview，line_count 设为 None
        let preview = read_file_head(path, MAX_PREVIEW_SIZE);
        let frontmatter = preview.as_ref().and_then(|content| {
            if path.extension().and_then(|e| e.to_str()) == Some("md") {
                parse_file_frontmatter(content, asset_type, path)
            } else {
                None
            }
        });
        let secret_issues = preview
            .as_ref()
            .map(|content| scanner.scan(content))
            .unwrap_or_default();
        (preview, true, None, frontmatter, secret_issues)
    } else {
        match fs::read_to_string(path) {
            Ok(content) => {
                let line_count = Some(content.lines().count());
                let preview = if content.len() > MAX_PREVIEW_SIZE {
                    // 安全截断
                    let mut end = MAX_PREVIEW_SIZE;
                    while end > 0 && !content.is_char_boundary(end) {
                        end -= 1;
                    }
                    Some(content[..end].to_string())
                } else {
                    Some(content.clone())
                };
                let frontmatter = if path.extension().and_then(|e| e.to_str()) == Some("md") {
                    parse_file_frontmatter(&content, asset_type, path)
                } else {
                    None
                };
                let secret_issues = scanner.scan(&content);
                (preview, false, line_count, frontmatter, secret_issues)
            }
            Err(_) => (None, false, None, None, Vec::new()),
        }
    };

    SerClaudeMemoryAsset {
        id,
        scope: scope.to_string(),
        asset_type: asset_type.to_string(),
        logical_path,
        native_path,
        content_hash: None,
        content_preview,
        content_truncated,
        line_count,
        byte_size,
        mtime_ms,
        frontmatter,
        secret_issues,
        exists: true,
    }
}

/// 不存在的文件占位 asset
fn missing_asset(path: &Path, scope: &str, asset_type: &str) -> SerClaudeMemoryAsset {
    let id = file_id(path);
    let native_path = path.to_string_lossy().to_string();
    let logical_path = native_path.replace('\\', "/");

    SerClaudeMemoryAsset {
        id,
        scope: scope.to_string(),
        asset_type: asset_type.to_string(),
        logical_path,
        native_path,
        content_hash: None,
        content_preview: None,
        content_truncated: false,
        line_count: None,
        byte_size: None,
        mtime_ms: None,
        frontmatter: None,
        secret_issues: Vec::new(),
        exists: false,
    }
}

/// 解析文件的 frontmatter
fn parse_file_frontmatter(content: &str, asset_type: &str, path: &Path) -> Option<SerFrontmatter> {
    let fallback = fallback_name(asset_type, path);

    if let Some((raw, _)) = extract_frontmatter(content) {
        let pairs = parse_frontmatter(raw);
        let map: HashMap<String, String> = pairs.into_iter().collect();

        let name = map.get("name").cloned().or(fallback);
        let description = map.get("description").cloned();
        let trigger = map.get("trigger").cloned();
        let paths = parse_list_field(raw, "paths");
        let memory_scope = map.get("memory_scope").cloned();

        Some(SerFrontmatter {
            name,
            description,
            trigger,
            paths,
            memory_scope,
            raw: raw.to_string(),
        })
    } else {
        // 没有 frontmatter，用 fallback name（如果有）
        fallback.map(|name| SerFrontmatter {
            name: Some(name),
            description: None,
            trigger: None,
            paths: None,
            memory_scope: None,
            raw: String::new(),
        })
    }
}

/// frontmatter name 缺失时的回退
fn fallback_name(asset_type: &str, path: &Path) -> Option<String> {
    if asset_type.contains("skill") {
        // 使用父目录名（skills/<name>/SKILL.md）
        path.parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
    } else if asset_type.contains("agent") {
        // 使用文件名（不含扩展名）
        path.file_stem()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
    } else {
        None
    }
}

/// 生成文件唯一 ID（基于 native_path 的简单 hash）
fn file_id(path: &Path) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    path.to_string_lossy().hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

/// 构建扫描摘要
fn build_summary(assets: &[SerClaudeMemoryAsset]) -> SerMemorySummary {
    let total_assets = assets.len();
    let total_existing = assets.iter().filter(|a| a.exists).count();
    let total_secret_issues: usize = assets.iter().map(|a| a.secret_issues.len()).sum();

    let mut by_scope = HashMap::new();
    let mut by_type = HashMap::new();

    for asset in assets {
        *by_scope.entry(asset.scope.clone()).or_insert(0) += 1;
        *by_type.entry(asset.asset_type.clone()).or_insert(0) += 1;
    }

    SerMemorySummary {
        total_assets,
        total_existing,
        by_scope,
        by_type,
        total_secret_issues,
    }
}

/// 构建 HostProfile
fn build_host_profile() -> SerHostProfile {
    let os = std::env::consts::OS.to_string();
    let home_dir = dirs::home_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
    let claude_config_dir = resolve_claude_config_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    let hostname = std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "unknown".to_string());

    let user_name = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "unknown".to_string());

    let host_id = {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        os.hash(&mut hasher);
        home_dir.hash(&mut hasher);
        user_name.hash(&mut hasher);
        hostname.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    };

    SerHostProfile {
        host_id,
        hostname,
        os,
        home_dir,
        claude_config_dir,
        user_name,
    }
}

/// 当前 Unix 时间戳（毫秒）
fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn test_scanner() -> SecretScanner {
        SecretScanner::new()
    }

    /// 测试：扫描不存在的目录应记录错误但不 panic
    #[test]
    fn test_scan_nonexistent_project_path() {
        let result = scan_claude_memory(Some("/tmp/__nonexistent_agent_scope_test__".to_string()));
        assert!(
            result.errors.iter().any(|e| e.message == "目录不存在"),
            "应包含'目录不存在'错误，实际 errors: {:?}",
            result.errors
        );
    }

    /// 测试：扫描文件（而非目录）应记录错误
    #[test]
    fn test_scan_file_as_project_path() {
        let tmp = std::env::temp_dir().join("agent-scope-test-file.tmp");
        fs::write(&tmp, "test").unwrap();

        let result = scan_claude_memory(Some(tmp.to_string_lossy().to_string()));
        assert!(
            result.errors.iter().any(|e| e.message == "路径不是目录"),
            "应包含'路径不是目录'错误，实际 errors: {:?}",
            result.errors
        );

        let _ = fs::remove_file(&tmp);
    }

    /// 测试：扫描真实临时目录
    #[test]
    fn test_scan_real_directory() {
        let tmp = std::env::temp_dir().join("agent-scope-scan-test");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        fs::write(tmp.join("CLAUDE.md"), "# Project\n").unwrap();

        let scanner = test_scanner();
        let mut assets = Vec::new();
        let mut errors = Vec::new();
        scan_project_level(&tmp, &mut assets, &scanner, &mut errors);

        assert!(assets
            .iter()
            .any(|a| a.asset_type == "project_claude_md" && a.exists));

        let _ = fs::remove_dir_all(&tmp);
    }

    /// 测试：frontmatter 解析
    #[test]
    fn test_frontmatter_in_scan() {
        let tmp = std::env::temp_dir().join("agent-scope-fm-test");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp.join(".claude").join("skills").join("git-helper")).unwrap();
        fs::write(
            tmp.join(".claude").join("skills").join("git-helper").join("SKILL.md"),
            "---\nname: git-commit-helper\ndescription: help with commits\ntrigger: on request\n---\n# Git Helper\n",
        )
        .unwrap();

        let scanner = test_scanner();
        let mut assets = Vec::new();
        let mut errors = Vec::new();
        scan_project_level(&tmp, &mut assets, &scanner, &mut errors);

        let skill = assets
            .iter()
            .find(|a| a.asset_type == "project_skill" && a.exists)
            .expect("应找到 skill");
        let fm = skill.frontmatter.as_ref().expect("应解析 frontmatter");
        assert_eq!(fm.name, Some("git-commit-helper".to_string()));
        assert_eq!(fm.description, Some("help with commits".to_string()));
        assert_eq!(fm.trigger, Some("on request".to_string()));

        let _ = fs::remove_dir_all(&tmp);
    }

    /// 测试：skill name 缺失时用目录名兜底
    #[test]
    fn test_skill_name_fallback() {
        let tmp = std::env::temp_dir().join("agent-scope-fallback-test");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp.join(".claude").join("skills").join("my-skill")).unwrap();
        fs::write(
            tmp.join(".claude")
                .join("skills")
                .join("my-skill")
                .join("SKILL.md"),
            "# No Frontmatter\n",
        )
        .unwrap();

        let scanner = test_scanner();
        let mut assets = Vec::new();
        let mut errors = Vec::new();
        scan_project_level(&tmp, &mut assets, &scanner, &mut errors);

        let skill = assets
            .iter()
            .find(|a| a.asset_type == "project_skill" && a.exists)
            .expect("应找到 skill");
        let fm = skill.frontmatter.as_ref().expect("应解析 frontmatter");
        assert_eq!(fm.name, Some("my-skill".to_string()));

        let _ = fs::remove_dir_all(&tmp);
    }

    /// 测试：大文件截断
    #[test]
    fn test_large_file_truncation() {
        let tmp = std::env::temp_dir().join("agent-scope-large-test");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        // 写入超过 1 MiB 的文件
        let large_content = "x".repeat(1_100_000);
        fs::write(tmp.join("CLAUDE.md"), &large_content).unwrap();

        let scanner = test_scanner();
        let mut assets = Vec::new();
        let mut errors = Vec::new();
        scan_project_level(&tmp, &mut assets, &scanner, &mut errors);

        let asset = assets
            .iter()
            .find(|a| a.asset_type == "project_claude_md" && a.exists)
            .expect("应找到 asset");
        assert!(asset.content_truncated);
        assert!(asset.content_preview.is_some());
        assert!(asset.content_preview.as_ref().unwrap().len() <= MAX_PREVIEW_SIZE);

        let _ = fs::remove_dir_all(&tmp);
    }

    /// 测试：summary 统计正确
    #[test]
    fn test_summary_counts() {
        let tmp = std::env::temp_dir().join("agent-scope-summary-test");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        fs::write(tmp.join("CLAUDE.md"), "# OK\n").unwrap();

        let scanner = test_scanner();
        let mut assets = Vec::new();
        let mut errors = Vec::new();
        scan_project_level(&tmp, &mut assets, &scanner, &mut errors);

        let summary = build_summary(&assets);
        assert!(summary.total_assets > 0);
        assert!(summary.by_scope.contains_key("project") || summary.by_scope.contains_key("local"));

        let _ = fs::remove_dir_all(&tmp);
    }

    /// 测试：host_profile 生成
    #[test]
    fn test_host_profile() {
        let hp = build_host_profile();
        assert!(!hp.host_id.is_empty());
        assert!(!hp.hostname.is_empty());
        assert!(!hp.os.is_empty());
        assert!(!hp.home_dir.is_empty());
        assert!(!hp.user_name.is_empty());
    }
}
