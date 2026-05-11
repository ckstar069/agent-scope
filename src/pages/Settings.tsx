import { useCallback, useEffect, useMemo, useState, type ComponentProps } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { AlertTriangle, CheckCircle2, FolderCog, FolderOpen, FolderPlus, Loader2, Monitor, Moon, RefreshCw, Sun, Trash2, Type } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { useFontSize, type FontSize } from "@/hooks/useFontSize";
import { useTauri } from "@/hooks/useTauri";
import { useTheme, type Theme } from "@/hooks/useTheme";
import { cn } from "@/lib/utils";

interface ProjectEntry {
  path: string;
  added_at: number;
}

const collator = new Intl.Collator("zh-CN", { numeric: true, sensitivity: "base" });
type FormSubmitEvent = Parameters<NonNullable<ComponentProps<"form">["onSubmit"]>>[0];

const fontSizeOptions: { value: FontSize; label: string; description: string }[] = [
  { value: "compact", label: "紧凑", description: "13px — 适合小屏 / 更多信息密度" },
  { value: "normal", label: "标准", description: "15px — 平衡阅读与信息密度" },
  { value: "large", label: "大字号", description: "17px — 大屏高分辨率首选" },
  { value: "xlarge", label: "超大字号", description: "19px — 远距离 / 视力辅助" },
];

export function Settings() {
  const { invoke } = useTauri();
  const { fontSize, setFontSize } = useFontSize();
  const [projects, setProjects] = useState<ProjectEntry[]>([]);
  const [projectPath, setProjectPath] = useState("");
  const [isLoading, setIsLoading] = useState(true);
  const [isAdding, setIsAdding] = useState(false);
  const [removingPath, setRemovingPath] = useState<string | null>(null);
  const [pendingRemoval, setPendingRemoval] = useState<ProjectEntry | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);
  const [templatePath, setTemplatePath] = useState("");
  const [isSaving, setIsSaving] = useState(false);
  const [templateError, setTemplateError] = useState<string | null>(null);
  const [templateSuccess, setTemplateSuccess] = useState<string | null>(null);

  const sortedProjects = useMemo(() => [...projects].sort((a, b) => collator.compare(a.path, b.path)), [projects]);

  const loadProjects = useCallback(async () => {
    setIsLoading(true);
    setError(null);

    try {
      const entries = await invoke<ProjectEntry[]>("list_projects");
      setProjects(entries);
    } catch (err) {
      setError(`项目列表加载失败：${normalizeCommandError(err)}`);
    } finally {
      setIsLoading(false);
    }
  }, [invoke]);

  useEffect(() => {
    loadProjects();
  }, [loadProjects]);

  useEffect(() => {
    invoke<string>("get_template_path")
      .then((path) => setTemplatePath(path))
      .catch((err) => setTemplateError(`加载模板路径失败：${normalizeCommandError(err)}`));
  }, [invoke]);

  async function handleAddProject(event: FormSubmitEvent) {
    event.preventDefault();

    const validationError = validateProjectPath(projectPath);
    if (validationError) {
      setSuccess(null);
      setError(validationError);
      return;
    }

    const trimmedPath = projectPath.trim();
    setIsAdding(true);
    setError(null);
    setSuccess(null);

    try {
      const entry = await invoke<ProjectEntry, { path: string }>("add_project", { path: trimmedPath });
      setProjectPath("");
      setSuccess(`已添加项目：${entry.path}`);
      await loadProjects();
    } catch (err) {
      setError(getAddProjectMessage(err));
    } finally {
      setIsAdding(false);
    }
  }

  async function handleRemoveProject() {
    if (!pendingRemoval) {
      return;
    }

    setRemovingPath(pendingRemoval.path);
    setError(null);
    setSuccess(null);

    try {
      await invoke<void, { path: string }>("remove_project", { path: pendingRemoval.path });
      setSuccess(`已移除项目：${pendingRemoval.path}`);
      setPendingRemoval(null);
      await loadProjects();
    } catch (err) {
      setError(`项目移除失败：${normalizeCommandError(err)}`);
    } finally {
      setRemovingPath(null);
    }
  }

  async function handleBrowse() {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: "选择 FPGA 项目目录",
      });

      if (selected && typeof selected === "string") {
        setProjectPath(selected);
        setError(null);
        setSuccess(null);
      }
    } catch (err) {
      console.error("浏览目录失败:", err);
    }
  }

  async function handleTemplateBrowse() {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: "选择模板项目目录",
      });

      if (selected && typeof selected === "string") {
        setTemplatePath(selected);
        setTemplateError(null);
        setTemplateSuccess(null);
      }
    } catch (err) {
      console.error("浏览目录失败:", err);
    }
  }

  async function handleSaveTemplate() {
    setIsSaving(true);
    setTemplateError(null);
    setTemplateSuccess(null);

    try {
      await invoke<void, { path: string }>("set_template_path", { path: templatePath });
      setTemplateSuccess("已保存模板项目路径");
    } catch (err) {
      setTemplateError(`保存失败：${normalizeCommandError(err)}`);
    } finally {
      setIsSaving(false);
    }
  }

  return (
    <section className="space-y-6">
      <div className="flex flex-col gap-4 lg:flex-row lg:items-end lg:justify-between">
        <div className="space-y-2">
          <p className="text-sm font-medium text-muted-foreground">Settings</p>
          <h1 className="text-3xl font-semibold tracking-tight">设置</h1>
          <p className="max-w-2xl text-sm text-muted-foreground">
            管理 ptv 正在监控的 FPGA 项目目录。添加后，Dashboard 和 Agent 监控会自动读取这些注册路径。
          </p>
        </div>
        <div className="rounded-xl border border-border bg-muted/40 px-4 py-3 text-sm text-muted-foreground">
          已注册 <span className="font-semibold text-foreground">{projects.length}</span> 个项目
        </div>
      </div>

      <Card className="overflow-hidden">
        <div className="h-1 bg-gradient-to-r from-primary/80 via-muted-foreground/40 to-transparent" />
        <CardHeader>
          <div className="flex items-start gap-3">
            <div className="flex size-10 shrink-0 items-center justify-center rounded-lg bg-muted text-muted-foreground">
              <FolderPlus className="size-5" aria-hidden="true" />
            </div>
            <div className="space-y-1">
              <CardTitle>添加监控项目</CardTitle>
              <CardDescription>请输入绝对路径，例如 macOS/Linux: /Users/me/project-a 或 Windows: C:\Users\me\project-a。</CardDescription>
            </div>
          </div>
        </CardHeader>
        <CardContent>
          <form className="flex flex-col gap-3 lg:flex-row" onSubmit={handleAddProject}>
            <div className="flex min-w-0 flex-1 gap-2">
              <label htmlFor="project-path" className="sr-only">
                项目路径
              </label>
              <Input
                id="project-path"
                value={projectPath}
                placeholder="/Users/me/project-a 或 C:\\Users\\me\\project-a"
                aria-invalid={Boolean(error)}
                disabled={isAdding}
                onChange={(event) => {
                  setProjectPath(event.target.value);
                  if (error) {
                    setError(null);
                  }
                  if (success) {
                    setSuccess(null);
                  }
                }}
              />
              <Button type="button" variant="outline" disabled={isAdding} onClick={handleBrowse} title="浏览目录">
                <FolderOpen className="size-4" aria-hidden="true" />
              </Button>
            </div>
            <Button type="submit" className="lg:w-32" disabled={isAdding}>
              {isAdding ? <Loader2 className="size-4 animate-spin" aria-hidden="true" /> : <FolderPlus className="size-4" aria-hidden="true" />}
              添加项目
            </Button>
          </form>

          <div className="mt-4 space-y-2" aria-live="polite">
            {error && <Message tone="error" text={error} />}
            {success && <Message tone="success" text={success} />}
          </div>
        </CardContent>
      </Card>

      <Card className="overflow-hidden">
        <div className="h-1 bg-gradient-to-r from-primary/80 via-muted-foreground/40 to-transparent" />
        <CardHeader>
          <div className="flex items-start gap-3">
            <div className="flex size-10 shrink-0 items-center justify-center rounded-lg bg-muted text-muted-foreground">
              <FolderCog className="size-5" aria-hidden="true" />
            </div>
            <div className="space-y-1">
              <CardTitle>模板项目路径</CardTitle>
              <CardDescription>用于区分文件来源（模板 vs 项目特有）</CardDescription>
            </div>
          </div>
        </CardHeader>
        <CardContent>
          <div className="flex flex-col gap-3 lg:flex-row">
            <div className="flex min-w-0 flex-1 gap-2">
              <label htmlFor="template-path" className="sr-only">
                模板路径
              </label>
              <Input
                id="template-path"
                value={templatePath}
                placeholder="/Users/me/ai_project_template 或 C:\\Users\\me\\ai_project_template"
                aria-invalid={Boolean(templateError)}
                disabled={isSaving}
                onChange={(event) => {
                  setTemplatePath(event.target.value);
                  if (templateError) setTemplateError(null);
                  if (templateSuccess) setTemplateSuccess(null);
                }}
              />
              <Button type="button" variant="outline" disabled={isSaving} onClick={handleTemplateBrowse} title="浏览目录">
                <FolderOpen className="size-4" aria-hidden="true" />
              </Button>
            </div>
            <Button type="button" className="lg:w-32" disabled={isSaving} onClick={handleSaveTemplate}>
              {isSaving ? <Loader2 className="size-4 animate-spin" aria-hidden="true" /> : <FolderCog className="size-4" aria-hidden="true" />}
              保存
            </Button>
          </div>

          <div className="mt-4 space-y-2" aria-live="polite">
            {templateError && <Message tone="error" text={templateError} />}
            {templateSuccess && <Message tone="success" text={templateSuccess} />}
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <div className="flex items-start gap-3">
            <div className="flex size-10 shrink-0 items-center justify-center rounded-lg bg-muted text-muted-foreground">
              <Type className="size-5" aria-hidden="true" />
            </div>
            <div className="space-y-1">
              <CardTitle>界面字号</CardTitle>
              <CardDescription>调整全局文字大小。较大字号会自动优化布局以防止内容溢出。</CardDescription>
            </div>
          </div>
        </CardHeader>
        <CardContent>
          <div className="grid gap-3 sm:grid-cols-3">
            {fontSizeOptions.map((option) => (
              <button
                key={option.value}
                type="button"
                onClick={() => setFontSize(option.value)}
                className={cn(
                  "relative flex flex-col items-start gap-1 rounded-xl border-2 p-4 text-left transition-all",
                  fontSize === option.value
                    ? "border-primary bg-primary/5"
                    : "border-border hover:border-primary/30 hover:bg-muted/40",
                )}
              >
                {fontSize === option.value && (
                  <span className="absolute right-3 top-3 flex size-5 items-center justify-center rounded-full bg-primary text-primary-foreground">
                    <CheckCircle2 className="size-3.5" aria-hidden="true" />
                  </span>
                )}
                <span className="text-sm font-semibold">{option.label}</span>
                <span className="text-xs text-muted-foreground">{option.description}</span>
              </button>
            ))}
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <div className="flex items-start gap-3">
            <div className="flex size-10 shrink-0 items-center justify-center rounded-lg bg-muted text-muted-foreground">
              <Sun className="size-5" aria-hidden="true" />
            </div>
            <div className="space-y-1">
              <CardTitle>界面主题</CardTitle>
              <CardDescription>选择浅色、深色或跟随系统偏好。</CardDescription>
            </div>
          </div>
        </CardHeader>
        <CardContent>
          <ThemeSelector />
        </CardContent>
      </Card>

      <Card>
        <CardHeader className="flex-row items-start justify-between gap-4">
          <div className="space-y-1.5">
            <CardTitle>已注册项目</CardTitle>
            <CardDescription>这些路径会被后端注册表持久化保存，移除后停止纳入跨项目监控。</CardDescription>
          </div>
          <Button type="button" variant="ghost" size="icon" onClick={loadProjects} title="刷新项目列表" disabled={isLoading}>
            <RefreshCw className={isLoading ? "size-4 animate-spin" : "size-4"} aria-hidden="true" />
          </Button>
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <div className="flex min-h-44 items-center justify-center rounded-xl border border-dashed border-border text-sm text-muted-foreground">
              <Loader2 className="mr-2 size-4 animate-spin" aria-hidden="true" />
              正在读取项目注册表…
            </div>
          ) : sortedProjects.length === 0 ? (
            <div className="relative flex min-h-56 overflow-hidden rounded-xl border border-dashed border-border bg-muted/20">
              <div className="absolute inset-x-8 top-0 h-px bg-gradient-to-r from-transparent via-primary/30 to-transparent" />
              <div className="m-auto flex max-w-md flex-col items-center p-8 text-center">
                <div className="mb-4 flex size-12 items-center justify-center rounded-xl bg-muted text-muted-foreground">
                  <FolderCog className="size-6" aria-hidden="true" />
                </div>
                <h2 className="text-xl font-semibold tracking-tight">还没有监控项目</h2>
                <p className="mt-2 text-sm text-muted-foreground">
                  从上方输入 ai_project_template 生成项目的绝对路径，添加成功后会出现在项目仪表盘中。
                </p>
              </div>
            </div>
          ) : (
            <div className="divide-y divide-border rounded-xl border border-border">
              {sortedProjects.map((project) => (
                <div key={project.path} className="flex flex-col gap-3 p-4 sm:flex-row sm:items-center sm:justify-between">
                  <div className="min-w-0 space-y-1">
                    <p className="truncate font-mono text-sm font-medium text-foreground">{project.path}</p>
                    <p className="text-xs text-muted-foreground">添加于 {formatAddedAt(project.added_at)}</p>
                  </div>
                  <Button
                    type="button"
                    variant="destructive"
                    className="shrink-0 sm:w-auto"
                    disabled={removingPath === project.path}
                    onClick={() => setPendingRemoval(project)}
                  >
                    {removingPath === project.path ? <Loader2 className="size-4 animate-spin" aria-hidden="true" /> : <Trash2 className="size-4" aria-hidden="true" />}
                    移除
                  </Button>
                </div>
              ))}
            </div>
          )}
        </CardContent>
      </Card>

      <Dialog open={Boolean(pendingRemoval)} onOpenChange={(open) => !open && setPendingRemoval(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>移除监控项目？</DialogTitle>
            <DialogDescription>
              该操作只会从 ptv 注册表移除项目，不会删除磁盘上的项目目录。
            </DialogDescription>
          </DialogHeader>
          {pendingRemoval && (
            <div className="rounded-lg border border-border bg-muted/40 p-3 font-mono text-sm text-muted-foreground break-all">
              {pendingRemoval.path}
            </div>
          )}
          <DialogFooter>
            <Button type="button" variant="outline" disabled={Boolean(removingPath)} onClick={() => setPendingRemoval(null)}>
              取消
            </Button>
            <Button type="button" variant="destructive" disabled={Boolean(removingPath)} onClick={handleRemoveProject}>
              {removingPath ? <Loader2 className="size-4 animate-spin" aria-hidden="true" /> : <Trash2 className="size-4" aria-hidden="true" />}
              确认移除
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </section>
  );
}

function Message({ tone, text }: { tone: "error" | "success"; text: string }) {
  const isError = tone === "error";

  return (
    <div className={isError ? "flex items-start gap-2 rounded-lg border border-destructive/30 bg-destructive/10 p-3 text-sm text-destructive" : "flex items-start gap-2 rounded-lg border border-primary/20 bg-primary/10 p-3 text-sm text-foreground"}>
      {isError ? <AlertTriangle className="mt-0.5 size-4 shrink-0" aria-hidden="true" /> : <CheckCircle2 className="mt-0.5 size-4 shrink-0 text-primary" aria-hidden="true" />}
      <span>{text}</span>
    </div>
  );
}

function validateProjectPath(path: string) {
  const value = path.trim();

  if (!value) {
    return "无效路径：请输入项目绝对路径。";
  }

  // 支持 Unix 路径 (/...) 和 Windows 路径 (C:\...)
  const isUnixPath = value.startsWith("/");
  const isWindowsPath = /^[A-Za-z]:\\/.test(value);

  if (!isUnixPath && !isWindowsPath) {
    return "无效路径：请输入绝对路径，Unix 以 / 开头，Windows 以盘符开头（如 C:\\）。";
  }

  if (value === "/") {
    return "无效路径：请指向具体项目目录，不能使用根目录。";
  }

  return null;
}

function getAddProjectMessage(err: unknown) {
  const message = normalizeCommandError(err);

  if (message.includes("项目已存在")) {
    return `已注册：${message}`;
  }

  if (message.includes("路径规范化失败") || message.includes("No such file") || message.includes("不存在")) {
    return `路径不存在或无法访问：${message}`;
  }

  return `项目添加失败：${message}`;
}

function normalizeCommandError(err: unknown) {
  if (err instanceof Error) {
    return err.message;
  }

  return String(err);
}

function formatAddedAt(timestampSeconds: number) {
  return new Intl.DateTimeFormat("zh-CN", {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  }).format(timestampSeconds * 1000);
}

const themeOptions: { value: Theme; label: string; icon: typeof Sun; description: string }[] = [
  { value: "light", label: "浅色", icon: Sun, description: "始终使用浅色主题" },
  { value: "dark", label: "深色", icon: Moon, description: "始终使用深色主题" },
  { value: "system", label: "跟随系统", icon: Monitor, description: "根据系统设置自动切换" },
];

function ThemeSelector() {
  const { theme, setTheme } = useTheme();

  return (
    <div className="grid gap-3 sm:grid-cols-3">
      {themeOptions.map((option) => {
        const Icon = option.icon;
        return (
          <button
            key={option.value}
            type="button"
            onClick={() => setTheme(option.value)}
            className={cn(
              "relative flex flex-col items-start gap-2 rounded-xl border-2 p-4 text-left transition-all",
              theme === option.value
                ? "border-primary bg-primary/5"
                : "border-border hover:border-primary/30 hover:bg-muted/40",
            )}
          >
            {theme === option.value && (
              <span className="absolute right-3 top-3 flex size-5 items-center justify-center rounded-full bg-primary text-primary-foreground">
                <CheckCircle2 className="size-3.5" aria-hidden="true" />
              </span>
            )}
            <Icon className="size-5 text-foreground/80" aria-hidden="true" />
            <div className="space-y-0.5">
              <span className="text-sm font-semibold">{option.label}</span>
              <span className="block text-xs text-muted-foreground">{option.description}</span>
            </div>
          </button>
        );
      })}
    </div>
  );
}
