import { RefreshCw } from "lucide-react";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import { ProjectList } from "@/components/claude-history/ProjectList";
import { SearchBar } from "@/components/claude-history/SearchBar";
import { SessionTimeline } from "@/components/claude-history/SessionTimeline";
import { useClaudeHistory } from "@/hooks/useClaudeHistory";
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
        <div className="flex flex-1 gap-4 overflow-hidden">
          <div className="flex w-64 shrink-0 flex-col gap-2">
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

          <div className="flex min-w-0 flex-1 flex-col gap-2">
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
