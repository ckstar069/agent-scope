import { useCallback, useEffect, useId, useRef, useState } from "react";
import { CircleHelp } from "lucide-react";
import { cn } from "@/lib/utils";

interface InfoHintProps {
  content: string;
  className?: string;
}

export function InfoHint({ content, className }: InfoHintProps) {
  const [open, setOpen] = useState(false);
  const id = useId();
  const triggerId = `${id}-trigger`;
  const tooltipId = `${id}-tooltip`;
  const triggerRef = useRef<HTMLButtonElement>(null);
  const tooltipRef = useRef<HTMLDivElement>(null);

  const handleOpen = useCallback(() => setOpen(true), []);
  const handleClose = useCallback(() => setOpen(false), []);
  const handleToggle = useCallback(() => setOpen((prev) => !prev), []);

  // 点击外部关闭
  useEffect(() => {
    if (!open) return;

    function handleClickOutside(event: MouseEvent) {
      const target = event.target as Node;
      if (
        tooltipRef.current?.contains(target) ||
        triggerRef.current?.contains(target)
      ) {
        return;
      }
      setOpen(false);
    }

    document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, [open]);

  // Escape 关闭
  useEffect(() => {
    if (!open) return;

    function handleKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape") {
        setOpen(false);
        triggerRef.current?.focus();
      }
    }

    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [open]);

  return (
    <span className={cn("relative inline-flex items-center", className)}>
      <button
        ref={triggerRef}
        type="button"
        id={triggerId}
        aria-describedby={open ? tooltipId : undefined}
        aria-label={`说明：${content}`}
        className="inline-flex items-center justify-center rounded-sm p-0.5 text-muted-foreground/60 outline-none transition-colors hover:text-muted-foreground focus-visible:text-muted-foreground focus-visible:ring-2 focus-visible:ring-ring"
        onMouseEnter={handleOpen}
        onMouseLeave={handleClose}
        onFocus={handleOpen}
        onBlur={handleClose}
        onClick={handleToggle}
      >
        <CircleHelp className="size-3" aria-hidden="true" />
      </button>

      {open && (
        <div
          ref={tooltipRef}
          id={tooltipId}
          role="tooltip"
          className={cn(
            "pointer-events-none absolute left-1/2 top-[calc(100%+6px)] z-50 w-max max-w-[280px] -translate-x-1/2",
            "rounded-md border border-border bg-popover px-3 py-2 shadow-md",
          )}
        >
          <p className="text-xs leading-relaxed text-popover-foreground">
            {content}
          </p>
          {/* 小三角箭头 */}
          <span
            className="absolute -top-1 left-1/2 h-2 w-2 -translate-x-1/2 rotate-45 border-l border-t border-border bg-popover"
            aria-hidden="true"
          />
        </div>
      )}
    </span>
  );
}
