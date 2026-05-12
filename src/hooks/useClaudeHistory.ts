import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

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
      const content = await invoke<string>("export_claude_session_cmd", {
        sessionId,
        format,
      });

      // 触发浏览器下载
      const blob = new Blob([content], {
        type: format === "Jsonl" ? "application/jsonl" : "text/markdown",
      });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `${sessionId}.${format === "Jsonl" ? "jsonl" : "md"}`;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
    } catch (e) {
      alert(`导出失败: ${e}`);
    }
  }, []);

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
  };
}
