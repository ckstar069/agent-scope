import { useCallback, useEffect, useState } from "react";
import { save } from "@tauri-apps/plugin-dialog";

import {
  listClaudeSessions,
  deleteClaudeSession,
  exportClaudeSession,
  previewClaudeSession,
} from "@/lib/api";

import type {
  PreviewMessage,
  SessionPreview,
  ClaudeSession,
  ProjectSessionGroup,
} from "../types";

export type { PreviewMessage, SessionPreview, ClaudeSession, ProjectSessionGroup };

export function useClaudeHistory() {
  const [projectGroups, setProjectGroups] = useState<ProjectSessionGroup[]>([]);
  const [selectedProject, setSelectedProject] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState("");
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [previewCache, setPreviewCache] = useState<Record<string, SessionPreview>>({});

  const fetchSessions = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    try {
      const groups = await listClaudeSessions<ProjectSessionGroup[]>();
      setProjectGroups(groups);
      // 只在之前没有选中项目时才设置默认值，避免循环依赖
      setSelectedProject((prev) => {
        if (groups.length > 0 && !prev) {
          return groups[0].project_path;
        }
        return prev;
      });
    } catch (e) {
      setError(String(e));
    } finally {
      setIsLoading(false);
    }
  }, []);

  const deleteSessionHandler = useCallback(async (sessionId: string) => {
    if (!confirm("此操作不可逆，删除后无法通过 /resume 恢复该会话。确定删除吗？")) {
      return;
    }
    try {
      await deleteClaudeSession(sessionId);
      await fetchSessions();
    } catch (e) {
      alert(`删除失败: ${e}`);
    }
  }, [fetchSessions]);

  const exportSessionHandler = useCallback(async (sessionId: string, format: "Jsonl" | "Markdown") => {
    try {
      const ext = format === "Jsonl" ? "jsonl" : "md";
      const outputPath = await save({
        defaultPath: `${sessionId}.${ext}`,
        filters: [
          {
            name: format === "Jsonl" ? "JSONL" : "Markdown",
            extensions: [ext],
          },
        ],
      });
      if (!outputPath) {
        return; // 用户取消
      }
      await exportClaudeSession(sessionId, format, outputPath);
    } catch (e) {
      alert(`导出失败: ${e}`);
    }
  }, []);

  const previewSessionHandler = useCallback(async (sessionId: string) => {
    // 如果已缓存，直接返回
    if (previewCache[sessionId]) {
      return previewCache[sessionId];
    }
    try {
      const preview = await previewClaudeSession<SessionPreview>(sessionId);
      setPreviewCache((prev) => ({ ...prev, [sessionId]: preview }));
      return preview;
    } catch (e) {
      alert(`预览加载失败: ${e}`);
      return null;
    }
  }, [previewCache]);

  useEffect(() => {
    fetchSessions();
    // 移除 5 秒全量轮询：会话管理是历史浏览页，不应频繁全量扫描 JSONL
    // 用户可通过手动刷新按钮重新加载
  }, [fetchSessions]);

  const filteredGroups = projectGroups.filter((group) => {
    const query = searchQuery.toLowerCase();
    const matchProject =
      group.project_name.toLowerCase().includes(query) ||
      group.project_path.toLowerCase().includes(query);
    const matchSession = group.sessions.some(
      (s) => s.name?.toLowerCase().includes(query)
    );
    return matchProject || matchSession;
  });

  const selectedGroup = projectGroups.find(
    (g) => g.project_path === selectedProject
  );

  return {
    projectGroups,
    filteredGroups,
    selectedGroup,
    selectedProject,
    setSelectedProject,
    searchQuery,
    setSearchQuery,
    isLoading,
    error,
    fetchSessions,
    deleteSession: deleteSessionHandler,
    exportSession: exportSessionHandler,
    previewSession: previewSessionHandler,
    previewCache,
  };
}
