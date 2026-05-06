use std::fs;
use std::path::Path;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use serde::Deserialize;

/// 每个 `parse_parameters_py()` 调用分配唯一 ID，避免并行测试时的临时文件冲突
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

// ============================================================================
// ProjectConfig — 模块参数配置结构体
// ============================================================================

/// 模块参数配置，对应 Python config/parameters.py 中的 ModuleParameters 类
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ProjectConfig {
    pub project_name: String,
    pub module_name: String,
    pub interface_type: String,
    #[serde(default)]
    pub reference_project: String,
    #[serde(default)]
    pub use_l0: bool,
    #[serde(default)]
    pub data_width: i64,
    #[serde(default)]
    pub iterations: i64,

    #[serde(default)]
    pub q_int_bits: i64,
    #[serde(default)]
    pub q_frac_bits: i64,
    #[serde(default)]
    pub rounding_mode: String,
    #[serde(default)]
    pub saturation: bool,

    #[serde(default)]
    pub pipeline_stages: i64,
    #[serde(default)]
    pub cycles_per_stage: i64,
    #[serde(default)]
    pub output_register: bool,

    #[serde(default)]
    pub axis_data_width: i64,
    #[serde(default)]
    pub axis_has_tlast: bool,
    #[serde(default)]
    pub axis_has_tkeep: bool,
    #[serde(default)]
    pub handshake_delay: i64,
    #[serde(default)]
    pub axi_lite_addr_width: i64,

    #[serde(default)]
    pub test_data_length: i64,
    #[serde(default)]
    pub random_seed: i64,
    #[serde(default)]
    pub float_tolerance: f64,
    #[serde(default)]
    pub fixed_tolerance: f64,

    #[serde(default)]
    pub clock_frequency: i64,
    #[serde(default)]
    pub reset_sync_stages: i64,
    #[serde(default)]
    pub use_clock_enable: bool,

    #[serde(default)]
    pub debug_mode: bool,
    #[serde(default)]
    pub debug_level: i64,

    // 派生参数，标记为 Option 以保持兼容性
    pub total_bits: Option<i64>,
    pub q_scale: Option<i64>,
    pub pipeline_latency: Option<i64>,
    pub max_positive: Option<f64>,
    pub min_negative: Option<f64>,
}

// ============================================================================
// ParameterError — 参数解析错误类型
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParameterError {
    PythonNotFound,
    FileNotFound(String),
    SyntaxError(String),
    RuntimeError(String),
    ExportFailed(String),
    ParseError(String),
}

fn truncate_details(details: &str) -> &str {
    const MAX_CHARS: usize = 200;

    match details.char_indices().nth(MAX_CHARS) {
        Some((idx, _)) => &details[..idx],
        None => details,
    }
}

impl std::fmt::Display for ParameterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParameterError::PythonNotFound => {
                write!(f, "系统未安装 Python3，无法解析参数文件")
            }
            ParameterError::FileNotFound(_) => {
                write!(f, "项目目录下未找到 config/parameters.py，请确认这是有效的 ai_project_template 项目")
            }
            ParameterError::SyntaxError(details) => {
                write!(
                    f,
                    "项目的 parameters.py 存在语法错误。常见原因：双花括号 {{、缩进错误、括号不匹配等。请检查该文件\n详细信息：{}",
                    truncate_details(details)
                )
            }
            ParameterError::RuntimeError(details) => {
                write!(
                    f,
                    "项目的 parameters.py 运行时出错。请检查该文件逻辑\n详细信息：{}",
                    truncate_details(details)
                )
            }
            ParameterError::ExportFailed(msg) => {
                write!(f, "参数导出失败：{}", msg)
            }
            ParameterError::ParseError(msg) => {
                write!(f, "参数解析失败：{}", msg)
            }
        }
    }
}

impl std::error::Error for ParameterError {}

// ============================================================================
// parse_parameters_py — 通过 shell out 到 Python 解析参数
// ============================================================================

/// 执行 `python3 <path> export json <tempfile>` 并解析输出的 JSON 为 ProjectConfig
pub fn parse_parameters_py(path: &Path) -> Result<ProjectConfig, ParameterError> {
    if !path.exists() {
        return Err(ParameterError::FileNotFound(path.to_string_lossy().to_string()));
    }

    let python_version = Command::new("python3")
        .arg("--version")
        .output()
        .map_err(|_| ParameterError::PythonNotFound)?;

    if !python_version.status.success() {
        return Err(ParameterError::PythonNotFound);
    }

    let temp_dir = std::env::temp_dir();
    let seq = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let temp_file = temp_dir.join(format!(
        "parameters_{}_{}.json",
        std::process::id(),
        seq,
    ));

    let output = Command::new("python3")
        .arg(path)
        .arg("export")
        .arg("json")
        .arg(&temp_file)
        .env_remove("PYTHONHOME")
        .env_remove("PYTHONPATH")
        .env_remove("PYTHONNOUSERSITE")
        .output()
        .map_err(|e| ParameterError::ExportFailed(format!("无法执行 python3: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let _ = fs::remove_file(&temp_file);

        if stderr.contains("SyntaxError") || stderr.contains("invalid syntax") {
            return Err(ParameterError::SyntaxError(stderr.trim().to_string()));
        }
        if stderr.contains("Traceback") && stderr.contains("TypeError") {
            return Err(ParameterError::SyntaxError(stderr.trim().to_string()));
        }
        if stderr.contains("Traceback") {
            return Err(ParameterError::RuntimeError(stderr.trim().to_string()));
        }

        return Err(ParameterError::ExportFailed(format!(
            "Python 导出失败 (exit code: {:?}): {}",
            output.status.code(),
            stderr.trim(),
        )));
    }

    let json_str = fs::read_to_string(&temp_file).map_err(|e| {
        let _ = fs::remove_file(&temp_file);
        ParameterError::ParseError(format!("无法读取导出的 JSON 文件: {}", e))
    })?;

    let _ = fs::remove_file(&temp_file);

    let config: ProjectConfig = serde_json::from_str(&json_str).map_err(|e| {
        ParameterError::ParseError(format!("JSON 解析错误: {}", e))
    })?;

    Ok(config)
}

// ============================================================================
// ConfigCollector — 项目配置采集器
// ============================================================================

/// 采集项目的参数配置
///
/// 通过调用 `parse_parameters_py()` 解析 `config/parameters.py` 文件。
pub struct ConfigCollector;

impl ConfigCollector {
    /// 采集指定路径的项目配置
    ///
    /// 自动在项目根目录下查找 `config/parameters.py` 文件。
    pub fn collect(path: &Path) -> Result<ProjectConfig, ParameterError> {
        let params_path = path.join("config").join("parameters.py");
        parse_parameters_py(&params_path)
    }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn temp_dir() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "config_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = fs::create_dir_all(&dir);
        dir
    }

    #[test]
    fn test_parse_nonexistent_path() {
        let result = parse_parameters_py(Path::new("/tmp/nonexistent_parameters_file.py"));
        assert!(matches!(result, Err(ParameterError::FileNotFound(msg)) if msg.contains("nonexistent_parameters_file.py")));
    }

    #[test]
    fn test_parse_export_failure() {
        let dir = temp_dir();
        let bad_script = dir.join("bad_parameters.py");

        let mut file = fs::File::create(&bad_script).expect("应能创建测试文件");
        writeln!(file, "#!/usr/bin/env python3").unwrap();
        writeln!(file, "import sys").unwrap();
        writeln!(file, "print('Intentional failure', file=sys.stderr)").unwrap();
        writeln!(file, "sys.exit(1)").unwrap();
        drop(file);

        let result = parse_parameters_py(&bad_script);
        assert!(
            matches!(result, Err(ParameterError::ExportFailed(ref msg)) if msg.contains("Intentional failure") || msg.contains("exit code")),
            "期望 ExportFailed 错误，得到: {:?}", result
        );

        let _ = fs::remove_file(&bad_script);
        let _ = fs::remove_dir(&dir);
    }

    #[test]
    fn test_parse_syntax_error() {
        let dir = temp_dir();
        let bad_script = dir.join("syntax_parameters.py");

        let mut file = fs::File::create(&bad_script).expect("应能创建测试文件");
        writeln!(file, "#!/usr/bin/env python3").unwrap();
        writeln!(file, "if True print('bad')").unwrap();
        drop(file);

        let result = parse_parameters_py(&bad_script);
        assert!(
            matches!(result, Err(ParameterError::SyntaxError(ref msg)) if msg.contains("SyntaxError") || msg.contains("invalid syntax")),
            "期望 SyntaxError 错误，得到: {:?}",
            result
        );

        let _ = fs::remove_file(&bad_script);
        let _ = fs::remove_dir(&dir);
    }

    #[test]
    fn test_parse_runtime_error() {
        let dir = temp_dir();
        let bad_script = dir.join("runtime_parameters.py");

        let mut file = fs::File::create(&bad_script).expect("应能创建测试文件");
        writeln!(file, "#!/usr/bin/env python3").unwrap();
        writeln!(file, "raise ValueError('boom')").unwrap();
        drop(file);

        let result = parse_parameters_py(&bad_script);
        assert!(
            matches!(result, Err(ParameterError::RuntimeError(ref msg)) if msg.contains("ValueError")),
            "期望 RuntimeError 错误，得到: {:?}",
            result
        );

        let _ = fs::remove_file(&bad_script);
        let _ = fs::remove_dir(&dir);
    }

    #[test]
    fn test_config_collector_file_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let result = ConfigCollector::collect(dir.path());
        assert!(matches!(result, Err(ParameterError::FileNotFound(_))));
    }
}
