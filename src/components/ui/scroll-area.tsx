import type { ComponentProps } from "react";

import { cn } from "@/lib/utils";

function ScrollArea({ className, children, ...props }: ComponentProps<"div">) {
  return (
    <div data-slot="scroll-area" className={cn("relative overflow-hidden", className)} {...props}>
      <div data-slot="scroll-area-viewport" className="h-full w-full overflow-auto rounded-[inherit]">
        {children}
      </div>
    </div>
  );
}

export { ScrollArea };
