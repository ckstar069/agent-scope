import { useCallback, useEffect, useRef, useState } from "react";
import { RefreshCw } from "lucide-react";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import { ProjectList } from "./components/ProjectList";
import { SearchBar } from "./components/SearchBar";
import { SessionTimeline } from "./components/SessionTimeline";
import { useClaudeHistory } from "./hooks/useClaudeHistory";
import { cn } from "@/lib/utils";

export function ClaudeHistory() {
  const {
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
  } = useClaudeHistory();

  const [leftWidth, setLeftWidth] = useState(256);
  const isDragging = useRef(false);
  const startXRef = useRef(0);
  const startWidthRef = useRef(256);

  const handleMouseMove = useCallback((e: MouseEvent) => {
    if (!isDragging.current) return;
    const delta = e.clientX - startXRef.current;
    const newWidth = Math.max(180, Math.min(600, startWidthRef.current + delta));
    setLeftWidth(newWidth);
  }, []);

  const handleMouseUp = useCallback(() => {
    isDragging.current = false;
    document.body.style.cursor = "";
    document.body.style.userSelect = "";
    document.removeEventListener("mousemove", handleMouseMove);
    document.removeEventListener("mouseup", handleMouseUp);
  }, [handleMouseMove]);

  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    isDragging.current = true;
    startXRef.current = e.clientX;
    startWidthRef.current = leftWidth;
    document.body.style.cursor = "col-resize";
    document.body.style.userSelect = "none";
    document.addEventListener("mousemove", handleMouseMove);
    document.addEventListener("mouseup", handleMouseUp);
  }, [leftWidth, handleMouseMove, handleMouseUp]);

  useEffect(() => {
    return () => {
      document.removeEventListener("mousemove", handleMouseMove);
      document.removeEventListener("mouseup", handleMouseUp);
    };
  }, [handleMouseMove, handleMouseUp]);

  return (
    <div className="flex h-full flex-col gap-4">
      <div className="flex items-center gap-4">
        <h1 className="text-xl font-semibold">会话管理</h1>
        <div className="flex-1">
          <SearchBar value={searchQuery} onChange={setSearchQuery} />
        </div>
        <Button
          type="button"
          variant="outline"
          size="sm"
          onClick={fetchSessions}
          disabled={isLoading}
        >
          <RefreshCw className={cn("mr-1 size-4", isLoading && "animate-spin")} />
          刷新
        </Button>
      </div>

      {error && (
        <div className="rounded-md border border-destructive bg-destructive/10 px-4 py-3 text-sm text-destructive">
          {error}
        </div>
      )}

      {!isLoading && filteredGroups.length === 0 && !error && (
        <div className="flex flex-1 flex-col items-center justify-center text-muted-foreground">
          <p>未找到 Claude Code 会话</p>
          <p className="text-sm">请确认已安装 Claude Code 且有过历史会话</p>
        </div>
      )}

      {filteredGroups.length > 0 && (
        <div className="flex flex-1 gap-0 overflow-hidden">
          <div
            className="flex shrink-0 flex-col gap-2"
            style={{ width: leftWidth }}
          >
            <p className="px-3 text-xs font-medium text-muted-foreground">
              项目 ({filteredGroups.length})
            </p>
            <ScrollArea className="flex-1">
              <ProjectList
                groups={filteredGroups}
                selectedPath={selectedProject}
                onSelect={setSelectedProject}
              />
            </ScrollArea>
          </div>

          {/* 拖拽手柄 */}
          <div
            className="w-1 shrink-0 cursor-col-resize bg-border/50 hover:bg-border active:bg-primary transition-colors"
            onMouseDown={handleMouseDown}
            title="拖动调整宽度"
          />

          <div className="flex min-w-0 flex-1 flex-col gap-2 pl-4">
            <p className="px-1 text-xs font-medium text-muted-foreground">
              {selectedGroup
                ? `${selectedGroup.project_name} (${selectedGroup.sessions.length} 个会话)`
                : "选择项目查看会话"}
            </p>
            <ScrollArea className="flex-1">
              {selectedGroup ? (
                <SessionTimeline
                  sessions={selectedGroup.sessions}
                  onDelete={deleteSession}
                  onExport={exportSession}
                  onPreview={previewSession}
                  previewCache={previewCache}
                />
              ) : (
                <p className="text-sm text-muted-foreground">请从左侧选择一个项目</p>
              )}
            </ScrollArea>
          </div>
        </div>
      )}
    </div>
  );
}
