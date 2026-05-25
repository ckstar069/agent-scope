use std::collections::HashMap;

use super::models::{SerClaudeMemoryAsset, SerMemoryDuplicateGroup};

/// 归一化文本：去 Markdown 标记 → 去多余空白 → 小写 → 去标点 → 规整连续空白
pub fn normalize_text(text: &str) -> String {
    let mut result = String::with_capacity(text.len());

    let mut in_code_block = false;
    for line in text.lines() {
        // 跳过代码块
        if line.trim_start().starts_with("```") {
            in_code_block = !in_code_block;
            continue;
        }
        if in_code_block {
            continue;
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // 跳过 frontmatter 分隔线
        if trimmed == "---" {
            continue;
        }

        // 去除行首 Markdown 标记
        let content = trimmed
            .trim_start_matches('#')
            .trim_start_matches('>')
            .trim_start_matches('-')
            .trim_start_matches('*')
            .trim_start_matches('+')
            .trim_start();

        if content.is_empty() {
            continue;
        }

        // 去除行内 Markdown 格式标记
        let cleaned = content
            .replace("**", "")
            .replace("__", "")
            .replace("*", "")
            .replace("_", "")
            .replace("`", "")
            .replace("~~", "");

        if !result.is_empty() {
            result.push(' ');
        }
        result.push_str(cleaned.trim());
    }

    // 小写化
    let lower = result.to_lowercase();

    // 去标点
    let no_punct: String = lower
        .chars()
        .filter(|c| {
            c.is_alphanumeric()
                || *c == ' '
                || *c == '/'
                || *c == '\\'
                || *c == '@'
                || *c == '.'
                || *c == '-'
        })
        .collect();

    // 规整连续空白为单空格
    let mut collapsed = String::with_capacity(no_punct.len());
    let mut prev_space = false;
    for ch in no_punct.chars() {
        if ch == ' ' {
            if !prev_space {
                collapsed.push(ch);
            }
            prev_space = true;
        } else {
            collapsed.push(ch);
            prev_space = false;
        }
    }

    collapsed
}

/// Markdown 段落
#[derive(Debug, Clone)]
pub struct Paragraph {
    pub heading: Option<String>,
    pub content: String,
    pub asset_id: String,
}

/// 按 Markdown 标题切分段落
pub fn split_paragraphs(content: &str, asset_id: &str) -> Vec<Paragraph> {
    let mut paragraphs: Vec<Paragraph> = Vec::new();
    let mut current_heading: Option<String> = None;
    let mut current_lines: Vec<String> = Vec::new();

    // 跳过 frontmatter
    let mut in_frontmatter = false;
    let mut frontmatter_count = 0;

    for line in content.lines() {
        let trimmed = line.trim();

        // frontmatter 检测
        if trimmed == "---" && frontmatter_count < 2 {
            in_frontmatter = !in_frontmatter;
            frontmatter_count += 1;
            continue;
        }
        if in_frontmatter {
            continue;
        }

        // 标题行检测
        if trimmed.starts_with('#') && !trimmed.starts_with("```") {
            // 保存当前段落
            if !current_lines.is_empty() {
                let para_content: String = current_lines.join("\n");
                if !para_content.trim().is_empty() {
                    paragraphs.push(Paragraph {
                        heading: current_heading.clone(),
                        content: para_content,
                        asset_id: asset_id.to_string(),
                    });
                }
                current_lines.clear();
            }
            current_heading = Some(trimmed.trim_start_matches('#').trim_start().to_string());
            continue;
        }

        current_lines.push(line.to_string());
    }

    // 保存最后一个段落
    if !current_lines.is_empty() {
        let para_content: String = current_lines.join("\n");
        if !para_content.trim().is_empty() {
            paragraphs.push(Paragraph {
                heading: current_heading,
                content: para_content,
                asset_id: asset_id.to_string(),
            });
        }
    }

    // 如果没有切出任何段落，整个内容作为一个段落
    if paragraphs.is_empty() && !content.trim().is_empty() {
        paragraphs.push(Paragraph {
            heading: None,
            content: content.to_string(),
            asset_id: asset_id.to_string(),
        });
    }

    paragraphs
}

/// 计算 64-bit lightweight 内容 hash（DefaultHasher，非 SHA-256）
pub fn compute_content_hash(text: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// 计算 Jaccard 相似度（词集合交集/并集）
pub fn compute_jaccard_similarity(text_a: &str, text_b: &str) -> f64 {
    let words_a: std::collections::HashSet<&str> = text_a.split_whitespace().collect();
    let words_b: std::collections::HashSet<&str> = text_b.split_whitespace().collect();

    if words_a.is_empty() && words_b.is_empty() {
        return 1.0;
    }
    if words_a.is_empty() || words_b.is_empty() {
        return 0.0;
    }

    let intersection = words_a.intersection(&words_b).count();
    let union = words_a.union(&words_b).count();

    if union == 0 {
        return 0.0;
    }

    intersection as f64 / union as f64
}

/// 重复检测阈值
const EXACT_DUP_THRESHOLD: f64 = 1.0;
const NEAR_DUP_THRESHOLD: f64 = 0.8;
const MAX_OVERLAP_CONTENT_LEN: usize = 200;

/// 从资产列表中检测重复
///
/// 策略：两阶段
/// 1. 精确 hash 匹配（归一化文本 hash 相同）
/// 2. Jaccard 相似度 ≥ 0.8
///
/// 性能约束：
/// - 只对 exists=true 且有 content_preview 的资产检测
/// - 资产数 ≤ 50 时全量比对；> 50 时只比对同 scope 内的资产
pub fn find_duplicates(assets: &[SerClaudeMemoryAsset]) -> Vec<SerMemoryDuplicateGroup> {
    // 收集可检测资产
    let eligible: Vec<&SerClaudeMemoryAsset> = assets
        .iter()
        .filter(|a| a.exists && a.content_preview.is_some())
        .collect();

    if eligible.len() < 2 {
        return Vec::new();
    }

    // 当资产数 > 50，按 scope 分组只比对同 scope 内的资产
    let scope_groups: HashMap<String, Vec<&SerClaudeMemoryAsset>> = if eligible.len() > 50 {
        let mut groups: HashMap<String, Vec<&SerClaudeMemoryAsset>> = HashMap::new();
        for asset in eligible {
            groups.entry(asset.scope.clone()).or_default().push(asset);
        }
        groups
    } else {
        // 全量比对：所有资产归入同一组
        let mut groups: HashMap<String, Vec<&SerClaudeMemoryAsset>> = HashMap::new();
        groups.insert("_all".to_string(), eligible.clone());
        groups
    };

    let mut all_groups: Vec<SerMemoryDuplicateGroup> = Vec::new();

    for scope_eligible in scope_groups.values() {
        if scope_eligible.len() < 2 {
            continue;
        }
        let scope_dups = find_duplicates_within(scope_eligible);
        all_groups.extend(scope_dups);
    }

    all_groups
}

/// 在一组资产内检测重复（两阶段：精确 hash + Jaccard）
fn find_duplicates_within(eligible: &[&SerClaudeMemoryAsset]) -> Vec<SerMemoryDuplicateGroup> {
    // 阶段 1：按归一化 hash 分组
    let mut hash_groups: HashMap<String, Vec<String>> = HashMap::new();
    let mut normalized_texts: HashMap<String, String> = HashMap::new();

    for asset in eligible {
        let preview = asset.content_preview.as_ref().unwrap();
        let paragraphs = split_paragraphs(preview, &asset.id);

        let all_normalized: String = paragraphs
            .iter()
            .map(|p| normalize_text(&p.content))
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(" ");

        if all_normalized.is_empty() {
            continue;
        }

        let hash = compute_content_hash(&all_normalized);
        hash_groups.entry(hash.clone()).or_default().push(asset.id.clone());
        normalized_texts.insert(asset.id.clone(), all_normalized);
    }

    let mut groups: Vec<SerMemoryDuplicateGroup> = Vec::new();
    let mut processed_ids: std::collections::HashSet<String> = std::collections::HashSet::new();

    // 精确重复组
    for asset_ids in hash_groups.values() {
        if asset_ids.len() < 2 {
            continue;
        }
        for id in asset_ids {
            processed_ids.insert(id.clone());
        }
        let overlap = get_overlap_content(&asset_ids[0], &normalized_texts);
        let hash_str = compute_content_hash(&asset_ids.join(","));
        groups.push(SerMemoryDuplicateGroup {
            group_id: format!("dup_{}", &hash_str[..8]),
            asset_ids: asset_ids.clone(),
            similarity: EXACT_DUP_THRESHOLD,
            overlap_content: overlap,
            suggestion: "merge".to_string(),
        });
    }

    // 阶段 2：Jaccard 近似重复（只对未处理过的资产）
    // 使用 connected components：Jaccard >= 阈值的 pair 作为边，输出连通分量
    let unprocessed: Vec<&SerClaudeMemoryAsset> = eligible
        .iter()
        .filter(|a| !processed_ids.contains(&a.id))
        .copied()
        .collect();

    // 收集所有 Jaccard >= 阈值的边
    let mut edges: Vec<(String, String, f64)> = Vec::new();
    for (i, a) in unprocessed.iter().enumerate() {
        let norm_a = match normalized_texts.get(&a.id) {
            Some(t) => t,
            None => continue,
        };
        for b in unprocessed.iter().skip(i + 1) {
            let norm_b = match normalized_texts.get(&b.id) {
                Some(t) => t,
                None => continue,
            };
            let sim = compute_jaccard_similarity(norm_a, norm_b);
            if sim >= NEAR_DUP_THRESHOLD {
                edges.push((a.id.clone(), b.id.clone(), sim));
            }
        }
    }

    // 构建邻接表 → BFS 求连通分量
    let mut adj: HashMap<String, Vec<String>> = HashMap::new();
    for (a, b, _sim) in &edges {
        adj.entry(a.clone()).or_default().push(b.clone());
        adj.entry(b.clone()).or_default().push(a.clone());
    }

    let mut visited: std::collections::HashSet<String> = std::collections::HashSet::new();
    for a in adj.keys() {
        if visited.contains(a) {
            continue;
        }
        // BFS
        let mut component: Vec<String> = Vec::new();
        let mut queue: std::collections::VecDeque<String> = std::collections::VecDeque::new();
        queue.push_back(a.clone());
        visited.insert(a.clone());
        while let Some(node) = queue.pop_front() {
            component.push(node.clone());
            if let Some(neighbors) = adj.get(&node) {
                for neighbor in neighbors {
                    if !visited.contains(neighbor) {
                        visited.insert(neighbor.clone());
                        queue.push_back(neighbor.clone());
                    }
                }
            }
        }

        if component.len() < 2 {
            continue;
        }

        // 计算组内代表 Jaccard（取首条边的相似度）
        let sim = edges
            .iter()
            .find(|(a, b, _)| component.contains(a) && component.contains(b))
            .map(|(_, _, s)| *s)
            .unwrap_or(NEAR_DUP_THRESHOLD);

        let overlap = get_overlap_content(&component[0], &normalized_texts);
        let hash_str = compute_content_hash(&component.join(","));
        groups.push(SerMemoryDuplicateGroup {
            group_id: format!("dup_{}", &hash_str[..8]),
            asset_ids: component,
            similarity: (sim * 100.0).round() / 100.0,
            overlap_content: overlap,
            suggestion: "review".to_string(),
        });
    }

    groups
}

/// 获取共现内容摘要
fn get_overlap_content(asset_id: &str, normalized_texts: &HashMap<String, String>) -> String {
    normalized_texts
        .get(asset_id)
        .map(|t| {
            if t.len() > MAX_OVERLAP_CONTENT_LEN {
                // 安全截断：找到不超过边界的 char boundary
                let mut end = MAX_OVERLAP_CONTENT_LEN;
                while !t.is_char_boundary(end) && end > 0 {
                    end -= 1;
                }
                format!("{}...", &t[..end])
            } else {
                t.clone()
            }
        })
        .unwrap_or_default()
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_text_basic() {
        let input = "# Hello World\n\nThis is **bold** and *italic* text\n";
        let normalized = normalize_text(input);
        assert_eq!(normalized, "hello world this is bold and italic text");
    }

    #[test]
    fn test_normalize_text_removes_code_blocks() {
        let input = "Some text\n```javascript\nconst x = 1;\n```\nMore text\n";
        let normalized = normalize_text(input);
        assert!(!normalized.contains("const"));
        assert!(normalized.contains("some text"));
        assert!(normalized.contains("more text"));
    }

    #[test]
    fn test_normalize_text_removes_frontmatter() {
        let input = "---\nname: test\n---\n# Title\nContent here\n";
        let normalized = normalize_text(input);
        assert!(!normalized.contains("name:"));
        assert!(normalized.contains("title"));
        assert!(normalized.contains("content here"));
    }

    #[test]
    fn test_normalize_text_removes_headings() {
        let input = "## Section Title\nBody text\n";
        let normalized = normalize_text(input);
        assert!(normalized.contains("section title"));
        assert!(normalized.contains("body text"));
    }

    #[test]
    fn test_split_paragraphs_single() {
        let content = "Just some text without headings.";
        let paras = split_paragraphs(content, "test_id");
        assert_eq!(paras.len(), 1);
        assert!(paras[0].heading.is_none());
    }

    #[test]
    fn test_split_paragraphs_by_heading() {
        let content = "# Section 1\nContent 1\n\n# Section 2\nContent 2\n";
        let paras = split_paragraphs(content, "test_id");
        assert_eq!(paras.len(), 2);
        assert_eq!(paras[0].heading, Some("Section 1".to_string()));
        assert_eq!(paras[1].heading, Some("Section 2".to_string()));
    }

    #[test]
    fn test_split_paragraphs_skips_frontmatter() {
        let content = "---\nname: test\n---\n# Title\nBody\n";
        let paras = split_paragraphs(content, "test_id");
        assert_eq!(paras.len(), 1);
        assert_eq!(paras[0].heading, Some("Title".to_string()));
    }

    #[test]
    fn test_compute_content_hash_deterministic() {
        let hash1 = compute_content_hash("hello world");
        let hash2 = compute_content_hash("hello world");
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_compute_content_hash_different() {
        let hash1 = compute_content_hash("hello world");
        let hash2 = compute_content_hash("goodbye world");
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_jaccard_similarity_identical() {
        let sim = compute_jaccard_similarity("hello world test", "hello world test");
        assert!((sim - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_jaccard_similarity_disjoint() {
        let sim = compute_jaccard_similarity("alpha beta", "gamma delta");
        assert!((sim - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_jaccard_similarity_partial() {
        let sim = compute_jaccard_similarity("hello world test", "hello world other");
        // 交集: {hello, world} = 2, 并集: {hello, world, test, other} = 4
        assert!((sim - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_jaccard_similarity_empty() {
        let sim = compute_jaccard_similarity("", "");
        assert!((sim - 1.0).abs() < 0.001);

        let sim = compute_jaccard_similarity("hello", "");
        assert!((sim - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_find_duplicates_exact() {
        let assets = vec![
            SerClaudeMemoryAsset {
                id: "a1".to_string(),
                scope: "project".to_string(),
                asset_type: "project_claude_md".to_string(),
                logical_path: "/a/CLAUDE.md".to_string(),
                native_path: "/a/CLAUDE.md".to_string(),
                content_hash: None,
                content_preview: Some("# Rules\nAlways use pnpm\n".to_string()),
                content_truncated: false,
                line_count: Some(2),
                byte_size: Some(24),
                mtime_ms: None,
                frontmatter: None,
                secret_issues: Vec::new(),
                exists: true,
            },
            SerClaudeMemoryAsset {
                id: "a2".to_string(),
                scope: "project".to_string(),
                asset_type: "project_claude_md".to_string(),
                logical_path: "/b/CLAUDE.md".to_string(),
                native_path: "/b/CLAUDE.md".to_string(),
                content_hash: None,
                content_preview: Some("# Rules\nAlways use pnpm\n".to_string()),
                content_truncated: false,
                line_count: Some(2),
                byte_size: Some(24),
                mtime_ms: None,
                frontmatter: None,
                secret_issues: Vec::new(),
                exists: true,
            },
        ];

        let groups = find_duplicates(&assets);
        assert_eq!(groups.len(), 1, "应检测到 1 个精确重复组");
        assert_eq!(groups[0].similarity, 1.0);
        assert_eq!(groups[0].suggestion, "merge");
    }

    #[test]
    fn test_find_duplicates_no_duplicates() {
        let assets = vec![
            SerClaudeMemoryAsset {
                id: "a1".to_string(),
                scope: "project".to_string(),
                asset_type: "project_claude_md".to_string(),
                logical_path: "/a/CLAUDE.md".to_string(),
                native_path: "/a/CLAUDE.md".to_string(),
                content_hash: None,
                content_preview: Some("Use pnpm for package management\n".to_string()),
                content_truncated: false,
                line_count: Some(1),
                byte_size: Some(35),
                mtime_ms: None,
                frontmatter: None,
                secret_issues: Vec::new(),
                exists: true,
            },
            SerClaudeMemoryAsset {
                id: "a2".to_string(),
                scope: "project".to_string(),
                asset_type: "project_claude_md".to_string(),
                logical_path: "/b/CLAUDE.md".to_string(),
                native_path: "/b/CLAUDE.md".to_string(),
                content_hash: None,
                content_preview: Some("Always write tests before code\n".to_string()),
                content_truncated: false,
                line_count: Some(1),
                byte_size: Some(31),
                mtime_ms: None,
                frontmatter: None,
                secret_issues: Vec::new(),
                exists: true,
            },
        ];

        let groups = find_duplicates(&assets);
        assert!(groups.is_empty(), "内容不同不应检测到重复");
    }

    #[test]
    fn test_find_duplicates_skips_nonexistent() {
        let assets = vec![SerClaudeMemoryAsset {
            id: "a1".to_string(),
            scope: "project".to_string(),
            asset_type: "project_claude_md".to_string(),
            logical_path: "/a/CLAUDE.md".to_string(),
            native_path: "/a/CLAUDE.md".to_string(),
            content_hash: None,
            content_preview: None,
            content_truncated: false,
            line_count: None,
            byte_size: None,
            mtime_ms: None,
            frontmatter: None,
            secret_issues: Vec::new(),
            exists: false,
        }];

        let groups = find_duplicates(&assets);
        assert!(groups.is_empty());
    }

    // ── 边界测试：大小写/空白/标点容忍度 ──

    /// 归一化后大小写差异应被消除，产生精确重复
    #[test]
    fn test_dedup_case_insensitive() {
        let assets = vec![
            make_asset("a1", "Always use pnpm for builds\n"),
            make_asset("a2", "always USE Pnpm for builds\n"),
        ];
        let groups = find_duplicates(&assets);
        assert_eq!(groups.len(), 1, "大小写差异归一化后应检测到精确重复");
        assert_eq!(groups[0].similarity, 1.0);
        assert_eq!(groups[0].suggestion, "merge", "精确重复应建议 merge");
    }

    /// 归一化后多余空白应被消除，产生精确重复
    #[test]
    fn test_dedup_whitespace_tolerance() {
        let assets = vec![
            make_asset("a1", "Always  use   pnpm for builds\n\n\n"),
            make_asset("a2", "Always use pnpm for builds\n"),
        ];
        let groups = find_duplicates(&assets);
        assert_eq!(groups.len(), 1, "多余空白归一化后应检测到精确重复");
        assert_eq!(groups[0].similarity, 1.0);
        assert_eq!(groups[0].suggestion, "merge", "精确重复应建议 merge");
    }

    /// 归一化后 Markdown 标记差异应被消除，产生精确重复
    #[test]
    fn test_dedup_punctuation_markdown_tolerance() {
        let assets = vec![
            make_asset("a1", "Always use **pnpm** for builds\n"),
            make_asset("a2", "Always use pnpm for builds\n"),
        ];
        let groups = find_duplicates(&assets);
        assert_eq!(groups.len(), 1, "Markdown 标记差异归一化后应检测到精确重复");
        assert_eq!(groups[0].similarity, 1.0);
        assert_eq!(groups[0].suggestion, "merge", "精确重复应建议 merge");
    }

    // ── 边界测试：短文本不误判重复 ──

    /// 短文本（1-2 个词）即使部分词重叠，Jaccard 也不应超过阈值
    #[test]
    fn test_dedup_short_text_no_false_positive() {
        let assets = vec![
            make_asset("a1", "pnpm\n"),
            make_asset("a2", "pnpm install\n"),
        ];
        let groups = find_duplicates(&assets);
        // "pnpm" vs "pnpm install": Jaccard = 1/2 = 0.5 < 0.8
        assert!(groups.is_empty(), "短文本部分重叠不应误判为近似重复");
    }

    /// 两个各只有 1 个不同词的短文本不应被判为重复
    #[test]
    fn test_dedul_short_text_single_word_different() {
        let assets = vec![
            make_asset("a1", "hello\n"),
            make_asset("a2", "world\n"),
        ];
        let groups = find_duplicates(&assets);
        assert!(groups.is_empty(), "单词完全不同不应误判");
    }

    // ── 边界测试：duplicate group 阈值边界 ──

    /// Jaccard 刚好 = 0.8 时应检测为近似重复
    /// 构造：集合 A 有 4 词 {alpha, beta, gamma, delta}
    /// 集合 B 有 5 词 {alpha, beta, gamma, delta, epsilon}（A ⊂ B）
    /// 交集 = 4，并集 = 5 → 4/5 = 0.8
    #[test]
    fn test_dedup_threshold_exact_boundary() {
        let assets = vec![
            make_asset("a1", "alpha beta gamma delta\n"),
            make_asset("a2", "alpha beta gamma delta epsilon\n"),
        ];
        let groups = find_duplicates(&assets);
        assert_eq!(groups.len(), 1, "Jaccard = 0.8 刚好在阈值边界应检测为近似重复");
        assert!((groups[0].similarity - 0.8).abs() < 0.01);
        assert_eq!(groups[0].suggestion, "review");
    }

    /// Jaccard < 0.8 不应检测为重复
    /// 构造：交集 = 3，并集 = 7 → 3/7 ≈ 0.43 < 0.8
    #[test]
    fn test_dedup_threshold_below_boundary() {
        let assets = vec![
            make_asset("a1", "alpha beta gamma delta\n"),
            make_asset("a2", "alpha beta gamma zeta eta\n"),
        ];
        let groups = find_duplicates(&assets);
        assert!(groups.is_empty(), "Jaccard < 0.8 不应检测为重复");
    }

    /// 3 个互相高度相似但非完全相同的资产应归入同一组，而非产生多个 pair
    /// a1={alpha,beta,gamma,delta} (4 词)
    /// a2={alpha,beta,gamma,delta,epsilon} (5 词，a1 ⊂ a2)
    /// a3={alpha,beta,gamma,delta,theta} (5 词，a1 ⊂ a3)
    /// a1-a2: Jaccard=4/5=0.8, a1-a3: Jaccard=4/5=0.8
    /// 预期：a1 作为代表，a2 和 a3 都加入同一组，只有 1 个组
    #[test]
    fn test_dedup_cluster_no_multiple_pairs() {
        let assets = vec![
            make_asset("a1", "alpha beta gamma delta\n"),
            make_asset("a2", "alpha beta gamma delta epsilon\n"),
            make_asset("a3", "alpha beta gamma delta theta\n"),
        ];
        let groups = find_duplicates(&assets);
        // 3 资产通过 a1 互相近似重复应归入 1 个组（不是多个 pair）
        assert_eq!(groups.len(), 1, "3 个互相近似的资产不应产生多个 pair");
        assert!(groups[0].asset_ids.len() >= 2, "组内至少有 2 个资产");
        assert_eq!(groups[0].suggestion, "review");
    }

    /// Connected components：A-B 相似、B-C 相似、A-C 不相似时，三者应归入同一 group
    /// a1={a,b,c,d,e} (5 词)
    /// a2={a,b,c,d,f} (5 词) — 与 a1 共享 4 词，Jaccard=4/6≈0.67 < 0.8
    /// 需要构造 Jaccard >= 0.8 的链式相似：
    /// a1={a,b,c,d} (4 词)
    /// a2={a,b,c,d,e} (5 词) — a1⊂a2, Jaccard=4/5=0.8
    /// a3={a,b,c,d,e,f} (6 词) — a2⊂a3, Jaccard=5/6≈0.83
    /// a1-a3: Jaccard=4/6≈0.67 < 0.8（不直接相似）
    /// 但 A-B 和 B-C 构成连通分量，三者应归入同一组
    #[test]
    fn test_dedup_connected_components_chain() {
        let assets = vec![
            make_asset("a1", "alpha beta gamma delta\n"),
            make_asset("a2", "alpha beta gamma delta epsilon\n"),
            make_asset("a3", "alpha beta gamma delta epsilon zeta\n"),
        ];
        let groups = find_duplicates(&assets);
        assert_eq!(groups.len(), 1, "链式相似应归入同一连通分量");
        assert_eq!(groups[0].asset_ids.len(), 3, "3 个资产都应在组内");
        assert_eq!(groups[0].suggestion, "review");
    }

    /// 3 个完全相同的资产应归入同一个精确重复组
    #[test]
    fn test_dedup_three_exact_duplicates() {
        let assets = vec![
            make_asset("a1", "Use pnpm for all builds\n"),
            make_asset("a2", "Use pnpm for all builds\n"),
            make_asset("a3", "Use pnpm for all builds\n"),
        ];
        let groups = find_duplicates(&assets);
        assert_eq!(groups.len(), 1, "3 个精确重复应归入同一组");
        assert_eq!(groups[0].asset_ids.len(), 3);
        assert_eq!(groups[0].similarity, 1.0);
        assert_eq!(groups[0].suggestion, "merge");
    }

    /// 辅助函数：快速创建测试资产
    fn make_asset(id: &str, preview: &str) -> SerClaudeMemoryAsset {
        SerClaudeMemoryAsset {
            id: id.to_string(),
            scope: "project".to_string(),
            asset_type: "project_claude_md".to_string(),
            logical_path: format!("/{}.md", id),
            native_path: format!("/{}.md", id),
            content_hash: None,
            content_preview: Some(preview.to_string()),
            content_truncated: false,
            line_count: None,
            byte_size: None,
            mtime_ms: None,
            frontmatter: None,
            secret_issues: Vec::new(),
            exists: true,
        }
    }
}
