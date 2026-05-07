# ptv v0.2 — AgentMonitor 增强

## TL;DR

> **快速摘要**：为 AgentMonitor 面板增加 3 个从 abtop 借鉴的核心功能：Tool Call 时间线（水平条形图）、子 Agent 树 + 文件审计日志、会话过滤/搜索 + 主题切换。
>
> **交付物**：
> - Rust 后端：`AgentInfo` 补充 `tool_calls`/`subagents`/`file_accesses` 字段 + 序列化映射
> - Tool Call 时间线：彩色水平条形图，`Thinking` 虚拟行，最长工具标记
> - 子 Agent 树：可折叠树形视图，工作状态 + token 消耗
> - 文件审计日志：R/W/E 颜色标签，去重文件列表
> - 内联展开/折叠交互：只展开一个 session（手风琴式）
> - Session 实时过滤搜索框
> - Light/Dark 主题切换 + `prefers-color-scheme` + localStorage 持久化
> - **macOS + Linux** 双平台
>
> **预估工作量**：Medium（6 个开发任务 + 4 个验证）
> **并行执行**：YES — 4 波次，峰值 4 个并行任务
> **关键路径**：Task 0 → Task 1.1/1.2/1.3/1.4 → Task 2.1/2.2 → Task 3.1/3.2/3.3 → F1-F4

---

## Context

### 原始需求

用户对比 abtop 后，选择 3 个优先功能加入 ptv AgentMonitor 面板：
1. **Tool Call 时间线** — Agent 工具调用水平条形图
2. **子 Agent 树 + 文件审计** — Sub-agent 嵌套结构 + 文件操作日志
3. **会话过滤/搜索 + 主题** — 实时文本过滤 + light/dark 切换

### 访谈摘要

**关键讨论**：
- 展开交互：**内联展开（手风琴式）**— 同时只展开一个 session
- Theme 范围：**仅 light/dark + 跟随系统** — 最小范围，localStorage 持久化
- Timeline/FileAudit 共存：**Tab 切换** — 展开区域内用小 Tab 切换 Timeline / SubAgents / FileAudit
- 测试策略：**Agent QA（Playwright E2E）**— 和 v0.1 一致
- 过滤范围：纯前端，搜索 session_id/project/model/status/cwd

### 研究结论

- `abtop-collector` 的 `AgentSession` 完整包含 `tool_calls`、`subagents`、`file_accesses`，但 `session_to_info()` 未传输到 `AgentInfo`
- abtop 的参考模式：`tool_label()` 名称映射、`tool_color()` 颜色映射、`draw_timeline()` 条形图、"Thinking" 虚拟行、* 最长标记
- abtop 文件审计：去重显示 `unique_files` 计数 + 操作码(R/W/E) + 路径 + turn_index
- `MAX_FILE_ACCESSES = 1000`，需截断或虚拟滚动

### Metis 审查

**识别的关键问题**（已解决）：
- **BLOCKER**：后端数据管道断层 — 计划 Phase 0 优先处理
- **交互模式**：用户选定手风琴式内联展开 ✓
- **性能策略**：tool_calls 截断（最近 N 条），file_accesses 去重显示
- **范围防护**：Theme 与核心功能无耦合，独立实现

---

## Work Objectives

### 核心目标

增强 AgentMonitor 面板，让用户能可视化 Agent 的工具调用时间线、子 Agent 层次结构、文件操作历史，并通过过滤和主题提升可用性。

### 具体交付物

- `src-tauri/src/collectors/agent/mod.rs` — `AgentInfo` 新增 3 个字段 + 序列化映射
- `src/pages/AgentMonitor.tsx` — 重构：内联展开 + Tab 切换 + 过滤搜索框
- `src/components/AgentToolTimeline.tsx` — Tool Call 时间线条形图组件
- `src/components/AgentSubTree.tsx` — 子 Agent 树组件
- `src/components/AgentFileAudit.tsx` — 文件审计日志组件
- `src/components/ThemeToggle.tsx` — 主题切换按钮
- `src/hooks/useTheme.ts` — 主题管理 hook（localStorage + prefers-color-scheme）
- `e2e/agent-monitor.spec.ts` — E2E 测试更新

### Must Have

- 当 session 有 tool_calls 时，展开区域内渲染 Tool Call Timeline
- "Thinking" 虚拟行在模型生成期间显示脉动动画
- 最长的 tool call 标记 * 号
- 子 Agent 树显示 name/status/tokens
- 文件审计显示 R/W/E 标签 + 文件路径
- 过滤文本实时搜索 session_id/project/model/status
- 主题切换后所有 UI 颜色立即更新，刷新后保持

### Must NOT Have（护栏）

- 不同时展开多个 session（手风琴互斥）
- 不将所有功能塞入紧凑的 AgentSessionRow（必须展开）
- 不在 Rust 传输数据完成前开始前端 UI
- 不添加速率限制面板（用户未选择）
- 不添加 MCP 面板（用户未选择）
- 不引入新的外部库（除已有的 Recharts）
- 不修改本项目的测试策略（继续 Agent QA）

---

## Verification Strategy

> **零人工干预** — 所有验证由 Agent 执行。

### 测试决策

- **基础设施存在**：YES（Playwright E2E + Rust unit tests）
- **自动化测试**：Agent QA（E2E） + Rust cargo test
- **框架**：Playwright（前端 E2E） + Rust `#[cfg(test)]`（后端）

### QA 策略

每个任务必须包含 Agent 可执行的 QA 场景。证据保存到 `.sisyphus/evidence/task-{N}-{scenario-slug}.{ext}`。

- **Rust 后端**：`cargo test -p ptv -- agent` 验证数据管道
- **前端 UI**：Playwright 打开 Tauri 窗口，操作 UI，断言 DOM
- **API/事件**：`invoke()` 或监听 `agent-update` 事件验证数据格式

---

## Execution Strategy

### Parallel Execution Waves

```
Wave 0 (立即开始 — BLOCKER 后端改造):
├── Task 0: AgentInfo + session_to_info 数据管道修补

Wave 1 (Wave 0 完成后 — MAX PARALLEL，4 个并行):
├── Task 1.1: 前端类型定义 + Tool Call Timeline 组件
├── Task 1.2: Sub-Agent Tree 组件
├── Task 1.3: File Audit 日志组件
├── Task 1.4: 主题切换（Theme Toggle + useTheme）

Wave 2 (Wave 1 完成后 — 集成):
├── Task 2.1: AgentSessionRow 内联展开（手风琴式）
├── Task 2.2: Tab 切换 + Session 过滤搜索
├── Task 2.3: Playwright E2E 测试

Wave FINAL (ALL tasks 完成后 — 4 并行审查):
├── Task F1: Plan Compliance Audit (oracle)
├── Task F2: Code Quality Review (unspecified-high)
├── Task F3: Real Manual QA (unspecified-high + playwright)
├── Task F4: Scope Fidelity Check (deep)
→ 呈现结果 → 获取用户明确确认

关键路径: Task 0 → Task 1.1 → Task 2.1 → Task 2.2 → Task 2.3 → F1-F4
并行加速: 约 40% 比串行快
最大并行: 4 (Wave 1)
```

### Agent 调度摘要

- **W0**: 1 — T0 → `unspecified-high`
- **W1**: 4 — T1.1/T1.2  → `visual-engineering`，T1.3 → `visual-engineering`，T1.4 → `unspecified-low`
- **W2**: 3 — T2.1 → `visual-engineering`，T2.2 → `quick`，T2.3 → `visual-engineering`（+ playwright skill）
- **FINAL**: 4 — F1 → `oracle`，F2 → `unspecified-high`，F3 → `unspecified-high`，F4 → `deep`

---

## TODOs

- [x] 0. **BLOCKER: 后端数据管道修补 — AgentInfo + session_to_info**

  **What to do**：
  - 在 `AgentInfo` struct 中添加 3 个字段：
    ```rust
    pub tool_calls: Vec<SerToolCall>,
    pub subagents: Vec<SerSubAgent>,
    pub file_accesses: Vec<SerFileAccess>,
    ```
  - 在 `agent/mod.rs` 顶部定义可序列化类型：
    ```rust
    #[derive(Debug, Clone, Serialize)]
    pub struct SerToolCall { pub name: String, pub arg: String, pub duration_ms: u64 }
    #[derive(Debug, Clone, Serialize)]
    pub struct SerSubAgent { pub name: String, pub status: String, pub tokens: u64 }
    #[derive(Debug, Clone, Serialize)]
    pub struct SerFileAccess { pub path: String, pub operation: String, pub turn_index: u32 }
    ```
  - 在 `session_to_info()` 中映射这 3 个字段（从 `AgentSession` → `AgentInfo`）
  - 更新现有测试 `test_session_to_info_basic` 和 `test_build_payload_mapping` 以验证新字段正确传输
  - 新增测试：验证 `tool_calls` 为空 vec 时不崩溃、`subagents` 为空 vec 时不崩溃、`file_accesses` 为空 vec 时不崩溃

  **Must NOT do**：
  - 不添加速率限制、MCP 服务器等其他 abtop 字段
  - 不修改 `AgentSession` 本身

  **Recommended Agent Profile**：
  > 后端 Rust 修改 + 测试，需要熟悉 abtop-collector 数据模型。
  - **Category**：`unspecified-high`
    - Reason：需要理解 abtop-collector 的 AgentSession 模型 + 编写 Rust 单元测试
  - **Skills**：`[]`

  **Parallelization**：
  - **Can Run In Parallel**: NO（BLOCKER，必须先完成）
  - **Parallel Group**: Wave 0（单独）
  - **Blocks**: Task 1.1, 1.2, 1.3, 1.4
  - **Blocked By**: None

  **References**：
  - `abtop-collector/src/model/session.rs:91-106` — `SubAgent`、`ToolCall` 结构体定义
  - `abtop-collector/src/model/session.rs:24-30` — `FileAccess` 结构体定义
  - `src-tauri/src/collectors/agent/mod.rs:55-113` — 当前 `AgentInfo` struct
  - `src-tauri/src/collectors/agent/mod.rs:284-352` — 当前 `session_to_info()` 映射函数
  - `src-tauri/src/collectors/agent/mod.rs:428-700` — 现有测试函数（需更新）

  **Acceptance Criteria**：
  - [ ] `cargo test -p ptv -- agent` → PASS（所有 agent 测试通过）
  - [ ] 新测试 `test_session_to_info_with_tool_calls` 验证 tool_calls 正确传输
  - [ ] 新测试 `test_session_to_info_with_subagents` 验证 subagents 正确传输
  - [ ] 新测试 `test_session_to_info_with_file_accesses` 验证 file_accesses 正确传输
  - [ ] 新测试 `test_session_to_info_empty_fields` 验证空数组不崩溃

  **QA Scenarios**：
  ```
  Scenario: 后端数据管道 — happy path
    Tool: Bash
    Preconditions: Rust 编译环境可用
    Steps:
      1. cd src-tauri && cargo test -p ptv -- agent
      2. 验证输出包含 test_session_to_info_basic ... ok
      3. 验证输出包含 test_session_to_info_with_tool_calls ... ok
    Expected Result: 所有 agent 测试通过，包括新增的 3 个字段验证测试
    Evidence: .sisyphus/evidence/task-0-cargo-test.txt

  Scenario: 后端数据管道 — 空数据不崩溃
    Tool: Bash
    Preconditions: Rust 编译环境可用
    Steps:
      1. cd src-tauri && cargo test test_session_to_info_empty_fields
      2. 验证输出为 ok
    Expected Result: 空 tool_calls/subagents/file_accesses 不 panic
    Evidence: .sisyphus/evidence/task-0-empty-fields.txt
  ```

  **Commit**: YES
  - Message: `feat(backend): 为 AgentInfo 添加 tool_calls/subagents/file_accesses 字段`
  - Files: `src-tauri/src/collectors/agent/mod.rs`
  - Pre-commit: `cargo test -p ptv -- agent`

---

- [x] 1.1 **Tool Call Timeline 组件**

  **What to do**：
  - 创建 `src/components/AgentToolTimeline.tsx`
  - 创建 `src/types/agent.ts` — 定义前端 TypeScript 类型（`AgentInfo` 更新 + 新增 `SerToolCall`、`SerSubAgent`、`SerFileAccess`）
  - 实现 Tool Call 水平条形图：
    - 使用内联 CSS/自定义组件渲染（不需要 Recharts — timeline 条形图简单且 Recharts 对水平布局支持有限）
    - 每行：tool_name 标签（彩色背景） + arg 截断（20 字符） + 时长格式化 + 条形（width 比例映射到 duration）
    - `TOOL_LABEL_MAP`：`exec_command→Bash`、`read_file→Read`、`write_to_file→Write`、`edit_file→Edit`、`search→Grep`、`update_plan→Plan`、其他→原始名截断
    - `TOOL_COLOR_MAP`：每种 tool 分配不同的 Tailwind 颜色 class
  - 实现 "Thinking" 虚拟行：
    - 当 `pending_since_ms > 0` 或 `thinking_since_ms > 0` 时，在 timeline 顶部显示一条半透明脉冲行
    - 脉冲动画：CSS `@keyframes pulse` 1.5s 循环 opacity 0.3 → 0.7
  - 最长的 tool call 在右侧显示 `*` 标记
  - `duration_ms` 格式化：`<1s`、`1.2s`、`2m 30s`
  - 当 `tool_calls` 为空时不渲染组件（return null）

  **Must NOT do**：
  - 不引入 Recharts（Timeline 条形图用纯 CSS/HTML 更轻量）
  - 不实现 abtop 的完整 TUI 滚动逻辑（不需要虚拟滚动，50 条以内直接渲染）

  **Recommended Agent Profile**：
  > 前端 UI 组件，涉及自定义 CSS 动画和布局。
  - **Category**：`visual-engineering`
    - Reason：自定义 CSS 条形图 + 脉冲动画 + 颜色映射表
  - **Skills**：`[]`

  **Parallelization**：
  - **Can Run In Parallel**: YES（与 1.2, 1.3, 1.4 并行）
  - **Parallel Group**: Wave 1
  - **Blocks**: Task 2.1
  - **Blocked By**: Task 0

  **References**：
  - `abtop/src/ui/sessions.rs:930-990` — abtop 的 `draw_timeline()` 参考实现（"Thinking" 行、* 最长标记）
  - `abtop/src/ui/sessions.rs:1010-1030` — `tool_label()` 名称映射、`tool_color()` 颜色映射
  - `abtop/src/ui/sessions.rs:1035-1045` — `fmt_duration()` 时长格式化
  - `src/pages/AgentMonitor.tsx:280-325` — 当前 AgentSessionRow 布局（了解嵌入位置）

  **Acceptance Criteria**：
  - [ ] 组件接收 `tool_calls: SerToolCall[]` prop
  - [ ] 至少 1 条 tool call 时渲染条形图
  - [ ] 每条显示：tool name（颜色标签）、arg（截断 20 字符）、时长（格式化）、宽度比例条
  - [ ] 最长的 tool call 右侧显示 `*`
  - [ ] `pending_since_ms > 0` 时显示 "Thinking" 脉冲行
  - [ ] tool_calls 为空时 return null（不渲染）
  - [ ] `npm run build` 无 TypeScript 错误

  **QA Scenarios**：
  ```
  Scenario: Tool Call Timeline — 有 tool_calls 时正常渲染
    Tool: Playwright
    Preconditions: Tauri dev 模式运行，AgentMonitor 页面打开，有活跃 Agent session 且有 tool_calls
    Steps:
      1. 点击 AgentSessionRow 展开详情
      2. 验证 .agent-tool-timeline 容器存在
      3. 验证至少 1 条 .tool-bar-row 渲染
      4. 验证每条包含 .tool-label（tool name 标签）和 .tool-duration（时长）
      5. 截图保存
    Expected Result: Timeline 至少渲染 1 条 tool call 条形
    Evidence: .sisyphus/evidence/task-1.1-timeline-render.png

  Scenario: Tool Call Timeline — Thinking 虚拟行显示
    Tool: Playwright
    Preconditions: 同上，Agent 处于 Thinking 状态
    Steps:
      1. 展开 session 详情
      2. 验证 .thinking-row 存在
      3. 验证 .thinking-row 有 CSS pulse 动画（opacity 变化）
    Expected Result: Thinking 行可见且有脉动效果
    Evidence: .sisyphus/evidence/task-1.1-thinking-row.png

  Scenario: Tool Call Timeline — 无 tool_calls 时不渲染
    Tool: Playwright
    Preconditions: session 的 tool_calls 为空数组
    Steps:
      1. 展开 session 详情
      2. 验证 .agent-tool-timeline 不存在（count=0）
    Expected Result: 无 timeline 区域
    Evidence: .sisyphus/evidence/task-1.1-empty-toolcalls.png
  ```

  **Commit**: YES
  - Message: `feat(ui): 添加 Tool Call Timeline 水平条形图组件`
  - Files: `src/components/AgentToolTimeline.tsx`, `src/types/agent.ts`
  - Pre-commit: `npm run build`

---

- [x] 1.2 **Sub-Agent Tree 组件**

  **What to do**：
  - 创建 `src/components/AgentSubTree.tsx`
  - 实现可折叠树形结构：
    - 根节点：显示 subagents 总数
    - 子节点：缩进，显示名称 + status 图标（working=● ● spinner、done=✓） + tokens 数
    - 使用 shadcn `Collapsible` 组件实现展开/折叠
  - Status 状态映射：
    - `"working"` / `"in_progress"` → ● 蓝色 spinner 图标
    - `"completed"` / `"done"` → ✓ 绿色对勾图标
    - 其他 → ○ 灰色圆圈
  - Token 数格式化：`12.5K`、`1.2M`
  - 当 `subagents` 为空时不渲染组件

  **Must NOT do**：
  - 不实现无限嵌套（一级子 Agent 即可）

  **Recommended Agent Profile**：
  > 前端 UI 组件，树形结构 + Collapsible。
  - **Category**：`visual-engineering`
    - Reason：树形布局 + shadcn Collapsible + 状态图标映射
  - **Skills**：`[]`

  **Parallelization**：
  - **Can Run In Parallel**: YES（与 1.1, 1.3, 1.4 并行）
  - **Parallel Group**: Wave 1
  - **Blocks**: Task 2.1
  - **Blocked By**: Task 0

  **References**：
  - `abtop/src/ui/sessions.rs:540-570` — abtop 的子 Agent 渲染参考
  - `https://ui.shadcn.com/docs/components/collapsible` — shadcn Collapsible API
  - `src/pages/AgentMonitor.tsx` — 现有状态徽章渲染（参考设计语言）

  **Acceptance Criteria**：
  - [ ] 组件接收 `subagents: SerSubAgent[]` prop
  - [ ] 显示 subagents 总数
  - [ ] 每个节点显示 name、status 图标、tokens
  - [ ] working 状态显示蓝色 spinner
  - [ ] done 状态显示绿色对勾
  - [ ] 点击展开/折叠
  - [ ] subagents 为空时不渲染
  - [ ] `npm run build` 无 TypeScript 错误

  **QA Scenarios**：
  ```
  Scenario: Sub-Agent Tree — 有 subagents 时正常渲染
    Tool: Playwright
    Preconditions: Tauri dev 模式，AgentMonitor 页面打开，有活跃 session 且 subagents 非空
    Steps:
      1. 展开 session 详情
      2. 切换 Tab 到 "Sub-Agents"
      3. 验证 .agent-sub-tree 容器存在
      4. 验证至少 1 个 .sub-agent-node 渲染
      5. 验证包含 working/done 状态图标
      6. 验证显示 token 数
    Expected Result: 树形结构正确渲染
    Evidence: .sisyphus/evidence/task-1.2-subagent-tree.png

  Scenario: Sub-Agent Tree — 空数组不渲染
    Tool: Playwright
    Preconditions: session 的 subagents 为空数组
    Steps:
      1. 展开 session 详情
      2. 切换 Tab 到 "Sub-Agents"
      3. 验证 .agent-sub-tree 不存在（count=0）
    Expected Result: 无 Sub-Agent Tree 区域
    Evidence: .sisyphus/evidence/task-1.2-empty-subagents.png
  ```

  **Commit**: YES
  - Message: `feat(ui): 添加 Sub-Agent Tree 可折叠树组件`
  - Files: `src/components/AgentSubTree.tsx`
  - Pre-commit: `npm run build`

---

- [x] 1.3 **File Audit 日志组件**

  **What to do**：
  - 创建 `src/components/AgentFileAudit.tsx`
  - 实现文件操作日志：
    - 顶部摘要：总操作数 + 去重文件数（`unique_files` 计数）
    - 每行：操作标签（R/W/E）+ 文件路径（显示最后 2 级目录）+ turn_index
    - R（Read）→ 蓝色标签、W（Write）→ 绿色标签、E（Edit）→ 橙色标签
    - 去重显示：相同文件多次操作只显示最近一次（按 `turn_index` 排序）
    - 最大显示 50 条，超出时显示 "…及另外 N 条"
  - 文件路径截断：显示 `…src/utils.ts`（保留最后 30 字符）
  - 当 `file_accesses` 为空时不渲染组件

  **Must NOT do**：
  - 不显示完整文件路径（过长）
  - 不实现虚拟滚动（50 条限制，不需要）

  **Recommended Agent Profile**：
  > 前端日志列表组件，去重逻辑 + 颜色标签。
  - **Category**：`visual-engineering`
    - Reason：日志列表布局 + 去重逻辑 + 操作颜色标签
  - **Skills**：`[]`

  **Parallelization**：
  - **Can Run In Parallel**: YES（与 1.1, 1.2, 1.4 并行）
  - **Parallel Group**: Wave 1
  - **Blocks**: Task 2.1
  - **Blocked By**: Task 0

  **References**：
  - `abtop/src/ui/sessions.rs:791-830` — abtop 的 `draw_file_audit()` 参考（去重逻辑 + 操作标签）
  - `abtop-collector/src/model/session.rs:24-30` — FileAccess 字段定义（path, operation, turn_index）
  - `src/pages/AgentMonitor.tsx:280-325` — 现有 AgentSessionRow 设计语言

  **Acceptance Criteria**：
  - [ ] 组件接收 `file_accesses: SerFileAccess[]` prop
  - [ ] 顶部显示总操作数 + 唯一文件数
  - [ ] 每条记录显示 R/W/E 彩色标签 + 截断路径 + turn_index
  - [ ] 相同文件去重（只显示最近一次）
  - [ ] 超过 50 条时截断并显示 "…及另外 N 条"
  - [ ] file_accesses 为空时不渲染
  - [ ] `npm run build` 无 TypeScript 错误

  **QA Scenarios**：
  ```
  Scenario: File Audit — 有 file_accesses 时正常渲染
    Tool: Playwright
    Preconditions: Tauri dev 模式，AgentMonitor 页面，session 有 file_accesses 记录
    Steps:
      1. 展开 session 详情
      2. 切换 Tab 到 "File Audit"
      3. 验证 .agent-file-audit 容器存在
      4. 验证顶部显示操作数 + 唯一文件数
      5. 验证每条有 R/W/E 标签 + 截断路径 + turn_index
      6. 截图保存
    Expected Result: 文件审计日志正确渲染
    Evidence: .sisyphus/evidence/task-1.3-file-audit.png

  Scenario: File Audit — 相同文件去重
    Tool: Playwright
    Preconditions: session 的 file_accesses 包含同一文件多次操作
    Steps:
      1. 展开 session 详情
      2. 切换 Tab 到 "File Audit"
      3. 验证同一路径只出现一次
      4. 验证操作标签显示最近一次的操作类型
    Expected Result: 去重生效
    Evidence: .sisyphus/evidence/task-1.3-dedup.png

  Scenario: File Audit — 空数组不渲染
    Tool: Playwright
    Preconditions: file_accesses 为空数组
    Steps:
      1. 展开 session 详情
      2. 切换 Tab 到 "File Audit"
      3. 验证 .agent-file-audit 不存在
    Expected Result: 无 File Audit 区域
    Evidence: .sisyphus/evidence/task-1.3-empty.png
  ```

  **Commit**: YES
  - Message: `feat(ui): 添加 File Audit 文件操作日志组件`
  - Files: `src/components/AgentFileAudit.tsx`
  - Pre-commit: `npm run build`

---

- [x] 1.4 **主题切换（Theme Toggle + useTheme）**

  **What to do**：
  - 创建 `src/hooks/useTheme.ts`：
    - 读取 `localStorage("ptv-theme")` 或 `prefers-color-scheme`
    - 应用 `document.documentElement.classList.toggle("dark")`
    - 监听 `matchMedia("(prefers-color-scheme: dark)")` 变化
    - 导出 `{ theme, setTheme, toggleTheme }`（theme 类型：`"light" | "dark" | "system"`）
  - 创建 `src/components/ThemeToggle.tsx`：
    - 按钮组件，点击循环切换 light → dark → system
    - 使用 `Sun`/`Moon`/`Monitor` 图标（lucide-react）
    - 放在 Layout 顶部或 Settings 页面中
  - 在 `Layout.tsx` 或 `App.tsx` 中调用 `useTheme()` 初始化
  - 确保 `index.css` 的 `.dark` 变量完整

  **Must NOT do**：
  - 不创建复杂主题选择器（仅 3 选项循环切换）
  - 不修改 shadcn/ui 的 Nova 主题变量

  **Recommended Agent Profile**：
  > 纯前端 hook + 按钮组件，无后端依赖。
  - **Category**：`unspecified-low`
    - Reason：简单 hook + 按钮，工作量小
  - **Skills**：`[]`

  **Parallelization**：
  - **Can Run In Parallel**: YES（与 1.1, 1.2, 1.3 并行，无依赖）
  - **Parallel Group**: Wave 1
  - **Blocks**: None（独立功能）
  - **Blocked By**: None

  **References**：
  - `src/index.css:1-20` — 当前 `:root` 和 `.dark` CSS 变量定义
  - `src/components/Layout.tsx` — 当前布局（插入 ThemeToggle 的位置）
  - `https://tailwindcss.com/docs/dark-mode` — Tailwind v4 dark mode 文档

  **Acceptance Criteria**：
  - [ ] `useTheme()` hook 正确读取 localStorage、prefers-color-scheme
  - [ ] `toggleTheme()` 循环 light → dark → system
  - [ ] 切换后 `document.documentElement.classList` 立即更新
  - [ ] 系统主题变化时自动跟随（system 模式下）
  - [ ] 刷新页面后保持上次选择
  - [ ] ThemeToggle 按钮在页面可见位置

  **QA Scenarios**：
  ```
  Scenario: Theme Toggle — light → dark 切换
    Tool: Playwright
    Preconditions: 页面加载，默认浅色主题
    Steps:
      1. 点击 .theme-toggle 按钮
      2. 验证 document.documentElement.classList 包含 "dark"
      3. 验证 body 背景色变为深色
      4. 刷新页面
      5. 验证 dark 类仍然存在（localStorage 持久化）
    Expected Result: 主题正确切换并持久化
    Evidence: .sisyphus/evidence/task-1.4-theme-switch.png

  Scenario: Theme Toggle — 跟随系统
    Tool: Playwright
    Preconditions: 页面 system 模式
    Steps:
      1. 使用 Playwright 模拟 prefers-color-scheme: dark
      2. 验证 dark 类自动添加
    Expected Result: 系统主题变化自动响应
    Evidence: .sisyphus/evidence/task-1.4-system-theme.png
  ```

  **Commit**: YES
  - Message: `feat(ui): 添加 Light/Dark 主题切换 + useTheme hook`
  - Files: `src/components/ThemeToggle.tsx`, `src/hooks/useTheme.ts`
  - Pre-commit: `npm run build`

- [x] 2.1 **AgentSessionRow 内联展开（手风琴式）**

  **What to do**：
  - 重构 `AgentMonitor.tsx` 的 `AgentSessionRow`：
    - 添加 `expandedSessionId` 状态（string | null）
    - 点击行 → 设置 `expandedSessionId`，其他行自动折叠（互斥）
    - 展开区域在选中行下方渲染（CSS transition：`max-height` 动画 300ms）
    - 展开区域内放 3 个小型 Tab：`工具调用` | `子 Agent` | `文件审计`
    - Tab 组件使用自定义简单实现（button group + 内容区域显示/隐藏），不引入额外库
  - 更新 AgentSessionRow 视觉：
    - 折叠时：`cursor-pointer` + hover 背景变化（`hover:bg-muted/50`）
    - 展开时：左侧蓝色竖线（`border-l-2 border-primary`）标识选中状态
    - 展开图标：折叠时 `ChevronRight`，展开时 `ChevronDown`
  - 将 Tool Call Timeline / Sub-Agent Tree / File Audit 组件集成到展开区域的对应 Tab 中
  - 过滤后自动折叠所有展开项（`expandedSessionId = null`）

  **Must NOT do**：
  - 不同时展开多个 session
  - 不使用第三方 Tab 库

  **Recommended Agent Profile**：
  > 前端重构，涉及状态管理 + CSS 动画 + 组件集成。
  - **Category**：`visual-engineering`
    - Reason：手风琴动画 + 状态管理 + 布局重构
  - **Skills**：`[]`

  **Parallelization**：
  - **Can Run In Parallel**: NO（依赖 1.1-1.4 所有组件）
  - **Parallel Group**: Wave 2
  - **Blocks**: Task 2.2
  - **Blocked By**: Task 1.1, 1.2, 1.3

  **References**：
  - `src/pages/AgentMonitor.tsx:280-325` — 当前 AgentSessionRow（重构目标）
  - `src/components/AgentToolTimeline.tsx` — Task 1.1 产出
  - `src/components/AgentSubTree.tsx` — Task 1.2 产出
  - `src/components/AgentFileAudit.tsx` — Task 1.3 产出
  - `src/pages/ProjectDetail.tsx` — 参考现有 Tab 切换实现

  **Acceptance Criteria**：
  - [ ] 点击折叠行 → 展开详情区域（动画 300ms）
  - [ ] 点击另一个行 → 前一个折叠，新行展开（互斥）
  - [ ] 展开时显示 `ChevronDown` 图标 + 左侧蓝色竖线
  - [ ] 展开区域包含 3 个 Tab 切换
  - [ ] 过滤文本变化时自动折叠
  - [ ] `npm run build` 无 TypeScript 错误

  **QA Scenarios**：
  ```
  Scenario: 内联展开 — 手风琴互斥
    Tool: Playwright
    Preconditions: Tauri dev 模式，AgentMonitor 页面，至少 2 个 session
    Steps:
      1. 点击第一个 AgentSessionRow → 验证展开区域渲染
      2. 点击第二个 AgentSessionRow → 验证第一个折叠，第二个展开
      3. 验证同一时间只有一个 .expanded-session 存在
      4. 截图对比折叠/展开状态
    Expected Result: 互斥展开行为正确
    Evidence: .sisyphus/evidence/task-2.1-accordion.png

  Scenario: 内联展开 — 过滤后自动折叠
    Tool: Playwright
    Preconditions: 已展开一个 session
    Steps:
      1. 展开一个 session
      2. 在过滤搜索框输入文本
      3. 验证展开的 session 自动折叠
    Expected Result: 过滤触发折叠
    Evidence: .sisyphus/evidence/task-2.1-filter-collapse.png
  ```

  **Commit**: YES
  - Message: `refactor(ui): AgentSessionRow 改为手风琴式内联展开`
  - Files: `src/pages/AgentMonitor.tsx`
  - Pre-commit: `npm run build`

---

- [x] 2.2 **Tab 切换 + Session 过滤搜索**

  **What to do**：
  - 在 `AgentMonitor.tsx` 顶部（`<h1>` 下方）添加搜索输入框：
    - 输入框左侧 `Search` 图标（lucide）
    - 输入框右侧清空按钮（仅在有文本时显示）
    - Placeholder："搜索会话 ID、项目、模型、状态…"
  - 实现过滤逻辑：
    - 使用 `useDeferredValue` 或 `useMemo` + 300ms debounce
    - 搜索字段：`session_id`、`project_name`、`model`、`status`、`cwd`
    - 不区分大小写
  - 过滤结果不改变 session 分组结构（只在渲染时过滤），保留项目分组卡片
  - 空状态：无匹配结果时显示 "没有匹配的会话" 提示 + 清空过滤按钮
  - 过滤文本旁显示匹配/总数（如 "3/12"）

  **Must NOT do**：
  - 不修改 Rust 后端
  - 不过滤项目级别的卡片（即保留项目分组空壳）

  **Recommended Agent Profile**：
  > 前端搜索功能，纯客户端逻辑。
  - **Category**：`quick`
    - Reason：纯前端过滤逻辑 + 输入框，无后端依赖
  - **Skills**：`[]`

  **Parallelization**：
  - **Can Run In Parallel**: NO（依赖 2.1 的布局重构）
  - **Parallel Group**: Wave 2
  - **Blocks**: Task 2.3
  - **Blocked By**: Task 2.1

  **References**：
  - `src/pages/AgentMonitor.tsx` — 当前页面（添加搜索框位置）
  - `src/pages/Settings.tsx:146-168` — 现有 Input 组件使用模式

  **Acceptance Criteria**：
  - [ ] 输入文本后实时过滤 AgentSessionRow 显示
  - [ ] 匹配 `session_id`、`project_name`、`model`、`status`、`cwd`
  - [ ] 清空后恢复完整列表
  - [ ] 无匹配时显示空状态
  - [ ] 显示匹配计数 "N/M"
  - [ ] `npm run build` 无 TypeScript 错误

  **QA Scenarios**：
  ```
  Scenario: Session 过滤 — 匹配 session_id
    Tool: Playwright
    Preconditions: AgentMonitor 页面，有多个 session
    Steps:
      1. 在搜索框输入已知 session_id 的部分字符
      2. 验证只有匹配的 session 可见
      3. 验证匹配计数正确（如 "2/10"）
      4. 截图保存
    Expected Result: 过滤生效，不匹配的 session 隐藏
    Evidence: .sisyphus/evidence/task-2.2-filter-match.png

  Scenario: Session 过滤 — 无匹配时显示空状态
    Tool: Playwright
    Preconditions: AgentMonitor 页面
    Steps:
      1. 输入不存在的内容如 "zzzznotexist"
      2. 验证显示 "没有匹配的会话" 空状态
      3. 点击 "清空过滤" 按钮恢复
    Expected Result: 空状态正确显示
    Evidence: .sisyphus/evidence/task-2.2-filter-empty.png

  Scenario: Session 过滤 — 清空恢复
    Tool: Playwright
    Preconditions: 过滤已有匹配
    Steps:
      1. 输入文本过滤
      2. 点击输入框右侧清空按钮
      3. 验证所有 session 恢复显示
    Expected Result: 清空后恢复完整列表
    Evidence: .sisyphus/evidence/task-2.2-filter-clear.png
  ```

  **Commit**: YES
  - Message: `feat(ui): 添加 Session 过滤搜索 + 详情 Tab 切换`
  - Files: `src/pages/AgentMonitor.tsx`
  - Pre-commit: `npm run build`

---

- [x] 2.3 **Playwright E2E 测试更新**

  **What to do**：
  - 更新 `e2e/agent-monitor.spec.ts`：
    - 新增测试：Tool Call Timeline 渲染验证
    - 新增测试：Sub-Agent Tree 渲染验证
    - 新增测试：File Audit 渲染验证
    - 新增测试：手风琴互斥展开行为
    - 新增测试：过滤搜索 + 清空
    - 新增测试：主题切换 + 持久化
    - 新增测试：空数据边界（tool_calls/subagents/file_accesses 均为空）
    - 新增测试：大数据集（50+ tool_calls 不崩溃）
    - 保留现有 4 个 agent-monitor 测试不变（项目总测试数 ≥29）
  - 确保所有测试在 macOS 上通过
  - 使用具体选择器（`.agent-tool-timeline`、`.agent-sub-tree`、`.agent-file-audit`、`.theme-toggle`、`.search-input`）
  - 截图保存到 `.sisyphus/evidence/e2e/`

  **Must NOT do**：
  - 不删除或减少现有测试

  **Recommended Agent Profile**：
  > E2E 测试编写，需要 playwright skill。
  - **Category**：`visual-engineering`
    - Reason：Playwright E2E 测试，需要 playwright skill 操作浏览器
  - **Skills**：`["playwright"]`
    - `playwright`：浏览器自动化 — 操作 UI、断言 DOM、截图

  **Parallelization**：
  - **Can Run In Parallel**: YES（与 Tauri 打包并行，无依赖关系）
  - **Parallel Group**: Wave 2（单独运行）
  - **Blocks**: None
  - **Blocked By**: Task 2.1, 2.2

  **References**：
  - `e2e/agent-monitor.spec.ts` — 现有测试（参考模式）
  - `playwright.config.ts` — Playwright 配置

  **Acceptance Criteria**：
  - [ ] `npm test` 所有测试通过
  - [ ] 新增至少 8 个 v0.2 相关测试
  - [ ] 测试覆盖：Tool Timeline / Sub-Agent Tree / File Audit / 手风琴 / 过滤 / 主题 / 空数据 / 大数据

  **QA Scenarios**：
  ```
  Scenario: E2E — 全面测试
    Tool: Bash
    Preconditions: 所有代码已提交，Tauri 可运行
    Steps:
      1. npm test
      2. 验证输出包含 "N passed" 且 N >= 37（项目原有约 29 + agent-monitor 新增 ≥8）
      3. 验证无失败测试
    Expected Result: 所有 Playwright 测试通过
    Evidence: .sisyphus/evidence/task-2.3-e2e-pass.txt
  ```

  **Commit**: YES
  - Message: `test(e2e): 添加 v0.2 AgentMonitor 增强的 E2E 测试`
  - Files: `e2e/agent-monitor.spec.ts`
  - Pre-commit: `npm test`

> 4 个审查 Agent 并行运行。ALL 必须 APPROVE。向用户呈现汇总结果并获取明确 "okay" 后再标记完成。

- [x] F1. **Plan Compliance Audit** — `oracle`
  阅读计划从头到尾。每个 "Must Have"：验证实现存在（读文件、curl 端点、运行命令）。每个 "Must NOT Have"：搜索代码库中的禁止模式 — 如发现以 file:line 标记拒绝。检查 `.sisyphus/evidence/` 中的证据文件存在。对比交付物与计划。
  输出：`Must Have [N/N] | Must NOT Have [N/N] | Tasks [N/N] | VERDICT: APPROVE/REJECT`

- [x] F2. **Code Quality Review** — `unspecified-high`
  运行 `tsc --noEmit` + linter + `cargo test -p ptv`。审查所有变更文件：`as any`/`@ts-ignore`、空 catch、console.log、注释掉的代码、未使用的导入。检查 AI slop：过度注释、过度抽象、泛型名称（data/result/item/temp）。
  输出：`Build [PASS/FAIL] | Lint [PASS/FAIL] | Tests [N pass/N fail] | Files [N clean/N issues] | VERDICT`

- [x] F3. **Real Manual QA** — `unspecified-high`（+ `playwright` skill）
  从干净状态开始。执行 EVERY 任务的 EVERY QA 场景 — 严格按步骤，捕获证据。测试跨任务集成（功能协作，非孤立）。测试边缘情况：空状态、无效输入、快速操作。保存到 `.sisyphus/evidence/final-qa/`。
  输出：`Scenarios [N/N pass] | Integration [N/N] | Edge Cases [N tested] | VERDICT`

- [x] F4. **Scope Fidelity Check** — `deep`
  对每个任务：读 "What to do"，读实际 diff（git log/diff）。验证 1:1 — spec 中的一切都实现了（无遗漏），spec 外的都没有实现（无蔓延）。检查 "Must NOT do" 合规性。检测跨任务污染：Task N 触碰到 Task M 的文件。标记未经说明的变更。
  输出：`Tasks [N/N compliant] | Contamination [CLEAN/N issues] | Unaccounted [CLEAN/N files] | VERDICT`

---

## Commit Strategy

- **T0**: `feat(backend): 为 AgentInfo 添加 tool_calls/subagents/file_accesses 字段` — `src-tauri/src/collectors/agent/mod.rs`
- **T1.1**: `feat(ui): 添加 Tool Call Timeline 水平条形图组件` — `src/components/AgentToolTimeline.tsx`, `src/types/agent.ts`
- **T1.2**: `feat(ui): 添加 Sub-Agent Tree 可折叠树组件` — `src/components/AgentSubTree.tsx`
- **T1.3**: `feat(ui): 添加 File Audit 文件操作日志组件` — `src/components/AgentFileAudit.tsx`
- **T1.4**: `feat(ui): 添加 Light/Dark 主题切换 + useTheme hook` — `src/components/ThemeToggle.tsx`, `src/hooks/useTheme.ts`, `src/index.css`
- **T2.1**: `refactor(ui): AgentSessionRow 改为手风琴式内联展开` — `src/pages/AgentMonitor.tsx`
- **T2.2**: `feat(ui): 添加详情 Tab 切换 + Session 过滤搜索框` — `src/pages/AgentMonitor.tsx`
- **T2.3**: `test(e2e): 添加 v0.2 AgentMonitor 增强的 E2E 测试` — `e2e/agent-monitor.spec.ts`

---

## Success Criteria

### Verification Commands
```bash
# 后端数据管道验证
cargo test -p ptv -- agent
# Expected: 所有 session_to_info 相关测试通过

# 前端构建验证
npm run build
# Expected: TypeScript 无错误，Vite 构建成功

# Tauri 打包验证
npm run tauri build
# Expected: macOS .dmg + Linux AppImage 生成成功

# E2E 测试验证
npm test
# Expected: 所有 Playwright 测试通过
```

### Final Checklist
- [ ] 所有 "Must Have" 存在
- [ ] 所有 "Must NOT Have" 缺失
- [ ] `cargo test -p ptv -- agent` 全部通过
- [ ] `npm test` 全部通过
- [ ] AgentMonitor 展开时显示 Tool Call Timeline / Sub-Agent Tree / File Audit 三个 Tab
- [ ] "Thinking" 虚拟行在模型生成时显示
- [ ] 搜索框实时过滤会话
- [ ] Light/Dark 切换生效，刷新后保持
- [ ] macOS .dmg 和 Linux AppImage 构建成功
