use chrono::{DateTime, Utc};
use serde_json::Value;

use super::models::{UsageParseContext, UsageParseError, UsageRecord};

/// 解析单条 JSONL 行
///
/// 返回 Ok(Some(record)) 表示成功提取到 assistant usage 记录
/// 返回 Ok(None) 表示该行不是 assistant 消息或无 usage（非错误）
/// 返回 Err 表示 JSON 解析失败或字段格式异常
pub fn parse_usage_line(
    line: &str,
    context: &UsageParseContext,
) -> Result<Option<UsageRecord>, UsageParseError> {
    // 1. 解析 JSON
    let value: Value = serde_json::from_str(line)
        .map_err(|e| UsageParseError::InvalidJson(e.to_string()))?;

    // 2. 只处理 assistant 类型消息
    let message_type = value
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if message_type != "assistant" {
        return Ok(None);
    }

    // 3. 提取 message.usage
    let usage = match value.get("message").and_then(|m| m.get("usage")) {
        Some(u) => u,
        None => return Ok(None),
    };

    // usage 为 null 时跳过（表示 usage 尚未写入）
    if usage.is_null() {
        return Ok(None);
    }

    // 4. 读取 token 字段（缺失则默认为 0）
    let input_tokens = usage
        .get("input_tokens")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let output_tokens = usage
        .get("output_tokens")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let cache_read_tokens = usage
        .get("cache_read_input_tokens")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let cache_create_tokens = usage
        .get("cache_creation_input_tokens")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    let total_tokens = input_tokens + output_tokens + cache_read_tokens + cache_create_tokens;

    // 如果总 token 为 0，跳过（避免噪音）
    if total_tokens == 0 {
        return Ok(None);
    }

    // 5. 提取模型名称
    let model = value
        .get("message")
        .and_then(|m| m.get("model"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // 6. 提取时间戳
    let timestamp_raw = value
        .get("timestamp")
        .and_then(|v| v.as_str())
        .ok_or_else(|| UsageParseError::MissingField("timestamp".to_string()))?;

    let timestamp = DateTime::parse_from_rfc3339(timestamp_raw)
        .map_err(|e| UsageParseError::InvalidFieldType(format!("timestamp: {}", e)))?
        .with_timezone(&Utc);

    // 7. 提取 session_id
    let session_id = context
        .session_id_from_file
        .clone()
        .or_else(|| {
            value
                .get("sessionId")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
        .or_else(|| {
            value
                .get("session_id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_default();

    if session_id.is_empty() {
        return Err(UsageParseError::MissingField("session_id".to_string()));
    }

    Ok(Some(UsageRecord {
        source: context.source.clone(),
        config_dir: context.config_dir.clone(),
        project_path: context.project_from_path.clone(),
        project_name: None, // Phase 3/4 再做 ProjectRegistry 映射
        session_id,
        model,
        timestamp,
        input_tokens,
        output_tokens,
        cache_read_tokens,
        cache_create_tokens,
        total_tokens,
        raw_file_path: context.raw_file_path.clone(),
        line_no: context.line_no,
    }))
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::models::UsageSource;
    use std::path::PathBuf;

    fn test_context() -> UsageParseContext {
        UsageParseContext {
            config_dir: PathBuf::from("/home/user/.claude"),
            raw_file_path: PathBuf::from(
                "/home/user/.claude/projects/test/550e8400-e29b-41d4-a716-446655440000.jsonl",
            ),
            line_no: 1,
            session_id_from_file: Some("550e8400-e29b-41d4-a716-446655440000".to_string()),
            project_from_path: Some("test".to_string()),
            source: UsageSource::ClaudeJsonl,
        }
    }

    #[test]
    fn test_parse_assistant_with_usage() {
        let line = r#"{"type":"assistant","timestamp":"2026-05-27T01:40:41.560Z","sessionId":"550e8400-e29b-41d4-a716-446655440000","message":{"model":"claude-sonnet-4-6","usage":{"input_tokens":100,"output_tokens":50,"cache_read_input_tokens":20,"cache_creation_input_tokens":10},"stop_reason":"end_turn"}}"#;

        let result = parse_usage_line(line, &test_context());
        assert!(result.is_ok());

        let record = result.unwrap().unwrap();
        assert_eq!(record.input_tokens, 100);
        assert_eq!(record.output_tokens, 50);
        assert_eq!(record.cache_read_tokens, 20);
        assert_eq!(record.cache_create_tokens, 10);
        assert_eq!(record.total_tokens, 180);
        assert_eq!(record.model, Some("claude-sonnet-4-6".to_string()));
        assert_eq!(record.session_id, "550e8400-e29b-41d4-a716-446655440000");
        assert_eq!(record.source, UsageSource::ClaudeJsonl);
    }

    #[test]
    fn test_parse_non_assistant_returns_none() {
        let line = r#"{"type":"user","timestamp":"2026-05-27T01:40:41.560Z","message":{"content":"hello"}}"#;

        let result = parse_usage_line(line, &test_context());
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_parse_assistant_without_usage_returns_none() {
        let line = r#"{"type":"assistant","timestamp":"2026-05-27T01:40:41.560Z","message":{"content":"hello"}}"#;

        let result = parse_usage_line(line, &test_context());
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_parse_assistant_with_null_usage_returns_none() {
        let line = r#"{"type":"assistant","timestamp":"2026-05-27T01:40:41.560Z","message":{"model":"claude-sonnet-4-6","usage":null}}"#;

        let result = parse_usage_line(line, &test_context());
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_parse_invalid_json_returns_error() {
        let line = r#"{invalid json"#;

        let result = parse_usage_line(line, &test_context());
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_missing_token_fields_defaults_to_zero() {
        let line = r#"{"type":"assistant","timestamp":"2026-05-27T01:40:41.560Z","sessionId":"550e8400-e29b-41d4-a716-446655440000","message":{"model":"claude-sonnet-4-6","usage":{"input_tokens":100,"output_tokens":50}}}"#;

        let result = parse_usage_line(line, &test_context()).unwrap().unwrap();
        assert_eq!(result.input_tokens, 100);
        assert_eq!(result.output_tokens, 50);
        assert_eq!(result.cache_read_tokens, 0);
        assert_eq!(result.cache_create_tokens, 0);
        assert_eq!(result.total_tokens, 150);
    }

    #[test]
    fn test_parse_zero_total_tokens_returns_none() {
        let line = r#"{"type":"assistant","timestamp":"2026-05-27T01:40:41.560Z","sessionId":"550e8400-e29b-41d4-a716-446655440000","message":{"model":"claude-sonnet-4-6","usage":{"input_tokens":0,"output_tokens":0,"cache_read_input_tokens":0,"cache_creation_input_tokens":0}}}"#;

        let result = parse_usage_line(line, &test_context());
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_session_id_from_file_priority() {
        let line = r#"{"type":"assistant","timestamp":"2026-05-27T01:40:41.560Z","sessionId":"fallback-id","message":{"model":"claude-sonnet-4-6","usage":{"input_tokens":100,"output_tokens":50}}}"#;

        let result = parse_usage_line(line, &test_context()).unwrap().unwrap();
        // 文件名 session_id 优先于行内 sessionId
        assert_eq!(result.session_id, "550e8400-e29b-41d4-a716-446655440000");
    }

    #[test]
    fn test_session_id_fallback_from_line() {
        let mut context = test_context();
        context.session_id_from_file = None;

        let line = r#"{"type":"assistant","timestamp":"2026-05-27T01:40:41.560Z","sessionId":"fallback-id","message":{"model":"claude-sonnet-4-6","usage":{"input_tokens":100,"output_tokens":50}}}"#;

        let result = parse_usage_line(line, &context).unwrap().unwrap();
        assert_eq!(result.session_id, "fallback-id");
    }

    #[test]
    fn test_timestamp_parsing() {
        let line = r#"{"type":"assistant","timestamp":"2026-05-27T01:40:41.560Z","sessionId":"550e8400-e29b-41d4-a716-446655440000","message":{"model":"claude-sonnet-4-6","usage":{"input_tokens":100,"output_tokens":50}}}"#;

        let result = parse_usage_line(line, &test_context()).unwrap().unwrap();
        assert_eq!(
            result.timestamp.to_rfc3339(),
            "2026-05-27T01:40:41.560+00:00"
        );
    }

    #[test]
    fn test_model_extraction() {
        let line = r#"{"type":"assistant","timestamp":"2026-05-27T01:40:41.560Z","sessionId":"550e8400-e29b-41d4-a716-446655440000","message":{"model":"kimi-for-coding","usage":{"input_tokens":100,"output_tokens":50}}}"#;

        let result = parse_usage_line(line, &test_context()).unwrap().unwrap();
        assert_eq!(result.model, Some("kimi-for-coding".to_string()));
    }

    #[test]
    fn test_parse_missing_timestamp_returns_error() {
        let line = r#"{"type":"assistant","sessionId":"550e8400","message":{"model":"claude","usage":{"input_tokens":100,"output_tokens":50}}}"#;

        let result = parse_usage_line(line, &test_context());
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("timestamp"), "错误应提及 timestamp: {}", err);
    }

    #[test]
    fn test_parse_invalid_timestamp_returns_error() {
        let line = r#"{"type":"assistant","timestamp":"not-a-valid-timestamp","sessionId":"550e8400","message":{"model":"claude","usage":{"input_tokens":100,"output_tokens":50}}}"#;

        let result = parse_usage_line(line, &test_context());
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("timestamp"), "错误应提及 timestamp: {}", err);
    }
}
