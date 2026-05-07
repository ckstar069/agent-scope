import type { ComponentProps, ReactNode } from "react";
import { useMemo } from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";

import { ScrollArea } from "@/components/ui/scroll-area";
import { Skeleton } from "@/components/ui/skeleton";
import { cn } from "@/lib/utils";

export interface MarkdownRendererProps {
  content: string;
  className?: string;
}

interface TocHeading {
  level: number;
  text: string;
  id: string;
}

type HeadingTag = "h1" | "h2" | "h3" | "h4";

const headingClassName: Record<HeadingTag, string> = {
  h1: "mt-0 scroll-mt-6 border-b border-border pb-3 text-3xl font-semibold tracking-tight text-foreground first:mt-0",
  h2: "mt-8 scroll-mt-6 border-b border-border/70 pb-2 text-2xl font-semibold tracking-tight text-foreground first:mt-0",
  h3: "mt-7 scroll-mt-6 text-xl font-semibold tracking-tight text-foreground first:mt-0",
  h4: "mt-6 scroll-mt-6 text-base font-semibold tracking-tight text-foreground first:mt-0",
};

const tocIndentClassName: Record<number, string> = {
  1: "pl-2 font-semibold text-foreground",
  2: "pl-5",
  3: "pl-8 text-muted-foreground",
  4: "pl-11 text-muted-foreground",
};

export function MarkdownRenderer({ content, className }: MarkdownRendererProps) {
  const headings = useMemo(() => extractHeadings(content), [content]);
  const headingQueues = buildHeadingQueues(headings);

  if (!content.trim()) {
    return <MarkdownSkeleton className={className} />;
  }

  return (
    <section className={cn("grid gap-4 lg:grid-cols-[16rem_minmax(0,1fr)]", className)} aria-label="Markdown 内容">
      <aside className="lg:sticky lg:top-6 lg:self-start" aria-label="目录导航">
        <ScrollArea className="max-h-[calc(100vh-4rem)] rounded-xl border border-border bg-card/80 shadow-sm backdrop-blur">
          <nav className="space-y-1 p-3">
            <p className="px-2 pb-2 text-xs font-semibold uppercase tracking-[0.18em] text-muted-foreground">目录</p>
            {headings.length > 0 ? (
              headings.map((heading) => (
                <button
                  key={heading.id}
                  type="button"
                  className={cn(
                    "block w-full rounded-lg py-1.5 pr-2 text-left text-xs leading-5 transition-colors hover:bg-muted hover:text-foreground focus-visible:outline-none focus-visible:ring-3 focus-visible:ring-ring/50",
                    tocIndentClassName[heading.level],
                  )}
                  onClick={() => scrollToHeading(heading.id)}
                >
                  <span className="line-clamp-2">{heading.text}</span>
                </button>
              ))
            ) : (
              <p className="rounded-lg bg-muted/45 px-3 py-2 text-xs text-muted-foreground">当前文档没有可导航标题</p>
            )}
          </nav>
        </ScrollArea>
      </aside>

      <article className="prose prose-neutral dark:prose-invert min-w-0 max-w-none rounded-xl border border-border bg-card/80 p-5 text-card-foreground shadow-sm backdrop-blur sm:p-6">
        <ReactMarkdown
          remarkPlugins={[remarkGfm]}
          components={{
            h1: createHeadingComponent("h1", headingQueues),
            h2: createHeadingComponent("h2", headingQueues),
            h3: createHeadingComponent("h3", headingQueues),
            h4: createHeadingComponent("h4", headingQueues),
            a: ({ className: linkClassName, ...props }) => (
              <a
                className={cn("font-medium text-primary underline underline-offset-4 transition-colors hover:text-primary/75", linkClassName)}
                target={isExternalHref(props.href) ? "_blank" : undefined}
                rel={isExternalHref(props.href) ? "noreferrer" : undefined}
                {...props}
              />
            ),
            code: ({ className: codeClassName, children, ...props }) => {
              const isInline = !codeClassName;

              return isInline ? (
                <code className="rounded-md bg-muted px-1.5 py-0.5 font-mono text-[0.85em] text-foreground" {...props}>
                  {children}
                </code>
              ) : (
                <code className={cn("font-mono text-[0.85rem] leading-6", codeClassName)} {...props}>
                  {children}
                </code>
              );
            },
            pre: ({ className: preClassName, ...props }) => (
              <pre
                className={cn(
                  "my-5 overflow-x-auto rounded-xl border border-border bg-muted/45 p-4 text-sm shadow-inner",
                  preClassName,
                )}
                {...props}
              />
            ),
            table: ({ className: tableClassName, ...props }) => (
              <div className="my-5 overflow-x-auto rounded-xl border border-border">
                <table className={cn("w-full border-collapse text-sm", tableClassName)} {...props} />
              </div>
            ),
            th: ({ className: thClassName, ...props }) => (
              <th className={cn("border-b border-border bg-muted/60 px-3 py-2 text-left font-semibold", thClassName)} {...props} />
            ),
            td: ({ className: tdClassName, ...props }) => <td className={cn("border-b border-border/70 px-3 py-2", tdClassName)} {...props} />,
          }}
        >
          {content}
        </ReactMarkdown>
      </article>
    </section>
  );
}

function MarkdownSkeleton({ className }: { className?: string }) {
  return (
    <section className={cn("grid gap-4 lg:grid-cols-[16rem_minmax(0,1fr)]", className)} aria-label="Markdown 内容加载中">
      <div className="rounded-xl border border-border bg-card/80 p-3 shadow-sm">
        <Skeleton className="mb-4 h-4 w-20" />
        <div className="space-y-2">
          <Skeleton className="h-7 w-full" />
          <Skeleton className="h-7 w-10/12" />
          <Skeleton className="ml-4 h-6 w-9/12" />
          <Skeleton className="ml-8 h-6 w-7/12" />
        </div>
      </div>
      <div className="rounded-xl border border-border bg-card/80 p-5 shadow-sm sm:p-6">
        <Skeleton className="mb-5 h-9 w-2/3" />
        <div className="space-y-3">
          <Skeleton className="h-4 w-full" />
          <Skeleton className="h-4 w-11/12" />
          <Skeleton className="h-4 w-10/12" />
          <Skeleton className="my-5 h-28 w-full rounded-xl" />
          <Skeleton className="h-4 w-9/12" />
          <Skeleton className="h-4 w-8/12" />
        </div>
      </div>
    </section>
  );
}

function createHeadingComponent(Tag: HeadingTag, headingQueues: Map<string, string[]>): (props: ComponentProps<HeadingTag>) => ReactNode {
  return function MarkdownHeading({ className, children, ...props }: ComponentProps<HeadingTag>) {
    const text = extractTextFromNode(children);
    const queue = headingQueues.get(text);
    const id = queue?.shift() ?? slugify(text);

    return (
      <Tag id={id} className={cn(headingClassName[Tag], className)} {...props}>
        {children}
      </Tag>
    );
  };
}

function extractHeadings(markdown: string): TocHeading[] {
  const headings: TocHeading[] = [];
  const slugCounts = new Map<string, number>();
  let isInFence = false;

  for (const line of markdown.split(/\r?\n/)) {
    if (/^\s*(```|~~~)/.test(line)) {
      isInFence = !isInFence;
      continue;
    }

    if (isInFence) {
      continue;
    }

    const match = /^(#{1,4})\s+(.+?)\s*#*\s*$/.exec(line);

    if (!match) {
      continue;
    }

    const text = cleanHeadingText(match[2]);

    if (!text) {
      continue;
    }

    const baseId = slugify(text);
    const count = slugCounts.get(baseId) ?? 0;
    slugCounts.set(baseId, count + 1);

    headings.push({
      level: match[1].length,
      text,
      id: count === 0 ? baseId : `${baseId}-${count + 1}`,
    });
  }

  return headings;
}

function buildHeadingQueues(headings: TocHeading[]): Map<string, string[]> {
  return headings.reduce<Map<string, string[]>>((acc, heading) => {
    const queue = acc.get(heading.text) ?? [];
    queue.push(heading.id);
    acc.set(heading.text, queue);
    return acc;
  }, new Map<string, string[]>());
}

function scrollToHeading(id: string) {
  document.getElementById(id)?.scrollIntoView({ behavior: "smooth", block: "start" });
}

function slugify(text: string): string {
  const slug = text
    .normalize("NFKD")
    .toLowerCase()
    .trim()
    .replace(/[`*_~]/g, "")
    .replace(/[^\p{Letter}\p{Number}\s-]/gu, "")
    .replace(/[\s_-]+/g, "-")
    .replace(/^-+|-+$/g, "");

  return slug || "heading";
}

function cleanHeadingText(text: string): string {
  return text
    .replace(/!\[([^\]]*)\]\([^)]*\)/g, "$1")
    .replace(/\[([^\]]+)\]\([^)]*\)/g, "$1")
    .replace(/[`*_~]/g, "")
    .trim();
}

function extractTextFromNode(node: ReactNode): string {
  if (typeof node === "string" || typeof node === "number") {
    return String(node);
  }

  if (Array.isArray(node)) {
    return cleanHeadingText(node.map(extractTextFromNode).join(""));
  }

  if (node && typeof node === "object" && "props" in node) {
    const props = node.props as { children?: ReactNode };
    return extractTextFromNode(props.children);
  }

  return "";
}

function isExternalHref(href?: string) {
  return Boolean(href && /^https?:\/\//.test(href));
}
