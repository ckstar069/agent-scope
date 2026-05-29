use tauri::State;

use crate::app_state::AppState;
use crate::services::usage_service::{parse_group_by, parse_time_range, UsageScanSummary};

#[tauri::command(rename = "get_usage_source_status")]
pub async fn get_usage_source_status_cmd(
    state: State<'_, AppState>,
) -> Result<crate::collectors::usage::models::UsageSourceStatus, String> {
    let service_arc = state.usage_service.clone();
    let status = tauri::async_runtime::spawn_blocking(move || {
        let mut service = service_arc.lock().map_err(|e| e.to_string())?;
        Ok::<_, String>(service.source_status())
    })
    .await
    .map_err(|e| format!("任务执行失败: {}", e))??;
    Ok(status)
}

#[tauri::command(rename = "scan_usage_data")]
pub async fn scan_usage_data_cmd(
    state: State<'_, AppState>,
) -> Result<UsageScanSummary, String> {
    let service_arc = state.usage_service.clone();
    let summary = tauri::async_runtime::spawn_blocking(move || {
        let mut service = service_arc.lock().map_err(|e| e.to_string())?;
        let result = service.scan();
        Ok::<_, String>(UsageScanSummary::from(result))
    })
    .await
    .map_err(|e| format!("任务执行失败: {}", e))??;
    Ok(summary)
}

#[tauri::command(rename = "get_usage_analytics")]
pub async fn get_usage_analytics_cmd(
    time_range: String,
    group_by: String,
    state: State<'_, AppState>,
) -> Result<crate::collectors::usage::models::UsageAggregate, String> {
    let time_range = parse_time_range(&time_range)?;
    let group_by = parse_group_by(&group_by)?;

    let service_arc = state.usage_service.clone();
    let aggregate = tauri::async_runtime::spawn_blocking(move || {
        let mut service = service_arc.lock().map_err(|e| e.to_string())?;
        Ok::<_, String>(service.analytics(time_range, group_by))
    })
    .await
    .map_err(|e| format!("任务执行失败: {}", e))??;
    Ok(aggregate)
}
