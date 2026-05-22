use regex::Regex;

use super::models::SerSecretIssue;

/// Secret Scanner — 本地正则匹配检测敏感信息
pub struct SecretScanner;

impl Default for SecretScanner {
    fn default() -> Self {
        Self
    }
}

impl SecretScanner {
    pub fn new() -> Self {
        Self
    }

    /// 扫描文本内容，返回所有命中的敏感信息 issue
    pub fn scan(&self, content: &str) -> Vec<SerSecretIssue> {
        let mut issues = Vec::new();

        // API key 模式
        let api_key_re =
            r"(?i)(api[_-]?key|apikey)\s*[:=]\s*[\x27\x22]?([a-zA-Z0-9_-]{16,})[\x27\x22]?";
        if let Ok(re) = Regex::new(api_key_re) {
            for mat in re.find_iter(content) {
                if let Some(caps) = re.captures(mat.as_str()) {
                    let matched = caps.get(2).map(|m| m.as_str()).unwrap_or(mat.as_str());
                    let (line, col_start, col_end) =
                        self.position_in_text(content, mat.start(), mat.end());
                    issues.push(SerSecretIssue {
                        issue_type: "api_key".to_string(),
                        line_number: line,
                        column_start: col_start,
                        column_end: col_end,
                        matched_text: Self::mask_key(matched),
                    });
                }
            }
        }

        // Token 模式（bearer/token 后可选 : 或 =）
        let token_re =
            r"(?i)(?:token|bearer)\s*[:=]?\s*[\x27\x22]?([a-zA-Z0-9_\-\.]{16,})[\x27\x22]?";
        if let Ok(re) = Regex::new(token_re) {
            for mat in re.find_iter(content) {
                if let Some(caps) = re.captures(mat.as_str()) {
                    let matched = caps.get(1).map(|m| m.as_str()).unwrap_or(mat.as_str());
                    let (line, col_start, col_end) =
                        self.position_in_text(content, mat.start(), mat.end());
                    issues.push(SerSecretIssue {
                        issue_type: "token".to_string(),
                        line_number: line,
                        column_start: col_start,
                        column_end: col_end,
                        matched_text: Self::mask_key(matched),
                    });
                }
            }
        }

        // Password 模式
        let pwd_re =
            r"(?i)(password|passwd|pwd)\s*[:=]\s*[\x27\x22]?([^\x27\x22\s]{8,})[\x27\x22]?";
        if let Ok(re) = Regex::new(pwd_re) {
            for mat in re.find_iter(content) {
                if let Some(_caps) = re.captures(mat.as_str()) {
                    let (line, col_start, col_end) =
                        self.position_in_text(content, mat.start(), mat.end());
                    issues.push(SerSecretIssue {
                        issue_type: "password".to_string(),
                        line_number: line,
                        column_start: col_start,
                        column_end: col_end,
                        matched_text: "***".to_string(),
                    });
                }
            }
        }

        // Private URL 模式（含凭据的 URL）
        if let Ok(re) = Regex::new(r"(?i)(https?://[^:]+:[^@]+@[^\s]+)") {
            for mat in re.find_iter(content) {
                let (line, col_start, col_end) =
                    self.position_in_text(content, mat.start(), mat.end());
                issues.push(SerSecretIssue {
                    issue_type: "private_url".to_string(),
                    line_number: line,
                    column_start: col_start,
                    column_end: col_end,
                    matched_text: Self::mask_url(mat.as_str()),
                });
            }
        }

        // ENV 敏感变量模式
        let env_re = r"(?i)(DATABASE_URL|SECRET_KEY|PRIVATE_KEY|AWS_ACCESS_KEY)\s*[:=]\s*[\x27\x22]?([^\x27\x22\n]+)";
        if let Ok(re) = Regex::new(env_re) {
            for mat in re.find_iter(content) {
                if let Some(caps) = re.captures(mat.as_str()) {
                    let matched = caps.get(2).map(|m| m.as_str()).unwrap_or(mat.as_str());
                    let (line, col_start, col_end) =
                        self.position_in_text(content, mat.start(), mat.end());
                    issues.push(SerSecretIssue {
                        issue_type: "env_content".to_string(),
                        line_number: line,
                        column_start: col_start,
                        column_end: col_end,
                        matched_text: Self::mask_env(matched),
                    });
                }
            }
        }

        issues
    }

    // ─── 脱敏策略 ───

    fn mask_key(key: &str) -> String {
        if key.len() <= 4 {
            "***".to_string()
        } else {
            format!("{}***", &key[..4])
        }
    }

    fn mask_url(url: &str) -> String {
        // 将 https://user:pass@host 转为 https://***@host
        if let Some(at_pos) = url.rfind('@') {
            let prefix = &url[..url.find("://").unwrap_or(0) + 3];
            let suffix = &url[at_pos..];
            format!("{}{}{}", prefix, "***", suffix)
        } else {
            "***".to_string()
        }
    }

    fn mask_env(value: &str) -> String {
        if value.len() <= 4 {
            "***".to_string()
        } else {
            format!("{}***", &value[..4])
        }
    }

    // ─── 位置计算 ───

    fn position_in_text(&self, text: &str, start: usize, end: usize) -> (usize, usize, usize) {
        let prefix = &text[..start];
        let line = prefix.chars().filter(|&c| c == '\n').count() + 1;
        let line_start = prefix.rfind('\n').map(|i| i + 1).unwrap_or(0);
        let col_start = start - line_start;
        let col_end = end - line_start;
        (line, col_start, col_end)
    }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_api_key() {
        let scanner = SecretScanner::new();
        let text = "api_key = sk-abc1234567890abcdef\nsome normal text";
        let issues = scanner.scan(text);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].issue_type, "api_key");
        assert_eq!(issues[0].matched_text, "sk-a***");
    }

    #[test]
    fn test_scan_token() {
        let scanner = SecretScanner::new();
        let text = "Authorization: bearer ghp_xxxxxxxxxxxxxxxxxxxx";
        let issues = scanner.scan(text);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].issue_type, "token");
        assert_eq!(issues[0].matched_text, "ghp_***");
    }

    #[test]
    fn test_scan_password() {
        let scanner = SecretScanner::new();
        let text = "password = mySecret123";
        let issues = scanner.scan(text);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].issue_type, "password");
        assert_eq!(issues[0].matched_text, "***");
    }

    #[test]
    fn test_scan_private_url() {
        let scanner = SecretScanner::new();
        let text = "connect to https://admin:secret@internal.server.com/api";
        let issues = scanner.scan(text);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].issue_type, "private_url");
        assert!(issues[0].matched_text.contains("***"));
    }

    #[test]
    fn test_scan_env_content() {
        let scanner = SecretScanner::new();
        let text = "DATABASE_URL=postgres://user:pass@localhost/db";
        let issues = scanner.scan(text);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].issue_type, "env_content");
    }

    #[test]
    fn test_masking_preserves_prefix() {
        let scanner = SecretScanner::new();
        let text = "api_key=sk-live-1234567890abcdef";
        let issues = scanner.scan(text);
        assert!(!issues.is_empty());
        assert!(issues[0].matched_text.starts_with("sk-l"));
        assert!(issues[0].matched_text.contains("***"));
    }

    #[test]
    fn test_no_false_positive_short_string() {
        let scanner = SecretScanner::new();
        let text = "token = abc\npassword = 123\n";
        let issues = scanner.scan(text);
        // 太短的值不应命中
        assert!(issues.is_empty());
    }

    #[test]
    fn test_line_number_calculation() {
        let scanner = SecretScanner::new();
        let text = "line1\nline2\nline3\nAPI_KEY = sk-1234567890abcdef\nline5";
        let issues = scanner.scan(text);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].line_number, 4);
    }
}
