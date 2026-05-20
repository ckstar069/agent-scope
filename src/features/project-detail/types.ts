import type { ComponentType } from "react";
import type { ProjectEntry } from "@/lib/types";

export type { ProjectEntry };

export interface ProjectDetailProps {
  projectPath?: string;
  onSelectProject?: (projectPath: string) => void;
}

export interface SerStage {
  name: string;
  description: string;
  ordinal: number;
}

export interface SerProjectConfig {
  project_name: string;
  module_name: string;
  interface_type: string;
  reference_project?: string;
  use_l0?: boolean;
  data_width?: number;
  iterations?: number;
  q_int_bits?: number;
  q_frac_bits?: number;
  rounding_mode?: string;
  saturation?: boolean;
  pipeline_stages?: number;
  cycles_per_stage?: number;
  output_register?: boolean;
  axis_data_width?: number;
  axis_has_tlast?: boolean;
  axis_has_tkeep?: boolean;
  handshake_delay?: number;
  axi_lite_addr_width?: number;
  test_data_length?: number;
  random_seed?: number;
  float_tolerance?: number;
  fixed_tolerance?: number;
  clock_frequency?: number;
  reset_sync_stages?: number;
  use_clock_enable?: boolean;
  debug_mode?: boolean;
  debug_level?: number;
  total_bits?: number | null;
  q_scale?: number | null;
  pipeline_latency?: number | null;
  max_positive?: number | null;
  min_negative?: number | null;
}

export interface SerGitStatus {
  branch: string;
  modified_count: number;
  staged_count: number;
  untracked_count: number;
  conflict_count: number;
  is_clean: boolean;
  changed_files: string[];
}

export interface TemplateDataPayload {
  project_path: string;
  stage: SerStage | null;
  stage_error: string | null;
  config: SerProjectConfig | null;
  config_error: string | null;
  git: SerGitStatus;
  git_error: string | null;
  layout: string;
  timestamp_ms: number;
}

export type ConfigValue = string | number | boolean | null | undefined;
export type DetailIcon = ComponentType<{ className?: string; "aria-hidden"?: boolean | "true" | "false" }>;

export type PanelAccent = "blue" | "green" | "amber" | "purple";
