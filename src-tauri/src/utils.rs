use std::path::PathBuf;

pub fn describe_path_error(path: &str) -> String {
    let path_buf = PathBuf::from(path);

    if !path_buf.exists() {
        return "项目路径不存在或已被删除".to_string();
    }

    if std::fs::read_dir(&path_buf).is_err() {
        return "无权访问该项目路径".to_string();
    }

    "无法访问该项目路径".to_string()
}
