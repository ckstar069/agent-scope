# ptv v0.2 开发笔记

## 2026-05-06: AgentInfo 新增 5 个字段

### 上下文
Wave 0 BLOCKER 任务：为前端 AgentMonitor 组件提供 `tool_calls`、`subagents`、`file_accesses`、`pending_since_ms`、`thinking_since_ms` 数据。

### 做了什么
- 在 `src-tauri/src/collectors/agent/mod.rs` 中新增 3 个可序列化结构体：
  - `SerToolCall { name, arg, duration_ms }`
  - `SerSubAgent { name, status, tokens }`
  - `SerFileAccess { path, operation, turn_index }`
- 在 `AgentInfo` 中添加 5 个新字段
- 更新 `session_to_info()` 映射逻辑
- 新增 4 个单元测试，原有的 6 个测试也全部保留并通过

### 关键决策
- **不修改 abtop-collector**：`AgentSession` 中的 `ToolCall`、`SubAgent`、`FileAccess`、`pending_since_ms`、`thinking_since_ms` 在 abtop-collector 侧已经存在，只需在 ptv 侧映射。
- **命名前缀 `Ser*`**：用于区分 abtop-collector 的原始类型和 ptv 侧的可序列化包装类型。
- **`FileOp` → `String` 映射**：使用 `fa.operation.to_string()` (impl Display)，返回 "R"/"W"/"E"。

### 测试结果
```
running 10 tests
collectors::agent::tests::test_serializable_status_from ... ok
collectors::agent::tests::test_agent_collector_new ... ok
collectors::agent::tests::test_session_to_info_basic ... ok
collectors::agent::tests::test_session_to_info_empty_fields ... ok
collectors::agent::tests::test_register_unregister_project ... ok
collectors::agent::tests::test_build_payload_empty_projects ... ok
collectors::agent::tests::test_session_to_info_with_file_accesses ... ok
collectors::agent::tests::test_session_to_info_with_subagents ... ok
collectors::agent::tests::test_build_payload_mapping ... ok
collectors::agent::tests::test_session_to_info_with_tool_calls ... ok

test result: ok. 10 passed; 0 failed
```

## 2026-05-06: AgentSubTree 子 Agent 折叠树组件

### 上下文
- 前端需要独立组件展示后端 `SerSubAgent { name, status, tokens }` 数据，后续再集成到 AgentMonitor。

### 做了什么
- 新增 `src/components/AgentSubTree.tsx`。
- 组件接收 `subagents: { name: string; status: string; tokens: number }[]`，空数组返回 `null`。
- 默认折叠，根节点显示 `子 Agent (N)`，展开后一级展示子 Agent。
- 状态图标映射：`working/in_progress` 使用蓝色旋转 `Loader2`，`completed/done` 使用绿色 `CheckCircle2`，其他使用灰色 `Circle`。
- Token 按任务约定格式化为原值、`K`、`M`。

### 验证结果
- `AgentSubTree.tsx` LSP diagnostics：无错误。
- `npm run build` 已执行 2 次；先暴露并修复了 `src/hooks/useTheme.ts` 中未使用事件参数。
- 第二次构建仍报 `src/components/Layout.tsx` 的 `useTheme`、`ThemeToggle` 未使用，达到本轮最多 2 次状态检查限制后停止。

## 2026-05-06: AgentToolTimeline 独立组件

### 上下文
Wave 2 Task 2.0：为 AgentMonitor 后续集成准备工具调用时间线组件，仅创建独立前端组件，不修改现有页面。

### 做了什么
- 新增 `src/components/AgentToolTimeline.tsx`，接收 `tool_calls`、`pending_since_ms`、`thinking_since_ms`。
- 按现有 AgentMonitor 行内风格使用 Tailwind token：`border-border`、`bg-background/60`、`bg-muted`、`text-muted-foreground`。
- 实现工具名称映射、参数 20 字符截断、耗时格式化、比例条宽度和最长调用 `*` 标记。
- 当存在 pending/thinking 时间戳时在顶部渲染 `Thinking...` 半透明虚拟行，使用组件内 `@keyframes pulse`。

### 验证结果
- `lsp_diagnostics`：`AgentToolTimeline.tsx` 无诊断。
- `npm run build`：通过。

## 2026-05-06: Light/Dark 主题切换实现

### 上下文
Wave 1 任务：创建 `useTheme` hook 和 `ThemeToggle` 按钮组件，支持 light/dark/system 三态循环切换。

### 做了什么
- 新建 `src/hooks/useTheme.ts`：
  - 从 `localStorage("ptv-theme")` 读取初始主题
  - `applyTheme()` 根据主题类型操作 `document.documentElement.classList`
  - `system` 模式监听 `prefers-color-scheme` 媒体查询变化
  - `toggleTheme()` 循环 light → dark → system
- 新建 `src/components/ThemeToggle.tsx`：
  - 使用 shadcn/ui `Button` (variant="ghost", size="icon")
  - 图标：`Sun`（light）、`Moon`（dark）、`Monitor`（system）
  - 带 `title` 和 `aria-label` 无障碍属性
- 修改 `src/components/Layout.tsx`：
  - 顶部调用 `useTheme()` 初始化主题副作用
  - 在 main 区域右上角渲染 `<ThemeToggle />`

### 关键实现细节
- **system 模式的动态响应**：仅在 `theme === "system"` 时才响应媒体查询变化，避免 light/dark 手动选择后被系统覆盖
- **localStorage 持久化**：每次主题变化后立即写入，刷新页面可恢复
- **CSS 变量切换**：`index.css` 已定义 `:root`（浅色）和 `.dark`（深色）变量，通过 `html.dark` 类名切换即可生效
- **Tailwind v4 支持**：`@custom-variant dark (&:is(.dark *))` 已配置，所有 `dark:` variant 会自动生效

### 构建验证
`npm run build` 通过，无 TypeScript 错误。

## 2026-05-06: AgentFileAudit 文件审计组件

### 上下文
Wave 2 独立组件任务：为后续 AgentMonitor 集成准备文件操作审计列表，仅创建 `src/components/AgentFileAudit.tsx`，不接入页面。

### 做了什么
- 新增 `AgentFileAudit` 组件，props 结构为 `file_accesses: { path, operation, turn_index }[]`。
- 空数组直接 `return null`，避免无审计数据时占位。
- 顶部摘要显示原始操作总数与按 `path` 去重后的文件数。
- 使用 `Map` 按 `path` 去重，相同路径保留 `turn_index` 最大的一条作为最近操作。
- 路径仅展示最后 30 个字符，超过时用 `...` 前缀，避免长路径撑开 UI。
- 列表按 `turn_index` 升序展示最近记录，去重后超过 50 条时仅显示最后 50 条，并提示 `…及另外 N 条`。

### 样式约定
- R/W/E 标签分别使用蓝/绿/橙色，并补充 dark 模式类：
  - R: `bg-blue-100 text-blue-700 dark:bg-blue-500/15 dark:text-blue-300`
  - W: `bg-green-100 text-green-700 dark:bg-green-500/15 dark:text-green-300`
  - E: `bg-orange-100 text-orange-700 dark:bg-orange-500/15 dark:text-orange-300`
- 组件整体采用现有 Nova/shadcn 风格：`border-border`、`bg-muted/*`、`text-muted-foreground`、圆角卡片密集布局。

### 验证结果
```
lsp_diagnostics src/components/AgentFileAudit.tsx: No diagnostics found
npm run build: tsc && vite build 通过
```

## 2026-05-06: AgentMonitor v0.2 E2E 增强测试

### 上下文
- 为 `e2e/agent-monitor.spec.ts` 补充搜索框、搜索空状态、主题按钮、无 Agent 控件边界测试。
- 浏览器 E2E 默认无 Tauri 后端，`agent-update` 不会自然触发。

### 关键发现
- 当前 `AgentMonitor` 在 `totalSessions === 0` 时优先展示“暂无活跃 Agent”，因此“没有匹配的会话”分支需要至少存在 1 个 Agent 快照才能覆盖。
- 在单个测试内通过轻量模拟 `window.__TAURI_INTERNALS__` 的 `plugin:event|listen` 注入一个最小 `agent-update` 快照，可只覆盖过滤空状态，不影响无 Agent 边界测试。
- ThemeToggle 初始默认值可能是 `system`，按钮 `aria-label` 为“跟随系统”，测试选择器需要同时兼容 `*模式` 和“跟随系统”。

### 验证结果
- `lsp_diagnostics e2e/agent-monitor.spec.ts`：无诊断。
- `npm test`：AgentMonitor 8 项全部通过；项目整体为 32 passed / 1 failed，失败项为既有 `navigation.spec.ts` 的 `div.dark` 断言，与本次修改文件无关。

## 2026-05-06: AgentMonitor 手风琴详情与搜索过滤

### 上下文
- Wave 2 Task 2.1/2.2：将已完成的 `AgentToolTimeline`、`AgentSubTree`、`AgentFileAudit` 集成到 `AgentMonitor`，并支持会话搜索过滤。

### 做了什么
- 在 `AgentMonitor` 增加 `expandedSessionId`，通过 `handleToggle()` 保证同一时间只展开一个 session。
- `AgentSessionRow` 改为手风琴行：折叠显示 `ChevronRight`，展开显示 `ChevronDown`，展开态添加左侧 `border-l-2 border-l-primary`。
- 展开区域使用 `max-height + opacity` 的 300ms CSS transition，内部使用三枚 button 实现简单 Tab：工具调用 / 子 Agent / 文件审计。
- 三个详情组件按后端新增字段直接接入，并对空数组补充轻量空状态。
- 页面顶部新增搜索框，实时过滤 `session_id`、`project_name`、`model`、`status`、`cwd`，展示 `N/M` 匹配计数；无匹配时显示空状态和清空按钮。

### 实现细节
- 行主体使用完整宽度 `<button>` 承载点击与键盘可访问性，详情 Tab 区域放在按钮外，避免按钮嵌套问题。
- 过滤变化通过 `useEffect` 自动折叠已展开 session，避免筛选后保留不可见展开态。

## 2026-05-07: MarkdownRenderer 与目录导航

### 上下文
- 为后续 ProjectMemoryPanel 集成准备纯渲染组件：输入 Markdown 字符串，输出带左侧目录的文档视图。

### 做了什么
- 安装 `react-markdown` 与 `remark-gfm`；`react-markdown@10.0.0` peerDependencies 为 `react >=18`、`@types/react >=18`，可兼容当前 React 19。
- 新增 `src/components/MarkdownRenderer.tsx`，props 为 `content: string`、`className?: string`。
- 自动扫描非代码围栏内的 h1-h4 标题，生成 `{ level, text, id }[]` 目录，重复标题追加序号后缀。
- 左侧目录使用现有 `ScrollArea`，点击通过 `scrollIntoView({ behavior: "smooth" })` 滚动到对应 heading。
- 空内容渲染 Skeleton loading；由于项目尚无 `ui/skeleton.tsx`，按 shadcn/ui 风格补充最小 `Skeleton` primitive。
- Markdown 内容使用 `react-markdown + remark-gfm`，保留 Tailwind prose 类，并为代码块、表格、链接使用现有 token 样式补强。

### 验证结果
- `lsp_diagnostics src/components`：无诊断。
- `npm run build`：通过。

## 2026-05-07: MemoryFileTree 记忆文件树组件

### 上下文
- Wave 任务：新增独立 `src/components/MemoryFileTree.tsx`，供后续 ProjectMemoryPanel L1 集成使用。

### 做了什么
- 通过 `npx shadcn@latest add collapsible` 新增 `src/components/ui/collapsible.tsx`。
- 新增 `MemoryFileTree` 组件，接收 `files`、`selectedPath`、`onSelect`、`changedPaths`。
- 按 `root → rules → notepads → plans → drafts → docs` 固定顺序分组，组内按 `relative_path` 排序。
- 使用 shadcn/ui `Collapsible` 实现每组折叠；文件项只展示从 `relative_path` 提取的文件名，点击回传原始 `relative_path`。
- 选中文件使用 `bg-accent` 高亮，变更文件使用 `bg-destructive` 小圆点提示；空数组展示“此项目未找到记忆文件”。

### 验证结果
- `lsp_diagnostics src/components/MemoryFileTree.tsx`：无诊断。
- `lsp_diagnostics src/components/ui/collapsible.tsx`：无诊断。
- `npm run build`：通过。
