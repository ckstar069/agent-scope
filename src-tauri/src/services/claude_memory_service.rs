use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::app_state::AppState;
use crate::collectors::claude_memory::load_chain::simulate_load_chain;
use crate::collectors::claude_memory::models::{SerClaudeMemoryScanResult, SerLoadChain};
use crate::collectors::claude_memory::path_resolver::resolve_claude_config_dir;
use crate::collectors::claude_memory::scanner::{scan_claude_memory, scan_project_level};
use crate::collectors::claude_memory::secret_scanner::SecretScanner;

const MAX_FILE_READ_SIZE: u64 = 1_048_576; // 1 MiB

/// 模拟加载链
pub fn simulate_load_chain_service(cwd: String) -> Result<SerLoadChain, String> {
    let path = PathBuf::from(&cwd);
    simulate_load_chain(&path)
}

/// 获取 Claude Code 记忆概览
pub fn get_claude_memory_overview_service(
    project_path: Option<String>,
    _force: bool,
    state: &AppState,
) -> Result<SerClaudeMemoryScanResult, String> {
    // 1. 执行基础扫描（用户级 + 可选 project_path）
    let mut result = scan_claude_memory(project_path.clone());

    // 2. 收集已扫描的 canonical project path（用于去重）
    let mut scanned_paths: HashSet<PathBuf> = HashSet::new();

    // 2a. 记录 project_path（如果已扫描）
    if let Some(ref extra) = project_path {
        if let Ok(canonical) = std::fs::canonicalize(extra) {
            scanned_paths.insert(canonical);
        }
    }

    // 3. 扫描已注册项目，跳过已扫描的路径
    let scanner = SecretScanner::new();
    let registry = state.registry.lock().map_err(|e| e.to_string())?;
    for project in registry.list() {
        let path = PathBuf::from(&project.path);
        let canonical = match std::fs::canonicalize(&path) {
            Ok(c) => c,
            Err(e) => {
                result.errors.push(
                    crate::collectors::claude_memory::models::SerMemoryScanError {
                        scope: "project".to_string(),
                        path: project.path.clone(),
                        message: format!("canonicalize 失败: {}", e),
                    },
                );
                continue;
            }
        };

        if scanned_paths.contains(&canonical) {
            continue; // 已扫描过，跳过
        }

        scanned_paths.insert(canonical);
        scan_project_level(&path, &mut result.assets, &scanner, &mut result.errors);
    }
    // 重新计算 summary（因为新增了已注册项目的资产）
    result.summary = build_summary(&result.assets);

    Ok(result)
}

/// 读取指定记忆文件的内容
pub fn get_claude_memory_file_content_service(
    native_path: String,
    project_path: Option<String>,
    state: &AppState,
) -> Result<String, String> {
    let path = PathBuf::from(&native_path);

    // 1. 必须是绝对路径
    if !path.is_absolute() {
        return Err("路径必须是绝对路径".to_string());
    }

    // 2. 扩展名校验
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    if !matches!(ext, "md" | "json") {
        return Err("不支持的文件类型，只允许 .md 或 .json".to_string());
    }

    // 3. canonicalize 待读取文件
    let canonical_file =
        std::fs::canonicalize(&path).map_err(|e| format!("无法解析路径: {}", e))?;

    // 4. 收集所有 allowlist root 并 canonicalize
    let mut allowed_roots: Vec<PathBuf> = Vec::new();

    // 4a. Claude config dir
    if let Ok(claude_dir) = resolve_claude_config_dir() {
        if let Ok(canonical) = std::fs::canonicalize(&claude_dir) {
            allowed_roots.push(canonical);
        }
    }

    // 4b. 已注册项目路径
    let registry = state.registry.lock().map_err(|e| e.to_string())?;
    for project in registry.list() {
        let project_path = Path::new(&project.path);
        if let Ok(canonical) = std::fs::canonicalize(project_path) {
            allowed_roots.push(canonical);
        }
    }
    drop(registry);

    // 4c. 额外指定的 project_path
    if let Some(extra) = project_path {
        let extra_path = Path::new(&extra);
        if extra_path.exists() && extra_path.is_dir() {
            if let Ok(canonical) = std::fs::canonicalize(extra_path) {
                allowed_roots.push(canonical);
            }
        }
    }

    // 5. canonicalize 后比对
    let is_allowed = allowed_roots
        .iter()
        .any(|root| canonical_file.starts_with(root));

    if !is_allowed {
        return Err("路径不在允许范围内".to_string());
    }

    // 6. 检查文件大小
    let metadata =
        std::fs::metadata(&canonical_file).map_err(|e| format!("无法读取文件元数据: {}", e))?;
    if metadata.len() > MAX_FILE_READ_SIZE {
        return Err("文件过大，无法读取".to_string());
    }

    // 7. 读取内容
    let content =
        std::fs::read_to_string(&canonical_file).map_err(|e| format!("无法读取文件内容: {}", e))?;

    Ok(content)
}

// ─── 辅助：重建 summary ───

use crate::collectors::claude_memory::models::SerMemorySummary;
use std::collections::HashMap;

fn build_summary(
    assets: &[crate::collectors::claude_memory::models::SerClaudeMemoryAsset],
) -> SerMemorySummary {
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

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::ProjectRegistry;
    use std::fs;

    fn test_state(registry: ProjectRegistry) -> AppState {
        AppState {
            registry: std::sync::Mutex::new(registry),
            watchers: std::sync::Mutex::new(std::collections::HashMap::new()),
            agent_collector: std::sync::Mutex::new(crate::collectors::agent::AgentCollector::new()),
            template_path: std::sync::Mutex::new(None),
            template_fingerprint: std::sync::Mutex::new(None),
        }
    }

    /// 测试：拒绝相对路径
    #[test]
    fn test_allowlist_rejects_relative_path() {
        let registry = ProjectRegistry::new(std::env::temp_dir().join("test.json"));
        let state = test_state(registry);

        let result =
            get_claude_memory_file_content_service("../../etc/passwd".to_string(), None, &state);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("绝对路径"));
    }

    /// 测试：拒绝非 allowlist 路径
    #[test]
    fn test_allowlist_rejects_outside_path() {
        let registry = ProjectRegistry::new(std::env::temp_dir().join("test.json"));
        let state = test_state(registry);

        // 在临时目录创建一个 .md 文件，但不在 allowlist 中
        let tmp_file = std::env::temp_dir().join("random-test-file.md");
        fs::write(&tmp_file, "# test").unwrap();

        let result = get_claude_memory_file_content_service(
            tmp_file.to_string_lossy().to_string(),
            None,
            &state,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("不在允许范围"));

        let _ = fs::remove_file(&tmp_file);
    }

    /// 测试：允许额外指定的 project_path
    #[test]
    fn test_allowlist_allows_project_path() {
        let registry = ProjectRegistry::new(std::env::temp_dir().join("test.json"));
        let state = test_state(registry);

        let tmp_dir = std::env::temp_dir().join("agent-scope-allow-test");
        let _ = fs::remove_dir_all(&tmp_dir);
        fs::create_dir_all(&tmp_dir).unwrap();
        let test_md = tmp_dir.join("CLAUDE.md");
        fs::write(&test_md, "# Hello\n").unwrap();

        let result = get_claude_memory_file_content_service(
            test_md.to_string_lossy().to_string(),
            Some(tmp_dir.to_string_lossy().to_string()),
            &state,
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "# Hello\n");

        let _ = fs::remove_dir_all(&tmp_dir);
    }

    /// 测试：拒绝过大文件
    #[test]
    fn test_rejects_oversized_file() {
        let registry = ProjectRegistry::new(std::env::temp_dir().join("test.json"));
        let state = test_state(registry);

        let tmp_dir = std::env::temp_dir().join("agent-scope-size-test");
        let _ = fs::remove_dir_all(&tmp_dir);
        fs::create_dir_all(&tmp_dir).unwrap();
        let test_md = tmp_dir.join("CLAUDE.md");
        // 写入超过 1 MiB
        fs::write(&test_md, "x".repeat(1_100_000)).unwrap();

        let result = get_claude_memory_file_content_service(
            test_md.to_string_lossy().to_string(),
            Some(tmp_dir.to_string_lossy().to_string()),
            &state,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("过大"));

        let _ = fs::remove_dir_all(&tmp_dir);
    }

    /// 测试：拒绝非 .md/.json 扩展名
    #[test]
    fn test_rejects_bad_extension() {
        let registry = ProjectRegistry::new(std::env::temp_dir().join("test.json"));
        let state = test_state(registry);

        let result =
            get_claude_memory_file_content_service("/tmp/test.txt".to_string(), None, &state);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("不支持的文件类型"));
    }
}
