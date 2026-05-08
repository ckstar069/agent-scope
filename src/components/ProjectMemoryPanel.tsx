import { useEffect, useState } from "react";
import { AlertCircle, BookOpen, Loader2, MessageSquare, Bookmark } from "lucide-react";

import { MarkdownRenderer } from "@/components/MarkdownRenderer";
import { MemoryFileTree, type SerProjectFile } from "@/components/MemoryFileTree";
import { SessionSearchView } from "@/components/SessionSearchView";
import { TranscriptDetailView, type SessionTurn } from "@/components/TranscriptDetailView";
import { CandidateMemoryBox, type CandidateMemory } from "@/components/CandidateMemoryBox";
import { useTauri } from "@/hooks/useTauri";
import { cn } from "@/lib/utils";

interface ProjectMemoryPanelProps {
  projectPath: string;
}

type MemoryTab = "l1" | "l2" | "l3";

export function ProjectMemoryPanel({ projectPath }: ProjectMemoryPanelProps) {
  const { invoke, listen } = useTauri();

  const [activeTab, setActiveTab] = useState<MemoryTab>("l1");

  const [files, setFiles] = useState<SerProjectFile[]>([]);
  const [selectedPath, setSelectedPath] = useState<string | null>(null);
  const [content, setContent] = useState("");
  const [isLoadingL1, setIsLoadingL1] = useState(false);
  const [errorL1, setErrorL1] = useState<string | null>(null);
  const [changedPaths, setChangedPaths] = useState<Set<string>>(new Set());

  const [selectedSessionId, setSelectedSessionId] = useState<string | null>(null);
  const [candidates, setCandidates] = useState<CandidateMemory[]>([]);
  const [markedTurns, setMarkedTurns] = useState<Set<number>>(new Set());

  useEffect(() => {
    if (!projectPath) return;

    let isActive = true;

    async function loadFiles() {
      setIsLoadingL1(true);
      setErrorL1(null);
      setFiles([]);
      setSelectedPath(null);
      setContent("");
      setChangedPaths(new Set());

      try {
        const result = await invoke<SerProjectFile[], { path: string }>("get_project_files", { path: projectPath });

        if (isActive) {
          setFiles(result);
          setSelectedPath((currentPath) => currentPath ?? result[0]?.relative_path ?? null);
        }
      } catch (err) {
        if (isActive) {
          setErrorL1(err instanceof Error ? err.message : String(err));
        }
      } finally {
        if (isActive) {
          setIsLoadingL1(false);
        }
      }
    }

    void loadFiles();

    return () => {
      isActive = false;
    };
  }, [invoke, projectPath]);

  useEffect(() => {
    if (!projectPath || !selectedPath) {
      setContent("");
      return;
    }

    let isActive = true;
    const currentPath = selectedPath;

    async function loadContent() {
      try {
        const result = await invoke<string, { path: string; relativePath: string }>(
          "get_project_file_content",
          { path: projectPath, relativePath: currentPath }
        );

        if (isActive) {
          setContent(result);
        }
      } catch (err) {
        if (isActive) {
          setContent(`## 读取失败\n\n${err instanceof Error ? err.message : String(err)}`);
        }
      }
    }

    void loadContent();

    return () => {
      isActive = false;
    };
  }, [invoke, projectPath, selectedPath]);

  useEffect(() => {
    if (!projectPath) return;

    let isActive = true;
    let unlisten: (() => void) | undefined;

    async function setupListener() {
      try {
        const dispose = await listen<{ project_path: string }>("template-update", (event) => {
          if (!isActive || event.payload.project_path !== projectPath) return;

          setChangedPaths(new Set(files.map((file) => file.relative_path)));
        });

        unlisten = dispose;
      } catch (err) {
        console.warn("template-update 监听失败", err);
      }
    }

    void setupListener();

    return () => {
      isActive = false;
      unlisten?.();
    };
  }, [listen, projectPath, files]);

  function handleMarkMemory(turn: SessionTurn, turnIndex: number) {
    if (!selectedSessionId || markedTurns.has(turnIndex)) return;

    const id = `${selectedSessionId}-${turnIndex}`;
    const newCandidate: CandidateMemory = {
      id,
      content: turn.text,
      source_session_id: selectedSessionId || "",
      source_turn_index: turnIndex,
      source_snippet: turn.text.slice(0, 100),
      category: "scope",
      status: "pending",
    };

    setMarkedTurns((prev) => {
      if (prev.has(turnIndex)) return prev;

      const next = new Set(prev);
      next.add(turnIndex);
      return next;
    });
    setCandidates((prev) => (prev.some((candidate) => candidate.id === id) ? prev : [...prev, newCandidate]));
  }

  function handleSelectSession(sessionId: string) {
    setSelectedSessionId(sessionId);
    setMarkedTurns(
      new Set(
        candidates
          .filter((candidate) => candidate.source_session_id === sessionId)
          .map((candidate) => candidate.source_turn_index)
      )
    );
  }

  const pendingCount = candidates.filter((c) => c.status === "pending").length;
  const changedCount = changedPaths.size;

  return (
    <div className="space-y-4">
      <div className="flex items-center gap-1 rounded-xl border border-border bg-card p-1 shadow-sm">
        <TabButton
          active={activeTab === "l1"}
          onClick={() => setActiveTab("l1")}
          icon={BookOpen}
          label="静态记忆"
          badge={changedCount > 0 ? changedCount : undefined}
        />
        <TabButton
          active={activeTab === "l2"}
          onClick={() => setActiveTab("l2")}
          icon={MessageSquare}
          label="对话搜索"
        />
        <TabButton
          active={activeTab === "l3"}
          onClick={() => setActiveTab("l3")}
          icon={Bookmark}
          label="候选记忆"
          badge={pendingCount > 0 ? pendingCount : undefined}
        />
      </div>

      {activeTab === "l1" && (
        <L1Panel
          files={files}
          selectedPath={selectedPath}
          content={content}
          isLoading={isLoadingL1}
          error={errorL1}
          changedPaths={changedPaths}
          onSelectPath={setSelectedPath}
        />
      )}

      {activeTab === "l2" && (
        <L2Panel
          projectPath={projectPath}
          selectedSessionId={selectedSessionId}
          onSelectSession={handleSelectSession}
          onMarkMemory={handleMarkMemory}
          markedTurns={markedTurns}
        />
      )}

      {activeTab === "l3" && (
        <L3Panel
          projectPath={projectPath}
          candidates={candidates}
          onUpdateCandidates={setCandidates}
        />
      )}
    </div>
  );
}

function TabButton({
  active,
  onClick,
  icon: Icon,
  label,
  badge,
}: {
  active: boolean;
  onClick: () => void;
  icon: React.ComponentType<{ className?: string }>;
  label: string;
  badge?: number;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={cn(
        "relative flex flex-1 items-center justify-center gap-2 rounded-lg px-3 py-2 text-sm font-medium transition-colors",
        active
          ? "bg-primary text-primary-foreground"
          : "text-muted-foreground hover:bg-accent hover:text-foreground"
      )}
    >
      <Icon className="size-4" />
      {label}
      {badge !== undefined && (
        <span className="ml-1 flex size-5 items-center justify-center rounded-full bg-destructive text-xs text-destructive-foreground">
          {badge}
        </span>
      )}
    </button>
  );
}

function L1Panel({
  files,
  selectedPath,
  content,
  isLoading,
  error,
  changedPaths,
  onSelectPath,
}: {
  files: SerProjectFile[];
  selectedPath: string | null;
  content: string;
  isLoading: boolean;
  error: string | null;
  changedPaths: Set<string>;
  onSelectPath: (path: string) => void;
}) {
  if (isLoading && files.length === 0) {
    return (
      <div className="flex items-center justify-center py-12">
        <Loader2 className="size-6 animate-spin text-muted-foreground" />
        <span className="ml-2 text-sm text-muted-foreground">加载记忆文件中...</span>
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex items-start gap-3 rounded-xl border border-amber-500/30 bg-amber-500/10 p-4 text-sm text-amber-700 dark:text-amber-300">
        <AlertCircle className="mt-0.5 size-4 shrink-0" />
        <div>
          <p className="font-medium">加载记忆文件失败</p>
          <p className="mt-1 text-xs opacity-85">{error}</p>
        </div>
      </div>
    );
  }

  return (
    <div className={cn("grid gap-4", files.length > 0 ? "lg:grid-cols-[16rem_minmax(0,1fr)]" : "")}>
      {files.length > 0 && (
        <aside className="lg:sticky lg:top-6 lg:self-start">
          <MemoryFileTree
            files={files}
            selectedPath={selectedPath}
            onSelect={onSelectPath}
            changedPaths={changedPaths}
          />
        </aside>
      )}

      <div>
        {selectedPath ? (
          <MarkdownRenderer content={content} />
        ) : (
          <div className="flex min-h-64 items-center justify-center rounded-xl border border-dashed border-border bg-card/60 p-5 text-sm text-muted-foreground">
            {files.length === 0 ? "此项目未找到记忆文件" : "选择左侧文件查看内容"}
          </div>
        )}
      </div>
    </div>
  );
}

function L2Panel({
  projectPath,
  selectedSessionId,
  onSelectSession,
  onMarkMemory,
  markedTurns,
}: {
  projectPath: string;
  selectedSessionId: string | null;
  onSelectSession: (sessionId: string) => void;
  onMarkMemory: (turn: SessionTurn, turnIndex: number) => void;
  markedTurns: Set<number>;
}) {
  return (
    <div className="overflow-hidden rounded-xl border border-border bg-card">
      <div className="flex flex-col lg:flex-row">
        <div className="space-y-4 border-b border-border bg-muted/40 p-4 lg:w-[22rem] lg:shrink-0 lg:border-b-0">
          <p className="sticky top-0 z-10 mb-3 border-b border-border bg-inherit pb-3 text-sm font-medium text-muted-foreground">搜索会话</p>
          <SessionSearchView
            projectPath={projectPath}
            selectedSessionId={selectedSessionId}
            onSelectSession={onSelectSession}
          />
        </div>

        <div className="hidden lg:block w-[2px] shrink-0 bg-border/80" />

        <div className="min-w-0 flex-1 space-y-4 bg-background p-4">
          <p className="sticky top-0 z-10 mb-3 border-b border-border bg-inherit pb-3 text-sm font-medium text-muted-foreground">对话详情</p>
          {selectedSessionId ? (
            <TranscriptDetailView
              sessionId={selectedSessionId}
              projectPath={projectPath}
              onMarkMemory={onMarkMemory}
              markedTurns={markedTurns}
            />
          ) : (
            <div className="flex min-h-64 items-center justify-center rounded-xl border border-dashed border-border bg-card/60 p-5 text-sm text-muted-foreground">
              选择左侧会话查看对话详情
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

function L3Panel({
  projectPath,
  candidates,
  onUpdateCandidates,
}: {
  projectPath: string;
  candidates: CandidateMemory[];
  onUpdateCandidates: (candidates: CandidateMemory[]) => void;
}) {
  return (
    <CandidateMemoryBox
      projectPath={projectPath}
      candidates={candidates}
      onUpdateCandidates={onUpdateCandidates}
    />
  );
}
