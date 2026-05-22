import { Folder, FolderOpen } from "lucide-react";
import { cn } from "@/lib/utils";

import type { ProjectListProps } from "../types";

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
              "flex items-center gap-2 rounded-lg border border-transparent px-3 py-2 text-left text-sm transition-colors",
              "hover:border-border hover:bg-card hover:text-accent-foreground",
              isSelected && "border-border bg-card text-accent-foreground shadow-xs"
            )}
            title={group.project_path}
          >
            {isSelected ? (
              <FolderOpen className="size-4 shrink-0 text-primary" />
            ) : (
              <Folder className="size-4 shrink-0 text-muted-foreground" />
            )}
            <div className="min-w-0 flex-1">
              <span className="block truncate">{group.project_name}</span>
              <span
                className="block truncate text-[10px] text-muted-foreground"
                style={{ direction: "rtl", textAlign: "left" }}
                title={group.project_path}
              >
                {group.project_path}
              </span>
            </div>
            {activeCount > 0 && (
              <span className="shrink-0 rounded bg-green-100 px-1.5 py-0.5 text-[10px] font-medium text-green-700 dark:bg-green-950/40 dark:text-green-400">
                {activeCount > 1 ? `${activeCount}个活跃` : '活跃'}
              </span>
            )}
            {group.is_orphaned && (
              <span
                className="shrink-0 rounded bg-muted px-1.5 py-0.5 text-[10px] font-medium text-muted-foreground"
                title="项目路径已不存在"
              >
                失效
              </span>
            )}
          </button>
        );
      })}
    </div>
  );
}
