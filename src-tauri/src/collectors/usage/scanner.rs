use chrono::Utc;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use super::models::{
    CandidateConfigDir, ConfigDirSource, ConfidenceLevel, DirErrorReason, RealtimeLevel,
    UnreadableDir, UsageParseContext, UsageScanResult, UsageSource, UsageSourceStatus,
};
use super::parser::parse_usage_line;

// ============================================================================
// Directory Discovery
// ============================================================================

/// 发现 Claude Code 配置目录
///
/// 核心原则：发现阶段只收集候选目录，不做可读性过滤，
/// 所有过滤和诊断在扫描阶段完成。
pub fn discover_claude_config_dirs() -> Vec<CandidateConfigDir> {
    let mut candidates = Vec::new();

    // 1. 默认目录（始终加入候选列表）
    #[cfg(target_os = "macos")]
    candidates.push(CandidateConfigDir {
        raw_path: dirs::home_dir().unwrap_or_default().join(".claude"),
        canonical_path: None,
        source: ConfigDirSource::DefaultClaude,
    });

    #[cfg(target_os = "linux")]
    {
        candidates.push(CandidateConfigDir {
            raw_path: dirs::home_dir().unwrap_or_default().join(".config/claude"),
            canonical_path: None,
            source: ConfigDirSource::DefaultXdg,
        });
        candidates.push(CandidateConfigDir {
            raw_path: dirs::home_dir().unwrap_or_default().join(".claude"),
            canonical_path: None,
            source: ConfigDirSource::DefaultClaude,
        });
    }

    // Windows 默认目录（待验证，需 Phase 2 前置核查确认）
    #[cfg(target_os = "windows")]
    candidates.push(CandidateConfigDir {
        raw_path: dirs::data_dir().unwrap_or_default().join("Claude"),
        canonical_path: None,
        source: ConfigDirSource::DefaultWindows,
    });

    // 2. CLAUDE_CONFIG_DIR 环境变量（追加合并，不覆盖）
    if let Ok(env_dirs) = std::env::var("CLAUDE_CONFIG_DIR") {
        for dir in env_dirs.split(',') {
            let trimmed = dir.trim();
            if !trimmed.is_empty() {
                candidates.push(CandidateConfigDir {
                    raw_path: PathBuf::from(trimmed),
                    canonical_path: None,
                    source: ConfigDirSource::EnvClaudeConfigDir,
                });
            }
        }
    }

    // 3. 尝试 canonicalize，但不丢弃失败的目录
    for candidate in &mut candidates {
        candidate.canonical_path = candidate.raw_path.canonicalize().ok();
    }

    // 4. 去重：canonicalize 成功的用 canonical_path 去重，
    //    失败的用 raw_path 去重
    let mut seen = HashSet::new();
    candidates
        .into_iter()
        .filter(|c| {
            let key = c.canonical_path.as_ref().unwrap_or(&c.raw_path);
            seen.insert(key.clone())
        })
        .collect()
}

// ============================================================================
// File Scanning
// ============================================================================

/// 单个 JSONL 文件候选（用于跨目录去重）
#[derive(Debug, Clone)]
struct JsonlFileCandidate {
    file_path: PathBuf,
    config_dir: PathBuf,
    source: UsageSource,
    session_id: String,
    mtime: SystemTime,
}

/// 目录扫描中间结果
struct DirScanResult {
    /// 发现的 session JSONL 文件路径
    session_files: Vec<PathBuf>,
    /// 是否存在 legacy usage.jsonl
    has_legacy: bool,
}

/// 扫描 usage 数据
///
/// 扫描流程：
/// 1. 遍历每个 config_dir，做目录诊断
/// 2. 收集所有 session JSONL 文件候选（含 mtime）
/// 3. 按 session_id 跨目录去重（保留 mtime 最新的文件）
/// 4. 扫描去重后的 ClaudeJsonl 文件
/// 5. 扫描 legacy usage.jsonl（不参与去重）
pub fn scan_usage_data(config_dirs: &[CandidateConfigDir]) -> UsageScanResult {
    let scan_start = Utc::now();
    let mut all_candidates: Vec<JsonlFileCandidate> = Vec::new();
    let mut legacy_files: Vec<(PathBuf, PathBuf)> = Vec::new();
    let mut readable_dirs = Vec::new();
    let mut unreadable_dirs = Vec::new();
    let mut errors = Vec::new();

    for candidate in config_dirs {
        match scan_config_dir(candidate) {
            Ok(result) => {
                readable_dirs.push(
                    candidate.canonical_path.clone().unwrap_or(candidate.raw_path.clone()),
                );

                // 收集 session JSONL 候选
                for file_path in result.session_files {
                    let session_id = file_path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .map(|s| s.to_string())
                        .unwrap_or_default();

                    if session_id.is_empty() {
                        continue;
                    }

                    let mtime = fs::metadata(&file_path)
                        .and_then(|m| m.modified())
                        .unwrap_or(SystemTime::UNIX_EPOCH);

                    all_candidates.push(JsonlFileCandidate {
                        file_path,
                        config_dir: candidate.raw_path.clone(),
                        source: UsageSource::ClaudeJsonl,
                        session_id,
                        mtime,
                    });
                }

                // 收集 legacy 文件
                if result.has_legacy {
                    let legacy_path = candidate.raw_path.join("usage.jsonl");
                    legacy_files.push((legacy_path, candidate.raw_path.clone()));
                }
            }
            Err(reason) => {
                unreadable_dirs.push(UnreadableDir {
                    path: candidate.raw_path.clone(),
                    canonical_path: candidate.canonical_path.clone(),
                    reason,
                    detail: None,
                });
            }
        }
    }

    // 按 session_id 去重：保留 mtime 最新的文件
    let mut deduped: HashMap<String, JsonlFileCandidate> = HashMap::new();
    for candidate in all_candidates {
        let entry = deduped
            .entry(candidate.session_id.clone())
            .or_insert_with(|| candidate.clone());
        if candidate.mtime > entry.mtime {
            *entry = candidate;
        }
    }

    // 扫描去重后的 ClaudeJsonl 文件
    let mut records = Vec::new();
    let mut scanned_files = 0usize;
    let mut scanned_lines = 0usize;

    for (_, candidate) in deduped {
        match scan_jsonl_file(
            &candidate.file_path,
            &candidate.config_dir,
            candidate.source,
            &mut records,
            &mut errors,
        ) {
            Ok((files, lines)) => {
                scanned_files += files;
                scanned_lines += lines;
            }
            Err(e) => {
                errors.push(format!(
                    "扫描文件 {} 失败: {}",
                    candidate.file_path.display(),
                    e
                ));
            }
        }
    }

    // 扫描 legacy usage.jsonl（不参与去重）
    for (file_path, config_dir) in legacy_files {
        match scan_jsonl_file(
            &file_path,
            &config_dir,
            UsageSource::LegacyOrGlobalUsage,
            &mut records,
            &mut errors,
        ) {
            Ok((files, lines)) => {
                scanned_files += files;
                scanned_lines += lines;
            }
            Err(e) => {
                errors.push(format!("扫描 legacy usage.jsonl 失败: {}", e));
            }
        }
    }

    // 计算 last_usage_at
    let last_usage_at = records.iter().map(|r| r.timestamp).max();

    // 计算 confidence
    let confidence = if !records.is_empty() && !readable_dirs.is_empty() {
        ConfidenceLevel::High
    } else if readable_dirs.is_empty() {
        ConfidenceLevel::Low
    } else {
        ConfidenceLevel::Medium
    };

    let source_status = UsageSourceStatus {
        source_type: "claude_jsonl".to_string(),
        config_dirs: config_dirs
            .iter()
            .map(|c| c.raw_path.clone())
            .collect(),
        readable_dirs,
        unreadable_dirs,
        last_scan_at: Some(scan_start),
        last_usage_at,
        confidence,
        realtime_level: RealtimeLevel::Delayed,
        notes: vec![
            "统计基于 Claude Code 已写入磁盘的 usage 记录".to_string(),
            "最终用量（对话结束后）通常较可靠".to_string(),
            "实时速率是基于已写入数据的估算，不是流式精确值".to_string(),
        ],
    };

    UsageScanResult {
        records,
        source_status,
        scanned_files,
        scanned_lines,
        errors,
    }
}

/// 扫描单个配置目录
///
/// 返回目录诊断信息和发现的文件列表，不做实际解析。
fn scan_config_dir(candidate: &CandidateConfigDir) -> Result<DirScanResult, DirErrorReason> {
    let dir = &candidate.raw_path;

    // 检查目录是否存在
    if !dir.exists() {
        return Err(DirErrorReason::NotFound);
    }

    // 检查是否为目录
    if !dir.is_dir() {
        return Err(DirErrorReason::NotADirectory);
    }

    // 检查是否可读（尝试列出目录内容）
    let entries: Vec<_> = match fs::read_dir(dir) {
        Ok(e) => e.filter_map(|e| e.ok()).collect(),
        Err(e) => {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                return Err(DirErrorReason::PermissionDenied);
            }
            return Err(DirErrorReason::InvalidPath);
        }
    };

    let is_empty = entries.is_empty();

    // 检查 projects 目录是否存在
    let projects_dir = dir.join("projects");
    let has_projects = projects_dir.exists() && projects_dir.is_dir();

    // 检查 legacy usage.jsonl 是否存在
    let legacy_usage = dir.join("usage.jsonl");
    let has_legacy = legacy_usage.exists() && legacy_usage.is_file();

    // 如果目录存在但既没有 projects 也没有 usage.jsonl
    if !has_projects && !has_legacy {
        if is_empty {
            return Err(DirErrorReason::Empty);
        } else {
            return Err(DirErrorReason::MissingStructure);
        }
    }

    // 收集 projects 目录下的 session JSONL 文件（单层，不递归）
    let mut session_files = Vec::new();
    if has_projects {
        match find_project_session_jsonl_files(&projects_dir) {
            Ok(files) => session_files = files,
            Err(_) => {
                // 读取 projects 目录失败，但不中断整体流程
                // 返回空列表，让调用方继续处理
            }
        }
    }

    Ok(DirScanResult {
        session_files,
        has_legacy,
    })
}

/// 只扫描 projects 目录下一层项目目录中的 .jsonl 文件
///
/// 扫描范围: `{projects_dir}/{project-dir}/*.jsonl`
/// 不递归进入子目录（避免附件/子目录中的无关文件）
fn find_project_session_jsonl_files(
    projects_dir: &Path,
) -> Result<Vec<PathBuf>, std::io::Error> {
    let mut files = Vec::new();

    if !projects_dir.exists() || !projects_dir.is_dir() {
        return Ok(files);
    }

    for entry in fs::read_dir(projects_dir)? {
        let entry = entry?;
        let project_dir = entry.path();

        if !project_dir.is_dir() {
            continue;
        }

        // 只扫描该 project 目录下的直接子文件中的 .jsonl
        for file_entry in fs::read_dir(&project_dir)? {
            let file_entry = file_entry?;
            let path = file_entry.path();

            if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("jsonl") {
                files.push(path);
            }
        }
    }

    Ok(files)
}

/// 扫描单个 JSONL 文件
fn scan_jsonl_file(
    file_path: &Path,
    config_dir: &Path,
    source: UsageSource,
    records: &mut Vec<super::models::UsageRecord>,
    errors: &mut Vec<String>,
) -> Result<(usize, usize), String> {
    let file = match fs::File::open(file_path) {
        Ok(f) => f,
        Err(e) => return Err(format!("无法打开文件: {}", e)),
    };

    let reader = BufReader::new(file);

    // 从文件名提取 session_id
    let session_id_from_file = file_path
        .file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.to_string());

    // 从文件路径提取 project 标识
    let project_from_path = extract_project_from_path(file_path, config_dir);

    let mut line_no: u64 = 0;
    let mut scanned_lines = 0usize;

    for line_result in reader.lines() {
        line_no += 1;

        let line = match line_result {
            Ok(l) => l,
            Err(e) => {
                errors.push(format!(
                    "{}:{} 读取行失败: {}",
                    file_path.display(),
                    line_no,
                    e
                ));
                continue;
            }
        };

        scanned_lines += 1;

        let context = UsageParseContext {
            config_dir: config_dir.to_path_buf(),
            raw_file_path: file_path.to_path_buf(),
            line_no,
            session_id_from_file: session_id_from_file.clone(),
            project_from_path: project_from_path.clone(),
            source: source.clone(),
        };

        match parse_usage_line(&line, &context) {
            Ok(Some(record)) => {
                records.push(record);
            }
            Ok(None) => {
                // 非 assistant 或无 usage，正常跳过
            }
            Err(e) => {
                // 解析错误，记录但不中断
                errors.push(format!(
                    "{}:{} 解析失败: {}",
                    file_path.display(),
                    line_no,
                    e
                ));
            }
        }
    }

    Ok((1, scanned_lines))
}

/// 从文件路径提取项目标识
///
/// 文件路径格式: `{config_dir}/projects/{encoded-project-dir}/{session_id}.jsonl`
/// 提取 `{encoded-project-dir}` 作为项目标识
fn extract_project_from_path(file_path: &Path, config_dir: &Path) -> Option<String> {
    // 找到 projects 目录后的第一个子目录名
    let relative = file_path.strip_prefix(config_dir).ok()?;

    // 路径应为: projects/{project-name}/{session_id}.jsonl
    let mut components = relative.components();

    // 跳过 "projects"
    let first = components.next()?;
    if first.as_os_str() != "projects" {
        return None;
    }

    // 下一个组件是项目目录名
    let project_component = components.next()?;
    let project_name = project_component.as_os_str().to_str()?;

    Some(project_name.to_string())
}

// 环境变量测试需要串行执行，避免并行修改 CLAUDE_CONFIG_DIR 互相干扰
#[cfg(test)]
pub(crate) static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_discover_includes_default_dirs() {
        let dirs = discover_claude_config_dirs();

        // 至少应该包含默认目录
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        {
            let has_claude = dirs.iter().any(|d| {
                d.raw_path.to_string_lossy().contains(".claude")
                    && !d.raw_path.to_string_lossy().contains(".config")
            });
            assert!(has_claude, "应包含 ~/.claude 默认目录");
        }
    }

    #[test]
    fn test_discover_env_var_appends() {
        let _guard = ENV_LOCK.lock().unwrap();
        // 设置临时环境变量
        let temp_dir = std::env::temp_dir().join("agent-scope-test-claude");
        let env_val = temp_dir.to_string_lossy().to_string();
        std::env::set_var("CLAUDE_CONFIG_DIR", &env_val);

        let dirs = discover_claude_config_dirs();

        // 应该包含环境变量指定的目录
        let has_env_dir = dirs.iter().any(|d| d.raw_path == temp_dir);
        assert!(has_env_dir, "应包含 CLAUDE_CONFIG_DIR 指定的目录");

        // 清理
        std::env::remove_var("CLAUDE_CONFIG_DIR");
    }

    #[test]
    fn test_discover_deduplication() {
        let _guard = ENV_LOCK.lock().unwrap();
        // 设置与默认目录重复的环境变量
        #[cfg(target_os = "macos")]
        let default = dirs::home_dir().unwrap().join(".claude");
        #[cfg(target_os = "linux")]
        let default = dirs::home_dir().unwrap().join(".claude");

        std::env::set_var(
            "CLAUDE_CONFIG_DIR",
            default.to_string_lossy().to_string(),
        );

        let discovered = discover_claude_config_dirs();

        // 统计包含 .claude 的路径数量（去重后应该只有一个）
        let claude_dirs: Vec<_> = discovered
            .iter()
            .filter(|d| d.raw_path.to_string_lossy().contains(".claude"))
            .collect();

        assert_eq!(claude_dirs.len(), 1, "重复目录应被去重");

        std::env::remove_var("CLAUDE_CONFIG_DIR");
    }

    #[test]
    fn test_discover_keeps_failed_canonicalize() {
        let _guard = ENV_LOCK.lock().unwrap();
        // 设置一个不存在的路径
        std::env::set_var("CLAUDE_CONFIG_DIR", "/nonexistent/path/that/does/not/exist");

        let dirs = discover_claude_config_dirs();

        // 应该包含这个不存在的路径（canonicalize 失败但不丢弃）
        let has_missing = dirs.iter().any(|d| {
            d.raw_path.to_string_lossy() == "/nonexistent/path/that/does/not/exist"
                && d.canonical_path.is_none()
        });
        assert!(has_missing, "canonicalize 失败的目录应被保留");

        std::env::remove_var("CLAUDE_CONFIG_DIR");
    }

    #[test]
    fn test_extract_project_from_path() {
        let config = PathBuf::from("/home/user/.claude");
        let file = PathBuf::from("/home/user/.claude/projects/agent-scope/550e8400.jsonl");

        let project = extract_project_from_path(&file, &config);
        assert_eq!(project, Some("agent-scope".to_string()));
    }

    #[test]
    fn test_extract_project_from_path_nested() {
        let config = PathBuf::from("/home/user/.claude");
        let file =
            PathBuf::from("/home/user/.claude/projects/my-org/my-project/550e8400.jsonl");

        let project = extract_project_from_path(&file, &config);
        // 只提取 projects 后的第一个目录名
        assert_eq!(project, Some("my-org".to_string()));
    }

    #[test]
    fn test_scan_usage_data_with_test_project() {
        // 创建临时目录结构
        let temp_dir = tempfile::tempdir().unwrap();
        let projects_dir = temp_dir.path().join("projects").join("test-project");
        std::fs::create_dir_all(&projects_dir).unwrap();

        // 创建一个测试 JSONL 文件
        let jsonl_path = projects_dir.join("550e8400-e29b-41d4-a716-446655440000.jsonl");
        let mut file = std::fs::File::create(&jsonl_path).unwrap();
        writeln!(
            file,
            r#"{{"type":"assistant","timestamp":"2026-05-27T01:40:41.560Z","sessionId":"550e8400-e29b-41d4-a716-446655440000","message":{{"model":"claude-sonnet-4-6","usage":{{"input_tokens":100,"output_tokens":50,"cache_read_input_tokens":20,"cache_creation_input_tokens":10}},"stop_reason":"end_turn"}}}}"#
        ).unwrap();
        writeln!(
            file,
            r#"{{"type":"user","timestamp":"2026-05-27T01:40:41.560Z","message":{{"content":"hello"}}}}"#
        ).unwrap();
        writeln!(
            file,
            r#"{{"type":"assistant","timestamp":"2026-05-27T01:40:42.560Z","sessionId":"550e8400-e29b-41d4-a716-446655440000","message":{{"model":"claude-sonnet-4-6","usage":{{"input_tokens":200,"output_tokens":100,"cache_read_input_tokens":40,"cache_creation_input_tokens":20}},"stop_reason":"end_turn"}}}}"#
        ).unwrap();

        let candidate = CandidateConfigDir {
            raw_path: temp_dir.path().to_path_buf(),
            canonical_path: Some(temp_dir.path().canonicalize().unwrap()),
            source: ConfigDirSource::EnvClaudeConfigDir,
        };

        let result = scan_usage_data(&[candidate]);

        assert_eq!(result.records.len(), 2, "应解析出 2 条 usage 记录");
        assert_eq!(result.scanned_files, 1, "应扫描 1 个文件");
        assert_eq!(result.scanned_lines, 3, "应扫描 3 行");
        assert_eq!(
            result.source_status.confidence,
            ConfidenceLevel::High,
            "应有数据，confidence 为 High"
        );

        // 验证聚合结果
        let first = &result.records[0];
        assert_eq!(first.input_tokens, 100);
        assert_eq!(first.project_path, Some("test-project".to_string()));
        assert_eq!(first.session_id, "550e8400-e29b-41d4-a716-446655440000");

        let second = &result.records[1];
        assert_eq!(second.input_tokens, 200);
    }

    #[test]
    fn test_scan_usage_data_with_bad_json_line() {
        let temp_dir = tempfile::tempdir().unwrap();
        let projects_dir = temp_dir.path().join("projects").join("test-project");
        std::fs::create_dir_all(&projects_dir).unwrap();

        let jsonl_path = projects_dir.join("550e8400.jsonl");
        let mut file = std::fs::File::create(&jsonl_path).unwrap();
        writeln!(
            file,
            r#"{{"type":"assistant","timestamp":"2026-05-27T01:40:41.560Z","sessionId":"550e8400","message":{{"model":"claude","usage":{{"input_tokens":100,"output_tokens":50}}}}}}"#
        ).unwrap();
        writeln!(file, "{{invalid json line").unwrap(); // 坏 JSON
        writeln!(
            file,
            r#"{{"type":"assistant","timestamp":"2026-05-27T01:40:42.560Z","sessionId":"550e8400","message":{{"model":"claude","usage":{{"input_tokens":200,"output_tokens":100}}}}}}"#
        ).unwrap();

        let candidate = CandidateConfigDir {
            raw_path: temp_dir.path().to_path_buf(),
            canonical_path: Some(temp_dir.path().canonicalize().unwrap()),
            source: ConfigDirSource::EnvClaudeConfigDir,
        };

        let result = scan_usage_data(&[candidate]);

        assert_eq!(result.records.len(), 2, "坏 JSON 行不应影响其他有效行");
        assert!(
            !result.errors.is_empty(),
            "应有解析错误记录"
        );
        assert_eq!(result.scanned_lines, 3, "应扫描所有行");
    }

    #[test]
    fn test_scan_usage_data_unreadable_dir() {
        let temp_dir = tempfile::tempdir().unwrap();
        let unreadable_dir = temp_dir.path().join("unreadable");
        std::fs::create_dir(&unreadable_dir).unwrap();

        // 创建一个不存在的目录作为候选
        let missing_dir = temp_dir.path().join("does-not-exist");

        let candidates = vec![
            CandidateConfigDir {
                raw_path: unreadable_dir.clone(),
                canonical_path: Some(unreadable_dir.canonicalize().unwrap()),
                source: ConfigDirSource::EnvClaudeConfigDir,
            },
            CandidateConfigDir {
                raw_path: missing_dir.clone(),
                canonical_path: None,
                source: ConfigDirSource::EnvClaudeConfigDir,
            },
        ];

        let result = scan_usage_data(&candidates);

        assert_eq!(
            result.source_status.unreadable_dirs.len(),
            2,
            "两个目录都应被标记为不可读/无效"
        );

        // 检查错误原因
        let reasons: Vec<_> = result
            .source_status
            .unreadable_dirs
            .iter()
            .map(|d| &d.reason)
            .collect();
        assert!(reasons.contains(&&DirErrorReason::Empty), "空目录应为 Empty");
        assert!(reasons.contains(&&DirErrorReason::NotFound), "不存在目录应为 NotFound");
    }

    #[test]
    fn test_scan_usage_data_missing_structure() {
        let temp_dir = tempfile::tempdir().unwrap();

        // 创建一个非空目录，但既没有 projects 也没有 usage.jsonl
        let other_file = temp_dir.path().join("some-other-file.txt");
        std::fs::File::create(&other_file).unwrap();
        writeln!(std::fs::File::create(&other_file).unwrap(), "hello").unwrap();

        let candidate = CandidateConfigDir {
            raw_path: temp_dir.path().to_path_buf(),
            canonical_path: Some(temp_dir.path().canonicalize().unwrap()),
            source: ConfigDirSource::EnvClaudeConfigDir,
        };

        let result = scan_usage_data(&[candidate]);

        assert_eq!(
            result.source_status.unreadable_dirs.len(),
            1,
            "非空目录应被标记为 MissingStructure"
        );
        assert_eq!(
            result.source_status.unreadable_dirs[0].reason,
            DirErrorReason::MissingStructure,
            "非空目录应返回 MissingStructure"
        );
    }

    #[test]
    fn test_scan_usage_data_legacy_usage_jsonl() {
        let temp_dir = tempfile::tempdir().unwrap();

        // 创建 legacy usage.jsonl（直接放在 config_dir 下）
        let legacy_path = temp_dir.path().join("usage.jsonl");
        let mut file = std::fs::File::create(&legacy_path).unwrap();
        writeln!(
            file,
            r#"{{"type":"assistant","timestamp":"2026-05-27T01:40:41.560Z","sessionId":"legacy-session","message":{{"model":"claude","usage":{{"input_tokens":500,"output_tokens":250}}}}}}"#
        ).unwrap();

        let candidate = CandidateConfigDir {
            raw_path: temp_dir.path().to_path_buf(),
            canonical_path: Some(temp_dir.path().canonicalize().unwrap()),
            source: ConfigDirSource::EnvClaudeConfigDir,
        };

        let result = scan_usage_data(&[candidate]);

        assert_eq!(result.records.len(), 1, "应扫描 legacy usage.jsonl");
        assert_eq!(
            result.records[0].source,
            UsageSource::LegacyOrGlobalUsage,
            "legacy 文件应标记为 LegacyOrGlobalUsage"
        );
    }

    #[test]
    fn test_scan_usage_data_does_not_recurse_into_subdirs() {
        let temp_dir = tempfile::tempdir().unwrap();
        let projects_dir = temp_dir.path().join("projects").join("test-project");
        std::fs::create_dir_all(&projects_dir).unwrap();

        // 在主目录下创建 JSONL
        let main_jsonl = projects_dir.join("550e8400.jsonl");
        let mut file = std::fs::File::create(&main_jsonl).unwrap();
        writeln!(
            file,
            r#"{{"type":"assistant","timestamp":"2026-05-27T01:40:41.560Z","sessionId":"550e8400","message":{{"model":"claude","usage":{{"input_tokens":100,"output_tokens":50}}}}}}"#
        ).unwrap();

        // 在子目录下创建 JSONL（不应被扫描）
        let sub_dir = projects_dir.join("sub-session");
        std::fs::create_dir_all(&sub_dir).unwrap();
        let sub_jsonl = sub_dir.join("660e8400.jsonl");
        let mut file = std::fs::File::create(&sub_jsonl).unwrap();
        writeln!(
            file,
            r#"{{"type":"assistant","timestamp":"2026-05-27T01:40:41.560Z","sessionId":"660e8400","message":{{"model":"claude","usage":{{"input_tokens":999,"output_tokens":999}}}}}}"#
        ).unwrap();

        let candidate = CandidateConfigDir {
            raw_path: temp_dir.path().to_path_buf(),
            canonical_path: Some(temp_dir.path().canonicalize().unwrap()),
            source: ConfigDirSource::EnvClaudeConfigDir,
        };

        let result = scan_usage_data(&[candidate]);

        assert_eq!(result.records.len(), 1, "不应递归扫描子目录");
        assert_eq!(result.scanned_files, 1, "应只扫描 1 个文件");
        assert_eq!(result.records[0].input_tokens, 100, "应是主目录下的文件");
    }

    #[test]
    fn test_scan_usage_data_dedup_by_session_id_keeps_newest() {
        let temp_dir = tempfile::tempdir().unwrap();

        // 创建两个 config_dir 结构，模拟跨目录相同 session_id
        let projects1 = temp_dir.path().join("dir1").join("projects").join("project-a");
        let projects2 = temp_dir.path().join("dir2").join("projects").join("project-b");
        std::fs::create_dir_all(&projects1).unwrap();
        std::fs::create_dir_all(&projects2).unwrap();

        // 两个文件同名（相同 session_id），但内容不同
        let jsonl1 = projects1.join("same-session.jsonl");
        let mut file = std::fs::File::create(&jsonl1).unwrap();
        writeln!(
            file,
            r#"{{"type":"assistant","timestamp":"2026-05-27T01:40:41.560Z","sessionId":"same-session","message":{{"model":"claude","usage":{{"input_tokens":100,"output_tokens":50}}}}}}"#
        ).unwrap();

        // 等待一小段时间确保 mtime 不同
        thread::sleep(Duration::from_millis(100));

        let jsonl2 = projects2.join("same-session.jsonl");
        let mut file = std::fs::File::create(&jsonl2).unwrap();
        writeln!(
            file,
            r#"{{"type":"assistant","timestamp":"2026-05-27T01:40:41.560Z","sessionId":"same-session","message":{{"model":"claude","usage":{{"input_tokens":200,"output_tokens":100}}}}}}"#
        ).unwrap();

        let candidates = vec![
            CandidateConfigDir {
                raw_path: temp_dir.path().join("dir1"),
                canonical_path: Some(temp_dir.path().join("dir1").canonicalize().unwrap()),
                source: ConfigDirSource::EnvClaudeConfigDir,
            },
            CandidateConfigDir {
                raw_path: temp_dir.path().join("dir2"),
                canonical_path: Some(temp_dir.path().join("dir2").canonicalize().unwrap()),
                source: ConfigDirSource::EnvClaudeConfigDir,
            },
        ];

        let result = scan_usage_data(&candidates);

        // 应只保留 mtime 最新的文件（jsonl2，200 input_tokens）
        assert_eq!(result.records.len(), 1, "相同 session_id 应去重只保留一个");
        assert_eq!(result.scanned_files, 1, "应只扫描 1 个文件");
        assert_eq!(
            result.records[0].input_tokens, 200,
            "应保留 mtime 最新的文件"
        );
        assert_eq!(
            result.records[0].project_path,
            Some("project-b".to_string()),
            "应保留 mtime 最新的文件的路径信息"
        );
    }
}
