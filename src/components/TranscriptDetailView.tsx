import { useEffect, useMemo, useState, type ReactNode } from "react";
import { AlertCircle, Bookmark } from "lucide-react";

import { getSessionTitle } from "@/components/SessionSearchView";
import { useTauri } from "@/hooks/useTauri";
import { Skeleton } from "@/components/ui/skeleton";
import { cn } from "@/lib/utils";

/**
 * 对话详情文档式渲染契约（L2「对话搜索」专用）。
 *
 * 目标形态：把会话转录视为可阅读的技术文档，而不是即时通讯界面。
 * 后续实现 Task 5/6/7 时必须遵守以下规则：
 *
 * 1. 文档结构
 *    - 顶部是会话摘要头：标题、模型、创建时间、修改文件数量。
 *    - 正文按原始顺序合并为 Q&A section：用户问题与助手回答都占满正文宽度。
 *    - 状态提示（如 [Request interrupted by user]）只渲染为 muted/gray 行内标签，不能作为主内容强调。
 *    - 工具调用仅允许渲染为极简 muted chip，禁止在详情视图暴露完整工具参数。
 *    - thinking / redacted_thinking 默认隐藏，不参与正文渲染；只有显式调试入口才允许查看。
 *
 * 2. 正文排版规则
 *    - 直接使用 prose/prose-sm、text-muted-foreground、border-border、bg-card 等现有设计 token。
 *    - 代码块使用 <pre>，并在代码块本地设置 overflow-x-auto；长行只能在代码块内横向滚动。
 *    - 禁止 max-w-[85%]、justify-end 或按角色左右错位的消息布局。
 *    - 禁止在转录段落内嵌入带目录侧栏的完整文档渲染组件；该组件仅用于 L1 静态记忆文档。
 *
 * 3. 标记动作规则
 *    - 标记按钮放在 section 级别：优先位于段落标题旁，其次位于段落末尾动作区。
 *    - 已标记状态用「已标记」badge 或按钮状态变化表达。
 *    - 标记动作继续调用 onMarkMemory(turn, turnIndex)，以保留现有候选记忆生成流程。
 */
export interface TranscriptTurnRenderContract {
  summaryHeader: {
    title: string;
    model?: string;
    createdAt: number;
    modifiedFileCount: number;
  };
  sections: TranscriptSectionContract[];
}

export interface TranscriptSectionContract {
  role: "user" | "assistant";
  content: string;
  timestamp?: number;
  statusHints: TranscriptStatusHint[];
  toolCalls: ToolCallDisplay[];
  markAction: TranscriptMarkAction;
}

export interface TranscriptStatusHint {
  text: "[Request interrupted by user]" | string;
  tone: "muted";
  inline: true;
}

export interface ToolCallDisplay {
  name: string;
  details?: string;
  defaultCollapsed: true;
}

export interface TranscriptMarkAction {
  placement: "section-header" | "section-footer";
  state: "idle" | "marked";
  createsCandidateMemory: true;
}

export interface SessionTurn {
  role: "user" | "assistant";
  text: string;
  tools: string[];
  timestamp?: number;
}

interface SerTranscript {
  session_id: string;
  initial_prompt: string;
  custom_title?: string;
  model?: string;
  turns: SessionTurn[];
  modified_files: string[];
  created_at: number;
}

type QAPair = {
  userText: string;
  assistantText: string;
  userTimestamp?: number;
  assistantTimestamp?: number;
  tools: string[];
  index: number;
};

interface TranscriptDetailViewProps {
  sessionId: string;
  projectPath: string;
  onMarkMemory: (turn: SessionTurn, turnIndex: number) => void;
  markedTurns?: Set<number>;
}

export function TranscriptDetailView({
  sessionId,
  projectPath,
  onMarkMemory,
  markedTurns = new Set(),
}: TranscriptDetailViewProps) {
  const { invoke } = useTauri();
  const [transcript, setTranscript] = useState<SerTranscript | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [displayCount, setDisplayCount] = useState(50);

  useEffect(() => {
    if (!sessionId || !projectPath) {
      setTranscript(null);
      return;
    }

    let isActive = true;

    async function loadTranscript() {
      setIsLoading(true);
      setError(null);
      setDisplayCount(50);

      try {
        const result = await invoke<SerTranscript, { path: string; sessionId: string }>(
          "get_session_transcript",
          { path: projectPath, sessionId }
        );

        if (isActive) {
          setTranscript(result);
        }
      } catch (err) {
        if (isActive) {
          setError(err instanceof Error ? err.message : String(err));
        }
      } finally {
        if (isActive) {
          setIsLoading(false);
        }
      }
    }

    void loadTranscript();

    return () => {
      isActive = false;
    };
  }, [invoke, sessionId, projectPath]);

  const qaPairs = useMemo(() => buildQAPairs(transcript?.turns ?? []), [transcript?.turns]);
  const visiblePairs = qaPairs.slice(0, displayCount);
  const hasMore = qaPairs.length > displayCount;

  if (isLoading) {
    return <TranscriptSkeleton />;
  }

  if (error) {
    return (
      <div className="flex items-start gap-3 rounded-xl border border-amber-500/30 bg-amber-500/10 p-4 text-sm text-amber-700 dark:text-amber-300">
        <AlertCircle className="mt-0.5 size-4 shrink-0" />
        <div>
          <p className="font-medium">加载对话失败</p>
          <p className="mt-1 text-xs opacity-85">{error}</p>
        </div>
      </div>
    );
  }

  if (!transcript) {
    return (
      <div className="flex min-h-64 items-center justify-center rounded-xl border border-dashed border-border bg-card/60 p-5 text-sm text-muted-foreground">
        选择一个会话查看对话详情
      </div>
    );
  }

  return (
    <div className="space-y-5">
      <div className="rounded-xl border border-border bg-card/95 p-4 shadow-sm">
        <h3 className="text-base font-medium leading-snug" title={getSessionTitle(transcript)}>
          {truncateText(getSessionTitle(transcript), 96)}
        </h3>
        <div className="mt-2 flex flex-wrap gap-2 text-xs text-muted-foreground">
          {transcript.model && <span>模型: {transcript.model}</span>}
          <span>时间: {new Date(transcript.created_at).toLocaleString("zh-CN")}</span>
          <span>文件: {transcript.modified_files.length}</span>
        </div>
      </div>

      <div className="space-y-6 divide-y divide-border/80">
        {visiblePairs.map((pair, index) => (
          <TranscriptQAPairSection
            key={`qa-${pair.index}-${pair.userText.slice(0, 20).replace(/\s/g, "-")}`}
            pair={pair}
            pairNumber={index + 1}
            isMarked={markedTurns.has(pair.index)}
            onMark={() => onMarkMemory(createMemoryTurnFromPair(pair), pair.index)}
          />
        ))}
      </div>

      {hasMore && (
        <button
          type="button"
          onClick={() => setDisplayCount((prev) => prev + 50)}
          className="w-full rounded-xl border border-border bg-card py-3 text-sm text-muted-foreground transition-colors hover:bg-accent/50"
        >
          加载更多 ({qaPairs.length - displayCount} 组剩余)
        </button>
      )}
    </div>
  );
}

function TranscriptQAPairSection({
  pair,
  pairNumber,
  isMarked,
  onMark,
}: {
  pair: QAPair;
  pairNumber: number;
  isMarked: boolean;
  onMark: () => void;
}) {
  const timestamp = pair.userTimestamp ?? pair.assistantTimestamp;

  return (
    <section
      className={cn(
        "overflow-hidden rounded-xl border-2 border-border/60 bg-card text-card-foreground",
        isMarked && "border-amber-500/70 bg-amber-500/5"
      )}
    >
      <div className="flex flex-wrap items-center gap-2 border-b border-border bg-muted/40 px-4 py-3">
        <span className="text-xs font-semibold uppercase tracking-[0.18em] text-muted-foreground">
          Q&A {pairNumber}
        </span>
        <span className="rounded-md bg-muted px-2 py-0.5 text-xs font-medium text-muted-foreground">
          Turn {pair.index + 1}
        </span>
        {timestamp !== undefined && (
          <span className="text-xs text-muted-foreground">
            {new Date(timestamp).toLocaleTimeString("zh-CN")}
          </span>
        )}

        <button
          type="button"
          onClick={onMark}
          className={cn(
            "ml-auto flex items-center gap-1 rounded-md px-2 py-1 text-xs transition-colors",
            isMarked
              ? "bg-amber-500/20 text-amber-700 dark:text-amber-300"
              : "text-muted-foreground hover:bg-accent hover:text-foreground"
          )}
        >
          <Bookmark className="size-3" />
          {isMarked ? "已标记" : "标记"}
        </button>
      </div>

      <div className="space-y-4 p-4">
        {pair.userText && (
          <TranscriptRoleBlock
            label="用户"
            timestamp={pair.userTimestamp}
            text={pair.userText}
          />
        )}

        {pair.assistantText ? (
          <div className={cn(pair.userText && "border-t border-border pt-4")}>
            <TranscriptRoleBlock
              label="助手"
              timestamp={pair.assistantTimestamp}
              text={pair.assistantText}
            />
          </div>
        ) : (
          <div className="rounded-lg border border-dashed border-border bg-muted/20 p-3 text-sm text-muted-foreground">
            暂无助手文本回复
          </div>
        )}
      </div>

      {pair.tools.length > 0 && (
        <div className="mx-4 mb-4 flex flex-wrap gap-1 border-t border-border pt-3 text-xs text-muted-foreground">
          <span className="rounded-md bg-muted/70 px-1.5 py-0.5">
            使用了 {pair.tools.join("、")}
          </span>
        </div>
      )}
    </section>
  );
}

function TranscriptRoleBlock({
  label,
  timestamp,
  text,
}: {
  label: "用户" | "助手";
  timestamp?: number;
  text: string;
}) {
  return (
      <article
        className={cn(
        "space-y-2 rounded-lg border border-border/80 bg-background p-3",
        label === "用户" ? "border-l-4 border-l-primary" : "border-l-4 border-l-muted-foreground/40 bg-muted/40"
      )}
    >
      <div className="flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
        <span className="rounded-md bg-muted px-2 py-0.5 font-medium text-muted-foreground">
          {label}
        </span>
        {timestamp !== undefined && (
          <span>{new Date(timestamp).toLocaleTimeString("zh-CN")}</span>
        )}
      </div>
      <div className="prose prose-sm dark:prose-invert max-w-none space-y-3 text-sm leading-6 text-foreground">
        {renderTranscriptText(text)}
      </div>
    </article>
  );
}

function buildQAPairs(turns: SessionTurn[]): QAPair[] {
  const pairs: QAPair[] = [];
  let pendingPair: QAPair | null = null;

  turns.forEach((turn, turnIndex) => {
    const text = turn.text.trim();
    const tools = normalizeTools(turn.tools);

    if (turn.role === "user") {
      if (!text) return;

      if (pendingPair) {
        pairs.push(pendingPair);
      }

      pendingPair = {
        userText: text,
        assistantText: "",
        userTimestamp: turn.timestamp,
        tools,
        index: turnIndex,
      };
      return;
    }

    if (!text && tools.length === 0) return;

    if (!pendingPair) {
      if (!text) return;

      pairs.push({
        userText: "",
        assistantText: text,
        assistantTimestamp: turn.timestamp,
        tools,
        index: turnIndex,
      });
      return;
    }

    pendingPair.assistantText = text;
    pendingPair.assistantTimestamp = turn.timestamp;
    pendingPair.tools = normalizeTools([...pendingPair.tools, ...tools]);
    pairs.push(pendingPair);
    pendingPair = null;
  });

  if (pendingPair) {
    pairs.push(pendingPair);
  }

  return pairs;
}

function createMemoryTurnFromPair(pair: QAPair): SessionTurn {
  const text = [
    pair.userText ? `用户：${pair.userText}` : "",
    pair.assistantText ? `助手：${pair.assistantText}` : "",
  ]
    .filter(Boolean)
    .join("\n\n");

  return {
    role: pair.userText ? "user" : "assistant",
    text,
    tools: pair.tools,
    timestamp: pair.userTimestamp ?? pair.assistantTimestamp,
  };
}

function normalizeTools(tools: string[]): string[] {
  return Array.from(new Set(tools.filter(Boolean))).sort();
}

function TranscriptSkeleton() {
  const items = ["a", "b", "c"];

  return (
    <div className="space-y-4">
      <div className="rounded-xl border border-border bg-card/95 p-4 shadow-sm">
        <Skeleton className="h-4 w-1/2" />
        <Skeleton className="mt-2 h-3 w-24" />
      </div>

      {items.map((key) => (
        <section key={key} className="overflow-hidden rounded-xl border-2 border-border/60 bg-card">
          <div className="flex items-center gap-2 border-b border-border bg-muted/40 px-4 py-3">
            <Skeleton className="h-3 w-16" />
            <Skeleton className="h-5 w-12" />
            <Skeleton className="ml-auto h-6 w-16" />
          </div>
          <div className="space-y-2 p-4">
            <Skeleton className="h-4 w-64" />
            <Skeleton className="h-4 w-48" />
            <Skeleton className="h-4 w-32" />
          </div>
        </section>
      ))}
    </div>
  );
}

function renderTextWithStatusHints(text: string): ReactNode[] {
  const statusHintPattern = /(\[Request interrupted by user\])/g;
  let charOffset = 0;

  return text.split(statusHintPattern).map((part) => {
    const key = `${part}-${charOffset}`;
    charOffset += part.length;

    if (part === "[Request interrupted by user]") {
      return (
        <span key={key} className="rounded-md bg-muted px-1.5 py-0.5 text-xs text-muted-foreground">
          {part}
        </span>
      );
    }

    return part;
  });
}

function renderTranscriptText(text: string): ReactNode[] {
  const blocks: ReactNode[] = [];
  const codeFencePattern = /```([^\n`]*)\n?([\s\S]*?)```/g;
  let lastIndex = 0;
  let blockIndex = 0;
  let match = codeFencePattern.exec(text);

  while (match !== null) {
    appendTextBlocks(text.slice(lastIndex, match.index), `text-${blockIndex}`, blocks);

    const language = match[1].trim();
    const code = match[2].replace(/\n$/, "");
    blocks.push(
      <pre
        key={`code-${blockIndex}`}
        className="overflow-x-auto rounded-lg border border-border bg-muted/45 p-3 text-xs text-foreground"
      >
        <code className={language ? `language-${language}` : undefined}>{code}</code>
      </pre>
    );

    lastIndex = match.index + match[0].length;
    blockIndex += 1;
    match = codeFencePattern.exec(text);
  }

  appendTextBlocks(text.slice(lastIndex), `text-${blockIndex}`, blocks);

  if (blocks.length === 0) {
    return [];
  }

  return blocks;
}

function appendTextBlocks(text: string, keyPrefix: string, blocks: ReactNode[]) {
  text
    .split(/\n{2,}/)
    .map((paragraph) => paragraph.replace(/^\n+|\n+$/g, ""))
    .filter(Boolean)
    .forEach((paragraph) => {
      blocks.push(
        <p key={`${keyPrefix}-${paragraph.slice(0, 24)}-${paragraph.length}`} className="whitespace-pre-wrap break-words">
          {renderTextWithStatusHints(paragraph)}
        </p>
      );
    });
}

function truncateText(text: string, maxLength: number): string {
  if (!text || text.length <= maxLength) return text || "";
  return text.slice(0, maxLength) + "...";
}
