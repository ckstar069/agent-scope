import { Folder, FolderOpen } from "lucide-react";
import { cn } from "@/lib/utils";
import type { ProjectSessionGroup } from "@/hooks/useClaudeHistory";

interface ProjectListProps {
  groups: ProjectSessionGroup[];
  selectedPath: string | null;
  onSelect: (path: string) => void;
}

export function ProjectList({ groups, selectedPath, onSelect }: ProjectListProps) {
  return (
    <div className="flex flex-col gap-1">
      {groups.map((group) => {
        const isSelected = group.project_path === selectedPath;
        const activeCount = group.sessions.filter((s) => s.is_active).length;

        return (
          <button
            key={group.project_path}
            type="button"
            onClick={() => onSelect(group.project_path)}
            className={cn(
              "flex items-center gap-2 rounded-md px-3 py-2 text-left text-sm transition-colors",
              "hover:bg-accent hover:text-accent-foreground",
              isSelected && "bg-accent text-accent-foreground"
            )}
            title={group.project_path}
          >
            {isSelected ? (
              <FolderOpen className="size-4 shrink-0 text-primary" />
            ) : (
              <Folder className="size-4 shrink-0 text-muted-foreground" />
            )}
            <span className="min-w-0 flex-1 truncate">{group.project_name}</span>
            {activeCount > 0 && (
              <span className="flex size-5 shrink-0 items-center justify-center rounded-full bg-primary text-[10px] text-primary-foreground">
                {activeCount}
              </span>
            )}
            {group.is_orphaned && (
              <span className="text-xs text-muted-foreground" title="项目路径已不存在">🚫</span>
            )}
          </button>
        );
      })}
    </div>
  );
}
