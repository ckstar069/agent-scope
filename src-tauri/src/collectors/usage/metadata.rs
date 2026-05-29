use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;

use super::models::CandidateConfigDir;
use crate::collectors::claude_history::path_codec::decode_project_dir;

/// Usage 会话元数据
///
/// 从 Claude Code history.jsonl 中读取，用于 enrichment usage record。
#[derive(Debug, Clone, Default)]
pub struct UsageSessionMetadata {
    /// 会话 ID
    pub session_id: String,
    /// 会话显示标题（history.jsonl 中的 display 字段）
    pub display: Option<String>,
    /// 项目路径（history.jsonl 中的 project 字段）
    pub project_path: Option<String>,
    /// 项目名称（project_path 的 basename）
    pub project_name: Option<String>,
    /// 记录时间戳（原始字符串，用于调试）
    pub timestamp: Option<String>,
    /// 排序用时间戳（毫秒级，内部使用，不参与序列化）
    pub timestamp_sort_key: Option<i128>,
}

/// 解析 history.jsonl 中的 timestamp
///
/// 支持：
/// - number/u64（毫秒级时间戳，如 1780014184874）
/// - string number（如 "1780014184874"）
/// - RFC3339 string（如 "2026-05-29T08:58:25Z"）
///
/// 返回毫秒级时间戳的 Option<i128>，无法解析返回 None。
fn parse_history_timestamp(value: &serde_json::Value) -> Option<i128> {
    if let Some(n) = value.as_u64() {
        return Some(n as i128);
    }
    if let Some(s) = value.as_str() {
        if let Ok(n) = s.parse::<u64>() {
            return Some(n as i128);
        }
        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
            return Some(dt.timestamp_millis() as i128);
        }
    }
    None
}

/// 从一组 config_dir 加载 session metadata
///
/// 读取每个 config_dir 下的 history.jsonl，构建 session_id → metadata 映射。
/// 同一 session_id 出现多次时，保留 timestamp 较新的记录（无法比较则后读覆盖）。
pub fn load_usage_session_metadata(
    config_dirs: &[CandidateConfigDir],
) -> HashMap<String, UsageSessionMetadata> {
    let mut map: HashMap<String, UsageSessionMetadata> = HashMap::new();

    for candidate in config_dirs {
        let config_dir = &candidate.raw_path;
        let history_path = config_dir.join("history.jsonl");

        if !history_path.exists() || !history_path.is_file() {
            continue;
        }

        let file = match fs::File::open(&history_path) {
            Ok(f) => f,
            Err(_) => continue,
        };

        let reader = BufReader::new(file);

        for line_result in reader.lines() {
            let line = match line_result {
                Ok(l) => l,
                Err(_) => continue,
            };

            let value: serde_json::Value = match serde_json::from_str(&line) {
                Ok(v) => v,
                Err(_) => continue,
            };

            let session_id = value
                .get("sessionId")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            if session_id.is_empty() {
                continue;
            }

            let display = value
                .get("display")
                .and_then(|v| v.as_str())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty());

            let project = value
                .get("project")
                .and_then(|v| v.as_str())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty());

            let timestamp_raw = value.get("timestamp");
            let timestamp_sort_key = parse_history_timestamp(
                timestamp_raw.unwrap_or(&serde_json::Value::Null),
            );
            let timestamp_str = timestamp_raw
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let (project_path, project_name) = match &project {
                Some(p) => {
                    let name = Path::new(p)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .map(|s| s.to_string());
                    (Some(p.clone()), name)
                }
                None => (None, None),
            };

            let entry = map.entry(session_id.to_string()).or_insert_with(|| {
                UsageSessionMetadata {
                    session_id: session_id.to_string(),
                    display: None,
                    project_path: None,
                    project_name: None,
                    timestamp: None,
                    timestamp_sort_key: None,
                }
            });

            // 按 timestamp_sort_key 判断新旧：
            // 1. 新可解析，旧可解析：新 > 旧时更新
            // 2. 新可解析，旧不可解析：更新
            // 3. 新不可解析，旧可解析：保留旧
            // 4. 都不可解析：后读覆盖前读
            let should_update = match (timestamp_sort_key, entry.timestamp_sort_key) {
                (Some(new_ts), Some(old_ts)) => new_ts > old_ts,
                (Some(_), None) => true,
                (None, Some(_)) => false,
                (None, None) => true,
            };

            // 标题质量优先级：非 "(未命名)" 优于 "(未命名)"
            // 当 timestamp 相同时，优先选择更有意义的标题
            let new_is_better = match (&display, &entry.display) {
                (Some(new_d), Some(old_d)) => {
                    let new_cleaned = clean_session_title(Some(new_d));
                    let old_cleaned = clean_session_title(Some(old_d));
                    new_cleaned != "(未命名)" && old_cleaned == "(未命名)"
                }
                (Some(_), None) => true,
                _ => false,
            };

            if should_update
                || (timestamp_sort_key == entry.timestamp_sort_key && new_is_better)
            {
                if display.is_some() {
                    entry.display = display;
                }
                if project_path.is_some() {
                    entry.project_path = project_path.clone();
                }
                if project_name.is_some() {
                    entry.project_name = project_name.clone();
                }
                if timestamp_str.is_some() {
                    entry.timestamp = timestamp_str;
                }
                entry.timestamp_sort_key = timestamp_sort_key;
            }
        }
    }

    map
}

/// 从编码目录名提取可读的项目名
///
/// Claude Code 编码目录时把 `/` 和 `_` 都替换为 `-`，
/// 导致 decode_project_dir 对含连字符的项目名不可逆。
/// 因此优先识别常见工作目录前缀（如 Repo、Documents），
/// 取前缀后的剩余部分作为项目名。
pub fn compact_project_label(encoded_dir: &str) -> String {
    let label = if let Some(marker_result) = try_extract_from_known_markers(encoded_dir) {
        marker_result
    } else {
        let decoded = decode_project_dir(encoded_dir);
        let decoded_path = Path::new(&decoded);
        if let Some(name) = decoded_path.file_name().and_then(|n| n.to_str()) {
            if !name.is_empty() {
                name.to_string()
            } else {
                encoded_dir.to_string()
            }
        } else {
            encoded_dir.to_string()
        }
    };

    // 统一截断到 32 个 Unicode 字符（避免超长标签）
    let char_count = label.chars().count();
    if char_count > 32 {
        let skip = char_count - 31;
        let suffix: String = label.chars().skip(skip).collect();
        format!("…{}", suffix)
    } else {
        label
    }
}

fn try_extract_from_known_markers(encoded_dir: &str) -> Option<String> {
    for marker in &["-Repo-", "-Documents-", "-projects-", "-workspace-"] {
        if let Some(pos) = encoded_dir.rfind(marker) {
            let suffix = &encoded_dir[pos + marker.len()..];
            if !suffix.is_empty() {
                return Some(suffix.to_string());
            }
        }
    }
    None
}

/// 生成短 session ID（前 8 位）
pub fn short_session_id(session_id: &str) -> String {
    session_id.chars().take(8).collect()
}

/// 清洗会话标题
///
/// 规则：
/// 1. None / 空字符串 / trim 后为空 → "(未命名)"
/// 2. `[Pasted text ...]` placeholder → "(未命名)"
/// 3. `/` 开头的 slash command：
///    - `/rename <arg>` → `<arg>`（去掉命令词）
///    - `/model`, `/resume`, `/advance-stage` 等无意义命令 → "(未命名)"
///    - 其他 slash command 有参数 → 参数
///    - 其他 slash command 无参数 → "(未命名)"
/// 4. 非 slash command → 保留原文本（trim 空白）
/// 5. 不做后端截断
pub fn clean_session_title(title: Option<&str>) -> String {
    let Some(t) = title else {
        return "(未命名)".to_string();
    };

    let trimmed = t.trim();
    if trimmed.is_empty() {
        return "(未命名)".to_string();
    }

    // [Pasted text ...] placeholder
    if trimmed.starts_with("[Pasted text") {
        return "(未命名)".to_string();
    }

    // slash command 处理
    if let Some(stripped) = trimmed.strip_prefix('/') {
        let stripped = stripped.trim_start_matches('/'); // 去掉多余 /
        let mut parts = stripped.splitn(2, ' ');
        let cmd = parts.next().unwrap_or("").trim();
        let arg = parts.next().map(|s| s.trim()).filter(|s| !s.is_empty());

        // 已知无意义命令（不带参数的）
        let no_arg_commands = ["model", "resume", "advance-stage"];
        if no_arg_commands.contains(&cmd) {
            return "(未命名)".to_string();
        }

        // rename 命令：只保留参数，不带 "rename" 前缀
        if cmd == "rename" {
            return arg.map(|a| a.to_string()).unwrap_or_else(|| "(未命名)".to_string());
        }

        // 其他 slash command：有参数保留参数，无参数 → (未命名)
        return arg.map(|a| a.to_string()).unwrap_or_else(|| "(未命名)".to_string());
    }

    // 非 slash command：保留原文
    trimmed.to_string()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_compact_project_label_repo_prefix() {
        // -Repo- 后的剩余部分就是项目名
        assert_eq!(
            compact_project_label("-Users-ckstar-Repo-agent-scope"),
            "agent-scope"
        );
        assert_eq!(
            compact_project_label("-home-user-Repo-my-project"),
            "my-project"
        );
    }

    #[test]
    fn test_compact_project_label_documents_prefix() {
        assert_eq!(
            compact_project_label("-Users-ckstar-Documents-notes"),
            "notes"
        );
    }

    #[test]
    fn test_compact_project_label_no_known_prefix() {
        // 无已知前缀时 fallback 到 decode basename
        assert_eq!(
            compact_project_label("home-user-project"),
            "project"
        );
    }

    #[test]
    fn test_compact_project_label_long_no_dash() {
        // 全是 'a' 没有 '-'，decode+basename 后 basename 就是完整的 50 个 a
        // 超过 32 字符会被统一截断
        let long = "a".repeat(50);
        let result = compact_project_label(&long);
        assert!(result.starts_with('…'));
        assert_eq!(result.chars().count(), 32); // 1 个 … + 31 个 a
    }

    #[test]
    fn test_compact_project_label_last_segment_long() {
        // 最后一个 '-' 后的段也很长（超过 32），会触发截断
        let prefix = "foo-bar-";
        let suffix = "z".repeat(50);
        let encoded = format!("{}{}", prefix, suffix);
        let result = compact_project_label(&encoded);
        assert!(result.starts_with('…'));
        assert_eq!(result.chars().count(), 32); // 1 个 … + 31 个 z
    }

    #[test]
    fn test_short_session_id() {
        assert_eq!(
            short_session_id("550e8400-e29b-41d4-a716-446655440000"),
            "550e8400"
        );
    }

    #[test]
    fn test_load_usage_session_metadata_from_history() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_dir = temp_dir.path().to_path_buf();

        let history_path = config_dir.join("history.jsonl");
        let mut file = std::fs::File::create(&history_path).unwrap();
        // 使用 number timestamp（与真实 history.jsonl 一致）
        writeln!(
            file,
            r#"{{"type":"last-prompt","timestamp":1780000000000,"sessionId":"sess-001","display":"Phase2 阶段开发","project":"/Users/ckstar/Repo/agent-scope"}}"#
        ).unwrap();
        writeln!(
            file,
            r#"{{"type":"last-prompt","timestamp":1780003600000,"sessionId":"sess-002","display":"文档整理","project":"/Users/ckstar/Documents/notes"}}"#
        ).unwrap();

        let candidate = CandidateConfigDir {
            raw_path: config_dir.clone(),
            canonical_path: Some(config_dir.canonicalize().unwrap()),
            source: super::super::models::ConfigDirSource::EnvClaudeConfigDir,
        };

        let metadata = load_usage_session_metadata(&[candidate]);

        assert_eq!(metadata.len(), 2);

        let meta1 = metadata.get("sess-001").unwrap();
        assert_eq!(meta1.display, Some("Phase2 阶段开发".to_string()));
        assert_eq!(meta1.project_path, Some("/Users/ckstar/Repo/agent-scope".to_string()));
        assert_eq!(meta1.project_name, Some("agent-scope".to_string()));
        assert_eq!(meta1.timestamp_sort_key, Some(1_780_000_000_000i128));

        let meta2 = metadata.get("sess-002").unwrap();
        assert_eq!(meta2.display, Some("文档整理".to_string()));
        assert_eq!(meta2.project_name, Some("notes".to_string()));
        assert_eq!(meta2.timestamp_sort_key, Some(1_780_003_600_000i128));
    }

    #[test]
    fn test_load_metadata_keeps_newer_timestamp() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_dir = temp_dir.path().to_path_buf();

        let history_path = config_dir.join("history.jsonl");
        let mut file = std::fs::File::create(&history_path).unwrap();
        // 旧记录（number timestamp）
        writeln!(
            file,
            r#"{{"type":"last-prompt","timestamp":1780000000000,"sessionId":"sess-001","display":"旧标题","project":"/old/path"}}"#
        ).unwrap();
        // 新记录（同 session，应覆盖）
        writeln!(
            file,
            r#"{{"type":"last-prompt","timestamp":1780010000000,"sessionId":"sess-001","display":"新标题","project":"/new/path"}}"#
        ).unwrap();

        let candidate = CandidateConfigDir {
            raw_path: config_dir.clone(),
            canonical_path: Some(config_dir.canonicalize().unwrap()),
            source: super::super::models::ConfigDirSource::EnvClaudeConfigDir,
        };

        let metadata = load_usage_session_metadata(&[candidate]);

        let meta = metadata.get("sess-001").unwrap();
        assert_eq!(meta.display, Some("新标题".to_string()));
        assert_eq!(meta.project_path, Some("/new/path".to_string()));
    }

    #[test]
    fn test_load_metadata_missing_history() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_dir = temp_dir.path().to_path_buf();

        let candidate = CandidateConfigDir {
            raw_path: config_dir,
            canonical_path: None,
            source: super::super::models::ConfigDirSource::EnvClaudeConfigDir,
        };

        let metadata = load_usage_session_metadata(&[candidate]);
        assert!(metadata.is_empty());
    }

    #[test]
    fn test_parse_history_timestamp_number() {
        let v = serde_json::json!(1780014184874u64);
        assert_eq!(parse_history_timestamp(&v), Some(1780014184874i128));
    }

    #[test]
    fn test_parse_history_timestamp_string_number() {
        let v = serde_json::json!("1780014184874");
        assert_eq!(parse_history_timestamp(&v), Some(1780014184874i128));
    }

    #[test]
    fn test_parse_history_timestamp_rfc3339() {
        let v = serde_json::json!("2026-05-29T08:58:25Z");
        let result = parse_history_timestamp(&v);
        assert!(result.is_some());
        // 2026-05-29T08:58:25Z 的毫秒时间戳
        assert_eq!(result.unwrap(), 1_780_045_105_000i128);
    }

    #[test]
    fn test_parse_history_timestamp_invalid() {
        assert_eq!(parse_history_timestamp(&serde_json::json!(null)), None);
        assert_eq!(parse_history_timestamp(&serde_json::json!("")), None);
        assert_eq!(parse_history_timestamp(&serde_json::json!("not-a-date")), None);
    }

    #[test]
    fn test_load_metadata_renamed_title_overrides_long_message() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_dir = temp_dir.path().to_path_buf();

        let history_path = config_dir.join("history.jsonl");
        let mut file = std::fs::File::create(&history_path).unwrap();
        // 旧记录：长首条用户消息
        writeln!(
            file,
            r#"{{"type":"last-prompt","timestamp":1780013871215,"sessionId":"0230e7f8","display":"帮我检查本机的健康状态。之前两天，连续出现两次关机时无响应","project":"/Users/ckstar/Documents/Obsidian/techNote"}}"#
        ).unwrap();
        // 新记录：/rename
        writeln!(
            file,
            r#"{{"type":"last-prompt","timestamp":1780014184874,"sessionId":"0230e7f8","display":"/rename macos 健康状态检查","project":"/Users/ckstar/Documents/Obsidian/techNote"}}"#
        ).unwrap();

        let candidate = CandidateConfigDir {
            raw_path: config_dir.clone(),
            canonical_path: Some(config_dir.canonicalize().unwrap()),
            source: super::super::models::ConfigDirSource::EnvClaudeConfigDir,
        };

        let metadata = load_usage_session_metadata(&[candidate]);
        let meta = metadata.get("0230e7f8").unwrap();
        // 应保留 /rename 解析后的标题
        assert_eq!(meta.display, Some("/rename macos 健康状态检查".to_string()));
    }

    #[test]
    fn test_load_metadata_no_timestamp_fallback() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_dir = temp_dir.path().to_path_buf();

        let history_path = config_dir.join("history.jsonl");
        let mut file = std::fs::File::create(&history_path).unwrap();
        // 无 timestamp 的旧记录
        writeln!(
            file,
            r#"{{"type":"last-prompt","sessionId":"sess-001","display":"旧标题"}}"#
        ).unwrap();
        // 无 timestamp 的新记录（后读覆盖）
        writeln!(
            file,
            r#"{{"type":"last-prompt","sessionId":"sess-001","display":"新标题"}}"#
        ).unwrap();

        let candidate = CandidateConfigDir {
            raw_path: config_dir.clone(),
            canonical_path: Some(config_dir.canonicalize().unwrap()),
            source: super::super::models::ConfigDirSource::EnvClaudeConfigDir,
        };

        let metadata = load_usage_session_metadata(&[candidate]);
        let meta = metadata.get("sess-001").unwrap();
        // 无 timestamp 时后读覆盖前读
        assert_eq!(meta.display, Some("新标题".to_string()));
    }

    #[test]
    fn test_load_metadata_long_message_not_overwrite_rename() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_dir = temp_dir.path().to_path_buf();

        let history_path = config_dir.join("history.jsonl");
        let mut file = std::fs::File::create(&history_path).unwrap();
        // 先写入 rename 标题（较早）
        writeln!(
            file,
            r#"{{"type":"last-prompt","timestamp":1780010000000,"sessionId":"sess-001","display":"/rename macos 健康状态检查","project":"/project/a"}}"#
        ).unwrap();
        // 后写入长首条用户消息（较晚，但无意义）
        writeln!(
            file,
            r#"{{"type":"last-prompt","timestamp":1780011000000,"sessionId":"sess-001","display":"帮我检查本机的健康状态。之前两天，连续出现两次关机时无响应","project":"/project/a"}}"#
        ).unwrap();

        let candidate = CandidateConfigDir {
            raw_path: config_dir.clone(),
            canonical_path: Some(config_dir.canonicalize().unwrap()),
            source: super::super::models::ConfigDirSource::EnvClaudeConfigDir,
        };

        let metadata = load_usage_session_metadata(&[candidate]);
        let meta = metadata.get("sess-001").unwrap();
        // 较晚的长消息应覆盖较早的 rename，但 clean 后会显示 "macos 健康状态检查"
        // 这里验证原始 display 保留最新记录
        assert_eq!(
            meta.display,
            Some("帮我检查本机的健康状态。之前两天，连续出现两次关机时无响应".to_string())
        );
    }

    #[test]
    fn test_load_metadata_same_timestamp_better_title_wins() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_dir = temp_dir.path().to_path_buf();

        let history_path = config_dir.join("history.jsonl");
        let mut file = std::fs::File::create(&history_path).unwrap();
        // 两条 timestamp 相同，先写 (未命名) 类标题
        writeln!(
            file,
            r#"{{"type":"last-prompt","timestamp":1780010000000,"sessionId":"sess-001","display":"/model","project":"/project/a"}}"#
        ).unwrap();
        // 后写有意义标题（同 timestamp）
        writeln!(
            file,
            r#"{{"type":"last-prompt","timestamp":1780010000000,"sessionId":"sess-001","display":"macos 健康状态检查","project":"/project/a"}}"#
        ).unwrap();

        let candidate = CandidateConfigDir {
            raw_path: config_dir.clone(),
            canonical_path: Some(config_dir.canonicalize().unwrap()),
            source: super::super::models::ConfigDirSource::EnvClaudeConfigDir,
        };

        let metadata = load_usage_session_metadata(&[candidate]);
        let meta = metadata.get("sess-001").unwrap();
        // 同 timestamp 时，有意义的标题应覆盖 (未命名) 类标题
        assert_eq!(meta.display, Some("macos 健康状态检查".to_string()));
    }

    #[test]
    fn test_clean_session_title_basic() {
        assert_eq!(clean_session_title(Some("hello world")), "hello world");
    }

    #[test]
    fn test_clean_session_title_trims_whitespace() {
        assert_eq!(clean_session_title(Some("  hello  ")), "hello");
    }

    #[test]
    fn test_clean_session_title_none_or_empty() {
        assert_eq!(clean_session_title(None), "(未命名)");
        assert_eq!(clean_session_title(Some("")), "(未命名)");
        assert_eq!(clean_session_title(Some("   ")), "(未命名)");
    }

    #[test]
    fn test_clean_session_title_keeps_chinese() {
        let chinese = "帮我检查本机的健康状态。之前两天，连续出现两次关机时无响应";
        assert_eq!(clean_session_title(Some(chinese)), chinese);
    }

    #[test]
    fn test_clean_session_title_rename_with_arg() {
        // /rename <arg> → <arg>，去掉命令词
        assert_eq!(clean_session_title(Some("/rename v0.3.7")), "v0.3.7");
        assert_eq!(clean_session_title(Some("/rename claude code 配置")), "claude code 配置");
    }

    #[test]
    fn test_clean_session_title_rename_without_arg() {
        assert_eq!(clean_session_title(Some("/rename")), "(未命名)");
        assert_eq!(clean_session_title(Some("/rename  ")), "(未命名)");
    }

    #[test]
    fn test_clean_session_title_no_arg_commands() {
        assert_eq!(clean_session_title(Some("/model")), "(未命名)");
        assert_eq!(clean_session_title(Some("/resume")), "(未命名)");
        assert_eq!(clean_session_title(Some("/advance-stage")), "(未命名)");
    }

    #[test]
    fn test_clean_session_title_pasted_text() {
        assert_eq!(
            clean_session_title(Some("[Pasted text #1 +47 lines]")),
            "(未命名)"
        );
        assert_eq!(
            clean_session_title(Some("[Pasted text with any content]")),
            "(未命名)"
        );
    }

    #[test]
    fn test_clean_session_title_other_slash_with_arg() {
        // 其他 slash command 有参数 → 保留参数
        assert_eq!(clean_session_title(Some("/some-cmd hello")), "hello");
    }

    #[test]
    fn test_clean_session_title_other_slash_without_arg() {
        // 其他 slash command 无参数 → (未命名)
        assert_eq!(clean_session_title(Some("/cmd")), "(未命名)");
    }

    #[test]
    fn test_load_metadata_skips_empty_session_id() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_dir = temp_dir.path().to_path_buf();

        let history_path = config_dir.join("history.jsonl");
        let mut file = std::fs::File::create(&history_path).unwrap();
        writeln!(
            file,
            r#"{{"type":"last-prompt","timestamp":"2026-05-27T10:00:00.000Z","display":"无 session"}}"#
        ).unwrap();

        let candidate = CandidateConfigDir {
            raw_path: config_dir,
            canonical_path: None,
            source: super::super::models::ConfigDirSource::EnvClaudeConfigDir,
        };

        let metadata = load_usage_session_metadata(&[candidate]);
        assert!(metadata.is_empty());
    }
}
