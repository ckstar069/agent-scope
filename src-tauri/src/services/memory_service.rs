use std::io::Write;
use std::path::PathBuf;

use crate::models::project::SerCandidateMemory;
use crate::utils::describe_path_error;

pub fn save_candidate_memory(
    path: String,
    memory: SerCandidateMemory,
) -> Result<(), String> {
    let path_buf = PathBuf::from(&path);
    if !path_buf.exists() || !path_buf.is_dir() {
        return Err(describe_path_error(&path));
    }

    let memory_dir = path_buf
        .join(".sisyphus")
        .join("notepads")
        .join("project-memory");
    let memory_file = memory_dir.join("decisions.md");

    // 确保目录存在
    if let Err(e) = std::fs::create_dir_all(&memory_dir) {
        return Err(format!("创建目录失败: {}", e));
    }

    // 生成时间戳（使用 std::time，无需 chrono 依赖）
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    // 简单日期计算（UTC），近似 YYYY-MM-DD HH:MM:SS
    let days = secs / 86400;
    let time_of_day = secs % 86400;
    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;
    let seconds = time_of_day % 60;

    // 从 epoch 天数计算日期（1970-01-01 = day 0）
    let mut y = 1970i64;
    let mut remaining_days = days as i64;
    loop {
        let year_days = if (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0) {
            366
        } else {
            365
        };
        if remaining_days < year_days {
            break;
        }
        remaining_days -= year_days;
        y += 1;
    }
    let month_days = if (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut m = 1usize;
    for &md in &month_days {
        if remaining_days < md as i64 {
            break;
        }
        remaining_days -= md as i64;
        m += 1;
    }
    let d = remaining_days + 1;

    let timestamp = format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        y, m, d, hours, minutes, seconds
    );

    let entry = format!(
        "\n## [{}] {}\n\n{}\n\n来源: {} / Turn {}\n",
        timestamp,
        memory.category,
        memory.content,
        memory.source_session_id,
        memory.source_turn_index
    );

    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&memory_file)
        .map_err(|e| format!("打开文件失败: {}", e))?;

    file.write_all(entry.as_bytes())
        .map_err(|e| format!("写入文件失败: {}", e))?;

    Ok(())
}
