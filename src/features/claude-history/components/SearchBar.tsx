import { Search } from "lucide-react";
import { Input } from "@/components/ui/input";

import type { SearchBarProps } from "../types";

export function SearchBar({ value, onChange }: SearchBarProps) {
  return (
    <div className="relative min-w-0 flex-1">
      <Search className="absolute left-3 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
      <Input
        type="text"
        placeholder="搜索会话或项目..."
        value={value}
        onChange={(e) => onChange(e.target.value)}
        className="border-0 bg-tile pl-9 shadow-none focus-visible:ring-1"
      />
    </div>
  );
}
