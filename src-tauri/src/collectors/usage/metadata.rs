use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;

use super::models::CandidateConfigDir;
use crate::collectors::claude_history::path_codec::decode_project_dir;

// ============================================================================
// 标题质量分类系统
// ============================================================================

/// 会话标题质量等级（越高越优先作为会话名）
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
enum TitleQuality {
    /// 空 / None
    #[default]
    Empty = 0,
    /// 长首条用户消息（>40 字符），不作为标题
    PromptLike = 5,
    /// Placeholder（[Pasted text...]、[Image #...]）
    Placeholder = 10,
    /// 无意义命令（exit/model/resume 等）
    Command = 20,
    /// 短直接标题（≤40 字符）
    DirectTitle = 70,
    /// /rename 明确重命名
    ExplicitRename = 100,
}

/// 标题候选
#[derive(Debug, Clone)]
struct TitleCandidate {
    /// 原始 display
    raw: Option<String>,
    /// 清洗后的标题（用于最终展示）
    cleaned: Option<String>,
    /// 质量等级
    quality: TitleQuality,
}

/// 对 history.jsonl display 字段进行候选分类
///
/// 规则：
/// 1. None / 空 → Empty，cleaned = None
/// 2. `[Pasted text...]` / `[Image #...]` → Placeholder，cleaned = None
/// 3. slash command：
///    - `/rename <arg>` → ExplicitRename，cleaned = arg
///    - `/exit`、`/model`、`/resume` 等 → Command，cleaned = None
///    - 其他 slash command → Command，cleaned = None
/// 4. 无斜杠命令词（exit、model、resume 等）→ Command，cleaned = None
/// 5. 普通文本：
///    - ≤40 字符 → DirectTitle，cleaned = 原文
///    - >40 字符 → PromptLike，cleaned = None
fn classify_title_candidate(display: Option<&str>) -> TitleCandidate {
    let Some(t) = display else {
        return TitleCandidate {
            raw: None,
            cleaned: None,
            quality: TitleQuality::Empty,
        };
    };

    let trimmed = t.trim();
    if trimmed.is_empty() {
        return TitleCandidate {
            raw: None,
            cleaned: None,
            quality: TitleQuality::Empty,
        };
    }

    // Placeholder 检测
    if trimmed.starts_with("[Pasted text")
        || trimmed.starts_with("[Image #")
        || trimmed.starts_with("[Image")
    {
        return TitleCandidate {
            raw: Some(trimmed.to_string()),
            cleaned: None,
            quality: TitleQuality::Placeholder,
        };
    }

    // slash command 处理
    if let Some(stripped) = trimmed.strip_prefix('/') {
        let stripped = stripped.trim_start_matches('/');
        let mut parts = stripped.splitn(2, ' ');
        let cmd = parts.next().unwrap_or("").trim();
        let arg = parts.next().map(|s| s.trim()).filter(|s| !s.is_empty());

        // 无意义 slash command（永远不能作为标题）
        let no_meaning_commands = [
            "exit", "quit", "q", "model", "resume", "advance-stage",
            "clear", "compact", "help", "status", "cost", "doctor", "effort",
            "new",
        ];
        if no_meaning_commands.contains(&cmd) {
            return TitleCandidate {
                raw: Some(trimmed.to_string()),
                cleaned: None,
                quality: TitleQuality::Command,
            };
        }

        // /rename 命令：只保留参数
        if cmd == "rename" {
            if let Some(arg_str) = arg {
                return TitleCandidate {
                    raw: Some(trimmed.to_string()),
                    cleaned: Some(arg_str.to_string()),
                    quality: TitleQuality::ExplicitRename,
                };
            }
            return TitleCandidate {
                raw: Some(trimmed.to_string()),
                cleaned: None,
                quality: TitleQuality::Command,
            };
        }

        // 其他 slash command（一律视为命令，不保留参数）
        return TitleCandidate {
            raw: Some(trimmed.to_string()),
            cleaned: None,
            quality: TitleQuality::Command,
        };
    }

    // 无斜杠的命令词检测
    let no_meaning_words = [
        "exit", "quit", "q", "model", "resume", "advance-stage",
        "clear", "compact", "help", "status", "cost", "doctor", "effort",
    ];
    if no_meaning_words.contains(&trimmed) {
        return TitleCandidate {
            raw: Some(trimmed.to_string()),
            cleaned: None,
            quality: TitleQuality::Command,
        };
    }

    // 普通直接标题：按长度区分
    let char_count = trimmed.chars().count();
    if char_count <= 40 {
        TitleCandidate {
            raw: Some(trimmed.to_string()),
            cleaned: Some(trimmed.to_string()),
            quality: TitleQuality::DirectTitle,
        }
    } else {
        TitleCandidate {
            raw: Some(trimmed.to_string()),
            cleaned: None,
            quality: TitleQuality::PromptLike,
        }
    }
}

// ============================================================================
// Usage 会话元数据
// ============================================================================

/// Usage 会话元数据
///
/// 从 Claude Code history.jsonl 中读取，用于 enrichment usage record。
#[derive(Debug, Clone, Default)]
pub struct UsageSessionMetadata {
    /// 会话 ID
    pub session_id: String,
    /// 会话显示标题（history.jsonl 中的 display 字段，经质量筛选后保留）
    pub display: Option<String>,
    /// 项目路径（history.jsonl 中的 project 字段）
    pub project_path: Option<String>,
    /// 项目名称（project_path 的 basename）
    pub project_name: Option<String>,
    /// 记录时间戳（原始字符串，用于调试）
    pub timestamp: Option<String>,
    /// 排序用时间戳（毫秒级，内部使用，不参与序列化）
    pub timestamp_sort_key: Option<i128>,
    /// 当前 display 的质量等级（内部字段，用于同 session 比较）
    title_quality: TitleQuality,
}

// ============================================================================
// Timestamp 解析
// ============================================================================

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

// ============================================================================
// Metadata 加载
// ============================================================================

/// 从一组 config_dir 加载 session metadata
///
/// 读取每个 config_dir 下的 history.jsonl，构建 session_id → metadata 映射。
/// 同一 session_id 出现多次时：
/// - project_path / project_name 按最新 timestamp 更新
/// - display 按标题质量优先选择（ExplicitRename > DirectTitle > 其他），
///   质量相同时 timestamp 更新者胜出
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

            let new_candidate = classify_title_candidate(display.as_deref());

            let entry = map.entry(session_id.to_string()).or_insert_with(|| {
                UsageSessionMetadata {
                    session_id: session_id.to_string(),
                    display: None,
                    project_path: None,
                    project_name: None,
                    timestamp: None,
                    timestamp_sort_key: None,
                    title_quality: TitleQuality::Empty,
                }
            });

            // 保存旧 timestamp 用于 title 比较（避免 project 更新后丢失旧值）
            let old_timestamp_sort_key = entry.timestamp_sort_key;

            // --- project 信息始终按最新 timestamp 更新 ---
            let should_update_project = match (timestamp_sort_key, entry.timestamp_sort_key) {
                (Some(new_ts), Some(old_ts)) => new_ts > old_ts,
                (Some(_), None) => true,
                (None, Some(_)) => false,
                (None, None) => true,
            };

            if should_update_project {
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

            // --- display 按质量选择 ---
            // 只有 cleaned 不为 None 的候选才可能成为标题
            let should_update_title = if new_candidate.cleaned.is_none() {
                false // 无有效标题，绝不覆盖
            } else if entry.display.is_none() || new_candidate.quality > entry.title_quality {
                true // 当前无标题，或新值质量更高
            } else if new_candidate.quality == entry.title_quality {
                // 质量相同：timestamp 更大者胜出（使用更新前的旧值）
                match (timestamp_sort_key, old_timestamp_sort_key) {
                    (Some(new_ts), Some(old_ts)) => new_ts > old_ts,
                    (Some(_), None) => true,
                    (None, Some(_)) => false,
                    (None, None) => true, // 都不可解析时后读覆盖
                }
            } else {
                false
            };

            if should_update_title {
                entry.display = new_candidate.raw;
                entry.title_quality = new_candidate.quality;
            }
        }
    }

    map
}

// ============================================================================
// 项目标签辅助
// ============================================================================

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

// ============================================================================
// 会话标题清洗
// ============================================================================

/// 清洗会话标题
///
/// 基于 classify_title_candidate，返回最终展示用字符串。
/// - ExplicitRename → arg（如 "v0.3.7"）
/// - DirectTitle → 原文
/// - 其他（Command、Placeholder、PromptLike、Empty）→ "(未命名)"
pub fn clean_session_title(title: Option<&str>) -> String {
    let candidate = classify_title_candidate(title);
    candidate
        .cleaned
        .unwrap_or_else(|| "(未命名)".to_string())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    // ------------------------------------------------------------------------
    // classify_title_candidate 测试
    // ------------------------------------------------------------------------

    #[test]
    fn test_classify_empty() {
        let c = classify_title_candidate(None);
        assert!(c.raw.is_none());
        assert!(c.cleaned.is_none());
        assert_eq!(c.quality, TitleQuality::Empty);
    }

    #[test]
    fn test_classify_pasted_text() {
        let c = classify_title_candidate(Some("[Pasted text #1 +47 lines]"));
        assert_eq!(c.quality, TitleQuality::Placeholder);
        assert!(c.cleaned.is_none());
    }

    #[test]
    fn test_classify_image_placeholder() {
        let c = classify_title_candidate(Some("[Image #1] 帮我检查..."));
        assert_eq!(c.quality, TitleQuality::Placeholder);
        assert!(c.cleaned.is_none());
    }

    #[test]
    fn test_classify_slash_exit() {
        let c = classify_title_candidate(Some("/exit"));
        assert_eq!(c.quality, TitleQuality::Command);
        assert!(c.cleaned.is_none());
    }

    #[test]
    fn test_classify_slash_model() {
        let c = classify_title_candidate(Some("/model"));
        assert_eq!(c.quality, TitleQuality::Command);
        assert!(c.cleaned.is_none());
    }

    #[test]
    fn test_classify_plain_exit() {
        let c = classify_title_candidate(Some("exit"));
        assert_eq!(c.quality, TitleQuality::Command);
        assert!(c.cleaned.is_none());
    }

    #[test]
    fn test_classify_plain_model() {
        let c = classify_title_candidate(Some("model"));
        assert_eq!(c.quality, TitleQuality::Command);
        assert!(c.cleaned.is_none());
    }

    #[test]
    fn test_classify_rename_with_arg() {
        let c = classify_title_candidate(Some("/rename v0.3.7"));
        assert_eq!(c.quality, TitleQuality::ExplicitRename);
        assert_eq!(c.cleaned, Some("v0.3.7".to_string()));
    }

    #[test]
    fn test_classify_rename_without_arg() {
        let c = classify_title_candidate(Some("/rename"));
        assert_eq!(c.quality, TitleQuality::Command);
        assert!(c.cleaned.is_none());
    }

    #[test]
    fn test_classify_unknown_slash_command() {
        let c = classify_title_candidate(Some("/some-cmd hello"));
        assert_eq!(c.quality, TitleQuality::Command);
        assert!(c.cleaned.is_none());
    }

    #[test]
    fn test_classify_direct_short_title() {
        let c = classify_title_candidate(Some("macos 健康状态检查"));
        assert_eq!(c.quality, TitleQuality::DirectTitle);
        assert_eq!(c.cleaned, Some("macos 健康状态检查".to_string()));
    }

    #[test]
    fn test_classify_prompt_like_long() {
        let c = classify_title_candidate(Some("帮我检查本机的健康状态。之前两天，连续出现两次关机时无响应、等半天、桌面dock/图标全部消失只有桌面的情况，等了很久没反应然后我强制关机了。"));
        assert_eq!(c.quality, TitleQuality::PromptLike);
        assert!(c.cleaned.is_none());
    }

    #[test]
    fn test_classify_effort_command() {
        let c = classify_title_candidate(Some("/effort"));
        assert_eq!(c.quality, TitleQuality::Command);
        assert!(c.cleaned.is_none());
    }

    // ------------------------------------------------------------------------
    // clean_session_title 测试
    // ------------------------------------------------------------------------

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
    fn test_clean_session_title_keeps_chinese_short() {
        let chinese = "macos 健康状态检查";
        assert_eq!(clean_session_title(Some(chinese)), chinese);
    }

    #[test]
    fn test_clean_session_title_long_prompt_becomes_unnamed() {
        let long = "帮我检查本机的健康状态。之前两天，连续出现两次关机时无响应、等半天、桌面dock/图标全部消失只有桌面的情况，等了很久没反应然后我强制关机了。";
        assert_eq!(clean_session_title(Some(long)), "(未命名)");
    }

    #[test]
    fn test_clean_session_title_rename_with_arg() {
        assert_eq!(clean_session_title(Some("/rename v0.3.7")), "v0.3.7");
        assert_eq!(
            clean_session_title(Some("/rename claude code 配置")),
            "claude code 配置"
        );
    }

    #[test]
    fn test_clean_session_title_rename_without_arg() {
        assert_eq!(clean_session_title(Some("/rename")), "(未命名)");
    }

    #[test]
    fn test_clean_session_title_no_arg_commands() {
        assert_eq!(clean_session_title(Some("/model")), "(未命名)");
        assert_eq!(clean_session_title(Some("/resume")), "(未命名)");
        assert_eq!(clean_session_title(Some("/advance-stage")), "(未命名)");
    }

    #[test]
    fn test_clean_session_title_plain_commands() {
        assert_eq!(clean_session_title(Some("exit")), "(未命名)");
        assert_eq!(clean_session_title(Some("model")), "(未命名)");
        assert_eq!(clean_session_title(Some("resume")), "(未命名)");
    }

    #[test]
    fn test_clean_session_title_pasted_text() {
        assert_eq!(
            clean_session_title(Some("[Pasted text #1 +47 lines]")),
            "(未命名)"
        );
    }

    #[test]
    fn test_clean_session_title_image_placeholder() {
        assert_eq!(
            clean_session_title(Some("[Image #1] 帮我检查...")),
            "(未命名)"
        );
    }

    #[test]
    fn test_clean_session_title_unknown_slash_command() {
        // 其他 slash command 不再保留参数
        assert_eq!(clean_session_title(Some("/some-cmd hello")), "(未命名)");
    }

    // ------------------------------------------------------------------------
    // compact_project_label 测试
    // ------------------------------------------------------------------------

    #[test]
    fn test_compact_project_label_repo_prefix() {
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
        assert_eq!(
            compact_project_label("home-user-project"),
            "project"
        );
    }

    #[test]
    fn test_compact_project_label_long_no_dash() {
        let long = "a".repeat(50);
        let result = compact_project_label(&long);
        assert!(result.starts_with('…'));
        assert_eq!(result.chars().count(), 32);
    }

    #[test]
    fn test_compact_project_label_last_segment_long() {
        let prefix = "foo-bar-";
        let suffix = "z".repeat(50);
        let encoded = format!("{}{}", prefix, suffix);
        let result = compact_project_label(&encoded);
        assert!(result.starts_with('…'));
        assert_eq!(result.chars().count(), 32);
    }

    // ------------------------------------------------------------------------
    // short_session_id 测试
    // ------------------------------------------------------------------------

    #[test]
    fn test_short_session_id() {
        assert_eq!(
            short_session_id("550e8400-e29b-41d4-a716-446655440000"),
            "550e8400"
        );
    }

    // ------------------------------------------------------------------------
    // parse_history_timestamp 测试
    // ------------------------------------------------------------------------

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
        assert_eq!(result.unwrap(), 1_780_045_105_000i128);
    }

    #[test]
    fn test_parse_history_timestamp_invalid() {
        assert_eq!(parse_history_timestamp(&serde_json::json!(null)), None);
        assert_eq!(parse_history_timestamp(&serde_json::json!("")), None);
        assert_eq!(parse_history_timestamp(&serde_json::json!("not-a-date")), None);
    }

    // ------------------------------------------------------------------------
    // load_usage_session_metadata 核心测试
    // ------------------------------------------------------------------------

    #[test]
    fn test_load_usage_session_metadata_from_history() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_dir = temp_dir.path().to_path_buf();

        let history_path = config_dir.join("history.jsonl");
        let mut file = std::fs::File::create(&history_path).unwrap();
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

        let meta2 = metadata.get("sess-002").unwrap();
        assert_eq!(meta2.display, Some("文档整理".to_string()));
        assert_eq!(meta2.project_name, Some("notes".to_string()));
    }

    #[test]
    fn test_load_metadata_keeps_newer_timestamp_for_project() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_dir = temp_dir.path().to_path_buf();

        let history_path = config_dir.join("history.jsonl");
        let mut file = std::fs::File::create(&history_path).unwrap();
        writeln!(
            file,
            r#"{{"type":"last-prompt","timestamp":1780000000000,"sessionId":"sess-001","display":"旧标题","project":"/old/path"}}"#
        ).unwrap();
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
        // /rename 质量(100) > 长消息质量(0)，应保留 rename
        assert_eq!(meta.display, Some("/rename macos 健康状态检查".to_string()));
    }

    #[test]
    fn test_load_metadata_no_timestamp_fallback() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_dir = temp_dir.path().to_path_buf();

        let history_path = config_dir.join("history.jsonl");
        let mut file = std::fs::File::create(&history_path).unwrap();
        writeln!(
            file,
            r#"{{"type":"last-prompt","sessionId":"sess-001","display":"旧标题"}}"#
        ).unwrap();
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
        // 后写入长首条用户消息（较晚，但质量更低）
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
        // 长消息质量(0) < rename 质量(100)，rename 应保留
        assert_eq!(
            meta.display,
            Some("/rename macos 健康状态检查".to_string())
        );
    }

    #[test]
    fn test_load_metadata_exit_does_not_overwrite_rename() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_dir = temp_dir.path().to_path_buf();

        let history_path = config_dir.join("history.jsonl");
        let mut file = std::fs::File::create(&history_path).unwrap();
        writeln!(
            file,
            r#"{{"type":"last-prompt","timestamp":1780010000000,"sessionId":"sess-001","display":"/rename v0.3.7","project":"/project/a"}}"#
        ).unwrap();
        writeln!(
            file,
            r#"{{"type":"last-prompt","timestamp":1780011000000,"sessionId":"sess-001","display":"exit","project":"/project/a"}}"#
        ).unwrap();

        let candidate = CandidateConfigDir {
            raw_path: config_dir.clone(),
            canonical_path: Some(config_dir.canonicalize().unwrap()),
            source: super::super::models::ConfigDirSource::EnvClaudeConfigDir,
        };

        let metadata = load_usage_session_metadata(&[candidate]);
        let meta = metadata.get("sess-001").unwrap();
        assert_eq!(meta.display, Some("/rename v0.3.7".to_string()));
    }

    #[test]
    fn test_load_metadata_slash_exit_does_not_overwrite_rename() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_dir = temp_dir.path().to_path_buf();

        let history_path = config_dir.join("history.jsonl");
        let mut file = std::fs::File::create(&history_path).unwrap();
        writeln!(
            file,
            r#"{{"type":"last-prompt","timestamp":1780010000000,"sessionId":"sess-001","display":"/rename v0.3.7","project":"/project/a"}}"#
        ).unwrap();
        writeln!(
            file,
            r#"{{"type":"last-prompt","timestamp":1780011000000,"sessionId":"sess-001","display":"/exit","project":"/project/a"}}"#
        ).unwrap();

        let candidate = CandidateConfigDir {
            raw_path: config_dir.clone(),
            canonical_path: Some(config_dir.canonicalize().unwrap()),
            source: super::super::models::ConfigDirSource::EnvClaudeConfigDir,
        };

        let metadata = load_usage_session_metadata(&[candidate]);
        let meta = metadata.get("sess-001").unwrap();
        assert_eq!(meta.display, Some("/rename v0.3.7".to_string()));
    }

    #[test]
    fn test_load_metadata_model_does_not_overwrite_direct_title() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_dir = temp_dir.path().to_path_buf();

        let history_path = config_dir.join("history.jsonl");
        let mut file = std::fs::File::create(&history_path).unwrap();
        writeln!(
            file,
            r#"{{"type":"last-prompt","timestamp":1780010000000,"sessionId":"sess-001","display":"macos 健康状态检查","project":"/project/a"}}"#
        ).unwrap();
        writeln!(
            file,
            r#"{{"type":"last-prompt","timestamp":1780011000000,"sessionId":"sess-001","display":"model","project":"/project/a"}}"#
        ).unwrap();

        let candidate = CandidateConfigDir {
            raw_path: config_dir.clone(),
            canonical_path: Some(config_dir.canonicalize().unwrap()),
            source: super::super::models::ConfigDirSource::EnvClaudeConfigDir,
        };

        let metadata = load_usage_session_metadata(&[candidate]);
        let meta = metadata.get("sess-001").unwrap();
        assert_eq!(meta.display, Some("macos 健康状态检查".to_string()));
    }

    #[test]
    fn test_load_metadata_only_exit_is_unnamed() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_dir = temp_dir.path().to_path_buf();

        let history_path = config_dir.join("history.jsonl");
        let mut file = std::fs::File::create(&history_path).unwrap();
        writeln!(
            file,
            r#"{{"type":"last-prompt","timestamp":1780010000000,"sessionId":"sess-001","display":"exit","project":"/project/a"}}"#
        ).unwrap();

        let candidate = CandidateConfigDir {
            raw_path: config_dir.clone(),
            canonical_path: Some(config_dir.canonicalize().unwrap()),
            source: super::super::models::ConfigDirSource::EnvClaudeConfigDir,
        };

        let metadata = load_usage_session_metadata(&[candidate]);
        let meta = metadata.get("sess-001").unwrap();
        // exit 是 Command，cleaned=None，不应写入 display
        assert!(meta.display.is_none());
    }

    #[test]
    fn test_load_metadata_only_long_prompt_is_unnamed() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_dir = temp_dir.path().to_path_buf();

        let history_path = config_dir.join("history.jsonl");
        let mut file = std::fs::File::create(&history_path).unwrap();
        writeln!(
            file,
            r#"{{"type":"last-prompt","timestamp":1780010000000,"sessionId":"sess-001","display":"帮我检查本机的健康状态。之前两天，连续出现两次关机时无响应、等半天、桌面dock/图标全部消失只有桌面的情况，等了很久没反应然后我强制关机了。","project":"/project/a"}}"#
        ).unwrap();

        let candidate = CandidateConfigDir {
            raw_path: config_dir.clone(),
            canonical_path: Some(config_dir.canonicalize().unwrap()),
            source: super::super::models::ConfigDirSource::EnvClaudeConfigDir,
        };

        let metadata = load_usage_session_metadata(&[candidate]);
        let meta = metadata.get("sess-001").unwrap();
        // 长消息是 PromptLike，cleaned=None，不应写入 display
        assert!(meta.display.is_none());
    }

    #[test]
    fn test_load_metadata_rename_after_long_prompt_wins() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_dir = temp_dir.path().to_path_buf();

        let history_path = config_dir.join("history.jsonl");
        let mut file = std::fs::File::create(&history_path).unwrap();
        writeln!(
            file,
            r#"{{"type":"last-prompt","timestamp":1780010000000,"sessionId":"sess-001","display":"帮我检查本机的健康状态。之前两天，连续出现两次关机时无响应","project":"/project/a"}}"#
        ).unwrap();
        writeln!(
            file,
            r#"{{"type":"last-prompt","timestamp":1780011000000,"sessionId":"sess-001","display":"/rename macos 健康状态检查","project":"/project/a"}}"#
        ).unwrap();

        let candidate = CandidateConfigDir {
            raw_path: config_dir.clone(),
            canonical_path: Some(config_dir.canonicalize().unwrap()),
            source: super::super::models::ConfigDirSource::EnvClaudeConfigDir,
        };

        let metadata = load_usage_session_metadata(&[candidate]);
        let meta = metadata.get("sess-001").unwrap();
        assert_eq!(meta.display, Some("/rename macos 健康状态检查".to_string()));
    }

    #[test]
    fn test_load_metadata_direct_title_after_prompt_wins() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_dir = temp_dir.path().to_path_buf();

        let history_path = config_dir.join("history.jsonl");
        let mut file = std::fs::File::create(&history_path).unwrap();
        writeln!(
            file,
            r#"{{"type":"last-prompt","timestamp":1780010000000,"sessionId":"sess-001","display":"帮我检查本机的健康状态。之前两天，连续出现两次关机时无响应","project":"/project/a"}}"#
        ).unwrap();
        writeln!(
            file,
            r#"{{"type":"last-prompt","timestamp":1780011000000,"sessionId":"sess-001","display":"macos 健康状态检查","project":"/project/a"}}"#
        ).unwrap();

        let candidate = CandidateConfigDir {
            raw_path: config_dir.clone(),
            canonical_path: Some(config_dir.canonicalize().unwrap()),
            source: super::super::models::ConfigDirSource::EnvClaudeConfigDir,
        };

        let metadata = load_usage_session_metadata(&[candidate]);
        let meta = metadata.get("sess-001").unwrap();
        assert_eq!(meta.display, Some("macos 健康状态检查".to_string()));
    }

    #[test]
    fn test_load_metadata_same_timestamp_better_title_wins() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_dir = temp_dir.path().to_path_buf();

        let history_path = config_dir.join("history.jsonl");
        let mut file = std::fs::File::create(&history_path).unwrap();
        writeln!(
            file,
            r#"{{"type":"last-prompt","timestamp":1780010000000,"sessionId":"sess-001","display":"/model","project":"/project/a"}}"#
        ).unwrap();
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
        // /model 是 Command（cleaned=None），不会写入 display
        // macos 健康状态检查 是 DirectTitle，应写入
        assert_eq!(meta.display, Some("macos 健康状态检查".to_string()));
    }

    #[test]
    fn test_load_metadata_unknown_slash_command_not_title() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_dir = temp_dir.path().to_path_buf();

        let history_path = config_dir.join("history.jsonl");
        let mut file = std::fs::File::create(&history_path).unwrap();
        writeln!(
            file,
            r#"{{"type":"last-prompt","timestamp":1780010000000,"sessionId":"sess-001","display":"/some-cmd hello","project":"/project/a"}}"#
        ).unwrap();

        let candidate = CandidateConfigDir {
            raw_path: config_dir.clone(),
            canonical_path: Some(config_dir.canonicalize().unwrap()),
            source: super::super::models::ConfigDirSource::EnvClaudeConfigDir,
        };

        let metadata = load_usage_session_metadata(&[candidate]);
        let meta = metadata.get("sess-001").unwrap();
        // 未知 slash command 不再保留参数作为标题
        assert!(meta.display.is_none());
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
