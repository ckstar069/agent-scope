import { useState } from "react";
import { Bookmark, Check, Edit2, Trash2, AlertCircle } from "lucide-react";

import { useTauri } from "@/hooks/useTauri";
import { cn } from "@/lib/utils";

export interface CandidateMemory {
  id: string;
  content: string;
  source_session_id: string;
  source_turn_index: number;
  source_snippet: string;
  category: MemoryCategory;
  status: "pending" | "saved";
}

export type MemoryCategory = "scope" | "product" | "tech" | "security" | "other";

const CATEGORY_LABELS: Record<MemoryCategory, string> = {
  scope: "范围约定",
  product: "产品约束",
  tech: "技术规范",
  security: "安全约束",
  other: "其他",
};

const CATEGORY_OPTIONS: { value: MemoryCategory; label: string }[] = [
  { value: "scope", label: "范围约定" },
  { value: "product", label: "产品约束" },
  { value: "tech", label: "技术规范" },
  { value: "security", label: "安全约束" },
  { value: "other", label: "其他" },
];

interface CandidateMemoryBoxProps {
  projectPath: string;
  candidates: CandidateMemory[];
  onUpdateCandidates: (candidates: CandidateMemory[]) => void;
}

export function CandidateMemoryBox({
  projectPath,
  candidates,
  onUpdateCandidates,
}: CandidateMemoryBoxProps) {
  const { invoke } = useTauri();
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editContent, setEditContent] = useState("");
  const [confirmingId, setConfirmingId] = useState<string | null>(null);

  const pendingCandidates = candidates.filter((c) => c.status === "pending");
  const savedCandidates = candidates.filter((c) => c.status === "saved");

  function handleEditStart(candidate: CandidateMemory) {
    setEditingId(candidate.id);
    setEditContent(candidate.content);
  }

  function handleEditSave(id: string) {
    onUpdateCandidates(
      candidates.map((c) => (c.id === id ? { ...c, content: editContent } : c))
    );
    setEditingId(null);
    setEditContent("");
  }

  function handleDelete(id: string) {
    onUpdateCandidates(candidates.filter((c) => c.id !== id));
  }

  function handleCategoryChange(id: string, category: MemoryCategory) {
    onUpdateCandidates(
      candidates.map((c) => (c.id === id ? { ...c, category } : c))
    );
  }

  async function handleConfirm(id: string) {
    const candidate = candidates.find((c) => c.id === id);
    if (!candidate) return;

    try {
      await invoke<void, { path: string; memory: unknown }>("save_candidate_memory", {
        path: projectPath,
        memory: {
          content: candidate.content,
          source_session_id: candidate.source_session_id,
          source_turn_index: candidate.source_turn_index,
          source_snippet: candidate.source_snippet,
          category: CATEGORY_LABELS[candidate.category],
        },
      });

      onUpdateCandidates(
        candidates.map((c) => (c.id === id ? { ...c, status: "saved" as const } : c))
      );
      setConfirmingId(null);
    } catch (err) {
      console.error("保存候选记忆失败", err);
    }
  }

  if (candidates.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center py-12 text-sm text-muted-foreground">
        <Bookmark className="mb-2 size-8 opacity-50" aria-hidden="true" />
      <p>暂无记忆标记</p>
      <p className="mt-1 text-xs">在对话中点击标记重要消息来添加</p>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      {pendingCandidates.length > 0 && (
        <div className="space-y-2">
          <p className="text-sm font-medium text-muted-foreground">
            待确认 ({pendingCandidates.length})
          </p>
          {pendingCandidates.map((candidate) => (
            <CandidateCard
              key={candidate.id}
              candidate={candidate}
              isEditing={editingId === candidate.id}
              editContent={editContent}
              isConfirming={confirmingId === candidate.id}
              onEditStart={() => handleEditStart(candidate)}
              onEditChange={setEditContent}
              onEditSave={() => handleEditSave(candidate.id)}
              onEditCancel={() => setEditingId(null)}
              onDelete={() => handleDelete(candidate.id)}
              onCategoryChange={(cat) => handleCategoryChange(candidate.id, cat)}
              onConfirm={() => setConfirmingId(candidate.id)}
              onConfirmCancel={() => setConfirmingId(null)}
              onConfirmSave={() => handleConfirm(candidate.id)}
            />
          ))}
        </div>
      )}

      {savedCandidates.length > 0 && (
        <div className="space-y-2">
          <p className="text-sm font-medium text-muted-foreground">
            已沉淀 ({savedCandidates.length})
          </p>
          {savedCandidates.map((candidate) => (
            <CandidateCard
              key={candidate.id}
              candidate={candidate}
              isEditing={false}
              editContent=""
              isConfirming={false}
              onEditStart={() => {}}
              onEditChange={() => {}}
              onEditSave={() => {}}
              onEditCancel={() => {}}
              onDelete={() => handleDelete(candidate.id)}
              onCategoryChange={() => {}}
              onConfirm={() => {}}
              onConfirmCancel={() => {}}
              onConfirmSave={() => {}}
            />
          ))}
        </div>
      )}
    </div>
  );
}

function CandidateCard({
  candidate,
  isEditing,
  editContent,
  isConfirming,
  onEditStart,
  onEditChange,
  onEditSave,
  onEditCancel,
  onDelete,
  onCategoryChange,
  onConfirm,
  onConfirmCancel,
  onConfirmSave,
}: {
  candidate: CandidateMemory;
  isEditing: boolean;
  editContent: string;
  isConfirming: boolean;
  onEditStart: () => void;
  onEditChange: (content: string) => void;
  onEditSave: () => void;
  onEditCancel: () => void;
  onDelete: () => void;
  onCategoryChange: (category: MemoryCategory) => void;
  onConfirm: () => void;
  onConfirmCancel: () => void;
  onConfirmSave: () => void;
}) {
  const isSaved = candidate.status === "saved";

  if (isConfirming) {
    return (
      <div className="rounded-xl border border-amber-500/30 bg-amber-500/10 p-4">
        <div className="flex items-start gap-2">
          <AlertCircle className="mt-0.5 size-4 shrink-0 text-amber-700 dark:text-amber-300" />
          <div className="space-y-2">
            <p className="text-sm font-medium text-amber-700 dark:text-amber-300">
              确认沉淀记忆？
            </p>
            <p className="text-xs text-amber-600 dark:text-amber-400">
              这将写入 .sisyphus/notepads/project-memory/decisions.md
            </p>
            <div className="flex gap-2">
              <button
                type="button"
                onClick={onConfirmCancel}
                className="rounded-md border border-border bg-background px-3 py-1.5 text-xs transition-colors hover:bg-accent"
              >
                取消
              </button>
              <button
                type="button"
                onClick={onConfirmSave}
                className="rounded-md bg-primary px-3 py-1.5 text-xs text-primary-foreground transition-colors hover:bg-primary/90"
              >
                确认写入
              </button>
            </div>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div
      className={cn(
        "rounded-xl border p-4",
        isSaved
          ? "border-green-500/30 bg-green-500/5"
          : "border-border bg-card"
      )}
    >
      <div className="mb-2 flex items-center gap-2 text-xs text-muted-foreground">
        <span className="font-mono">{candidate.source_session_id.slice(0, 8)}</span>
        <span>/ Turn {candidate.source_turn_index}</span>
        {isSaved && (
          <span className="ml-auto flex items-center gap-1 text-green-600 dark:text-green-400">
            <Check className="size-3" />
            已保存
          </span>
        )}
      </div>

      {isEditing ? (
        <textarea
          value={editContent}
          onChange={(e) => onEditChange(e.target.value)}
          className="w-full rounded-lg border border-border bg-background p-2 text-sm outline-none focus-visible:ring-3 focus-visible:ring-ring/50"
          rows={3}
        />
      ) : (
        <p className="text-sm">{truncateText(candidate.content, 100)}</p>
      )}

      <div className="mt-3 flex items-center justify-between gap-2">
        <select
          value={candidate.category}
          onChange={(e) => onCategoryChange(e.target.value as MemoryCategory)}
          disabled={isSaved}
          className="rounded-md border border-border bg-background px-2 py-1 text-xs outline-none disabled:opacity-50"
        >
          {CATEGORY_OPTIONS.map((opt) => (
            <option key={opt.value} value={opt.value}>
              {opt.label}
            </option>
          ))}
        </select>

        <div className="flex items-center gap-1">
          {isEditing ? (
            <>
              <button
                type="button"
                onClick={onEditSave}
                className="rounded-md p-1.5 text-xs transition-colors hover:bg-accent"
              >
                保存
              </button>
              <button
                type="button"
                onClick={onEditCancel}
                className="rounded-md p-1.5 text-xs transition-colors hover:bg-accent"
              >
                取消
              </button>
            </>
          ) : (
            <>
              {!isSaved && (
                <>
                  <button
                    type="button"
                    onClick={onEditStart}
                    className="rounded-md p-1.5 text-muted-foreground transition-colors hover:bg-accent"
                    title="编辑"
                  >
                    <Edit2 className="size-3.5" />
                  </button>
                  <button
                    type="button"
                    onClick={onConfirm}
                    className="rounded-md p-1.5 text-green-600 transition-colors hover:bg-green-500/10"
                    title="确认沉淀"
                  >
                    <Check className="size-3.5" />
                  </button>
                </>
              )}
              <button
                type="button"
                onClick={onDelete}
                className="rounded-md p-1.5 text-muted-foreground transition-colors hover:bg-accent"
                title="删除"
              >
                <Trash2 className="size-3.5" />
              </button>
            </>
          )}
        </div>
      </div>
    </div>
  );
}

function truncateText(text: string, maxLength: number): string {
  if (!text || text.length <= maxLength) return text || "";
  return text.slice(0, maxLength) + "...";
}
