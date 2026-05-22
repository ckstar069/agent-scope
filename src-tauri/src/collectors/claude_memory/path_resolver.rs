use std::env;
use std::path::{Path, PathBuf};

// ============================================================================
// 测试辅助（跨模块共享）
// ============================================================================

#[cfg(test)]
pub(crate) mod test_helpers {
    use std::sync::Mutex;

    /// 全局环境变量锁，防止并发测试互相覆盖 CLAUDE_CONFIG_DIR
    pub static ENV_LOCK: Mutex<()> = Mutex::new(());

    /// 安全设置环境变量：获取全局锁，执行闭包后恢复原始值
    pub fn with_env_var<F, R>(key: &str, value: &str, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        let _guard = ENV_LOCK.lock().unwrap();
        let old = std::env::var(key).ok();
        std::env::set_var(key, value);
        let result = f();
        match old {
            Some(v) => std::env::set_var(key, v),
            None => std::env::remove_var(key),
        }
        result
    }
}

/// 解析 Claude Code 配置根目录
/// 优先级：CLAUDE_CONFIG_DIR > dirs::home_dir().join(".claude")
pub fn resolve_claude_config_dir() -> Result<PathBuf, String> {
    // 1. 优先读取 CLAUDE_CONFIG_DIR 环境变量
    if let Ok(val) = env::var("CLAUDE_CONFIG_DIR") {
        let path = PathBuf::from(val);
        if !path.as_os_str().is_empty() {
            return Ok(path);
        }
    }

    // 2. 回退到用户主目录下的 .claude
    dirs::home_dir()
        .map(|home| home.join(".claude"))
        .ok_or_else(|| "无法获取用户主目录".to_string())
}

// ─── 用户级路径 ───

pub fn resolve_user_claude_md() -> Result<PathBuf, String> {
    resolve_claude_config_dir().map(|dir| dir.join("CLAUDE.md"))
}

pub fn resolve_user_rules_dir() -> Result<PathBuf, String> {
    resolve_claude_config_dir().map(|dir| dir.join("rules"))
}

pub fn resolve_user_skills_dir() -> Result<PathBuf, String> {
    resolve_claude_config_dir().map(|dir| dir.join("skills"))
}

pub fn resolve_user_agents_dir() -> Result<PathBuf, String> {
    resolve_claude_config_dir().map(|dir| dir.join("agents"))
}

pub fn resolve_auto_memory_dir() -> Result<PathBuf, String> {
    resolve_claude_config_dir().map(|dir| dir.join("projects"))
}

// ─── 项目级路径 ───

pub fn resolve_project_claude_md(project_root: &Path) -> PathBuf {
    project_root.join("CLAUDE.md")
}

pub fn resolve_project_dot_claude_md(project_root: &Path) -> PathBuf {
    project_root.join(".claude").join("CLAUDE.md")
}

pub fn resolve_project_local_md(project_root: &Path) -> PathBuf {
    project_root.join("CLAUDE.local.md")
}

pub fn resolve_project_rules_dir(project_root: &Path) -> PathBuf {
    project_root.join(".claude").join("rules")
}

pub fn resolve_project_skills_dir(project_root: &Path) -> PathBuf {
    project_root.join(".claude").join("skills")
}

pub fn resolve_project_agents_dir(project_root: &Path) -> PathBuf {
    project_root.join(".claude").join("agents")
}

// ─── 组织级 managed 路径（v0.1 预留，不扫描） ───

pub fn resolve_managed_dir() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        Some(PathBuf::from("/Library/Application Support/ClaudeCode"))
    }
    #[cfg(target_os = "linux")]
    {
        Some(PathBuf::from("/etc/claude-code"))
    }
    #[cfg(target_os = "windows")]
    {
        Some(PathBuf::from(r"C:\Program Files\ClaudeCode"))
    }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::test_helpers::with_env_var;
    use super::*;

    /// 测试：优先使用 CLAUDE_CONFIG_DIR
    #[test]
    fn test_claude_config_dir_env_priority() {
        with_env_var("CLAUDE_CONFIG_DIR", "/tmp/my-claude-config", || {
            let result = resolve_claude_config_dir().unwrap();
            assert_eq!(result, PathBuf::from("/tmp/my-claude-config"));
        });
    }

    /// 测试：空字符串的 CLAUDE_CONFIG_DIR 应回退到 home
    #[test]
    fn test_claude_config_dir_empty_env_fallback() {
        with_env_var("CLAUDE_CONFIG_DIR", "", || {
            let result = resolve_claude_config_dir();
            // 应回退到 home/.claude，不应返回空路径
            assert!(result.is_ok());
            let path = result.unwrap();
            assert!(path.to_string_lossy().ends_with(".claude"));
        });
    }

    /// 测试：未设置 CLAUDE_CONFIG_DIR 时回退到 home
    #[test]
    fn test_claude_config_dir_home_fallback() {
        let _guard = super::test_helpers::ENV_LOCK.lock().unwrap();
        let old = env::var("CLAUDE_CONFIG_DIR").ok();
        env::remove_var("CLAUDE_CONFIG_DIR");

        let result = resolve_claude_config_dir().unwrap();
        assert!(result.to_string_lossy().ends_with(".claude"));

        // 恢复
        match old {
            Some(v) => env::set_var("CLAUDE_CONFIG_DIR", v),
            None => env::remove_var("CLAUDE_CONFIG_DIR"),
        }
    }

    /// 测试：用户级路径基于配置目录
    #[test]
    fn test_user_paths() {
        with_env_var("CLAUDE_CONFIG_DIR", "/tmp/test-claude", || {
            assert_eq!(
                resolve_user_claude_md().unwrap(),
                PathBuf::from("/tmp/test-claude/CLAUDE.md")
            );
            assert_eq!(
                resolve_user_rules_dir().unwrap(),
                PathBuf::from("/tmp/test-claude/rules")
            );
            assert_eq!(
                resolve_user_skills_dir().unwrap(),
                PathBuf::from("/tmp/test-claude/skills")
            );
            assert_eq!(
                resolve_user_agents_dir().unwrap(),
                PathBuf::from("/tmp/test-claude/agents")
            );
            assert_eq!(
                resolve_auto_memory_dir().unwrap(),
                PathBuf::from("/tmp/test-claude/projects")
            );
        });
    }

    /// 测试：项目级路径基于给定根目录
    #[test]
    fn test_project_paths() {
        let root = PathBuf::from("/tmp/my-project");

        assert_eq!(
            resolve_project_claude_md(&root),
            PathBuf::from("/tmp/my-project/CLAUDE.md")
        );
        assert_eq!(
            resolve_project_dot_claude_md(&root),
            PathBuf::from("/tmp/my-project/.claude/CLAUDE.md")
        );
        assert_eq!(
            resolve_project_local_md(&root),
            PathBuf::from("/tmp/my-project/CLAUDE.local.md")
        );
        assert_eq!(
            resolve_project_rules_dir(&root),
            PathBuf::from("/tmp/my-project/.claude/rules")
        );
        assert_eq!(
            resolve_project_skills_dir(&root),
            PathBuf::from("/tmp/my-project/.claude/skills")
        );
        assert_eq!(
            resolve_project_agents_dir(&root),
            PathBuf::from("/tmp/my-project/.claude/agents")
        );
    }
}
