use std::fmt;
use std::path::Path;
use std::process::Command;

// ============================================================================
// GitStatus — Git 仓库状态
// ============================================================================

/// Git 仓库当前状态摘要
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitStatus {
    /// 当前分支名称
    pub branch: String,
    /// 已修改但未暂存的文件数
    pub modified_count: usize,
    /// 已暂存但未提交的文件数
    pub staged_count: usize,
    /// 未追踪的文件数
    pub untracked_count: usize,
    /// 冲突文件数
    pub conflict_count: usize,
    /// 是否有未提交的更改
    pub is_clean: bool,
    /// 变更文件列表（包括修改/暂存/未追踪/冲突）
    pub changed_files: Vec<String>,
}

impl GitStatus {
    /// 创建表示"无 Git 仓库"的状态
    pub fn no_repo() -> Self {
        Self {
            branch: String::new(),
            modified_count: 0,
            staged_count: 0,
            untracked_count: 0,
            conflict_count: 0,
            is_clean: true,
            changed_files: Vec::new(),
        }
    }

    /// 创建表示"Git 命令不可用"的状态
    pub fn git_not_available() -> Self {
        Self {
            branch: String::from("(git unavailable)"),
            modified_count: 0,
            staged_count: 0,
            untracked_count: 0,
            conflict_count: 0,
            is_clean: true,
            changed_files: Vec::new(),
        }
    }
}

impl Default for GitStatus {
    fn default() -> Self {
        Self::no_repo()
    }
}

// ============================================================================
// GitCollector — Git 状态采集器
// ============================================================================

/// 采集 Git 仓库的分支和状态信息
///
/// 执行 `git branch` 和 `git status --porcelain` 获取当前分支和
/// 工作区修改状态。
pub struct GitCollector;

impl GitCollector {
    /// 采集指定路径的 Git 状态
    ///
    /// # 参数
    /// - `path`: 项目根目录路径（需要是 Git 仓库或在其内部）
    ///
    /// # 返回
    /// - `Ok(GitStatus)`: 成功获取的 Git 状态
    /// - `Err(GitError)`: Git 命令执行失败
    ///
    /// # 行为
    /// - 如果路径不是 Git 仓库，返回 `GitStatus::no_repo()`
    /// - 如果 `git` 命令不可用，返回 `GitStatus::git_not_available()`
    pub fn collect(path: &Path) -> Result<GitStatus, GitError> {
        // 检查 git 是否可用
        let git_check = Command::new("git").arg("--version").output();
        if git_check.is_err() || !git_check.unwrap().status.success() {
            return Ok(GitStatus::git_not_available());
        }

        // 检查是否为 git 仓库
        let rev_parse = Command::new("git")
            .arg("-C")
            .arg(path)
            .arg("rev-parse")
            .arg("--git-dir")
            .output()
            .map_err(|e| GitError::CommandFailed(format!("git rev-parse 失败: {}", e)))?;

        if !rev_parse.status.success() {
            // 不是 git 仓库，返回空状态
            return Ok(GitStatus::no_repo());
        }

        // 获取当前分支
        let branch = Self::get_current_branch(path)?;

        // 获取状态
        let status = Self::get_status(path)?;

        let is_clean = status.modified_count == 0
            && status.staged_count == 0
            && status.untracked_count == 0
            && status.conflict_count == 0;

        Ok(GitStatus {
            branch,
            is_clean,
            ..status
        })
    }

    fn get_current_branch(path: &Path) -> Result<String, GitError> {
        let output = Command::new("git")
            .arg("-C")
            .arg(path)
            .arg("branch")
            .arg("--show-current")
            .output()
            .map_err(|e| GitError::CommandFailed(format!("git branch 失败: {}", e)))?;

        if output.status.success() {
            let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if branch.is_empty() {
                // 分离 HEAD 状态，尝试获取简短哈希
                let hash_output = Command::new("git")
                    .arg("-C")
                    .arg(path)
                    .arg("rev-parse")
                    .arg("--short")
                    .arg("HEAD")
                    .output()
                    .map_err(|e| GitError::CommandFailed(format!("git rev-parse 失败: {}", e)))?;

                if hash_output.status.success() {
                    let hash = String::from_utf8_lossy(&hash_output.stdout).trim().to_string();
                    Ok(format!("(HEAD detached at {})", hash))
                } else {
                    Ok(String::from("(unknown)"))
                }
            } else {
                Ok(branch)
            }
        } else {
            Ok(String::from("(unknown)"))
        }
    }

    fn get_status(path: &Path) -> Result<GitStatus, GitError> {
        let output = Command::new("git")
            .arg("-C")
            .arg(path)
            .arg("status")
            .arg("--porcelain")
            .output()
            .map_err(|e| GitError::CommandFailed(format!("git status 失败: {}", e)))?;

        if !output.status.success() {
            return Ok(GitStatus::no_repo());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut modified = 0;
        let mut staged = 0;
        let mut untracked = 0;
        let mut conflict = 0;
        let mut changed_files: Vec<String> = Vec::new();

        for line in stdout.lines() {
            if line.len() < 2 {
                continue;
            }

            let xy = &line[..2];
            let x = xy.as_bytes()[0] as char;
            let y = xy.as_bytes()[1] as char;

            //  porcelain v1 格式:
            //  XY PATH 或 XY ORIG_PATH -> PATH
            //  X = index status, Y = working tree status

            match x {
                'M' | 'A' | 'D' | 'R' | 'C' => staged += 1,
                'U' => conflict += 1,
                _ => {}
            }

            match y {
                'M' | 'D' => modified += 1,
                '?' => untracked += 1,
                'U' => conflict += 1,
                _ => {}
            }

            // 提取文件名
            let file_name = if line.contains(" -> ") {
                // 重命名: XY ORIG_PATH -> PATH
                line.split(" -> ").nth(1).unwrap_or(line).trim().to_string()
            } else {
                line[2..].trim().to_string()
            };
            changed_files.push(file_name);
        }

        Ok(GitStatus {
            branch: String::new(),
            modified_count: modified,
            staged_count: staged,
            untracked_count: untracked,
            conflict_count: conflict,
            is_clean: false,
            changed_files,
        })
    }
}

// ============================================================================
// GitError — Git 采集错误
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitError {
    CommandFailed(String),
}

impl fmt::Display for GitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GitError::CommandFailed(msg) => write!(f, "Git 命令执行失败: {}", msg),
        }
    }
}

impl std::error::Error for GitError {}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use std::process::Command;

    fn init_git_repo(dir: &Path) {
        Command::new("git")
            .arg("init")
            .arg(dir)
            .output()
            .expect("git init should work");

        Command::new("git")
            .arg("-C")
            .arg(dir)
            .arg("config")
            .arg("user.email")
            .arg("test@test.com")
            .output()
            .unwrap();

        Command::new("git")
            .arg("-C")
            .arg(dir)
            .arg("config")
            .arg("user.name")
            .arg("Test User")
            .output()
            .unwrap();
    }

    #[test]
    fn test_git_collector_no_repo() {
        let dir = tempfile::tempdir().unwrap();
        let status = GitCollector::collect(dir.path()).unwrap();
        assert!(status.branch.is_empty() || status.branch == "(unknown)");
        assert!(status.is_clean);
    }

    #[test]
    fn test_git_collector_clean_repo() {
        let dir = tempfile::tempdir().unwrap();
        init_git_repo(dir.path());

        let status = GitCollector::collect(dir.path()).unwrap();
        // 新创建的仓库可能在 master 或 main 分支
        assert!(
            status.branch == "master" || status.branch == "main" || status.branch.contains("detached"),
            "unexpected branch: {}", status.branch
        );
        assert!(status.is_clean);
    }

    #[test]
    fn test_git_collector_with_untracked() {
        let dir = tempfile::tempdir().unwrap();
        init_git_repo(dir.path());

        let file_path = dir.path().join("untracked.txt");
        let mut file = fs::File::create(&file_path).unwrap();
        writeln!(file, "hello").unwrap();
        drop(file);

        let status = GitCollector::collect(dir.path()).unwrap();
        assert_eq!(status.untracked_count, 1);
        assert!(!status.is_clean);
    }

    #[test]
    fn test_git_collector_with_modified() {
        let dir = tempfile::tempdir().unwrap();
        init_git_repo(dir.path());

        // 创建并提交一个文件
        let file_path = dir.path().join("test.txt");
        let mut file = fs::File::create(&file_path).unwrap();
        writeln!(file, "initial").unwrap();
        drop(file);

        Command::new("git")
            .arg("-C")
            .arg(dir.path())
            .arg("add")
            .arg(".")
            .output()
            .unwrap();

        Command::new("git")
            .arg("-C")
            .arg(dir.path())
            .arg("commit")
            .arg("-m")
            .arg("initial")
            .output()
            .unwrap();

        // 修改文件
        let mut file = fs::File::create(&file_path).unwrap();
        writeln!(file, "modified").unwrap();
        drop(file);

        let status = GitCollector::collect(dir.path()).unwrap();
        assert_eq!(status.modified_count, 1);
        assert!(!status.is_clean);
    }

    #[test]
    fn test_git_collector_with_staged() {
        let dir = tempfile::tempdir().unwrap();
        init_git_repo(dir.path());

        let file_path = dir.path().join("staged.txt");
        let mut file = fs::File::create(&file_path).unwrap();
        writeln!(file, "content").unwrap();
        drop(file);

        Command::new("git")
            .arg("-C")
            .arg(dir.path())
            .arg("add")
            .arg(".")
            .output()
            .unwrap();

        let status = GitCollector::collect(dir.path()).unwrap();
        assert_eq!(status.staged_count, 1);
        assert!(!status.is_clean);
    }
}
