use serde::{Deserialize, Serialize};

use crate::collectors::template::{
    GitStatus, ProjectConfig, ProjectFile, SessionSummary, SessionTranscript, SessionTurn,
    SourceLayout, TemplateData,
};

#[derive(Debug, Clone, Serialize)]
pub struct SerStage {
    pub name: String,
    pub description: String,
    pub ordinal: u8,
}

impl From<crate::collectors::template::Stage> for SerStage {
    fn from(stage: crate::collectors::template::Stage) -> Self {
        Self {
            name: stage.as_str().to_string(),
            description: stage.description().to_string(),
            ordinal: stage.ordinal(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerProjectConfig {
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
    #[serde(default)]
    pub total_bits: Option<i64>,
    #[serde(default)]
    pub q_scale: Option<i64>,
    #[serde(default)]
    pub pipeline_latency: Option<i64>,
    #[serde(default)]
    pub max_positive: Option<f64>,
    #[serde(default)]
    pub min_negative: Option<f64>,
}

impl From<ProjectConfig> for SerProjectConfig {
    fn from(cfg: ProjectConfig) -> Self {
        Self {
            project_name: cfg.project_name,
            module_name: cfg.module_name,
            interface_type: cfg.interface_type,
            reference_project: cfg.reference_project,
            use_l0: cfg.use_l0,
            data_width: cfg.data_width,
            iterations: cfg.iterations,
            q_int_bits: cfg.q_int_bits,
            q_frac_bits: cfg.q_frac_bits,
            rounding_mode: cfg.rounding_mode,
            saturation: cfg.saturation,
            pipeline_stages: cfg.pipeline_stages,
            cycles_per_stage: cfg.cycles_per_stage,
            output_register: cfg.output_register,
            axis_data_width: cfg.axis_data_width,
            axis_has_tlast: cfg.axis_has_tlast,
            axis_has_tkeep: cfg.axis_has_tkeep,
            handshake_delay: cfg.handshake_delay,
            axi_lite_addr_width: cfg.axi_lite_addr_width,
            test_data_length: cfg.test_data_length,
            random_seed: cfg.random_seed,
            float_tolerance: cfg.float_tolerance,
            fixed_tolerance: cfg.fixed_tolerance,
            clock_frequency: cfg.clock_frequency,
            reset_sync_stages: cfg.reset_sync_stages,
            use_clock_enable: cfg.use_clock_enable,
            debug_mode: cfg.debug_mode,
            debug_level: cfg.debug_level,
            total_bits: cfg.total_bits,
            q_scale: cfg.q_scale,
            pipeline_latency: cfg.pipeline_latency,
            max_positive: cfg.max_positive,
            min_negative: cfg.min_negative,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SerGitStatus {
    pub branch: String,
    pub modified_count: usize,
    pub staged_count: usize,
    pub untracked_count: usize,
    pub conflict_count: usize,
    pub is_clean: bool,
    pub changed_files: Vec<String>,
}

impl From<GitStatus> for SerGitStatus {
    fn from(status: GitStatus) -> Self {
        Self {
            branch: status.branch,
            modified_count: status.modified_count,
            staged_count: status.staged_count,
            untracked_count: status.untracked_count,
            conflict_count: status.conflict_count,
            is_clean: status.is_clean,
            changed_files: status.changed_files,
        }
    }
}

impl Default for SerGitStatus {
    fn default() -> Self {
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
}

#[derive(Debug, Clone, Serialize)]
pub struct SerProjectFile {
    pub relative_path: String,
    pub source_group: String,
    pub content_preview: String,
    pub mtime_ms: u64,
    pub origin: String,
}

impl From<ProjectFile> for SerProjectFile {
    fn from(f: ProjectFile) -> Self {
        let content_preview = if f.content.chars().count() > 200 {
            let truncated: String = f.content.chars().take(200).collect();
            format!("{}...", truncated)
        } else {
            f.content.clone()
        };
        let source_group = match f.source_group.as_str() {
            "design" | "specs" => "docs".to_string(),
            other => other.to_string(),
        };
        Self {
            relative_path: f.relative_path,
            source_group,
            content_preview,
            mtime_ms: f.mtime_ms,
            origin: f.origin,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SerTurn {
    pub role: String,
    pub text: String,
    pub tools: Vec<String>,
    pub timestamp: Option<u64>,
}

impl From<SessionTurn> for SerTurn {
    fn from(turn: SessionTurn) -> Self {
        Self {
            role: turn.role,
            text: turn.text,
            tools: turn.tools,
            timestamp: turn.timestamp,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SerTranscript {
    pub session_id: String,
    pub initial_prompt: String,
    pub custom_title: Option<String>,
    pub model: Option<String>,
    pub turns: Vec<SerTurn>,
    pub modified_files: Vec<String>,
    pub created_at: u64,
}

impl From<SessionTranscript> for SerTranscript {
    fn from(t: SessionTranscript) -> Self {
        Self {
            session_id: t.session_id,
            initial_prompt: t.initial_prompt,
            custom_title: t.custom_title,
            model: t.model,
            turns: t.turns.into_iter().map(SerTurn::from).collect(),
            modified_files: t.modified_files,
            created_at: t.created_at,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SerSessionSummary {
    pub session_id: String,
    pub initial_prompt: String,
    pub custom_title: Option<String>,
    pub model: Option<String>,
    pub turn_count: usize,
    pub modified_files: Vec<String>,
    pub created_at: u64,
}

impl From<SessionSummary> for SerSessionSummary {
    fn from(s: SessionSummary) -> Self {
        Self {
            session_id: s.session_id,
            initial_prompt: s.initial_prompt,
            custom_title: s.custom_title,
            model: s.model,
            turn_count: s.turn_count,
            modified_files: s.modified_files,
            created_at: s.created_at,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct SerCandidateMemory {
    pub content: String,
    pub source_session_id: String,
    pub source_turn_index: usize,
    pub source_snippet: String,
    pub category: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TemplateDataPayload {
    pub project_path: String,
    pub stage: Option<SerStage>,
    pub stage_error: Option<String>,
    pub config: Option<SerProjectConfig>,
    pub config_error: Option<String>,
    pub git: SerGitStatus,
    pub git_error: Option<String>,
    pub layout: String,
    pub timestamp_ms: u64,
}

impl TemplateDataPayload {
    pub fn from_data(project_path: String, data: TemplateData) -> Self {
        let timestamp_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let (stage, stage_error) = match data.stage {
            Ok(s) => (Some(SerStage::from(s)), None),
            Err(e) => (None, Some(e.to_string())),
        };

        let (config, config_error) = match data.config {
            Ok(c) => (Some(SerProjectConfig::from(c)), None),
            Err(e) => (None, Some(e.to_string())),
        };

        let (git, git_error) = match data.git {
            Ok(g) => (SerGitStatus::from(g), None),
            Err(e) => (SerGitStatus::default(), Some(e.to_string())),
        };

        let layout = match data.layout {
            SourceLayout::Flat => "flat".to_string(),
            SourceLayout::Namespaced(name) => format!("namespaced:{}", name),
            SourceLayout::Unknown => "unknown".to_string(),
        };

        Self {
            project_path,
            stage,
            stage_error,
            config,
            config_error,
            git,
            git_error,
            layout,
            timestamp_ms,
        }
    }
}
