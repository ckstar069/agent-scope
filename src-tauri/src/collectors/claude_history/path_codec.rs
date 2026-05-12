use std::path::PathBuf;

/// 获取 Claude Code 配置目录（跨平台）
pub fn claude_config_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(".claude"))
}

/// 将项目路径编码为 Claude Code 项目目录名格式
///
/// macOS/Linux: /Users/name/Repo → -Users-name-Repo
/// Windows: C:\Repo → C--Repo
///
/// # 示例
///
/// ```
/// use ptv_lib::collectors::claude_history::path_codec::encode_cwd_path;
/// assert_eq!(encode_cwd_path("/Users/ckstar/Repo/my_project"), "-Users-ckstar-Repo-my-project");
/// assert_eq!(encode_cwd_path("/home/user/project"), "-home-user-project");
/// assert_eq!(encode_cwd_path("relative/path"), "relative-path");
/// ```
pub fn encode_cwd_path(cwd: &str) -> String {
    if cfg!(target_os = "windows") {
        cwd.replace("\\", "--")
    } else {
        let without_leading = cwd.strip_prefix('/').unwrap_or(cwd);
        let encoded = without_leading.replace("/", "-").replace("_", "-");
        if cwd.starts_with('/') {
            format!("-{}", encoded)
        } else {
            encoded
        }
    }
}

/// 将编码目录名还原为原始项目路径
pub fn decode_project_dir(encoded: &str) -> String {
    if cfg!(target_os = "windows") {
        encoded.replace("--", "\\").replace('-', "\\")
    } else {
        if let Some(stripped) = encoded.strip_prefix('-') {
            format!("/{}", stripped.replace('-', "/"))
        } else {
            encoded.replace('-', "/")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_claude_config_dir() {
        let dir = claude_config_dir();
        assert!(dir.is_some());
        let path = dir.unwrap();
        assert!(path.to_string_lossy().ends_with(".claude"));
    }

    #[test]
    fn test_encode_cwd_path_unix() {
        assert_eq!(encode_cwd_path("/Users/ckstar/Repo/my_project"), "-Users-ckstar-Repo-my-project");
        assert_eq!(encode_cwd_path("/home/user/project"), "-home-user-project");
    }

    #[test]
    fn test_decode_project_dir_unix() {
        assert_eq!(decode_project_dir("-Users-ckstar-Repo"), "/Users/ckstar/Repo");
        assert_eq!(decode_project_dir("home-user-project"), "home/user/project");
    }

    #[test]
    fn test_encode_decode_roundtrip_unix() {
        let original = "/Users/name/project";
        let encoded = encode_cwd_path(original);
        let decoded = decode_project_dir(&encoded);
        assert_eq!(decoded, original);
    }
}
