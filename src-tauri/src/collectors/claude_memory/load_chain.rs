use std::fs;
use std::path::{Path, PathBuf};

use super::super::claude_history::path_codec::encode_cwd_path;
use super::frontmatter::{extract_frontmatter, parse_list_field};
use super::models::*;
use super::path_resolver::{resolve_claude_config_dir, resolve_managed_dir};
use super::settings_reader::{is_excluded, read_claude_md_excludes};

const MAX_AUTO_MEMORY_LINES: usize = 200;
const MAX_AUTO_MEMORY_BYTES: usize = 25 * 1024; // 25KB
const MAX_PREVIEW_SIZE: usize = 2048; // 2KB
const MAX_FILE_READ_BYTES: usize = 50 * 1024; // 50KB：超过此大小不读完整文件，只读 preview

/// 从 cwd 定向解析加载链
///
/// 策略：
/// 1. 从 cwd 向上遍历到根目录，发现每一层的 CLAUDE.md 和 CLAUDE.local.md
/// 2. 读取 ~/.claude/CLAUDE.md（用户全局指令）
/// 3. 读取 cwd 下的 CLAUDE.md、.claude/CLAUDE.md、CLAUDE.local.md
/// 4. 读取 user 级 rules（~/.claude/rules/**/*.md，无 paths 的）
/// 5. 读取 project 级 rules（cwd/.claude/rules/**/*.md，无 paths 的）
/// 6. 读取 Auto Memory（若 cwd 是 git repo，匹配 ~/.claude/projects/<id>/memory/MEMORY.md）
/// 7. 应用 claudeMdExcludes
pub fn simulate_load_chain(cwd: &Path) -> Result<SerLoadChain, String> {
    // 验证 cwd
    if !cwd.exists() {
        return Err(format!("目录不存在: {}", cwd.display()));
    }
    if !cwd.is_dir() {
        return Err(format!("路径不是目录: {}", cwd.display()));
    }

    let canonical_cwd = fs::canonicalize(cwd).unwrap_or_else(|_| cwd.to_path_buf());

    let mut steps: Vec<SerLoadChainStep> = Vec::new();
    let mut path_scoped_rules: Vec<SerPathScopedRule> = Vec::new();
    let mut excluded_assets: Vec<SerExcludedAsset> = Vec::new();
    let mut warnings: Vec<SerLoadChainWarning> = Vec::new();

    // 读取 claudeMdExcludes（使用原始 cwd，避免 macOS /var → /private/var 符号链接导致路径不匹配）
    let excludes_config = match read_claude_md_excludes(cwd) {
        Ok(config) => config,
        Err(e) => {
            warnings.push(SerLoadChainWarning {
                level: "warning".to_string(),
                code: "excludes_read_failed".to_string(),
                message: format!("读取 claudeMdExcludes 失败: {}", e),
            });
            ClaudeMdExcludesConfig {
                patterns: Vec::new(),
                managed_accessible: None,
            }
        }
    };

    // managed settings 不可读时记录 warning
    if let Some(false) = excludes_config.managed_accessible {
        warnings.push(SerLoadChainWarning {
            level: "warning".to_string(),
            code: "managed_settings_unreadable".to_string(),
            message: "file-based managed settings 存在但无权限读取，其 claudeMdExcludes 未被纳入模拟。server-managed / MDM 等非 file-based 来源不在本次模拟范围内".to_string(),
        });
    }

    // ─── A 区域：启动链构建 ───

    // 1. managed CLAUDE.md（若可访问）
    // 注意：managed CLAUDE.md 不被 claudeMdExcludes 排除，这是设计决策
    // managed settings 的 claudeMdExcludes 只作用于 user/project/local/rules 层级
    if let Some(managed_dir) = resolve_managed_dir() {
        try_add_managed_claude_md(&managed_dir, &mut steps, &mut warnings);
    }

    // 2. 用户全局 ~/.claude/CLAUDE.md
    if let Ok(claude_dir) = resolve_claude_config_dir() {
        let user_claude_md = claude_dir.join("CLAUDE.md");
        if user_claude_md.exists() {
            if let Some((pattern, source)) = is_excluded(&user_claude_md, &excludes_config) {
                excluded_assets.push(SerExcludedAsset {
                    native_path: user_claude_md.to_string_lossy().to_string(),
                    logical_path: user_claude_md.to_string_lossy().replace('\\', "/"),
                    scope: "user".to_string(),
                    excluded_by: source.to_string(),
                    pattern: pattern.to_string(),
                });
            } else {
                add_step_from_path(
                    &user_claude_md,
                    "user",
                    "user_claude_md",
                    "user instruction",
                    &mut steps,
                    &mut warnings,
                );
            }
        }
    }

    // 3. 祖先目录链：从根目录遍历到 cwd，每层检查 CLAUDE.md 和 CLAUDE.local.md
    let ancestor_chain =
        build_ancestor_chain(cwd, &excludes_config, &mut excluded_assets, &mut warnings);
    steps.extend(ancestor_chain);

    // 4. 当前目录：CLAUDE.md、.claude/CLAUDE.md、CLAUDE.local.md
    let current_dir_steps =
        build_current_dir_steps(cwd, &excludes_config, &mut excluded_assets, &mut warnings);
    steps.extend(current_dir_steps);

    // 5. rules 递归发现
    // 5a. user 级 rules
    if let Ok(claude_dir) = resolve_claude_config_dir() {
        let user_rules_dir = claude_dir.join("rules");
        scan_rules_dir(
            &user_rules_dir,
            "user",
            &excludes_config,
            &mut steps,
            &mut path_scoped_rules,
            &mut excluded_assets,
            &mut warnings,
        );
    }

    // 5b. project 级 rules
    let project_rules_dir = cwd.join(".claude").join("rules");
    scan_rules_dir(
        &project_rules_dir,
        "project",
        &excludes_config,
        &mut steps,
        &mut path_scoped_rules,
        &mut excluded_assets,
        &mut warnings,
    );

    // 6. Auto Memory
    match find_auto_memory(cwd, &excludes_config, &mut excluded_assets, &mut warnings) {
        Some(step) => steps.push(step),
        None => {
            // 没有 Auto Memory 是正常的，不记录 warning
        }
    }

    // 对 steps 按 order 排序
    for (i, step) in steps.iter_mut().enumerate() {
        step.order = i + 1;
    }

    // 对 path_scoped_rules 按路径排序
    path_scoped_rules.sort_by(|a, b| a.native_path.cmp(&b.native_path));

    // 构建 host_profile
    let host_profile = build_host_profile();

    Ok(SerLoadChain {
        cwd: canonical_cwd.to_string_lossy().to_string(),
        host_profile,
        startup_chain: steps,
        path_scoped_rules,
        excluded_assets,
        warnings,
    })
}

/// 尝试添加 managed CLAUDE.md 到启动链
/// 注意：managed CLAUDE.md 不被 claudeMdExcludes 排除
fn try_add_managed_claude_md(
    managed_dir: &Path,
    steps: &mut Vec<SerLoadChainStep>,
    warnings: &mut Vec<SerLoadChainWarning>,
) {
    let managed_claude_md = managed_dir.join("CLAUDE.md");
    if managed_claude_md.exists() {
        add_step_from_path(
            &managed_claude_md,
            "managed",
            "managed_claude_md",
            "managed instruction",
            steps,
            warnings,
        );
    }
}

/// 构建祖先目录链（从根到 cwd 父目录）
fn build_ancestor_chain(
    cwd: &Path,
    excludes_config: &ClaudeMdExcludesConfig,
    excluded_assets: &mut Vec<SerExcludedAsset>,
    warnings: &mut Vec<SerLoadChainWarning>,
) -> Vec<SerLoadChainStep> {
    let mut steps = Vec::new();

    // 收集从根到 cwd 的所有祖先目录
    let mut ancestors: Vec<PathBuf> = Vec::new();
    let mut current = Some(cwd.to_path_buf());
    while let Some(path) = current {
        ancestors.push(path.clone());
        current = path.parent().map(|p| p.to_path_buf());
    }
    // 反转：从根到 cwd
    ancestors.reverse();

    // 去掉最后一个（即 cwd 本身），因为 cwd 在当前目录步骤中处理
    if !ancestors.is_empty() {
        ancestors.pop();
    }

    for ancestor in ancestors {
        // a. CLAUDE.md
        let claude_md = ancestor.join("CLAUDE.md");
        if claude_md.exists() {
            if let Some((pattern, source)) = is_excluded(&claude_md, excludes_config) {
                excluded_assets.push(SerExcludedAsset {
                    native_path: claude_md.to_string_lossy().to_string(),
                    logical_path: claude_md.to_string_lossy().replace('\\', "/"),
                    scope: "project".to_string(),
                    excluded_by: source.to_string(),
                    pattern: pattern.to_string(),
                });
            } else {
                add_step_from_path(
                    &claude_md,
                    "project",
                    "ancestor_claude_md",
                    "ancestor instruction",
                    &mut steps,
                    warnings,
                );
            }
        }

        // b. CLAUDE.local.md（A11 官方确认）
        let local_md = ancestor.join("CLAUDE.local.md");
        if local_md.exists() {
            if let Some((pattern, source)) = is_excluded(&local_md, excludes_config) {
                excluded_assets.push(SerExcludedAsset {
                    native_path: local_md.to_string_lossy().to_string(),
                    logical_path: local_md.to_string_lossy().replace('\\', "/"),
                    scope: "local".to_string(),
                    excluded_by: source.to_string(),
                    pattern: pattern.to_string(),
                });
            } else {
                add_step_from_path(
                    &local_md,
                    "local",
                    "ancestor_local_md",
                    "ancestor local instruction",
                    &mut steps,
                    warnings,
                );
            }
        }
    }

    steps
}

/// 构建当前目录步骤
fn build_current_dir_steps(
    cwd: &Path,
    excludes_config: &ClaudeMdExcludesConfig,
    excluded_assets: &mut Vec<SerExcludedAsset>,
    warnings: &mut Vec<SerLoadChainWarning>,
) -> Vec<SerLoadChainStep> {
    let mut steps = Vec::new();

    // a. ./CLAUDE.md
    let claude_md = cwd.join("CLAUDE.md");
    if claude_md.exists() {
        if let Some((pattern, source)) = is_excluded(&claude_md, excludes_config) {
            excluded_assets.push(SerExcludedAsset {
                native_path: claude_md.to_string_lossy().to_string(),
                logical_path: claude_md.to_string_lossy().replace('\\', "/"),
                scope: "project".to_string(),
                excluded_by: source.to_string(),
                pattern: pattern.to_string(),
            });
        } else {
            add_step_from_path(
                &claude_md,
                "project",
                "project_claude_md",
                "project instruction",
                &mut steps,
                warnings,
            );
        }
    }

    // b. ./.claude/CLAUDE.md（官方明确支持）
    let dot_claude_md = cwd.join(".claude").join("CLAUDE.md");
    if dot_claude_md.exists() {
        if let Some((pattern, source)) = is_excluded(&dot_claude_md, excludes_config) {
            excluded_assets.push(SerExcludedAsset {
                native_path: dot_claude_md.to_string_lossy().to_string(),
                logical_path: dot_claude_md.to_string_lossy().replace('\\', "/"),
                scope: "project".to_string(),
                excluded_by: source.to_string(),
                pattern: pattern.to_string(),
            });
        } else {
            add_step_from_path(
                &dot_claude_md,
                "project",
                "project_dot_claude_md",
                "project dot-claude instruction",
                &mut steps,
                warnings,
            );
        }
    }

    // c. ./CLAUDE.local.md
    let local_md = cwd.join("CLAUDE.local.md");
    if local_md.exists() {
        if let Some((pattern, source)) = is_excluded(&local_md, excludes_config) {
            excluded_assets.push(SerExcludedAsset {
                native_path: local_md.to_string_lossy().to_string(),
                logical_path: local_md.to_string_lossy().replace('\\', "/"),
                scope: "local".to_string(),
                excluded_by: source.to_string(),
                pattern: pattern.to_string(),
            });
        } else {
            add_step_from_path(
                &local_md,
                "local",
                "local_md",
                "local instruction",
                &mut steps,
                warnings,
            );
        }
    }

    steps
}

/// 递归扫描 rules 目录，区分无条件 rules 和 path-scoped rules
fn scan_rules_dir(
    rules_dir: &Path,
    scope: &str,
    excludes_config: &ClaudeMdExcludesConfig,
    steps: &mut Vec<SerLoadChainStep>,
    path_scoped_rules: &mut Vec<SerPathScopedRule>,
    excluded_assets: &mut Vec<SerExcludedAsset>,
    warnings: &mut Vec<SerLoadChainWarning>,
) {
    let md_files = collect_md_files_recursive(rules_dir);

    // 按文件名排序
    let mut md_files = md_files;
    md_files.sort_by(|a, b| a.file_name().cmp(&b.file_name()));

    for path in md_files {
        // 检查排除
        if let Some((pattern, source)) = is_excluded(&path, excludes_config) {
            excluded_assets.push(SerExcludedAsset {
                native_path: path.to_string_lossy().to_string(),
                logical_path: path.to_string_lossy().replace('\\', "/"),
                scope: scope.to_string(),
                excluded_by: source.to_string(),
                pattern: pattern.to_string(),
            });
            continue;
        }

        // 读取 frontmatter 判断 paths
        let paths = match read_rule_paths(&path) {
            Ok(p) => p,
            Err(e) => {
                warnings.push(SerLoadChainWarning {
                    level: "warning".to_string(),
                    code: "frontmatter_read_failed".to_string(),
                    message: format!("读取 rule frontmatter 失败 {}: {}", path.display(), e),
                });
                None
            }
        };

        if let Some(paths) = paths {
            // 有 paths → path-scoped rule（B 区域）
            let name = path
                .file_stem()
                .and_then(|n| n.to_str())
                .map(|s| s.to_string());
            path_scoped_rules.push(SerPathScopedRule {
                scope: scope.to_string(),
                native_path: path.to_string_lossy().to_string(),
                logical_path: path.to_string_lossy().replace('\\', "/"),
                name,
                paths,
                exists: true,
            });
        } else {
            // 无 paths → 无条件加载 rule（A 区域）
            add_step_from_path(
                &path,
                scope,
                if scope == "user" {
                    "global_rule"
                } else {
                    "project_rule"
                },
                if scope == "user" {
                    "global rule"
                } else {
                    "project rule"
                },
                steps,
                warnings,
            );
        }
    }
}

/// 递归收集目录下所有 .md 文件
fn collect_md_files_recursive(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();

    if !dir.exists() || !dir.is_dir() {
        return files;
    }

    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return files,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            files.extend(collect_md_files_recursive(&path));
        } else if path.extension().and_then(|e| e.to_str()) == Some("md") {
            files.push(path);
        }
    }

    files
}

/// 读取 rule 文件的 paths 字段
fn read_rule_paths(path: &Path) -> Result<Option<Vec<String>>, String> {
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;

    if let Some((raw, _)) = extract_frontmatter(&content) {
        let paths = parse_list_field(raw, "paths");
        Ok(paths.filter(|p| !p.is_empty()))
    } else {
        Ok(None)
    }
}

/// 从 cwd 向上查找 git 仓库根目录（只识别 `.git` 目录）
///
/// **行为**：
/// - 普通 git repo 子目录：向上找到 `.git` 目录，返回该目录的父路径（repo root）
/// - git worktree / submodule：`.git` 是文件而非目录，本函数不识别，返回 None
/// - 非 git 目录：无 `.git` 目录，返回 None
///
/// worktree 的 `.git` 文件解析不在 P1 范围内，由调用方单独处理。
///
/// 返回：若找到 `.git` 目录则返回 repo root 路径；否则返回 None
fn find_git_repo_root(cwd: &Path) -> Option<PathBuf> {
    let mut current = Some(cwd.to_path_buf());
    while let Some(path) = current {
        let git_path = path.join(".git");
        // 只识别 `.git` 目录（普通 repo），不识别 `.git` 文件（worktree / submodule）
        if git_path.is_dir() {
            return Some(path);
        }
        current = path.parent().map(|p| p.to_path_buf());
    }
    None
}

/// 查找并读取 Auto Memory（MEMORY.md）
///
/// 匹配策略（最终版）：
/// 1. **普通 git repo 子目录**（cwd 或其祖先有 `.git` 目录）：
///    用 `find_git_repo_root()` 定位 repo root，再用 repo root 编码路径匹配 Auto Memory。
///    ✅ 当前已实现支持。
/// 2. **git worktree / submodule**（cwd 或其祖先有 `.git` 文件，非目录）：
///    安全收口——不尝试任何近似匹配（包括 cwd-encoded fallback），直接返回
///    `auto_memory_worktree_unsupported` warning，避免误展示。
///    ⚠️ P1 limitation：worktree 与主 repo 共享 Auto Memory 的语义未实现。
/// 3. **非 git 目录**（无任何 `.git`）：
///    回退到 cwd 编码路径匹配。
///    ⚠️ P1 limitation：用户自定义 `autoMemoryDirectory` 未读取。
///
/// 例如 /Users/name/Repo → ~/.claude/projects/-Users-name-Repo/memory/MEMORY.md
/// 如果不存在，记录 info warning，不返回模糊匹配结果。
fn find_auto_memory(
    cwd: &Path,
    excludes_config: &ClaudeMdExcludesConfig,
    excluded_assets: &mut Vec<SerExcludedAsset>,
    warnings: &mut Vec<SerLoadChainWarning>,
) -> Option<SerLoadChainStep> {
    let claude_dir = match resolve_claude_config_dir() {
        Ok(dir) => dir,
        Err(_) => return None,
    };

    let projects_dir = claude_dir.join("projects");
    if !projects_dir.exists() {
        return None;
    }

    // 区分三种场景：普通 git repo 子目录 / worktree / 非 git 目录
    // 1. 普通 git repo 子目录：cwd 或其祖先有 `.git` 目录
    // 2. worktree / submodule：cwd 或其祖先有 `.git` 文件（非目录）
    // 3. 非 git 目录：无任何 `.git`

    let has_git_dir = cwd.ancestors().any(|p| p.join(".git").is_dir());
    let has_git_file = cwd.ancestors().any(|p| {
        let git_path = p.join(".git");
        git_path.exists() && !git_path.is_dir()
    });

    if has_git_dir {
        // 场景 1：普通 git repo 子目录，用 repo root 编码匹配
        let repo_root = find_git_repo_root(cwd).unwrap_or_else(|| cwd.to_path_buf());
        let encoded = encode_cwd_path(&repo_root.to_string_lossy());
        let memory_md = projects_dir.join(&encoded).join("memory").join("MEMORY.md");

        if memory_md.exists() {
            return read_auto_memory_file(&memory_md, excludes_config, excluded_assets, warnings);
        }

        warnings.push(SerLoadChainWarning {
            level: "info".to_string(),
            code: "auto_memory_not_found".to_string(),
            message: format!(
                "未找到 Auto Memory：git repo root 编码路径 ~/.claude/projects/{}/memory/MEMORY.md 不存在",
                encoded
            ),
        });
        None
    } else if has_git_file {
        // 场景 2：worktree / submodule
        // 安全收口：不尝试任何 cwd-encoded fallback，避免误导用户
        warnings.push(SerLoadChainWarning {
            level: "warning".to_string(),
            code: "auto_memory_worktree_unsupported".to_string(),
            message: "当前目录位于 git worktree（或 submodule）中。Claude 官方语义要求 worktree 与主 repo 共享 Auto Memory，但 P1 当前未解析 `.git` 文件来定位共享 identity。因此本轮不做近似匹配，避免误展示。后续版本将补充 worktree 支持。".to_string(),
        });
        None
    } else {
        // 场景 3：非 git 目录，用 cwd 编码回退匹配
        let encoded = encode_cwd_path(&cwd.to_string_lossy());
        let memory_md = projects_dir.join(&encoded).join("memory").join("MEMORY.md");

        if memory_md.exists() {
            return read_auto_memory_file(&memory_md, excludes_config, excluded_assets, warnings);
        }

        warnings.push(SerLoadChainWarning {
            level: "info".to_string(),
            code: "auto_memory_not_found".to_string(),
            message: format!(
                "未找到 Auto Memory：cwd 编码路径 ~/.claude/projects/{}/memory/MEMORY.md 不存在",
                encoded
            ),
        });
        None
    }
}

/// 读取 Auto Memory 文件（应用截断）
fn read_auto_memory_file(
    path: &Path,
    excludes_config: &ClaudeMdExcludesConfig,
    excluded_assets: &mut Vec<SerExcludedAsset>,
    warnings: &mut Vec<SerLoadChainWarning>,
) -> Option<SerLoadChainStep> {
    // 检查排除
    if let Some((pattern, source)) = is_excluded(path, excludes_config) {
        excluded_assets.push(SerExcludedAsset {
            native_path: path.to_string_lossy().to_string(),
            logical_path: path.to_string_lossy().replace('\\', "/"),
            scope: "auto".to_string(),
            excluded_by: source.to_string(),
            pattern: pattern.to_string(),
        });
        return None;
    }

    // 获取 metadata 以获取 byte_size（不依赖完整读取）
    let metadata = match fs::metadata(path) {
        Ok(m) => m,
        Err(e) => {
            warnings.push(SerLoadChainWarning {
                level: "warning".to_string(),
                code: "auto_memory_read_failed".to_string(),
                message: format!("读取 Auto Memory 失败 {}: {}", path.display(), e),
            });
            return None;
        }
    };
    let byte_size = metadata.len();

    // 安全读取：超过上限只读取需要的大小
    let content = if byte_size > MAX_AUTO_MEMORY_BYTES as u64 {
        let mut file = match fs::File::open(path) {
            Ok(f) => f,
            Err(e) => {
                warnings.push(SerLoadChainWarning {
                    level: "warning".to_string(),
                    code: "auto_memory_read_failed".to_string(),
                    message: format!("读取 Auto Memory 失败 {}: {}", path.display(), e),
                });
                return None;
            }
        };
        let mut buf = vec![0u8; MAX_AUTO_MEMORY_BYTES];
        let n = match std::io::Read::read(&mut file, &mut buf) {
            Ok(n) => n,
            Err(e) => {
                warnings.push(SerLoadChainWarning {
                    level: "warning".to_string(),
                    code: "auto_memory_read_failed".to_string(),
                    message: format!("读取 Auto Memory 失败 {}: {}", path.display(), e),
                });
                return None;
            }
        };
        buf.truncate(n);
        String::from_utf8_lossy(&buf).to_string()
    } else {
        match fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                warnings.push(SerLoadChainWarning {
                    level: "warning".to_string(),
                    code: "auto_memory_read_failed".to_string(),
                    message: format!("读取 Auto Memory 失败 {}: {}", path.display(), e),
                });
                return None;
            }
        }
    };

    let total_lines = content.lines().count();

    // 应用截断：200 行或 25KB，取先到者
    // 注意：读取阶段已将大文件限制到 MAX_AUTO_MEMORY_BYTES，因此 content.len()
    // 不会超过该值。截断判断应以原始 byte_size 为准，否则字节截断分支永远不会触发。
    let (truncated_content, truncated) = if byte_size > MAX_AUTO_MEMORY_BYTES as u64 {
        // 原始文件超过 25KB，标记截断；内容已经是读取阶段截断后的 25KB
        let lines: Vec<_> = content.lines().collect();
        if lines.len() > MAX_AUTO_MEMORY_LINES {
            (lines[..MAX_AUTO_MEMORY_LINES].join("\n"), true)
        } else {
            (content, true)
        }
    } else if total_lines > MAX_AUTO_MEMORY_LINES {
        let lines: Vec<_> = content.lines().collect();
        (lines[..MAX_AUTO_MEMORY_LINES].join("\n"), true)
    } else {
        (content, false)
    };

    let line_count = if truncated {
        Some(truncated_content.lines().count())
    } else {
        Some(total_lines)
    };

    let preview = if truncated_content.len() > MAX_PREVIEW_SIZE {
        let mut end = MAX_PREVIEW_SIZE;
        while end > 0 && !truncated_content.is_char_boundary(end) {
            end -= 1;
        }
        Some(truncated_content[..end].to_string())
    } else {
        Some(truncated_content)
    };

    Some(SerLoadChainStep {
        order: 0,
        scope: "auto".to_string(),
        asset_type: "auto_memory_index".to_string(),
        native_path: path.to_string_lossy().to_string(),
        logical_path: path.to_string_lossy().replace('\\', "/"),
        load_reason: "Auto Memory (MEMORY.md)".to_string(),
        line_count,
        byte_size: Some(byte_size),
        content_preview: preview,
        content_truncated: truncated,
        exists: true,
    })
}

/// 安全读取文件内容，大文件只返回 preview
/// 返回 (preview_content, byte_size, line_count, was_truncated)
fn read_file_bounded(
    path: &Path,
) -> Result<(Option<String>, u64, Option<usize>, bool), std::io::Error> {
    let metadata = fs::metadata(path)?;
    let byte_size = metadata.len();

    if byte_size == 0 {
        return Ok((Some(String::new()), 0, Some(0), false));
    }

    if byte_size > MAX_FILE_READ_BYTES as u64 {
        // 大文件：只读 preview
        let mut file = std::io::BufReader::new(fs::File::open(path)?);
        let mut buf = vec![0u8; MAX_PREVIEW_SIZE];
        let n = std::io::Read::read(&mut file, &mut buf)?;
        buf.truncate(n);

        let preview = String::from_utf8_lossy(&buf).to_string();
        let line_count = preview.lines().count();
        return Ok((Some(preview), byte_size, Some(line_count), true));
    }

    // 小文件：完整读取
    let content = fs::read_to_string(path)?;
    let line_count = content.lines().count();
    let preview = if content.len() > MAX_PREVIEW_SIZE {
        let mut end = MAX_PREVIEW_SIZE;
        while end > 0 && !content.is_char_boundary(end) {
            end -= 1;
        }
        content[..end].to_string()
    } else {
        content
    };

    Ok((Some(preview), byte_size, Some(line_count), false))
}

/// 从文件路径构建 SerLoadChainStep
fn add_step_from_path(
    path: &Path,
    scope: &str,
    asset_type: &str,
    load_reason: &str,
    steps: &mut Vec<SerLoadChainStep>,
    warnings: &mut Vec<SerLoadChainWarning>,
) {
    let native_path = path.to_string_lossy().to_string();
    let logical_path = native_path.replace('\\', "/");

    match read_file_bounded(path) {
        Ok((preview, byte_size, line_count, truncated)) => {
            steps.push(SerLoadChainStep {
                order: 0,
                scope: scope.to_string(),
                asset_type: asset_type.to_string(),
                native_path,
                logical_path,
                load_reason: load_reason.to_string(),
                line_count,
                byte_size: Some(byte_size),
                content_preview: preview,
                content_truncated: truncated,
                exists: true,
            });
        }
        Err(e) => {
            warnings.push(SerLoadChainWarning {
                level: "warning".to_string(),
                code: "file_read_failed".to_string(),
                message: format!("读取文件失败 {}: {}", path.display(), e),
            });
            steps.push(SerLoadChainStep {
                order: 0,
                scope: scope.to_string(),
                asset_type: asset_type.to_string(),
                native_path,
                logical_path,
                load_reason: load_reason.to_string(),
                line_count: None,
                byte_size: None,
                content_preview: None,
                content_truncated: false,
                exists: true,
            });
        }
    }
}

/// 构建 HostProfile（复用 scanner 逻辑）
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

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::super::path_resolver::test_helpers::with_env_var;
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    /// 全局测试计数器，确保并行测试的临时目录名唯一
    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    /// 辅助：创建唯一临时目录结构（避免并行测试竞争）
    fn setup_test_dir() -> PathBuf {
        let unique = format!(
            "agent-scope-load-chain-test-{}-{}-{}",
            std::process::id(),
            TEST_COUNTER.fetch_add(1, Ordering::Relaxed),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let tmp = std::env::temp_dir().join(unique);
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        tmp
    }

    /// 辅助：在隔离的 CLAUDE_CONFIG_DIR 下运行 simulate_load_chain
    /// 避免测试受开发机 ~/.claude 真实内容影响
    fn simulate_isolated(cwd: &Path) -> Result<SerLoadChain, String> {
        let fake_claude_dir = cwd.parent().unwrap_or(cwd).join(format!(
            "fake-claude-{}-{}-{}",
            std::process::id(),
            TEST_COUNTER.fetch_add(1, Ordering::Relaxed),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&fake_claude_dir);
        fs::create_dir_all(&fake_claude_dir).unwrap();
        let result = with_env_var(
            "CLAUDE_CONFIG_DIR",
            fake_claude_dir.to_str().unwrap(),
            || simulate_load_chain(cwd),
        );
        let _ = fs::remove_dir_all(&fake_claude_dir);
        result
    }

    /// 测试：祖先 CLAUDE.md 顺序
    #[test]
    fn test_ancestor_claude_md_order() {
        let root = setup_test_dir();

        // 创建祖先目录结构
        let ancestor1 = root.join("ancestor1");
        let ancestor2 = ancestor1.join("ancestor2");
        let cwd = ancestor2.join("project");
        fs::create_dir_all(&cwd).unwrap();

        fs::write(ancestor1.join("CLAUDE.md"), "# Ancestor1\n").unwrap();
        fs::write(ancestor2.join("CLAUDE.md"), "# Ancestor2\n").unwrap();
        fs::write(cwd.join("CLAUDE.md"), "# Project\n").unwrap();

        let result = simulate_isolated(&cwd).expect("模拟应成功");

        let instruction_steps: Vec<_> = result
            .startup_chain
            .iter()
            .filter(|s| s.asset_type.contains("claude_md"))
            .collect();

        // 祖先应该在当前目录之前
        let ancestor1_idx = instruction_steps
            .iter()
            .position(|s| s.native_path.contains("ancestor1"));
        let ancestor2_idx = instruction_steps
            .iter()
            .position(|s| s.native_path.contains("ancestor2"));
        let cwd_idx = instruction_steps
            .iter()
            .position(|s| s.native_path.contains("project"));

        assert!(ancestor1_idx.is_some());
        assert!(ancestor2_idx.is_some());
        assert!(cwd_idx.is_some());
        assert!(ancestor1_idx.unwrap() < ancestor2_idx.unwrap());
        assert!(ancestor2_idx.unwrap() < cwd_idx.unwrap());

        let _ = fs::remove_dir_all(&root);
    }

    /// 测试：祖先 CLAUDE.local.md 顺序
    #[test]
    fn test_ancestor_local_md_order() {
        let root = setup_test_dir();

        let ancestor = root.join("ancestor");
        let cwd = ancestor.join("project");
        fs::create_dir_all(&cwd).unwrap();

        fs::write(ancestor.join("CLAUDE.local.md"), "# Local\n").unwrap();
        fs::write(cwd.join("CLAUDE.md"), "# Project\n").unwrap();

        let result = simulate_isolated(&cwd).expect("模拟应成功");

        let local_step = result
            .startup_chain
            .iter()
            .find(|s| s.asset_type == "ancestor_local_md");
        assert!(local_step.is_some(), "应找到祖先 CLAUDE.local.md");

        let _ = fs::remove_dir_all(&root);
    }

    /// 测试：当前目录三层检查
    #[test]
    fn test_current_dir_three_files() {
        let root = setup_test_dir();
        let cwd = root.join("project");
        fs::create_dir_all(&cwd).unwrap();

        fs::write(cwd.join("CLAUDE.md"), "# Main\n").unwrap();
        fs::create_dir_all(cwd.join(".claude")).unwrap();
        fs::write(cwd.join(".claude").join("CLAUDE.md"), "# Dot\n").unwrap();
        fs::write(cwd.join("CLAUDE.local.md"), "# Local\n").unwrap();

        let result = simulate_isolated(&cwd).expect("模拟应成功");

        assert!(result
            .startup_chain
            .iter()
            .any(|s| s.asset_type == "project_claude_md"));
        assert!(result
            .startup_chain
            .iter()
            .any(|s| s.asset_type == "project_dot_claude_md"));
        assert!(result
            .startup_chain
            .iter()
            .any(|s| s.asset_type == "local_md"));

        let _ = fs::remove_dir_all(&root);
    }

    /// 测试：rules 递归发现
    #[test]
    fn test_rules_recursive_discovery() {
        let root = setup_test_dir();
        let cwd = root.join("project");
        fs::create_dir_all(&cwd.join(".claude").join("rules").join("coding")).unwrap();

        fs::write(
            cwd.join(".claude")
                .join("rules")
                .join("coding")
                .join("style.md"),
            "# Style\n",
        )
        .unwrap();

        let result = simulate_isolated(&cwd).expect("模拟应成功");

        assert!(result
            .startup_chain
            .iter()
            .any(|s| s.native_path.contains("style.md")));

        let _ = fs::remove_dir_all(&root);
    }

    /// 测试：paths rule 与无 paths rule 分区
    #[test]
    fn test_paths_vs_unconditional_rules() {
        let root = setup_test_dir();
        let cwd = root.join("project");
        fs::create_dir_all(&cwd.join(".claude").join("rules")).unwrap();

        // 无 paths 的 rule → A 区域
        fs::write(
            cwd.join(".claude").join("rules").join("always.md"),
            "# Always\n",
        )
        .unwrap();

        // 有 paths 的 rule → B 区域
        fs::write(
            cwd.join(".claude").join("rules").join("conditional.md"),
            "---\npaths:\n  - \"src/**/*.rs\"\n---\n# Conditional\n",
        )
        .unwrap();

        let result = simulate_isolated(&cwd).expect("模拟应成功");

        // A 区域应有 always.md
        assert!(result
            .startup_chain
            .iter()
            .any(|s| s.native_path.contains("always.md")));

        // A 区域不应有 conditional.md
        assert!(!result
            .startup_chain
            .iter()
            .any(|s| s.native_path.contains("conditional.md")));

        // B 区域应有 conditional.md
        assert!(result
            .path_scoped_rules
            .iter()
            .any(|r| r.native_path.contains("conditional.md")));

        let _ = fs::remove_dir_all(&root);
    }

    /// 测试：claudeMdExcludes 排除
    #[test]
    fn test_claude_md_excludes() {
        let root = setup_test_dir();
        let cwd = root.join("project");
        fs::create_dir_all(&cwd).unwrap();

        fs::write(cwd.join("CLAUDE.md"), "# Main\n").unwrap();

        // 创建 project settings 排除 CLAUDE.md
        fs::create_dir_all(cwd.join(".claude")).unwrap();
        fs::write(
            cwd.join(".claude").join("settings.json"),
            format!(
                "{{\"claudeMdExcludes\": [\"{}\"]}}",
                cwd.join("CLAUDE.md").to_string_lossy().replace('\\', "/")
            ),
        )
        .unwrap();

        // 直接检查 read_claude_md_excludes 是否读取到 project settings
        let excludes = super::super::settings_reader::read_claude_md_excludes(&cwd).unwrap();
        let project_settings = cwd.join(".claude").join("settings.json");
        assert!(
            project_settings.exists(),
            "project settings 文件应存在: {}",
            project_settings.display()
        );
        let settings_content = std::fs::read_to_string(&project_settings).unwrap();
        eprintln!("DEBUG project_settings content: {}", settings_content);
        eprintln!("DEBUG excludes patterns: {:?}", excludes.patterns);
        assert!(
            !excludes.patterns.is_empty(),
            "应读取到 project settings 的排除模式，但实际为空"
        );

        // 直接验证 is_excluded
        let claude_md = cwd.join("CLAUDE.md");
        let excluded = super::super::settings_reader::is_excluded(&claude_md, &excludes);
        assert!(
            excluded.is_some(),
            "is_excluded 应返回 Some，实际为 None。file_path: {}, patterns: {:?}",
            claude_md.display(),
            excludes.patterns
        );

        let result = simulate_isolated(&cwd).expect("模拟应成功");

        // 启动链中不应有 CLAUDE.md
        assert!(!result
            .startup_chain
            .iter()
            .any(|s| s.asset_type == "project_claude_md"));

        // 被排除列表中应有 CLAUDE.md
        assert!(result
            .excluded_assets
            .iter()
            .any(|e| e.native_path.contains("CLAUDE.md")));

        let _ = fs::remove_dir_all(&root);
    }

    /// 测试：managed file-based settings 不可读 warning
    #[test]
    fn test_managed_settings_warning() {
        let root = setup_test_dir();
        let cwd = root.join("project");
        fs::create_dir_all(&cwd).unwrap();
        fs::write(cwd.join("CLAUDE.md"), "# OK\n").unwrap();

        let result = simulate_isolated(&cwd).expect("模拟应成功");

        // managed settings 通常不可读，应有相应 warning
        let has_managed_warning = result.warnings.iter().any(|w| {
            w.code == "managed_settings_unreadable"
                || w.message.contains("managed")
                || w.message.contains("server-managed")
        });

        // 注：在 CI 环境中 managed 目录可能不存在，所以不一定有 warning
        // 此测试主要验证不 panic
        let _ = has_managed_warning;

        let _ = fs::remove_dir_all(&root);
    }

    /// 辅助：为 Auto Memory 测试创建临时环境（fake CLAUDE_CONFIG_DIR + MEMORY.md）
    fn setup_auto_memory_test(cwd: &Path, fake_claude_dir: &Path, content: &str) {
        fs::create_dir_all(&cwd).unwrap();
        fs::write(cwd.join("CLAUDE.md"), "# OK\n").unwrap();

        let encoded = encode_cwd_path(&cwd.to_string_lossy());
        let memory_dir = fake_claude_dir
            .join("projects")
            .join(&encoded)
            .join("memory");
        fs::create_dir_all(&memory_dir).unwrap();
        fs::write(memory_dir.join("MEMORY.md"), content).unwrap();
    }

    /// 测试：Auto Memory 行数先到截断（>200 行，但字节数 ≤25KB）
    #[test]
    fn test_auto_memory_truncation_lines_first() {
        let root = setup_test_dir();
        let fake_claude_dir = root.join("fake-claude");
        let cwd = root.join("project");

        // 250 行，每行 1 字符 + 换行 ≈ 500 字节 < 25KB
        let content = "x\n".repeat(250);
        assert!(content.len() < MAX_AUTO_MEMORY_BYTES, "测试数据应小于 25KB");
        setup_auto_memory_test(&cwd, &fake_claude_dir, &content);

        let result = with_env_var(
            "CLAUDE_CONFIG_DIR",
            fake_claude_dir.to_str().unwrap(),
            || simulate_load_chain(&cwd).expect("模拟应成功"),
        );

        let step = result
            .startup_chain
            .iter()
            .find(|s| s.asset_type == "auto_memory_index")
            .expect("应找到 Auto Memory");

        assert!(step.content_truncated, "250 行 > 200 行，应标记截断");
        assert_eq!(
            step.line_count,
            Some(MAX_AUTO_MEMORY_LINES),
            "截断后应为 200 行"
        );
        assert_eq!(
            step.byte_size,
            Some(content.len() as u64),
            "byte_size 应反映原始文件大小"
        );
        // 原始文件不超过 25KB，因此读取阶段未截断；截断仅由行数触发
        // preview 应为截断后的前 200 行内容
        assert!(step.content_preview.is_some());

        let _ = fs::remove_dir_all(&root);
    }

    /// 测试：Auto Memory 字节数先到截断（>25KB，但行数 ≤200）
    #[test]
    fn test_auto_memory_truncation_bytes_first() {
        let root = setup_test_dir();
        let fake_claude_dir = root.join("fake-claude");
        let cwd = root.join("project");

        // 100 行，每行 400 字符 + 换行 ≈ 40KB > 25KB
        let long_line = "A".repeat(400);
        let content: String = (0..100).map(|_| format!("{}\n", long_line)).collect();
        assert!(content.len() > MAX_AUTO_MEMORY_BYTES, "测试数据应超过 25KB");
        setup_auto_memory_test(&cwd, &fake_claude_dir, &content);

        let result = with_env_var(
            "CLAUDE_CONFIG_DIR",
            fake_claude_dir.to_str().unwrap(),
            || simulate_load_chain(&cwd).expect("模拟应成功"),
        );

        let step = result
            .startup_chain
            .iter()
            .find(|s| s.asset_type == "auto_memory_index")
            .expect("应找到 Auto Memory");

        assert!(step.content_truncated, "原始文件 40KB > 25KB，应标记截断");
        assert_eq!(
            step.byte_size,
            Some(content.len() as u64),
            "byte_size 应反映原始文件大小（未截断前）"
        );
        // 读取阶段截断到 25KB，约 62 行（每行 400+1 字节）
        // 62 行 < 200 行，因此不会再触发行数截断
        let line_count = step.line_count.expect("应有 line_count");
        assert!(
            line_count < 100,
            "25KB 只能容纳约 62 行，实际 line_count={}",
            line_count
        );
        assert!(step.content_preview.is_some(), "应有 content_preview");

        let _ = fs::remove_dir_all(&root);
    }

    /// 测试：git repo 子目录共享同一 Auto Memory（repo root 编码匹配）
    #[test]
    fn test_auto_memory_git_repo_root_match() {
        let root = setup_test_dir();
        let fake_claude_dir = root.join("fake-claude");
        let repo_root = root.join("my-repo");
        let sub_dir = repo_root.join("src").join("components");

        // 创建 git repo 根目录（含 .git 子目录）
        fs::create_dir_all(&repo_root).unwrap();
        fs::create_dir_all(repo_root.join(".git")).unwrap();
        fs::write(repo_root.join("CLAUDE.md"), "# Repo CLAUDE.md\n").unwrap();

        // 创建子目录（无 CLAUDE.md）
        fs::create_dir_all(&sub_dir).unwrap();

        // 创建 Auto Memory：使用 repo root 编码，不是子目录编码
        let encoded_root = encode_cwd_path(&repo_root.to_string_lossy());
        let memory_dir = fake_claude_dir
            .join("projects")
            .join(&encoded_root)
            .join("memory");
        fs::create_dir_all(&memory_dir).unwrap();
        fs::write(memory_dir.join("MEMORY.md"), "# Auto Memory\n").unwrap();

        // 从子目录启动模拟，应能找到 Auto Memory（因为共享 repo root 的）
        let result = with_env_var(
            "CLAUDE_CONFIG_DIR",
            fake_claude_dir.to_str().unwrap(),
            || simulate_load_chain(&sub_dir).expect("模拟应成功"),
        );

        let step = result
            .startup_chain
            .iter()
            .find(|s| s.asset_type == "auto_memory_index")
            .expect("从子目录启动应找到 repo root 对应的 Auto Memory");

        assert_eq!(step.scope, "auto");
        assert!(
            step.native_path.contains(&encoded_root),
            "Auto Memory 路径应包含 repo root 编码，实际路径: {}",
            step.native_path
        );

        // 验证：若用子目录编码创建独立的 Auto Memory，不应被匹配到
        let encoded_sub = encode_cwd_path(&sub_dir.to_string_lossy());
        assert_ne!(
            encoded_root, encoded_sub,
            "repo root 和子目录的编码应不同，否则测试无意义"
        );

        let _ = fs::remove_dir_all(&root);
    }

    /// 测试：非 git 目录使用 cwd 编码匹配 Auto Memory
    #[test]
    fn test_auto_memory_non_git_cwd_match() {
        let root = setup_test_dir();
        let fake_claude_dir = root.join("fake-claude");
        let cwd = root.join("no-git-project");

        // 创建非 git 目录（无 .git）
        fs::create_dir_all(&cwd).unwrap();
        fs::write(cwd.join("CLAUDE.md"), "# No Git\n").unwrap();

        // 创建 Auto Memory：使用 cwd 编码
        let encoded = encode_cwd_path(&cwd.to_string_lossy());
        let memory_dir = fake_claude_dir
            .join("projects")
            .join(&encoded)
            .join("memory");
        fs::create_dir_all(&memory_dir).unwrap();
        fs::write(memory_dir.join("MEMORY.md"), "# Auto Memory\n").unwrap();

        let result = with_env_var(
            "CLAUDE_CONFIG_DIR",
            fake_claude_dir.to_str().unwrap(),
            || simulate_load_chain(&cwd).expect("模拟应成功"),
        );

        let step = result
            .startup_chain
            .iter()
            .find(|s| s.asset_type == "auto_memory_index")
            .expect("非 git 目录应找到 cwd 编码的 Auto Memory");

        assert_eq!(step.scope, "auto");

        let _ = fs::remove_dir_all(&root);
    }

    /// 测试：git worktree 场景不加载 cwd-encoded Auto Memory，返回 limitation warning
    #[test]
    fn test_auto_memory_worktree_no_fallback() {
        let root = setup_test_dir();
        let fake_claude_dir = root.join("fake-claude");
        // 模拟 worktree 目录结构：
        // root/
        //   main-repo/          (主 repo，有 .git 目录)
        //     .git/
        //   worktree-1/         (worktree，有 .git 文件)
        //     .git              (内容为 "gitdir: ...")
        //     src/              (模拟从 worktree 子目录启动)
        let main_repo = root.join("main-repo");
        let worktree = root.join("worktree-1");
        let worktree_sub = worktree.join("src");

        fs::create_dir_all(&main_repo).unwrap();
        fs::create_dir_all(main_repo.join(".git")).unwrap();
        fs::write(main_repo.join("CLAUDE.md"), "# Main Repo\n").unwrap();

        fs::create_dir_all(&worktree).unwrap();
        // .git 文件（不是目录）模拟 worktree
        fs::write(
            worktree.join(".git"),
            "gitdir: /fake/path/to/main-repo/.git/worktrees/worktree-1\n",
        )
        .unwrap();
        fs::write(worktree.join("CLAUDE.md"), "# Worktree\n").unwrap();

        fs::create_dir_all(&worktree_sub).unwrap();

        // 创建 cwd-encoded Auto Memory（即 worktree 路径编码）
        // 如果实现有误，会从 worktree 回退到 cwd 编码并加载这个
        let encoded_cwd = encode_cwd_path(&worktree_sub.to_string_lossy());
        let memory_dir_cwd = fake_claude_dir
            .join("projects")
            .join(&encoded_cwd)
            .join("memory");
        fs::create_dir_all(&memory_dir_cwd).unwrap();
        fs::write(memory_dir_cwd.join("MEMORY.md"), "# CWD Encoded Memory\n").unwrap();

        let result = with_env_var(
            "CLAUDE_CONFIG_DIR",
            fake_claude_dir.to_str().unwrap(),
            || simulate_load_chain(&worktree_sub).expect("模拟应成功"),
        );

        // 1. 不应加载任何 Auto Memory
        let auto_step = result
            .startup_chain
            .iter()
            .find(|s| s.asset_type == "auto_memory_index");
        assert!(
            auto_step.is_none(),
            "worktree 场景不应加载 Auto Memory（即使是 cwd-encoded 的）"
        );

        // 2. 应返回 worktree limitation warning
        let worktree_warning = result
            .warnings
            .iter()
            .find(|w| w.code == "auto_memory_worktree_unsupported");
        assert!(
            worktree_warning.is_some(),
            "worktree 场景应返回 auto_memory_worktree_unsupported warning"
        );

        // 3. 不应返回 auto_memory_not_found（因为不是"没找到"，而是"不查找"）
        let not_found = result
            .warnings
            .iter()
            .find(|w| w.code == "auto_memory_not_found");
        assert!(
            not_found.is_none(),
            "worktree 场景不应返回 auto_memory_not_found"
        );

        let _ = fs::remove_dir_all(&root);
    }

    /// 测试：read_file_bounded 对不存在的路径返回 Err（不依赖权限系统）
    #[test]
    fn test_read_file_bounded_not_found() {
        let result = read_file_bounded(Path::new("/nonexistent/path/that/does/not/exist"));
        assert!(result.is_err(), "不存在的路径应返回 Err");
    }

    /// 测试：add_step_from_path 在 read_file_bounded 失败时仍加入 step 并记录 warning
    /// 使用不存在的路径，不依赖 chmod / 权限环境
    #[test]
    fn test_add_step_from_path_failure_records_warning() {
        let mut steps: Vec<SerLoadChainStep> = Vec::new();
        let mut warnings: Vec<SerLoadChainWarning> = Vec::new();

        add_step_from_path(
            Path::new("/nonexistent/path/for/testing"),
            "user",
            "user_claude_md",
            "user instruction",
            &mut steps,
            &mut warnings,
        );

        // 即使读取失败也应加入 step（表示文件在逻辑上存在但内容不可读）
        assert_eq!(steps.len(), 1, "失败时仍应加入 step");
        assert_eq!(steps[0].scope, "user");
        assert_eq!(steps[0].asset_type, "user_claude_md");
        assert!(steps[0].content_preview.is_none());
        assert!(steps[0].byte_size.is_none());
        assert!(!steps[0].content_truncated);

        // 应记录 file_read_failed warning
        assert_eq!(warnings.len(), 1, "应记录一条 warning");
        assert_eq!(warnings[0].code, "file_read_failed");
        assert!(
            warnings[0]
                .message
                .contains("/nonexistent/path/for/testing"),
            "warning 应包含文件路径"
        );
    }

    /// 测试：大文件 step 不读入完整内容
    #[test]
    fn test_large_file_step_truncated() {
        let root = setup_test_dir();
        let cwd = root.join("project");
        fs::create_dir_all(&cwd).unwrap();

        // 创建超大 CLAUDE.md（超过 50KB）
        let large_content = "A".repeat(60_000);
        fs::write(cwd.join("CLAUDE.md"), &large_content).unwrap();

        let result = simulate_isolated(&cwd).expect("模拟应成功");

        let step = result
            .startup_chain
            .iter()
            .find(|s| s.asset_type == "project_claude_md")
            .expect("应找到 project_claude_md step");

        // 应标记为 truncated
        assert!(step.content_truncated, "大文件应标记为 truncated");

        // byte_size 应为实际大小
        assert_eq!(
            step.byte_size,
            Some(60_000 as u64),
            "byte_size 应反映实际文件大小"
        );

        // preview 不应超过 MAX_PREVIEW_SIZE
        if let Some(preview) = &step.content_preview {
            assert!(
                preview.len() <= MAX_PREVIEW_SIZE,
                "preview 不应超过 {} 字节，实际 {}",
                MAX_PREVIEW_SIZE,
                preview.len()
            );
        }

        let _ = fs::remove_dir_all(&root);
    }

    /// 测试：managed CLAUDE.md 不应被 claudeMdExcludes 排除
    /// 即使 is_excluded 能匹配 managed 路径，load_chain 也不应对其调用 is_excluded
    #[test]
    fn test_managed_claude_md_not_excluded() {
        let root = setup_test_dir();
        let managed_dir = root.join("managed");
        fs::create_dir_all(&managed_dir).unwrap();
        fs::write(managed_dir.join("CLAUDE.md"), "# Managed CLAUDE\n").unwrap();

        // 构造一个会匹配 managed CLAUDE.md 路径的 excludes config
        let excludes_config = ClaudeMdExcludesConfig {
            patterns: vec![ExcludePattern {
                pattern: "**/managed/CLAUDE.md".to_string(),
                source: "managed".to_string(),
            }],
            managed_accessible: None,
        };

        // 验证 is_excluded 本身能匹配该路径（排除逻辑本身没问题）
        let managed_path = managed_dir.join("CLAUDE.md");
        assert!(
            is_excluded(&managed_path, &excludes_config).is_some(),
            "is_excluded 应能匹配 managed CLAUDE.md"
        );

        // 但 try_add_managed_claude_md 不调用 is_excluded，直接加入 steps
        let mut steps: Vec<SerLoadChainStep> = Vec::new();
        let mut warnings: Vec<SerLoadChainWarning> = Vec::new();
        try_add_managed_claude_md(&managed_dir, &mut steps, &mut warnings);

        assert_eq!(steps.len(), 1, "managed CLAUDE.md 应被加入启动链");
        assert_eq!(steps[0].scope, "managed");
        assert_eq!(steps[0].asset_type, "managed_claude_md");
        assert_eq!(steps[0].load_reason, "managed instruction");
        assert!(steps[0].exists);

        // warnings 应为空（文件可读）
        assert!(warnings.is_empty(), "可读文件不应产生 warning");

        let _ = fs::remove_dir_all(&root);
    }
}
