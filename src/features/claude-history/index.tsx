import { useCallback, useEffect, useRef, useState } from "react";
import { History, Loader2, RefreshCw } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
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
    <section className="flex h-full flex-col gap-4">
      <div className="flex flex-col gap-4 xl:flex-row xl:items-end xl:justify-between">
        <div className="space-y-2">
          <p className="text-sm font-medium text-muted-foreground">Claude Code</p>
          <h1 className="text-3xl font-semibold tracking-tight">会话管理</h1>
          <p className="max-w-2xl text-sm text-muted-foreground">
            按工作目录浏览 Claude Code 会话历史，预览对话、导出记录并回看工具调用痕迹。
          </p>
        </div>
        <Card className="w-full shadow-xs xl:max-w-xl">
          <CardContent className="flex items-center gap-2 p-2">
            <SearchBar value={searchQuery} onChange={setSearchQuery} />
            <Button
              type="button"
              variant="outline"
              className="shrink-0"
              onClick={fetchSessions}
              disabled={isLoading}
            >
              <RefreshCw className={cn("size-4", isLoading && "animate-spin")} />
              刷新
            </Button>
          </CardContent>
        </Card>
      </div>

      {error && (
        <div className="rounded-xl border border-destructive/40 bg-destructive/10 px-4 py-3 text-sm text-destructive">
          {error}
        </div>
      )}

      {!isLoading && filteredGroups.length === 0 && !error && (
        <Card className="flex flex-1 items-center justify-center border-dashed shadow-xs">
          <CardContent className="flex max-w-md flex-col items-center p-8 text-center text-muted-foreground">
            <span className="mb-4 flex size-12 items-center justify-center rounded-xl border border-border bg-tile">
              {isLoading ? (
                <Loader2 className="size-5 animate-spin" aria-hidden="true" />
              ) : (
                <History className="size-5" aria-hidden="true" />
              )}
            </span>
            <p className="font-medium text-foreground">未找到 Claude Code 会话</p>
            <p className="mt-2 text-sm">请确认已安装 Claude Code 且有过历史会话</p>
          </CardContent>
        </Card>
      )}

      {isLoading && filteredGroups.length === 0 && !error && (
        <Card className="flex flex-1 items-center justify-center border-dashed shadow-xs">
          <CardContent className="flex items-center gap-3 p-8 text-sm text-muted-foreground">
            <Loader2 className="size-4 animate-spin" aria-hidden="true" />
            正在读取 Claude Code 会话…
          </CardContent>
        </Card>
      )}

      {filteredGroups.length > 0 && (
        <Card className="flex flex-1 overflow-hidden shadow-xs">
          <div className="flex min-w-0 flex-1 gap-0 overflow-hidden">
            <div
              className="flex shrink-0 flex-col gap-2 border-r border-border bg-tile/60"
              style={{ width: leftWidth }}
            >
              <p className="border-b border-border px-4 py-3 text-xs font-medium text-muted-foreground">
                项目 ({filteredGroups.length})
              </p>
              <ScrollArea className="flex-1">
                <div className="p-2">
                  <ProjectList
                    groups={filteredGroups}
                    selectedPath={selectedProject}
                    onSelect={setSelectedProject}
                  />
                </div>
              </ScrollArea>
            </div>

            {/* 拖拽手柄 */}
            <div
              className="w-1 shrink-0 cursor-col-resize bg-border/50 transition-colors hover:bg-border active:bg-primary"
              onMouseDown={handleMouseDown}
              title="拖动调整宽度"
            />

            <div className="flex min-w-0 flex-1 flex-col">
              <p className="border-b border-border bg-card px-4 py-3 text-xs font-medium text-muted-foreground">
                {selectedGroup
                  ? `${selectedGroup.project_name} (${selectedGroup.sessions.length} 个会话)`
                  : "选择项目查看会话"}
              </p>
              <ScrollArea className="flex-1">
                <div className="p-4">
                  {selectedGroup ? (
                    <SessionTimeline
                      sessions={selectedGroup.sessions}
                      onDelete={deleteSession}
                      onExport={exportSession}
                      onPreview={previewSession}
                      previewCache={previewCache}
                    />
                  ) : (
                    <p className="rounded-xl border border-dashed border-border bg-tile p-5 text-sm text-muted-foreground">
                      请从左侧选择一个项目
                    </p>
                  )}
                </div>
              </ScrollArea>
            </div>
          </div>
        </Card>
      )}
    </section>
  );
}
