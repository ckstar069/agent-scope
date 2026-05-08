
## 2026-05-07 — L2 对话转录文档式 UI 契约

- 决定将「对话搜索」详情区的转录内容定义为文档阅读流：会话摘要头 + 按原始顺序排列的连续 section，用户与助手内容均占满正文宽度。
- 决定转录正文只使用现有 token/组件语义：`prose/prose-sm`、`bg-card`、`border-border`、`text-muted-foreground`、`rounded-xl`、`shadow-sm`，避免脱离现有 shadcn/Tailwind 设计体系。
- 决定状态提示（例如 `[Request interrupted by user]`）仅作为 muted/gray 行内标签渲染，不进入主内容层级。
- 决定工具调用区使用默认收起的 `<details>` 语义，摘要行展示工具名 chip/badge，展开后显示详情，避免长工具输出打断阅读。
- 决定 `thinking` / `redacted_thinking` 默认隐藏；除非后续显式增加调试入口，否则不进入正文。
- 决定标记动作放在 section 标题行，已标记态显示「已标记」，并继续调用 `onMarkMemory(turn, turnIndex)` 保留候选记忆流程。

## Task 4 UI 决策 - 会话列表选中态与标题回退

- 复用 shadcn/Tailwind v4 语义 token（bg-card、bg-accent、border-border、text-muted-foreground、border-l-primary），不新增硬编码颜色。
- 会话主标题统一走 getSessionTitle：过滤空白、$@ 和 [SYSTEM DIRECTIVE: 占位内容，避免污染列表与详情头部。
- 选中项使用 bg-accent/40 + 2px border-l-primary + shadow-sm，保持左栏宽度不变，同时给当前会话清晰锚点。
- 模型、短 session id、时间、轮数、文件数降级为 text-xs text-muted-foreground 元数据行，主标题保持 text-base font-medium。

## Task 3 Parser 决策 — 噪声过滤与 top-level content 支持

- 决定为 `initial_prompt` 和 `custom_title` 引入统一的 `is_noisy_text()` 过滤器，过滤规则：
  - 空/仅空白字符
  - 精确 `$@`
  - 以 `[SYSTEM DIRECTIVE:` 开头
  - 以 `[Request interrupted by user]` 开头（保留为 turn 文本供 UI 渲染，但不作为 prompt/title）
  - 包含 `<!-- OMO_INTERNAL_INITIATOR -->`
  - 包含 title-generator boilerplate（"You are a conversation title generator"）
- 决定 user 条目优先读取 top-level `content`，再回退到 `message.content`，确保跨版本 JSONL 兼容性。
- 决定噪声文本仍然注册为 `SessionTurn`，仅在设置 `initial_prompt` / `custom_title` 时过滤，保证 UI 能渲染完整对话流。
- 决定不修改前端 UI 文件，不引入新依赖，parser 改动完全局限于 Rust 后端。

## Wave 2 UI 决策 — Task 5/6/7

- 决定保留 L2 转录详情的文档式 section 结构，不引入 `MarkdownRenderer`，并补足 fenced code 的轻量解析：代码块直接输出 `<pre>` 且局部 `overflow-x-auto`。
- 决定 `ProjectMemoryPanel` 持有 `markedTurns` 状态，传入 `L2Panel` / `TranscriptDetailView`，避免每次渲染 `new Set()` 导致已标记态丢失。
- 决定候选记忆 id 使用 `${selectedSessionId}-${turnIndex}` 稳定键，并在 `setCandidates` 内按 id 去重，移除 `Date.now()` 带来的重复候选风险。
- 决定切换会话时根据现有 candidates 重建当前会话的 `markedTurns`，避免不同会话相同 turnIndex 的标记态互相污染。
- 决定 L2 桌面布局改为 `22rem + minmax(0,1fr)`，固定搜索列表宽度，把剩余空间优先给对话详情阅读区。
