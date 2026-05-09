import type { ComponentType, ReactNode } from "react";
import { useEffect, useMemo, useRef, useState } from "react";
import {
  AlertCircle,
  ArrowDown,
  ArrowLeft,
  ArrowRight,
  BookOpen,
  CheckCircle2,
  ChevronDown,
  Circle,
  Clock,
  Cpu,
  FileText,
  FolderKanban,
  GitBranch,
  GitCompare,
  HardDrive,
  Layers3,
  Loader2,
  SlidersHorizontal,
} from "lucide-react";

import { Button } from "@/components/ui/button";
import { Collapsible, CollapsibleContent } from "@/components/ui/collapsible";
import { ProjectMemoryPanel } from "@/components/ProjectMemoryPanel";
import { ScrollArea } from "@/components/ui/scroll-area";
import { useTauri } from "@/hooks/useTauri";
import { cn } from "@/lib/utils";

interface ProjectDetailProps {
  projectPath?: string;
  onSelectProject?: (projectPath: string) => void;
  onBack?: () => void;
}

interface SerStage {
  name: string;
  description: string;
  ordinal: number;
}

interface SerProjectConfig {
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

interface SerGitStatus {
  branch: string;
  modified_count: number;
  staged_count: number;
  untracked_count: number;
  conflict_count: number;
  is_clean: boolean;
  changed_files: string[];
}

interface TemplateDataPayload {
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

type ConfigValue = string | number | boolean | null | undefined;
type DetailIcon = ComponentType<{ className?: string; "aria-hidden"?: boolean | "true" | "false" }>;

const stageSteps = [
  { name: "L0", label: "L0", description: "需求 / 基线" },
  { name: "L1", label: "L1", description: "原型" },
  { name: "L2", label: "L2", description: "模型" },
  { name: "L3", label: "L3", description: "验证" },
  { name: "L4", label: "L4", description: "定点" },
  { name: "L5", label: "L5", description: "接口" },
  { name: "L6", label: "L6", description: "冻结" },
  { name: "Verilog", label: "Verilog", description: "RTL" },
  { name: "Synthesis", label: "Synthesis", description: "综合" },
  { name: "Hardware", label: "Hardware", description: "硬件" },
] as const;

const summaryConfigFields: Array<{ key: keyof SerProjectConfig; label: string; icon: DetailIcon }> = [
  { key: "project_name", label: "Project", icon: FolderKanban },
  { key: "reference_project", label: "Reference", icon: Layers3 },
  { key: "data_width", label: "Data width", icon: Cpu },
  { key: "iterations", label: "Iterations", icon: SlidersHorizontal },
];

const configSections: Array<{
  title: string;
  fields: Array<{ key: keyof SerProjectConfig; label: string }>;
}> = [
  {
    title: "项目骨架",
    fields: [
      { key: "project_name", label: "项目名" },
      { key: "module_name", label: "模块名" },
      { key: "interface_type", label: "接口类型" },
      { key: "reference_project", label: "参考项目" },
      { key: "use_l0", label: "启用 L0" },
    ],
  },
  {
    title: "数值参数",
    fields: [
      { key: "data_width", label: "数据位宽" },
      { key: "iterations", label: "迭代次数" },
      { key: "q_int_bits", label: "整数位" },
      { key: "q_frac_bits", label: "小数位" },
      { key: "rounding_mode", label: "舍入模式" },
      { key: "saturation", label: "饱和处理" },
      { key: "total_bits", label: "总位数" },
      { key: "q_scale", label: "Q Scale" },
      { key: "max_positive", label: "最大正值" },
      { key: "min_negative", label: "最小负值" },
    ],
  },
  {
    title: "流水线 / 总线",
    fields: [
      { key: "pipeline_stages", label: "流水级数" },
      { key: "cycles_per_stage", label: "单级周期" },
      { key: "pipeline_latency", label: "流水延迟" },
      { key: "output_register", label: "输出寄存" },
      { key: "axis_data_width", label: "AXIS 位宽" },
      { key: "axis_has_tlast", label: "TLAST" },
      { key: "axis_has_tkeep", label: "TKEEP" },
      { key: "handshake_delay", label: "握手延迟" },
      { key: "axi_lite_addr_width", label: "AXI Lite 地址位宽" },
    ],
  },
  {
    title: "测试 / 调试",
    fields: [
      { key: "test_data_length", label: "测试数据长度" },
      { key: "random_seed", label: "随机种子" },
      { key: "float_tolerance", label: "浮点容差" },
      { key: "fixed_tolerance", label: "定点容差" },
      { key: "clock_frequency", label: "时钟频率" },
      { key: "reset_sync_stages", label: "复位同步级" },
      { key: "use_clock_enable", label: "时钟使能" },
      { key: "debug_mode", label: "调试模式" },
      { key: "debug_level", label: "调试级别" },
    ],
  },
];

function normalizePath(path: string) {
  return path.replace(/\/+$/, "");
}

function isSameProjectPath(left: string, right: string) {
  return normalizePath(left) === normalizePath(right);
}

function formatValue(value: ConfigValue) {
  if (value === null || value === undefined || value === "") {
    return "--";
  }
  if (typeof value === "boolean") {
    return value ? "是" : "否";
  }
  return String(value);
}

function formatTimestamp(timestampMs?: number) {
  if (timestampMs == null) {
    return "--";
  }
  return new Intl.DateTimeFormat("zh-CN", {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  }).format(new Date(timestampMs));
}

function describeTauriError(error: unknown) {
  const message = error instanceof Error ? error.message : String(error);
  if (message.includes("项目路径不存在") || message.includes("路径不存在")) {
    return "项目路径不存在或已被删除";
  }
  if (message.includes("Permission denied") || message.includes("权限") || message.includes("无权访问")) {
    return "无权访问该项目路径";
  }
  return message;
}

function classifyConfigError(message: string) {
  if (message.includes("语法错误")) {
    return { title: "参数文件语法错误", tone: "destructive" as const };
  }
  if (message.includes("运行时出错")) {
    return { title: "参数文件运行时错误", tone: "warning" as const };
  }
  if (message.includes("未安装 Python3")) {
    return { title: "缺少 Python3", tone: "warning" as const };
  }
  if (message.includes("未找到") || message.includes("不存在")) {
    return { title: "缺少参数文件", tone: "muted" as const };
  }
  if (message.includes("解析失败")) {
    return { title: "参数解析失败", tone: "warning" as const };
  }
  return { title: "参数读取失败", tone: "destructive" as const };
}

function classifyStageError(message: string) {
  if (message.includes("未找到")) {
    return { title: "阶段文件缺失", tone: "muted" as const };
  }
  if (message.includes("内容为空")) {
    return { title: "阶段文件为空", tone: "warning" as const };
  }
  if (message.includes("无法识别")) {
    return { title: "阶段值无法识别", tone: "warning" as const };
  }
  return { title: "阶段读取失败", tone: "destructive" as const };
}

function classifyGitError(message: string) {
  if (message.includes("不是 Git 仓库")) {
    return { title: "非 Git 仓库", tone: "muted" as const };
  }
  if (message.includes("未安装 Git")) {
    return { title: "缺少 Git", tone: "warning" as const };
  }
  return { title: "Git 状态读取失败", tone: "destructive" as const };
}

export function ProjectDetail({ projectPath = "", onSelectProject, onBack }: ProjectDetailProps) {
  const { invoke, listen } = useTauri();
  const [data, setData] = useState<TemplateDataPayload | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!projectPath) {
      setData(null);
      setError(null);
      setIsLoading(false);
      return;
    }

    let isActive = true;
    let disposeListener: (() => void) | undefined;

    async function refreshProjectData() {
      setIsLoading(true);
      setError(null);

      try {
        const payload = await invoke<TemplateDataPayload, { path: string }>("get_project_data", { path: projectPath });
        if (isActive) {
          setData(payload);
        }
      } catch (requestError) {
        if (isActive) {
          setData(null);
          setError(describeTauriError(requestError));
        }
      } finally {
        if (isActive) {
          setIsLoading(false);
        }
      }
    }

    async function setupRealtimeUpdates() {
      try {
        const unlisten = await listen<TemplateDataPayload>("template-update", (event) => {
          if (isSameProjectPath(event.payload.project_path, projectPath)) {
            setData(event.payload);
            setError(null);
            setIsLoading(false);
          }
        });

        if (!isActive) {
          unlisten();
          return;
        }

        disposeListener = unlisten;

        try {
          await invoke<void, { path: string }>("start_watching", { path: projectPath });
        } catch (watchError) {
          if (isActive) {
            console.warn("项目实时监听启动失败", watchError);
          }
        }
      } catch (listenError) {
        if (isActive) {
          console.warn("template-update 事件监听注册失败", listenError);
        }
      }
    }

    void refreshProjectData();
    void setupRealtimeUpdates();

    return () => {
      isActive = false;
      disposeListener?.();
      void invoke<void, { path: string }>("stop_watching", { path: projectPath }).catch(() => undefined);
    };
  }, [invoke, listen, projectPath]);

  const currentStageIndex = useMemo(() => {
    if (!data?.stage) {
      return -1;
    }
    return stageSteps.findIndex((stage) => stage.name.toLowerCase() === data.stage?.name.toLowerCase());
  }, [data?.stage]);

  const config = data?.config ?? null;
  const git = data?.git ?? null;
  const hasProjectPath = projectPath.trim().length > 0;

  if (!hasProjectPath) {
    return <EmptyProjectState onSelectProject={onSelectProject} />;
  }

  return (
    <section className="space-y-6">
      <div className="flex flex-col gap-4 lg:flex-row lg:items-end lg:justify-between">
        <div className="space-y-2">
          <div className="flex items-center gap-2">
            {onBack && (
              <Button type="button" variant="ghost" size="sm" className="h-8 px-2 text-foreground/70" onClick={onBack}>
                <ArrowLeft className="size-4" aria-hidden="true" />
                返回仪表盘
              </Button>
            )}
          </div>
          <h1 className="text-3xl font-semibold tracking-tight">项目详情</h1>
          <p className="max-w-3xl break-all text-sm text-foreground/70">{projectPath}</p>
        </div>
        <div className="flex items-center gap-2 rounded-lg border border-border bg-card px-3 py-2 text-sm text-foreground/70 shadow-sm">
          {isLoading ? <Loader2 className="size-4 animate-spin" aria-hidden="true" /> : <Clock className="size-4" aria-hidden="true" />}
          <span>更新：{isLoading ? "采集中" : formatTimestamp(data?.timestamp_ms)}</span>
        </div>
      </div>

      {error && <ErrorBanner message={error} />}

      {(!error || data) && (
        <>
          <div className="flex flex-col gap-4 lg:flex-row">
            <div className="flex-1">
              <Panel title="Stage 时间线" icon={Layers3} subtitle="L0 → L6 → Verilog → Synthesis → Hardware" accent="blue">
                <StageTimeline currentStageIndex={currentStageIndex} stageError={data?.stage_error ?? null} />
              </Panel>
            </div>

            <div className="hidden lg:block w-[2px] shrink-0 bg-border/80 self-stretch" />

            <div className="lg:w-[38%]">
              <Panel title="Git" icon={GitBranch} subtitle="工作区状态快照">
                <GitPanel git={git} gitError={data?.git_error ?? null} />
              </Panel>
            </div>
          </div>

              <Panel title="参数快照" icon={SlidersHorizontal} subtitle="config/parameters.py 解析结果" accent="green">
            <ConfigSnapshot config={config} configError={data?.config_error ?? null} />
          </Panel>

          <Panel title="项目记忆" icon={BookOpen} subtitle="CLAUDE.md、规则、笔记、设计文档" accent="purple">
            <ProjectMemoryPanel projectPath={projectPath} />
          </Panel>
        </>
      )}
    </section>
  );
}

interface ProjectEntry {
  path: string;
  added_at: number;
}

function EmptyProjectState({ onSelectProject }: { onSelectProject?: (projectPath: string) => void }) {
  const { invoke } = useTauri();
  const [projects, setProjects] = useState<ProjectEntry[]>([]);
  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    let isActive = true;

    async function loadProjects() {
      setIsLoading(true);
      try {
        const entries = await invoke<ProjectEntry[]>("list_projects");
        if (isActive) {
          setProjects(entries);
        }
      } catch {
        if (isActive) {
          setProjects([]);
        }
      } finally {
        if (isActive) {
          setIsLoading(false);
        }
      }
    }

    void loadProjects();

    return () => {
      isActive = false;
    };
  }, [invoke]);

  return (
    <section className="space-y-6">
      <div className="space-y-2">
        <p className="text-sm font-medium text-foreground/70">Projects</p>
        <h1 className="text-3xl font-semibold tracking-tight">项目详情</h1>
        <p className="max-w-2xl text-sm text-foreground/70">
          {projects.length > 0
            ? "选择一个已注册的项目查看 Stage、参数、Memory 与 Git 快照。"
            : "请先在设置中添加监控项目，或从仪表盘选择一个项目。"}
        </p>
      </div>

      {isLoading ? (
        <div className="flex min-h-72 items-center justify-center rounded-lg border border-dashed border-border bg-card text-card-foreground">
          <Loader2 className="size-6 animate-spin text-foreground/60" aria-hidden="true" />
        </div>
      ) : projects.length === 0 ? (
        <div className="flex min-h-72 items-center justify-center rounded-lg border border-dashed border-border bg-card text-card-foreground">
          <div className="text-center">
            <FolderKanban className="mx-auto mb-3 size-8 text-foreground/60" aria-hidden="true" />
            <p className="text-sm text-foreground/70">暂无已注册项目</p>
          </div>
        </div>
      ) : (
        <div className="grid gap-4" style={{ gridTemplateColumns: "repeat(auto-fit, minmax(min(100%, 280px), 1fr))" }}>
          {projects.map((project) => (
            <button
              key={project.path}
              type="button"
              onClick={() => onSelectProject?.(project.path)}
              className="flex flex-col gap-2 rounded-lg border border-border/60 bg-card p-4 text-left transition-all hover:border-primary/40 hover:shadow-sm"
            >
              <div className="flex items-center gap-3">
                <div className="flex size-10 shrink-0 items-center justify-center rounded-lg bg-muted text-foreground/60">
                  <FolderKanban className="size-5" aria-hidden="true" />
                </div>
                <div className="min-w-0">
                  <p className="truncate font-mono text-sm font-medium text-foreground">{project.path}</p>
                </div>
              </div>
            </button>
          ))}
        </div>
      )}
    </section>
  );
}

function ErrorBanner({ message }: { message: string }) {
  return (
    <div className="flex items-start gap-3 rounded-xl border border-destructive/40 bg-destructive/10 p-4 text-sm text-destructive">
      <AlertCircle className="mt-0.5 size-4 shrink-0" aria-hidden="true" />
      <div>
        <p className="font-medium">无法加载项目数据</p>
        <p className="mt-1 text-destructive/80">{message}</p>
      </div>
    </div>
  );
}

type PanelAccent = "blue" | "green" | "amber" | "purple";

const ACCENT_TOP_STYLES: Record<PanelAccent, string> = {
  blue: "from-blue-500/40 via-blue-400/20 to-transparent",
  green: "from-green-500/40 via-green-400/20 to-transparent",
  amber: "from-amber-500/40 via-amber-400/20 to-transparent",
  purple: "from-purple-500/40 via-purple-400/20 to-transparent",
};

function Panel({
  children,
  icon: Icon,
  subtitle,
  title,
  accent = "blue",
}: {
  children: ReactNode;
  icon: DetailIcon;
  subtitle: string;
  title: string;
  accent?: PanelAccent;
}) {
  const topGradient = ACCENT_TOP_STYLES[accent];
  const [open, setOpen] = useState(true);

  return (
    <Collapsible open={open} onOpenChange={setOpen}>
      <article className="overflow-hidden rounded-xl border border-border/60 bg-card text-card-foreground">
        <div className={cn("h-[2px] bg-gradient-to-r", topGradient)} />
        <button
          type="button"
          className="flex w-full cursor-pointer items-start justify-between gap-4 p-4 pb-3 text-left transition-colors hover:bg-muted/30"
          onClick={() => setOpen((prev) => !prev)}
        >
          <div className="min-w-0 flex-1">
            <div className="flex items-center gap-2">
              <div className="flex size-8 shrink-0 items-center justify-center rounded-lg bg-muted text-foreground/70">
                <Icon className="size-4" aria-hidden="true" />
              </div>
              <h2 className="text-base font-semibold tracking-tight">{title}</h2>
            </div>
            <p className="mt-1.5 text-sm text-foreground/70">{subtitle}</p>
          </div>
          <div className="mt-1 shrink-0 text-foreground/50 transition-transform duration-200">
            <ChevronDown className={cn("size-5 transition-transform duration-200", open && "rotate-180")} aria-hidden="true" />
          </div>
        </button>
        <CollapsibleContent>
          <div className="border-t border-border/60 px-4 pb-4">
            {children}
          </div>
        </CollapsibleContent>
      </article>
    </Collapsible>
  );
}

function StageTimeline({ currentStageIndex, stageError }: { currentStageIndex: number; stageError: string | null }) {
  const containerRef = useRef<HTMLDivElement | null>(null);
  const [itemsPerRow, setItemsPerRow] = useState(5);

  useEffect(() => {
    const container = containerRef.current;

    if (!container) {
      return;
    }

    const updateItemsPerRow = () => {
      const rootFontSize = parseFloat(getComputedStyle(document.documentElement).fontSize);
      const itemMinWidth = rootFontSize * 8.5;
      const nextItemsPerRow = Math.max(1, Math.min(5, Math.floor(container.clientWidth / itemMinWidth)));
      setItemsPerRow(nextItemsPerRow);
    };

    updateItemsPerRow();

    const resizeObserver = new ResizeObserver(updateItemsPerRow);
    resizeObserver.observe(container);

    return () => resizeObserver.disconnect();
  }, []);

  if (stageError) {
    const error = classifyStageError(stageError);
    return <ErrorNotice message={stageError} title={error.title} tone={error.tone} />;
  }

  const rows: Array<Array<{ stage: (typeof stageSteps)[number]; index: number }>> = [];

  for (let index = 0; index < stageSteps.length; index += itemsPerRow) {
    rows.push(stageSteps.slice(index, index + itemsPerRow).map((stage, rowOffset) => ({ stage, index: index + rowOffset })));
  }

  return (
    <div ref={containerRef} className="space-y-5 pb-2">
      {rows.map((row, rowIndex) => {
        const isReversedRow = rowIndex % 2 === 1;
        const displayRow = isReversedRow ? [...row].reverse() : row;

        return (
          <ol key={row[0]?.stage.name ?? rowIndex} className="flex items-start justify-between gap-2">
            {displayRow.map(({ stage, index }, displayIndex) => {
              const isDone = currentStageIndex > index;
              const isCurrent = currentStageIndex === index;
              const Icon = isDone ? CheckCircle2 : isCurrent ? Clock : Circle;
              const nextVisualItem = displayRow[displayIndex + 1];
              const hasNextInRow = nextVisualItem !== undefined;
              const isLogicalRowEnd = index === row[row.length - 1].index;
              const hasNextRow = rowIndex < rows.length - 1;
              const connectorDone = nextVisualItem ? currentStageIndex > Math.min(index, nextVisualItem.index) : isDone;

              return (
                <li key={stage.name} className="flex min-w-0 flex-1 items-start last:flex-none">
                  <div className="flex w-24 shrink-0 flex-col items-center text-center">
                    <div
                      className={cn(
                        "flex size-10 items-center justify-center rounded-full border transition-colors",
                        isCurrent && "border-primary bg-primary text-primary-foreground",
                        isDone && "border-primary/50 bg-primary/10 text-primary",
                        !isCurrent && !isDone && "border-border bg-muted text-foreground/60",
                      )}
                    >
                      <Icon className="size-4" aria-hidden="true" />
                    </div>
                    <p className={cn("mt-3 text-sm font-medium", isCurrent ? "text-foreground" : "text-foreground/70")}>{stage.label}</p>
                    <p className="mt-1 text-xs text-foreground/60">{stage.description}</p>
                    <p className="mt-2 rounded-md bg-muted px-2 py-1 text-xs text-foreground/60">耗时 --</p>
                    {isLogicalRowEnd && hasNextRow && (
                      <div className="mt-3 flex flex-col items-center" aria-hidden="true">
                        <div className={cn("h-4 w-px", isDone ? "bg-primary/60" : "bg-border")} />
                        <ArrowDown className={cn("-mt-1 size-3", isDone ? "text-primary/60" : "text-border")} />
                      </div>
                    )}
                  </div>
                  {hasNextInRow && (
                    <div className="mt-5 flex min-w-4 flex-1 items-center px-1" aria-hidden="true">
                      <div className={cn("h-px flex-1", connectorDone ? "bg-primary/60" : "bg-border")} />
                      <ArrowRight
                        className={cn(
                          "-ml-1 size-3 shrink-0",
                          isReversedRow && "rotate-180",
                          connectorDone ? "text-primary/60" : "text-border",
                        )}
                      />
                    </div>
                  )}
                </li>
              );
            })}
          </ol>
        );
      })}
    </div>
  );
}

function ConfigSnapshot({ config, configError }: { config: SerProjectConfig | null; configError: string | null }) {
  if (configError) {
    const error = classifyConfigError(configError);
    return <ErrorNotice message={configError} title={error.title} tone={error.tone} />;
  }

  if (!config) {
    return <InlineNotice message="未读取到 config/parameters.py 参数。" />;
  }

  return (
    <div className="space-y-4">
      <div
        className="grid gap-3"
        style={{ gridTemplateColumns: "repeat(auto-fit, minmax(min(100%, 200px), 1fr))" }}
      >
        {summaryConfigFields.map((field) => {
          const Icon = field.icon;
          return (
            <div key={field.key} className="rounded-lg border border-border/50 bg-muted/30 p-3">
              <div className="mb-2 flex items-center justify-between gap-3">
                <p className="text-xs font-medium uppercase tracking-wide text-foreground/60">{field.label}</p>
                <Icon className="size-4 text-foreground/60" aria-hidden="true" />
              </div>
              <p className="truncate text-base font-semibold">{formatValue(config[field.key])}</p>
            </div>
          );
        })}
      </div>

      <div
        className="grid gap-4"
        style={{ gridTemplateColumns: "repeat(auto-fit, minmax(min(100%, 360px), 1fr))" }}
      >
        {configSections.map((section) => (
          <div key={section.title} className="rounded-lg border border-border/50">
            <div className="border-b border-border/60 bg-muted/20 px-3 py-2.5">
              <h3 className="text-sm font-semibold">{section.title}</h3>
            </div>
            <dl className="divide-y divide-border/60">
              {section.fields.map((field) => (
                <div key={field.key} className="grid grid-cols-[minmax(0,0.8fr)_minmax(0,1fr)] gap-3 px-3 py-2.5 text-sm">
                  <dt className="text-foreground/70">{field.label}</dt>
                  <dd className="min-w-0 truncate text-right font-medium">{formatValue(config[field.key])}</dd>
                </div>
              ))}
            </dl>
          </div>
        ))}
      </div>
    </div>
  );
}

function GitPanel({ git, gitError }: { git: SerGitStatus | null; gitError: string | null }) {
  if (gitError) {
    const error = classifyGitError(gitError);
    return <ErrorNotice message={gitError} title={error.title} tone={error.tone} />;
  }

  if (!git) {
    return <InlineNotice message="未读取到 Git 状态。" />;
  }

  const statusItems = [
    { label: "修改", value: git.modified_count, icon: GitCompare, tone: "warning" as const },
    { label: "暂存", value: git.staged_count, icon: CheckCircle2, tone: "success" as const },
    { label: "未追踪", value: git.untracked_count, icon: FileText, tone: "info" as const },
    { label: "冲突", value: git.conflict_count, icon: AlertCircle, tone: "error" as const },
  ];

  return (
    <div className="space-y-3">
      <div className="rounded-lg border border-border/50 bg-muted/30 p-3">
        <div className="mb-1.5 flex items-center gap-2 text-sm text-foreground/70">
          <GitBranch className="size-4" aria-hidden="true" />
          <span>当前分支</span>
        </div>
        <p className="truncate text-lg font-semibold">{git.branch || "非 Git 仓库"}</p>
      </div>

      <div
        className="grid gap-3"
        style={{ gridTemplateColumns: "repeat(auto-fit, minmax(min(100%, 100px), 1fr))" }}
      >
        {statusItems.map((item) => {
          const Icon = item.icon;
          const toneStyles = {
            success: "border-[var(--status-success-border)] bg-[var(--status-success-bg)] text-[var(--status-success)]",
            warning: "border-[var(--status-warning-border)] bg-[var(--status-warning-bg)] text-[var(--status-warning)]",
            error: "border-[var(--status-error-border)] bg-[var(--status-error-bg)] text-[var(--status-error)]",
            info: "border-[var(--status-info-border)] bg-[var(--status-info-bg)] text-[var(--status-info)]",
          };
          return (
            <div key={item.label} className={cn("rounded-lg border p-2.5", toneStyles[item.tone])}>
              <div className="mb-1.5 flex items-center justify-between gap-2 text-xs opacity-80">
                <span>{item.label}</span>
                <Icon className="size-3" aria-hidden="true" />
              </div>
              <p className="text-xl font-semibold">{item.value}</p>
            </div>
          );
        })}
      </div>

      <div
        className={cn(
          "flex items-center gap-2 rounded-lg border p-2.5 text-sm",
          git.is_clean
            ? "border-[var(--status-success-border)] bg-[var(--status-success-bg)] text-[var(--status-success)]"
            : "border-[var(--status-warning-border)] bg-[var(--status-warning-bg)] text-[var(--status-warning)]",
        )}
      >
        <HardDrive className="size-4" aria-hidden="true" />
        <span>{git.is_clean ? "工作区干净" : "存在未提交变化"}</span>
      </div>

      {!git.is_clean && git.changed_files.length > 0 && (
        <div className="space-y-2">
          <p className="text-xs font-medium text-foreground/70">变更文件</p>
          <ScrollArea className="h-40 rounded-lg border border-border/50">
            <div className="divide-y divide-border/60">
              {git.changed_files.map((file) => (
                <div key={file} className="flex items-center gap-2 px-3 py-2 text-xs">
                  <GitCompare className="size-3 text-foreground/60" />
                  <span className="truncate font-mono">{file}</span>
                </div>
              ))}
            </div>
          </ScrollArea>
        </div>
      )}
    </div>
  );
}

function InlineNotice({ message }: { message: string }) {
  return (
    <div className="flex items-center gap-2 rounded-lg border border-dashed border-border bg-muted/30 p-3 text-sm text-foreground/70">
      <AlertCircle className="size-4 shrink-0" aria-hidden="true" />
      <span>{message}</span>
    </div>
  );
}

function ErrorNotice({ message, title, tone }: { message: string; title: string; tone: "destructive" | "warning" | "muted" }) {
  const className = cn(
    "flex items-start gap-3 rounded-lg border p-3 text-sm",
    tone === "destructive" && "border-[var(--status-error-border)] bg-[var(--status-error-bg)] text-[var(--status-error)]",
    tone === "warning" && "border-[var(--status-warning-border)] bg-[var(--status-warning-bg)] text-[var(--status-warning)]",
    tone === "muted" && "border-border bg-muted/30 text-foreground/70",
  );

  return (
    <div className={className}>
      <AlertCircle className="mt-0.5 size-4 shrink-0" aria-hidden="true" />
      <div className="min-w-0 space-y-1">
        <p className="font-medium">{title}</p>
        <p className="whitespace-pre-wrap text-xs opacity-90">{message}</p>
      </div>
    </div>
  );
}
