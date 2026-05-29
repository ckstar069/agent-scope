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
    /// 记录时间戳（用于去重时保留较新记录）
    pub timestamp: Option<String>,
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

            let timestamp = value
                .get("timestamp")
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
                }
            });

            // 如果新记录 timestamp 更晚，或当前没有 display/project，则更新
            let should_update = match (&timestamp, &entry.timestamp) {
                (Some(new_ts), Some(old_ts)) => new_ts > old_ts,
                (Some(_), None) => true,
                (None, _) => entry.display.is_none() && entry.project_path.is_none(),
            };

            if should_update {
                if display.is_some() {
                    entry.display = display;
                }
                if project_path.is_some() {
                    entry.project_path = project_path.clone();
                }
                if project_name.is_some() {
                    entry.project_name = project_name.clone();
                }
                if timestamp.is_some() {
                    entry.timestamp = timestamp;
                }
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
/// 1. trim 首尾空白
/// 2. 如果以 "/" 开头，去掉开头所有 "/"
/// 3. 清洗后为空，返回 "(未命名)"
/// 4. 原始值为 None 或空，返回 "(未命名)"
/// 5. 不做截断（前端负责）
/// 6. 保留 "[Pasted text ...]" 等特殊格式
pub fn clean_session_title(title: Option<&str>) -> String {
    let Some(t) = title else {
        return "(未命名)".to_string();
    };

    let cleaned = t.trim().trim_start_matches('/');

    if cleaned.is_empty() {
        "(未命名)".to_string()
    } else {
        cleaned.to_string()
    }
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
        writeln!(
            file,
            r#"{{"type":"last-prompt","timestamp":"2026-05-27T10:00:00.000Z","sessionId":"sess-001","display":"Phase2 阶段开发","project":"/Users/ckstar/Repo/agent-scope"}}"#
        ).unwrap();
        writeln!(
            file,
            r#"{{"type":"last-prompt","timestamp":"2026-05-27T11:00:00.000Z","sessionId":"sess-002","display":"文档整理","project":"/Users/ckstar/Documents/notes"}}"#
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
    fn test_load_metadata_keeps_newer_timestamp() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_dir = temp_dir.path().to_path_buf();

        let history_path = config_dir.join("history.jsonl");
        let mut file = std::fs::File::create(&history_path).unwrap();
        // 旧记录
        writeln!(
            file,
            r#"{{"type":"last-prompt","timestamp":"2026-05-27T10:00:00.000Z","sessionId":"sess-001","display":"旧标题","project":"/old/path"}}"#
        ).unwrap();
        // 新记录（同 session，应覆盖）
        writeln!(
            file,
            r#"{{"type":"last-prompt","timestamp":"2026-05-27T12:00:00.000Z","sessionId":"sess-001","display":"新标题","project":"/new/path"}}"#
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
    fn test_clean_session_title_basic() {
        assert_eq!(clean_session_title(Some("hello world")), "hello world");
    }

    #[test]
    fn test_clean_session_title_trims_leading_slash() {
        assert_eq!(clean_session_title(Some("/rename v0.3.7")), "rename v0.3.7");
        assert_eq!(clean_session_title(Some("//model")), "model");
        assert_eq!(clean_session_title(Some("///")).as_str(), "(未命名)");
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
    fn test_clean_session_title_keeps_pasted_text() {
        assert_eq!(
            clean_session_title(Some("[Pasted text #1 +47 lines]")),
            "[Pasted text #1 +47 lines]"
        );
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
