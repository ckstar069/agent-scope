/// 提取文件中的 frontmatter
/// 返回 (frontmatter_text, content_offset)
/// 只识别文件开头被 `---` 包围的 YAML 块
pub fn extract_frontmatter(content: &str) -> Option<(&str, usize)> {
    if !content.starts_with("---") {
        return None;
    }

    let after_open = &content[3..];
    let close_idx = after_open.find("\n---")?;

    let fm = &after_open[..close_idx];
    // 跳过 frontmatter 块末尾的 `\n---` 和后续换行
    let after_close = &after_open[close_idx + 4..];
    let leading_newlines = after_close
        .chars()
        .take_while(|c| *c == '\n' || *c == '\r')
        .count();
    let content_offset = 3 + close_idx + 4 + leading_newlines;

    // 确保 close_idx 后确实有结束标记
    Some((fm.trim(), content_offset))
}

/// 解析 frontmatter 文本为键值对
/// 支持 "key: value" 和 "key:\n  - item1\n  - item2" 两种形式
pub fn parse_frontmatter(raw: &str) -> Vec<(String, String)> {
    let mut result = Vec::new();

    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // 简单键值对：key: value
        if let Some(pos) = trimmed.find(':') {
            let key = trimmed[..pos].trim().to_string();
            let value = trimmed[pos + 1..].trim().to_string();
            result.push((key, value));
        }
    }

    result
}

/// 解析列表形式的 frontmatter 字段（如 paths）
/// 从起始行开始，读取后续的 "  - value" 行
pub fn parse_list_field(raw: &str, field_name: &str) -> Option<Vec<String>> {
    let lines: Vec<&str> = raw.lines().collect();

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if let Some(pos) = trimmed.find(':') {
            let key = trimmed[..pos].trim();
            if key == field_name {
                let mut values = Vec::new();
                // 读取后续缩进的列表项
                for next in lines.iter().skip(i + 1) {
                    let next_trimmed = next.trim();
                    if let Some(stripped) = next_trimmed.strip_prefix("- ") {
                        values.push(stripped.trim().to_string());
                    } else if !next_trimmed.is_empty()
                        && !next.starts_with(' ')
                        && !next.starts_with('\t')
                    {
                        // 非缩进行，列表结束
                        break;
                    }
                }
                if !values.is_empty() {
                    return Some(values);
                }
            }
        }
    }

    None
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_frontmatter_basic() {
        let content = "---\nname: test\ndescription: hello\n---\n# Title\n";
        let (fm, offset) = extract_frontmatter(content).unwrap();
        assert!(fm.contains("name: test"));
        assert!(fm.contains("description: hello"));
        assert_eq!(offset, content.find("# Title").unwrap());
    }

    #[test]
    fn test_extract_frontmatter_not_at_start() {
        let content = "# Title\n---\nname: test\n---\n";
        assert!(extract_frontmatter(content).is_none());
    }

    #[test]
    fn test_extract_frontmatter_no_closing() {
        let content = "---\nname: test\n# Title\n";
        assert!(extract_frontmatter(content).is_none());
    }

    #[test]
    fn test_parse_frontmatter_basic() {
        let raw = "name: git-commit-helper\ndescription: help with commits\ntrigger: on request";
        let pairs = parse_frontmatter(raw);
        let map: std::collections::HashMap<_, _> = pairs.into_iter().collect();
        assert_eq!(map.get("name"), Some(&"git-commit-helper".to_string()));
        assert_eq!(
            map.get("description"),
            Some(&"help with commits".to_string())
        );
        assert_eq!(map.get("trigger"), Some(&"on request".to_string()));
    }

    #[test]
    fn test_parse_list_field() {
        let raw = "name: test\npaths:\n  - src/**/*.rs\n  - tests/**/*.rs\ndescription: ok";
        let paths = parse_list_field(raw, "paths").unwrap();
        assert_eq!(paths, vec!["src/**/*.rs", "tests/**/*.rs"]);
    }

    #[test]
    fn test_parse_list_field_not_found() {
        let raw = "name: test\ndescription: ok";
        assert!(parse_list_field(raw, "paths").is_none());
    }

    #[test]
    fn test_parse_list_field_empty_list() {
        let raw = "name: test\npaths:\ndescription: ok";
        assert!(parse_list_field(raw, "paths").is_none());
    }
}
