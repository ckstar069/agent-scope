use std::fmt;
use std::fs;
use std::path::Path;

// ============================================================================
// Stage — 项目阶段枚举
// ============================================================================

/// FPGA 项目开发阶段
///
/// 对应 `.current_stage` 文件中的字符串值，从 L0（外部库接口）到
/// Hardware（硬件部署）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Stage {
    /// L0: 第三方库接口阶段
    L0,
    /// L1: Python 原型阶段
    L1,
    /// L2: 结构化 Python 阶段
    L2,
    /// L3: 流水线 Python 阶段
    L3,
    /// L4: 周期精确 Python 阶段
    L4,
    /// L5: 定点数 Python 阶段
    L5,
    /// L6: 资源优化 Python 阶段
    L6,
    /// Verilog 实现阶段
    Verilog,
    /// 综合阶段
    Synthesis,
    /// 硬件部署阶段
    Hardware,
}

impl Stage {
    /// 从字符串解析 Stage，支持多种常见格式
    ///
    /// 支持的输入格式：
    /// - `l0`, `l1`, ..., `l6`（大小写不敏感）
    /// - `verilog`, `synthesis`, `hardware`（大小写不敏感）
    /// - 带下划线的变体如 `l0_external`, `l1_prototype` 等（取前缀）
    pub fn parse(s: &str) -> Option<Self> {
        let lower = s.trim().to_lowercase();
        // 处理带下划线的变体，如 "l0_external" -> "l0"
        let prefix = lower.split('_').next().unwrap_or(&lower);
        match prefix {
            "l0" => Some(Stage::L0),
            "l1" => Some(Stage::L1),
            "l2" => Some(Stage::L2),
            "l3" => Some(Stage::L3),
            "l4" => Some(Stage::L4),
            "l5" => Some(Stage::L5),
            "l6" => Some(Stage::L6),
            "verilog" => Some(Stage::Verilog),
            "synthesis" => Some(Stage::Synthesis),
            "hardware" => Some(Stage::Hardware),
            _ => None,
        }
    }

    /// 返回阶段的简短标识符（如 "l0"）
    pub fn as_str(&self) -> &'static str {
        match self {
            Stage::L0 => "l0",
            Stage::L1 => "l1",
            Stage::L2 => "l2",
            Stage::L3 => "l3",
            Stage::L4 => "l4",
            Stage::L5 => "l5",
            Stage::L6 => "l6",
            Stage::Verilog => "verilog",
            Stage::Synthesis => "synthesis",
            Stage::Hardware => "hardware",
        }
    }

    /// 返回阶段的中文描述
    pub fn description(&self) -> &'static str {
        match self {
            Stage::L0 => "L0: 外部库接口",
            Stage::L1 => "L1: Python 原型",
            Stage::L2 => "L2: 结构化 Python",
            Stage::L3 => "L3: 流水线 Python",
            Stage::L4 => "L4: 周期精确 Python",
            Stage::L5 => "L5: 定点数 Python",
            Stage::L6 => "L6: 资源优化 Python",
            Stage::Verilog => "Verilog 实现",
            Stage::Synthesis => "综合",
            Stage::Hardware => "硬件部署",
        }
    }

    /// 返回阶段的序号（用于排序和进度计算）
    ///
    /// 返回值范围：0 (L0) ~ 9 (Hardware)
    pub fn ordinal(&self) -> u8 {
        match self {
            Stage::L0 => 0,
            Stage::L1 => 1,
            Stage::L2 => 2,
            Stage::L3 => 3,
            Stage::L4 => 4,
            Stage::L5 => 5,
            Stage::L6 => 6,
            Stage::Verilog => 7,
            Stage::Synthesis => 8,
            Stage::Hardware => 9,
        }
    }
}

impl fmt::Display for Stage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ============================================================================
// StageCollector — 采集当前阶段
// ============================================================================

/// 采集项目的当前阶段
///
/// 读取项目根目录下的 `.current_stage` 文件，解析为 [`Stage`] 枚举。
pub struct StageCollector;

impl StageCollector {
    /// 采集指定路径的项目阶段
    ///
    /// # 参数
    /// - `path`: 项目根目录路径
    ///
    /// # 返回
    /// - `Ok(Stage)`: 成功解析的阶段
    /// - `Err(StageError)`: 文件不存在、读取失败或内容无法识别
    pub fn collect(path: &Path) -> Result<Stage, StageError> {
        let stage_file = path.join(".current_stage");

        if !stage_file.exists() {
            return Err(StageError::FileNotFound(stage_file.to_string_lossy().to_string()));
        }

        let content = fs::read_to_string(&stage_file).map_err(|e| {
            StageError::ReadError(stage_file.to_string_lossy().to_string(), e.to_string())
        })?;

        let trimmed = content.trim();
        if trimmed.is_empty() {
            return Err(StageError::EmptyFile);
        }

        Stage::parse(trimmed).ok_or_else(|| StageError::UnknownStage(trimmed.to_string()))
    }
}

// ============================================================================
// StageError — 阶段采集错误
// ============================================================================

/// 阶段采集过程中可能发生的错误
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StageError {
    /// `.current_stage` 文件不存在
    FileNotFound(String),
    /// 文件读取失败
    ReadError(String, String),
    /// 文件内容为空
    EmptyFile,
    /// 无法识别的阶段字符串
    UnknownStage(String),
}

impl fmt::Display for StageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StageError::FileNotFound(_) => write!(f, "未找到 .current_stage 文件，项目阶段未知"),
            StageError::ReadError(_, err) => {
                write!(f, "读取 .current_stage 失败：{}", err)
            }
            StageError::EmptyFile => write!(f, ".current_stage 文件内容为空"),
            StageError::UnknownStage(s) => write!(f, "无法识别的阶段：'{}'，支持 L0-L6、Verilog、Synthesis、Hardware", s),
        }
    }
}

impl std::error::Error for StageError {}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// 创建临时目录和文件辅助函数
    fn temp_stage_file(content: &str) -> (std::path::PathBuf, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(".current_stage");
        let mut file = fs::File::create(&path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
        file.sync_all().unwrap();
        (dir.path().to_path_buf(), dir)
    }

    #[test]
    fn test_stage_parse_basic() {
        assert_eq!(Stage::parse("l0"), Some(Stage::L0));
        assert_eq!(Stage::parse("L1"), Some(Stage::L1));
        assert_eq!(Stage::parse("L6"), Some(Stage::L6));
        assert_eq!(Stage::parse("verilog"), Some(Stage::Verilog));
        assert_eq!(Stage::parse("SYNTHESIS"), Some(Stage::Synthesis));
        assert_eq!(Stage::parse("Hardware"), Some(Stage::Hardware));
    }

    #[test]
    fn test_stage_parse_prefixed() {
        // 支持带下划线的变体
        assert_eq!(Stage::parse("l0_external"), Some(Stage::L0));
        assert_eq!(Stage::parse("l1_prototype"), Some(Stage::L1));
        assert_eq!(Stage::parse("l3_pipeline_v2"), Some(Stage::L3));
    }

    #[test]
    fn test_stage_parse_whitespace() {
        assert_eq!(Stage::parse("  l2  "), Some(Stage::L2));
        assert_eq!(Stage::parse("l4\n"), Some(Stage::L4));
    }

    #[test]
    fn test_stage_parse_invalid() {
        assert_eq!(Stage::parse("invalid"), None);
        assert_eq!(Stage::parse(""), None);
        assert_eq!(Stage::parse("l7"), None);
        assert_eq!(Stage::parse("l10"), None);
    }

    #[test]
    fn test_stage_ordinal() {
        assert_eq!(Stage::L0.ordinal(), 0);
        assert_eq!(Stage::L5.ordinal(), 5);
        assert_eq!(Stage::Verilog.ordinal(), 7);
        assert_eq!(Stage::Hardware.ordinal(), 9);
    }

    #[test]
    fn test_stage_display() {
        assert_eq!(format!("{}", Stage::L3), "l3");
        assert_eq!(format!("{}", Stage::Synthesis), "synthesis");
    }

    #[test]
    fn test_stage_collector_success() {
        let (path, _dir) = temp_stage_file("l3");
        let stage = StageCollector::collect(&path).unwrap();
        assert_eq!(stage, Stage::L3);
    }

    #[test]
    fn test_stage_collector_with_newline() {
        let (path, _dir) = temp_stage_file("verilog\n");
        let stage = StageCollector::collect(&path).unwrap();
        assert_eq!(stage, Stage::Verilog);
    }

    #[test]
    fn test_stage_collector_file_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let result = StageCollector::collect(dir.path());
        assert!(matches!(result, Err(StageError::FileNotFound(_))));
    }

    #[test]
    fn test_stage_collector_empty_file() {
        let (path, _dir) = temp_stage_file("");
        let result = StageCollector::collect(&path);
        assert!(matches!(result, Err(StageError::EmptyFile)));
    }

    #[test]
    fn test_stage_collector_unknown_stage() {
        let (path, _dir) = temp_stage_file("unknown_stage");
        let result = StageCollector::collect(&path);
        assert!(matches!(result, Err(StageError::UnknownStage(s)) if s == "unknown_stage"));
    }
}
