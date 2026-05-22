import type { AppDomain } from "@/App";
import { cn } from "@/lib/utils";
import { Bot, FolderKanban, Settings } from "lucide-react";

interface TopNavProps {
  activeDomain: AppDomain;
  onDomainChange: (domain: AppDomain) => void;
}

const domains: Array<{
  id: AppDomain;
  label: string;
  icon: typeof FolderKanban;
}> = [
  { id: "projects", label: "模板项目", icon: FolderKanban },
  { id: "monitoring", label: "Claude Code", icon: Bot },
  { id: "settings", label: "设置", icon: Settings },
];

export function TopNav({ activeDomain, onDomainChange }: TopNavProps) {
  return (
    <header className="flex h-12 shrink-0 items-center border-b border-border bg-sidebar px-4">
      <div className="flex items-center gap-2.5 px-1">
        <span className="flex size-7 shrink-0 items-center justify-center rounded-md border border-primary/20 bg-primary/10 text-primary shadow-xs">
          <svg
            width="18"
            height="18"
            viewBox="0 0 20 20"
            fill="none"
            aria-hidden="true"
          >
            <path d="M10 2a8 8 0 0 0 0 16v-16z" fill="currentColor" />
            <path d="M10 2a8 8 0 0 1 0 16" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
          </svg>
        </span>
        <span className="text-base font-bold tracking-tight text-foreground">AgentScope</span>
      </div>

      <div className="h-4 w-px bg-border mx-3" />

      <nav className="flex items-center gap-0.5" aria-label="大域导航">
        {domains.map((domain) => {
          const Icon = domain.icon;
          const isActive = activeDomain === domain.id;

          return (
            <button
              key={domain.id}
              type="button"
              aria-current={isActive ? "page" : undefined}
              onClick={() => onDomainChange(domain.id)}
              className={cn(
                "relative flex items-center gap-1.5 rounded-md px-3 py-1.5 text-sm font-medium transition-colors",
                isActive
                  ? "text-foreground"
                  : "text-muted-foreground hover:bg-accent hover:text-foreground",
              )}
            >
              <Icon className="size-4" aria-hidden="true" />
              {domain.label}
              {isActive && (
                <span className="absolute inset-x-3 -bottom-[9px] h-0.5 rounded-full bg-primary" />
              )}
            </button>
          );
        })}
      </nav>
    </header>
  );
}
