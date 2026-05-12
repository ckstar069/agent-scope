import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { save } from "@tauri-apps/plugin-dialog";

export interface PreviewMessage {
  role: string;
  content: string;
  timestamp: number | null;
}

export interface SessionPreview {
  session_id: string;
  messages: PreviewMessage[];
  total_turns: number;
}

export interface ClaudeSession {
  session_id: string;
  name: string | null;
  cwd: string;
  status: "Active" | "Idle" | "Exited" | "Unknown";
  started_at: number | null;
  updated_at: number | null;
  turn_count: number | null;
  is_active: boolean;
}

export interface ProjectSessionGroup {
  project_path: string;
  project_name: string;
  sessions: ClaudeSession[];
  session_count: number;
  is_orphaned: boolean;
}

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
      const groups = await invoke<ProjectSessionGroup[]>("list_claude_sessions_cmd");
      setProjectGroups(groups);
      if (groups.length > 0 && !selectedProject) {
        setSelectedProject(groups[0].project_path);
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setIsLoading(false);
    }
  }, [selectedProject]);

  const deleteSession = useCallback(async (sessionId: string) => {
    if (!confirm("此操作不可逆，删除后无法通过 /resume 恢复该会话。确定删除吗？")) {
      return;
    }
    try {
      await invoke("delete_claude_session_cmd", { sessionId });
      await fetchSessions();
    } catch (e) {
      alert(`删除失败: ${e}`);
    }
  }, [fetchSessions]);

  const exportSession = useCallback(async (sessionId: string, format: "Jsonl" | "Markdown") => {
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
      await invoke("export_claude_session_cmd", {
        sessionId,
        format,
        outputPath,
      });
    } catch (e) {
      alert(`导出失败: ${e}`);
    }
  }, []);

  const previewSession = useCallback(async (sessionId: string) => {
    // 如果已缓存，直接返回
    if (previewCache[sessionId]) {
      return previewCache[sessionId];
    }
    try {
      const preview = await invoke<SessionPreview>("preview_claude_session_cmd", {
        sessionId,
      });
      setPreviewCache((prev) => ({ ...prev, [sessionId]: preview }));
      return preview;
    } catch (e) {
      alert(`预览加载失败: ${e}`);
      return null;
    }
  }, [previewCache]);

  useEffect(() => {
    fetchSessions();
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
    deleteSession,
    exportSession,
    previewSession,
    previewCache,
  };
}
