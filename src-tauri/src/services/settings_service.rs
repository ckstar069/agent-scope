use std::path::PathBuf;
use tauri::State;

use crate::app_state::{AppState, TemplateFingerprintCache};
use crate::collectors::template::{save_template_path, TemplateFingerprint};
use crate::registry::ProjectRegistry;

pub fn set_template_path(
    path: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let path_buf = PathBuf::from(&path);

    if !path_buf.exists() {
        return Err(format!("路径不存在: {}", path));
    }
    if !path_buf.is_dir() {
        return Err(format!("路径不是目录: {}", path));
    }

    let fingerprint = TemplateFingerprint::build(&path_buf)
        .map_err(|e| format!("构建模板指纹失败: {}", e))?;

    {
        let mut tp = state.template_path.lock().map_err(|e| format!("锁获取失败: {}", e))?;
        *tp = Some(path_buf.clone());
    }
    {
        let mut fp = state
            .template_fingerprint
            .lock()
            .map_err(|e| format!("锁获取失败: {}", e))?;
        *fp = Some(TemplateFingerprintCache {
            paths: fingerprint.paths,
            generated_at: std::time::Instant::now(),
        });
    }

    let data_dir = ProjectRegistry::default_data_dir();
    save_template_path(&data_dir, &path_buf)
        .map_err(|e| format!("保存模板路径失败: {}", e))?;

    Ok(())
}

pub fn get_template_path(state: State<'_, AppState>) -> Result<Option<String>, String> {
    let tp = state
        .template_path
        .lock()
        .map_err(|e| format!("锁获取失败: {}", e))?;
    Ok(tp.as_ref().map(|p| p.to_string_lossy().to_string()))
}
