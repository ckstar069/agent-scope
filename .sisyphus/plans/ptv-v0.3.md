# ptv v0.3 — 项目记忆面板 (Project Memory Viewer)

## TL;DR

> **快速摘要**：为 ptv 新增"项目记忆"面板，分两阶段实现：L1 读取项目目录内的静态 Markdown 记忆文件（CLAUDE.md、规则、积累知识、设计文档），L2 提供对话全文搜索 + 文件关联 + 候选记忆标记 + 人工确认沉淀，解决多会话后核心约定被遗忘的问题。核心理念：**不做自动提取，做帮助用户发现和沉淀记忆的工具**。
>
> **交付物**：
> - Rust 后端：`project_files.rs` 收集器 + `session_transcript.rs` 收集器 + 全文搜索索引 + 5 个新 Tauri 命令
> - 前端：`ProjectMemoryPanel` 标签页（文件树 + Markdown 渲染 + 对话搜索 + 候选记忆箱）
> - 记忆沉淀流程：候选标记 → 人工确认 → 写入 `.sisyphus/notepads/project-memory/`
> - E2E 测试：Playwright 覆盖空状态、填充状态、搜索、候选记忆标记
>
> **预估工作量**：中等（L1: 6 任务，L2: 6 任务，Final: 4 任务）
> **并行执行**：YES — 3 个 Wave
> **关键路径**：Task 1 → Task 6 → Task 8 → Task 10 → F1-F4

---

## 上下文

### 原始需求
用户希望读取被监控项目的 Claude 记忆文件（Markdown 文档 + 对话会话记录），以合适的形式提取关键内容独立展示。核心痛点是：使用 Claude 时对话太多、会话太多后，核心要求和约定会被遗忘（用户忘记、AI 也可能忘记）。

### 访谈总结
**关键讨论**：
- 记忆文件范围：全部记忆文件（CLAUDE.md、AGENTS.md、.claude/rules/*、.sisyphus/notepads/*、docs/design/*）
- ⭐ **核心补充**：不仅读 Markdown 文档，还需要读对话会话记录（重要决定常只存在于对话中）
- 展示方式：Markdown 渲染 + 左侧标题目录导航
- 展示粒度：单项目面板，在 ProjectDetail 中新增"项目记忆"Tab
- 编辑能力：只读，不提供编辑
- 变更检测：文件变化时用红点标记（利用已有 FileWatcher）
- 测试策略：Agent QA（Playwright E2E）

**研究结论**：
- 模板项目记忆结构丰富：CLAUDE.md（133行）+ .claude/rules/（26文件）+ docs/design/（7文件）
- PTV 项目积累知识丰富：.sisyphus/notepads/（6个主题目录，learnings/decisions/issues）
- 对话记录位置：`~/.claude/projects/{encoded_cwd}/{session_id}.jsonl`（7个会话 ~18MB）
- 现有 infrastructure：`memory.rs` 已有 `.claude/memory/` 读取管道，`WatchedCollector` 已有文件监控
- 关键缺失：`memory.rs` 仅读 `.claude/memory/`（不读根级文件），无对话记录读取，前端无 Markdown 渲染库

### Metis 审查要点
**已识别并解决的缺口**：
- 分两个阶段实现（L1 静态文件 + L2 对话记录），降低风险
- L1 文件目录白名单（防意外扫描 node_modules），L2 100MB 上限 + 延迟加载
- 新 `SerProjectFile` 类型（不修改现有 `SerMemoryEntry` 以保持向后兼容）
- 排除范围：Codex 会话、AI App 对话、实时监控、文件交叉引用、跨项目聚合
- 新增 Tauri fs scope 配置（读取 `~/.claude/projects/` 的权限）
- 路径编码冲突处理、正在查看时文件被删除等边缘情况

---

## 工作目标

### 核心目标
为 ptv 新增两层"项目记忆"面板：L1 展示静态记忆文件的完整内容（Markdown 渲染 + TOC），L2 提供对话全文搜索 + 候选记忆标记 + 人工确认沉淀的完整工作流，帮助用户从海量对话中找回和沉淀重要约定。

### 具体交付物
- `src-tauri/src/collectors/template/project_files.rs` — L1 静态文件收集器
- `src-tauri/src/collectors/template/session_transcript.rs` — L2 对话收集器 + 全文索引
- `src-tauri/src/commands.rs` — 新增 `get_project_files`、`list_project_sessions`、`search_sessions`、`get_session_transcript`、`save_candidate_memory` 命令
- `src/components/ProjectMemoryPanel.tsx` — 项目记忆面板（两个子面板）
- `src/components/MarkdownRenderer.tsx` — Markdown 渲染（react-markdown + TOC）
- `src/components/SessionSearchView.tsx` — 对话搜索视图（搜索框 + 搜索结果 + 文件关联）
- `src/components/CandidateMemoryBox.tsx` — 候选记忆箱（标记 → 确认 → 沉淀）
- `e2e/project-memory.spec.ts` — E2E 测试

### 完成定义
- [ ] `cargo test -p ptv` 全部通过
- [ ] `npm run build` 通过
- [ ] `npm test` 全部通过
- [ ] 在 ai_project_template 项目中可见所有 26 个规则文件 + 7 个会话
- [ ] 变更检测：修改文件后 Tab 徽章更新

### 必须包含
- L1：读取白名单目录中的所有 Markdown 文件
- L1：Markdown 渲染 + 标题目录导航
- L1：变更检测（红点标记）
- L2：全文搜索（跨所有会话的对话内容搜索）
- L2：会话列表（搜索前显示全部，搜索后过滤匹配）
- L2：文件关联（每个会话关联其修改过的文件列表）
- L2：候选记忆标记（用户标记重要消息 → 存入候选箱）
- L2：记忆沉淀（候选箱中确认 → 写入 `.sisyphus/notepads/project-memory/decisions.md`）
- 两层独立面板，Tab 切换

### 必须不包含（护栏）
- ❌ 不自动判断"哪条消息重要"——由用户标记
- ❌ 不调用外部 AI 做摘要/提取
- ❌ 不编辑任何文件（记忆沉淀除外，且需用户确认）
- ❌ 不包含 Codex 会话
- ❌ 不包含 Claude App transcripts（`~/.claude/transcripts/` 中的 1286 个文件）
- ❌ 不实时监控对话（仅历史会话）
- ❌ 不提供静态文件与会话之间的交叉引用链接
- ❌ 不修改现有 `SerMemoryEntry` 或 `MemoryPanel`
- ❌ 不跨项目聚合记忆

---

## 验证策略

> **零人工干预** — 所有验证均由 agent 执行。

### 测试决策
- **Infrastructure 存在**：YES（Playwright E2E）
- **自动化测试**：Agent QA（E2E after implementation）
- **框架**：Playwright

### QA 策略
每个任务 MUST 包含 agent 可执行的 QA 场景。
- 前端/UI：使用 Playwright（playwright skill）
- CLI：使用 interactive_bash（tmux）
- API：使用 Bash（curl）

---

## 执行策略

### 并行执行 Wave

```
Wave 1（立即开始 — L1 后端 + 前端基础设施）：
├── Task 1: project_files.rs 收集器 + 单元测试 [unspecified-high]
├── Task 2: 扩展 WatchedCollector + Tauri 命令 [unspecified-high]
├── Task 3: npm install react-markdown + MarkdownRenderer 组件 [visual-engineering]
├── Task 4: MemoryFileTree 组件（TOC 树形导航） [visual-engineering]
└── Task 5: ProjectMemoryPanel L1 集成 [visual-engineering]

Wave 2（Wave 1 之后 — L2 后端 + 搜索 + 候选记忆）：
├── Task 6: session_transcript.rs 收集器 + 全文索引 + encode_cwd_path [unspecified-high]
├── Task 7: L2 Tauri 命令（list_sessions + search_sessions + get_transcript + save_memory） [unspecified-high]
├── Task 8: SessionSearchView 组件（搜索框 + 搜索结果列表） [visual-engineering]
├── Task 9: TranscriptDetailView 组件（对话详情 + 消息标记） [visual-engineering]
├── Task 10: CandidateMemoryBox 组件（候选箱 + 确认沉淀） [visual-engineering]
├── Task 11: ProjectMemoryPanel L2 集成 + Tab 切换 [visual-engineering]
└── Task 12: tauri.conf.json fs scope 配置 [quick]

Wave 3（Wave 2 之后 — 测试 + 打磨）：
├── Task 13: Playwright E2E 测试 [visual-engineering + playwright]
├── Task 14: 变更检测（L1 + L2）+ Tab 徽章 [unspecified-low]
└── Task 15: 边缘情况处理 + 错误状态 UI [unspecified-low]

Wave FINAL（ALL 任务之后 — 4 并行审查）：
├── F1: Plan Compliance Audit (oracle)
├── F2: Code Quality Review (unspecified-high)
├── F3: Real Manual QA (unspecified-high + playwright)
└── F4: Scope Fidelity Check (deep)

关键路径：Task 1 → Task 6 → Task 8 → Task 11 → Task 13 → F1-F4
并行加速：~55% 比顺序快
最大并发：6（Wave 2）
```

### 依赖矩阵

| 任务 | 阻塞于 | 阻塞 | Wave |
|:-----|:------|:-----|:-----|
| 1-5 | - | 6-7, 11 | 1 |
| 6 | 1 | 7, 11 | 2 |
| 7 | 1, 6 | 11, 13 | 2 |
| 8 | - | 11 | 2 |
| 9 | - | 11 | 2 |
| 10 | - | 11 | 2 |
| 11 | 5, 7, 8, 9 | 13, 14 | 2 |
| 12 | 1 | - | 2 |
| 13 | 11 | - | 3 |
| 14 | 11 | - | 3 |
| 15 | 11, 13 | - | 3 |
| F1-F4 | ALL | - | FINAL |

### Agent 调度摘要

- **Wave 1**: 5 agents — T1-T2: `unspecified-high`, T3-T5: `visual-engineering`
- **Wave 2**: 7 agents — T6-T7: `unspecified-high`, T8-T11: `visual-engineering`, T12: `quick`
- **Wave 3**: 3 agents — T13: `visual-engineering` + `playwright`, T14-T15: `unspecified-low`
- **FINAL**: 4 agents — 并行审查

---

## TODOs

- [x] 1. **Rust 后端：`project_files.rs` 收集器 + 单元测试**

  **做什么**：
  - 在 `src-tauri/src/collectors/template/` 下新建 `project_files.rs`
  - 实现 `ProjectFilesCollector` 结构体，读取白名单目录中的 Markdown 文件：
    - 项目根目录：`CLAUDE.md`、`AGENTS.md`
    - `.claude/rules/*.md`（递归 1 层，不含 stage/ 子目录的更深嵌套）
    - `.claude/rules/stage/*.md`
    - `.claude/rules/python/*.md`、`.claude/rules/fpga/*.md`
    - `.sisyphus/notepads/**/*.md`（递归，限于 learnings.md / decisions.md / issues.md）
    - `.sisyphus/plans/*.md`、`.sisyphus/drafts/*.md`
    - `docs/design/*.md`、`docs/specs/**/*.md`
  - 定义 `ProjectFile` 结构体：
    ```rust
    pub struct ProjectFile {
        pub relative_path: String,  // 如 ".claude/rules/00-core.md"
        pub content: String,        // 完整内容（截断至 1MB）
        pub content_truncated: bool,
        pub source_group: String,   // "root" | "rules" | "notepads" | "plans" | "drafts" | "docs"
        pub mtime_ms: u64,
    }
    ```
  - 实现 `ProjectFilesCollector::collect(&self, project_path: &Path) -> Result<Vec<ProjectFile>, ProjectFilesError>`
  - 定义 `ProjectFilesError` 枚举（Io、PermissionDenied、FileTooLarge）
  - 文件大小限制：单文件最大 1MB，总内存上限 50MB（超出则截断目录，优先保留根级文件）
  - 编码处理：非 UTF-8 文件降级为 "(binary/encoding error)" 占位
  - 将 `ProjectFilesCollector` 注册到 `TemplateDataCollector` 的 `collect()` 方法中
  - 在 `TemplateData` 中新增 `project_files: Result<Vec<ProjectFile>, ProjectFilesError>` 字段
  - 编写单元测试（至少 4 个）：
    - 空目录 → 返回空 Vec
    - 有 CLAUDE.md + rules/ → 正确分类 source_group
    - .sisyphus/notepads/ 嵌套目录 → 递归读取
    - 文件 > 1MB → 截断 + content_truncated = true

  **绝不能做**：
  - ❌ 不扫描 `.git/`、`node_modules/`、`target/`、`dist/`、`__pycache__/`
  - ❌ 不修改 `MemoryEntry` 或 `SerMemoryEntry`
  - ❌ 不读取 `.claude/memory/*.md`（那是现有 `memory.rs` 的职责）

  **推荐 Agent Profile**：
  - **类别**：`unspecified-high` — 纯 Rust 后端实现，需要文件系统操作和序列化
  - **技能**：无特定技能

  **并行化**：
  - **可并行**：YES（Wave 1，与 Task 3-5 并行；Task 2 依赖本任务但可在同 Wave 内串行）
  - **并行组**：Wave 1
  - **阻塞**：Task 6, 7, 10
  - **被阻塞**：无（可立即开始）

  **参考**：
  - `src-tauri/src/collectors/template/memory.rs:1-152` — 参照现有收集器模式（collect() 签名、错误枚举、测试结构）
  - `src-tauri/src/collectors/template/mod.rs:1-60` — TemplateData 结构体，新增 project_files 字段的位置
  - `src-tauri/src/collectors/template/mod.rs:180-230` — WatchedCollector 的 watch_paths，需扩展

  **验收标准**：
  - [ ] `cargo test -p ptv -- project_files` 通过（≥4 测试）
  - [ ] `cargo check -p ptv` 无错误
  - [ ] 结构体正确派生 Serialize/Deserialize/Clone

  **QA 场景**：
  ```
  Scenario: 读取 ai_project_template 的静态记忆文件
    Tool: Bash (cargo test)
    Steps:
      1. cd src-tauri && cargo test -p ptv -- project_files -- --nocapture
      2. 检查测试输出包含 "test result: ok. N passed"
    Expected Result: 所有 project_files 测试通过
    Evidence: .sisyphus/evidence/task-1-test-output.txt

  Scenario: 空项目目录返回空结果
    Tool: Bash (cargo test)
    Steps:
      1. 运行 test_collect_empty_project
      2. 验证返回 Ok(vec![])
    Expected Result: 不 panic，不报错，返回空 Vec
    Evidence: .sisyphus/evidence/task-1-empty-project.txt
  ```

  **提交**：YES
  - 消息：`feat(collector): 新增 project_files.rs 静态文件收集器`
  - 文件：`src-tauri/src/collectors/template/project_files.rs`、`src-tauri/src/collectors/template/mod.rs`

- [x] 2. **Rust 后端：扩展 WatchedCollector + Tauri 命令**

  **做什么**：
  - 在 `WatchedCollector` 的 `watch_paths` 中添加所有白名单目录
  - 新增 Tauri 命令 `get_project_files(path: String) -> Vec<SerProjectFile>`：
    - 调用 `ProjectFilesCollector::collect()`
    - 返回序列化后的 `SerProjectFile`（不含 content 字段，仅元数据 + content_preview 前 200 字符）
  - 新增 Tauri 命令 `get_project_file_content(path: String, relative_path: String) -> String`：
    - 读取单个文件的完整内容
    - 按需加载，避免一次性传输所有内容
  - 在 `tauri.conf.json` 的 `fs` scope 中确认项目目录已被允许访问
  - 在 `commands.rs` 中注册新命令

  **绝不能做**：
  - ❌ 不在 `get_project_files` 中返回完整文件内容（元数据 + 预览即可）
  - ❌ 不修改现有 `get_project_data` 命令

  **推荐 Agent Profile**：
  - **类别**：`unspecified-high` — Rust 后端 + Tauri 命令注册
  - **技能**：无特定技能

  **并行化**：
  - **可并行**：YES（Wave 1，依赖 Task 1 完成，但可同 Wave）
  - **并行组**：Wave 1（与 Task 3-5 并行）
  - **阻塞**：Task 7, 10
  - **被阻塞**：Task 1

  **参考**：
  - `src-tauri/src/commands.rs:60-120` — 现有 `get_project_data` 命令的实现模式
  - `src-tauri/src/watcher.rs:1-80` — FileWatcher 实现
  - `src-tauri/src/collectors/template/mod.rs:210-230` — watch_paths 现有列表

  **验收标准**：
  - [ ] `cargo check -p ptv` 无错误
  - [ ] 新命令可在 Tauri dev 模式下通过 `invoke` 调用
  - [ ] FileWatcher 在文件修改时触发 `template-update` 事件

  **QA 场景**：
  ```
  Scenario: 通过 Tauri 命令获取项目文件列表
    Tool: Bash (curl/invoke via dev mode)
    Steps:
      1. 启动 tauri dev
      2. 调用 invoke("get_project_files", { path: "/Users/ckstar/Repo/ai_project_template" })
      3. 验证返回 JSON 数组，包含 CLAUDE.md、rules/、docs/ 等条目
    Expected Result: 返回 ≥20 个文件条目，每个包含 relative_path + source_group + content_preview
    Evidence: .sisyphus/evidence/task-2-get-files.json

  Scenario: 获取单个文件完整内容
    Tool: Bash (curl)
    Steps:
      1. 调用 invoke("get_project_file_content", { path: "...", relative_path: "CLAUDE.md" })
      2. 验证返回内容包含 "Rosie-s-Cat" 或 "项目记忆"
    Expected Result: 返回完整 Markdown 文本
    Evidence: .sisyphus/evidence/task-2-get-content.md
  ```

  **提交**：YES
  - 消息：`feat(backend): 新增 get_project_files + get_project_file_content 命令`
  - 文件：`src-tauri/src/commands.rs`、`src-tauri/src/lib.rs`、`src-tauri/tauri.conf.json`

- [x] 3. **前端：安装 react-markdown + MarkdownRenderer 组件**

  **做什么**：
  - 运行 `npm install react-markdown remark-gfm`
  - 新建 `src/components/MarkdownRenderer.tsx`：
    - Props: `content: string`、`className?: string`
    - 使用 `react-markdown` + `remark-gfm` 渲染 Markdown
    - 自动从内容中提取标题（h1-h4），生成 TOC 数组 `{ level, text, id }[]`
    - 为每个标题自动生成 `id`（基于文本的 slug）
    - TOC 渲染为左侧导航，点击滚动到对应标题
    - 支持代码块渲染（使用 Tailwind prose 样式）
    - 加载态：内容为空时显示骨架屏
  - 在 `src/pages/ProjectDetail.tsx` 旁跑通 `npm run build` 确认依赖兼容

  **绝不能做**：
  - ❌ 不使用 `rehype-highlight`（额外依赖，代码块用简单 pre 样式）
  - ❌ 不在此组件中处理文件加载逻辑（仅接收 content prop）

  **推荐 Agent Profile**：
  - **类别**：`visual-engineering` — 前端 UI 组件，需要 Markdown 渲染经验
  - **技能**：无特定技能

  **并行化**：
  - **可并行**：YES（Wave 1，与 Task 1-2 和 Task 4-5 并行）
  - **并行组**：Wave 1
  - **阻塞**：Task 5
  - **被阻塞**：无

  **参考**：
  - `src/components/ui/scroll-area.tsx` — 现有 shadcn/ui 滚动区域组件，可用于 TOC
  - `src/pages/ProjectDetail.tsx:80-160` — Panel 组件使用模式
  - react-markdown 官方文档：`https://github.com/remarkjs/react-markdown`

  **验收标准**：
  - [ ] `npm run build` 通过
  - [ ] 组件接收 Markdown 字符串，渲染为 HTML
  - [ ] TOC 从标题中正确提取（h1-h4）
  - [ ] 点击 TOC 项滚动到对应标题位置

  **QA 场景**：
  ```
  Scenario: 渲染包含多级标题的 Markdown
    Tool: Playwright
    Preconditions: 在 ProjectDetail 页面中挂载 MarkdownRenderer，传入测试内容
    Steps:
      1. 传入 "# 项目概述\n\n## 技术栈\n\n### 前端\n\nReact\n\n## 部署"
      2. 断言渲染区域包含 "项目概述" 文本
      3. 断言 TOC 包含 3 个条目（"项目概述"、"技术栈"、"部署"）
      4. 点击 TOC 中的 "部署"，断言视口滚动到该标题
    Expected Result: TOC 正确反映标题结构，点击可跳转
    Evidence: .sisyphus/evidence/task-3-toc-navigation.png

  Scenario: 空内容显示骨架屏
    Tool: Playwright
    Steps:
      1. 传入 content="" 
      2. 断言显示骨架屏占位（.skeleton 类）
    Expected Result: 无报错，显示加载骨架屏
    Evidence: .sisyphus/evidence/task-3-empty-state.png
  ```

  **提交**：YES
  - 消息：`feat(frontend): 新增 MarkdownRenderer 组件 (react-markdown + TOC)`
  - 文件：`src/components/MarkdownRenderer.tsx`、`package.json`、`package-lock.json`

- [x] 4. **前端：MemoryFileTree 组件（TOC 树形导航）**

  **做什么**：
  - 新建 `src/components/MemoryFileTree.tsx`
  - Props: `files: SerProjectFile[]`、`selectedPath: string | null`、`onSelect: (path: string) => void`、`changedPaths: Set<string>`
  - 按 `source_group` 分组显示为可折叠树形结构
  - 分组顺序：root → rules → notepads → plans → drafts → docs
  - 每个文件条目：文件名（从 relative_path 提取）+ 变更红点
  - 点击文件触发 `onSelect(relative_path)`，选中项高亮（bg-accent）
  - 空状态："此项目未找到记忆文件"
  - 使用 shadcn/ui Collapsible 组件

  **绝不能做**：
  - ❌ 不在此组件中加载文件内容
  - ❌ 不渲染 Markdown

  **推荐 Agent Profile**：
  - **类别**：`visual-engineering`
  - **技能**：无

  **并行化**：
  - **可并行**：YES（Wave 1，与 Task 1-3, 5 并行）
  - **阻塞**：Task 5
  - **被阻塞**：无

  **参考**：
  - `src/components/ui/collapsible.tsx` — Collapsible 组件
  - `src/pages/AgentMonitor.tsx:200-240` — 列表项选择模式

  **验收标准**：
  - [ ] `npm run build` 通过
  - [ ] 文件按 source_group 正确分组
  - [ ] 点击文件触发 onSelect，变更文件显示红点

  **QA 场景**：
  ```
  Scenario: 文件树分组显示 + 选中交互
    Tool: Playwright
    Steps:
      1. 传入含 3 个 group 的文件列表
      2. 断言 CLAUDE.md 在 "根目录" 分组下
      3. 点击 CLAUDE.md，断言 onSelect("CLAUDE.md") 被调用
      4. 传入 changedPaths = Set(["CLAUDE.md"])，断言红点出现
    Expected Result: 分组正确、选中高亮、变更指示器生效
    Evidence: .sisyphus/evidence/task-4-file-tree.png
  ```

  **提交**：YES
  - 消息：`feat(frontend): 新增 MemoryFileTree 树形导航组件`
  - 文件：`src/components/MemoryFileTree.tsx`

- [x] 5. **前端：ProjectMemoryPanel L1 静态记忆集成**

  **做什么**：
  - 新建 `src/components/ProjectMemoryPanel.tsx`
  - Props: `projectPath: string`
  - 使用 `useTauri()` hook 调用 `get_project_files` + `get_project_file_content`
  - 左右分栏：左侧 MemoryFileTree（w-64），右侧 MarkdownRenderer
  - 左侧选中文件后按需加载完整内容到右侧
  - 加载态：Skeleton，错误态：黄色警告横幅，空状态：友好提示
  - 在 ProjectDetail.tsx 中新增第 5 个 Tab "项目记忆"（Stage→Git→Config→Memory→项目记忆）
  - 监听 template-update 事件维护 changedPaths

  **绝不能做**：
  - ❌ 不包含 L2 对话内容（Task 10 单独集成）
  - ❌ 不硬编码文件路径

  **推荐 Agent Profile**：
  - **类别**：`visual-engineering`
  - **技能**：无

  **并行化**：
  - **可并行**：YES（Wave 1，依赖 Task 3, 4）
  - **阻塞**：Task 10, 12, 13
  - **被阻塞**：Task 3, 4

  **参考**：
  - `src/pages/ProjectDetail.tsx:1-30` — Tab 结构
  - `src/hooks/useTauri.ts` — invoke/listen 模式
  - `src/pages/AgentMonitor.tsx:50-80` — 事件监听

  **验收标准**：
  - [ ] `npm run build` 通过
  - [ ] "项目记忆" Tab 可见，左侧文件树渲染正确
  - [ ] 点击文件后右侧渲染 Markdown 内容 + TOC
  - [ ] 切换 Tab 恢复滚动位置

  **QA 场景**：
  ```
  Scenario: L1 完整流程（打开→选文件→渲染→TOC导航）
    Tool: Playwright
    Preconditions: ai_project_template 已注册
    Steps:
      1. 导航到 ProjectDetail，点击 "项目记忆" Tab
      2. 等待文件列表加载，断言包含 CLAUDE.md
      3. 点击 CLAUDE.md，等待右侧渲染
      4. 断言右侧含 "Rosie-s-Cat" 文本 + TOC 导航
      5. 点击 TOC 项，断言页面滚动
    Expected Result: 完整 L1 功能可用
    Evidence: .sisyphus/evidence/task-5-l1-full-flow.png

  Scenario: 空项目优雅降级
    Tool: Playwright
    Steps:
      1. 导航到无记忆文件的项目
      2. 点击 "项目记忆" Tab
      3. 断言 "此项目未找到记忆文件" 占位文本
    Expected Result: 优雅空状态，无报错
    Evidence: .sisyphus/evidence/task-5-empty-state.png
  ```

  **提交**：YES
  - 消息：`feat(frontend): 新增 ProjectMemoryPanel L1 静态记忆面板`
  - 文件：`src/components/ProjectMemoryPanel.tsx`、`src/pages/ProjectDetail.tsx`

- [x] 6. **Rust 后端：session_transcript.rs 收集器 + 简单搜索**

  **做什么**：
  - 新建 `src-tauri/src/collectors/template/session_transcript.rs`
  - 实现 `encode_cwd_path(cwd: &str) -> String`（参照 tui.py 逻辑：去首 `/`、替换 `/` → `-`；精确匹配失败时用项目名模糊查找）
  - 实现 `SessionTranscriptCollector`：
    - 扫描 `~/.claude/projects/{encoded_cwd}/`，按 mtime 降序排列 `.jsonl` 文件
    - **默认加载最新 1 个会话**（解析全部轮次，仿 tui.py 的 parse_session 逻辑）
    - 其他会话仅解析元数据（不加载完整内容）
  - **JSONL 解析逻辑**（参照 tui.py）：
    - 只处理 `type: "user"` 和 `type: "assistant"`，跳过 tool_use/tool_result/custom-title/system
    - 处理 `message.content` 的两种格式：简单字符串 / 结构化 `[{type:"text", text:"..."}, {type:"tool_use",...}]` 数组
    - 从结构化 content 中提取文本块和工具名
    - **合并连续同角色轮次**（user-user → 合并为一条）
    - 文本截断至 1KB 单条（比 tui.py 的 500 更宽松）
    - 工具名去重
  - **简单搜索**：对会话的 initial_prompt + custom-title 做大小写不敏感包含匹配（不建全文索引，纯字符串匹配）
  - 编写 4 个单元测试：encode 转换、两种 content 格式解析、轮次合并、空目录

  **绝不能做**：
  - ❌ 不加载 Codex/AI App 会话
  - ❌ 不自动判断重要性
  - ❌ 不建复杂全文索引（简单字符串匹配即可）

  **推荐 Agent Profile**：`unspecified-high`
  **并行化**：YES（Wave 2，与 Task 8-10, 12 并行）| **阻塞**：Task 7, 11 | **被阻塞**：Task 1

  **参考**：
  - `abtop-collector/src/collector/claude.rs:1197-1357` — JSONL 解析
  - tui.py 的 `parse_session()` + `_extract_text()` + `_extract_tools()` — 内容格式处理 + 轮次合并逻辑

  **验收标准**：
  - [ ] `cargo test -p ptv -- session_transcript` 通过（≥4 测试）
  - [ ] 字符串 content 和 blocks 数组 content 两种格式均正确解析
  - [ ] 连续 2 条 user 消息合并为 1 条
  - [ ] 默认返回最新会话的完整内容

  **提交**：YES
  - 消息：`feat(collector): 新增 session_transcript.rs 对话收集器 + 全文索引`
  - 文件：`src-tauri/src/collectors/template/session_transcript.rs`

- [x] 7. **Rust 后端：L2 Tauri 命令（latest + search + transcript + save_memory）**

  **做什么**：
  - 新增 Tauri 命令：
    - `get_latest_session(path: String) -> Option<SerTranscript>` — 返回最新会话的完整内容（打开即看）
    - `list_project_sessions(path: String) -> Vec<SerSessionSummary>` — 列出所有会话元数据
    - `search_sessions(path: String, query: String) -> Vec<SerSessionSummary>` — 对 initial_prompt + custom_title 做字符串包含匹配
    - `get_session_transcript(path: String, session_id: String) -> SerTranscript` — 获取指定会话完整内容
    - `save_candidate_memory(path: String, memory: SerCandidateMemory) -> ()` — 保存候选记忆到 `.sisyphus/notepads/project-memory/decisions.md`
  - `SerTranscript` 包含：session_id、turns[]（每条 { role, text, tools[], timestamp }，已经过合并和截断）
  - `SerCandidateMemory` 包含：content、source_session_id、source_turn_index、source_snippet、category
  - 工具调用参数剥离：不传输 tool_use.input 到前端

  **推荐 Agent Profile**：`unspecified-high` | **并行化**：YES（Wave 2，依赖 Task 6）| **被阻塞**：Task 1, 6

  **参考**：
  - `src-tauri/src/commands.rs:60-120` — 现有命令模式
  - `src-tauri/tauri.conf.json` — fs scope 配置
  - `.sisyphus/notepads/watcher-integration/decisions.md` — 决策文件格式参考

  **验收标准**：
  - [ ] `cargo check -p ptv` 通过
  - [ ] `search_sessions("P60")` 返回匹配结果
  - [ ] `save_candidate_memory` 正确写入文件

  **QA 场景**：
  ```
  Scenario: 搜索并保存候选记忆
    Tool: Bash (curl via dev mode)
    Steps:
      1. 调用 search_sessions，验证返回匹配会话
      2. 调用 save_candidate_memory 保存测试条目
      3. 读取 .sisyphus/notepads/project-memory/decisions.md 验证内容
    Expected Result: 搜索返回结果，文件写入正确
    Evidence: .sisyphus/evidence/task-7-save-memory.md
  ```

  **提交**：YES
  - 消息：`feat(backend): 新增 L2 对话搜索 + 候选记忆 Tauri 命令`
  - 文件：`src-tauri/src/commands.rs`、`src-tauri/src/lib.rs`、`src-tauri/tauri.conf.json`

- [x] 8. **前端：SessionSearchView 组件（搜索框 + 搜索结果列表）**

  **做什么**：
  - 新建 `src/components/SessionSearchView.tsx`
  - Props: `projectPath: string`、`onSelectSession: (id: string) => void`
  - 搜索框：顶部输入框，输入时 300ms debounce 后调用 `search_sessions`
  - 搜索结果列表：每行显示 session_id（短）+ matching_message_count + first_matching_line（截断）
  - 搜索前默认显示全部会话列表（调用 `list_project_sessions`）
  - 每个会话行显示：initial_prompt（截断80字符）、日期（相对时间）、model、modified_files 标签
  - 文件关联标签：每个 modified_file 显示为小型 tag（如 `📄 CLAUDE.md`）
  - 空搜索：显示 "输入关键词搜索对话内容"
  - 无结果：显示 "未找到匹配 'xxx' 的对话"
  - 加载态：Skeleton

  **绝不能做**：
  - ❌ 不在前端做全文搜索（调用后端命令）
  - ❌ 不自动判断重要性

  **推荐 Agent Profile**：
  - **类别**：`visual-engineering`
  - **技能**：无

  **并行化**：
  - **可并行**：YES（Wave 2，与 Task 6-7, 9-10, 12 并行）
  - **阻塞**：Task 11
  - **被阻塞**：无

  **参考**：
  - `src/pages/AgentMonitor.tsx:80-200` — 会话行渲染 + 搜索过滤模式
  - `src/components/AgentFileAudit.tsx` — 文件标签样式

  **验收标准**：
  - [ ] `npm run build` 通过
  - [ ] 搜索 "P60" 返回匹配会话，显示 matching_message_count
  - [ ] 默认显示全部会话，含 modified_files 标签

  **QA 场景**：
  ```
  Scenario: 搜索 + 文件关联
    Tool: Playwright
    Steps:
      1. 打开对话搜索 Tab
      2. 断言默认显示全部会话列表
      3. 在搜索框输入 "P60"，等待 300ms
      4. 断言结果列表仅包含匹配的会话
      5. 断言 matching_message_count ≥ 1
      6. 断言每个会话行显示 modified_files 标签
    Expected Result: 搜索正确过滤，文件关联可见
    Evidence: .sisyphus/evidence/task-8-search.png
  ```

  **提交**：YES
  - 消息：`feat(frontend): 新增 SessionSearchView 对话搜索组件`
  - 文件：`src/components/SessionSearchView.tsx`

- [x] 9. **前端：TranscriptDetailView 组件（对话详情 + 消息标记）**

  **做什么**：
  - 新建 `src/components/TranscriptDetailView.tsx`
  - Props: `sessionId: string`、`projectPath: string`、`onMarkMemory: (turn: Turn) => void`
  - 对话渲染：类似聊天气泡（user 右蓝，assistant 左灰）
  - **assistant 消息用 MarkdownRenderer 渲染**（仿 tui.py：助理回复含代码块、列表等需要 Markdown 渲染）
  - user 消息用纯文本
  - **消息标记按钮**：每条消息旁 "📌 标记" → 触发 onMarkMemory
  - 标记过的消息黄色背景 + "已标记"
  - 搜索结果匹配行高亮
  - 原对话中已合并的连续同角色轮次直接展示（后端已完成合并）
  - 分页加载：超过 100 条消息时 "加载更多"

  **推荐 Agent Profile**：`visual-engineering` | **并行化**：YES（Wave 2）
  **提交**：YES — `feat(frontend): 新增 TranscriptDetailView 对话详情 + 消息标记`

- [x] 10. **前端：CandidateMemoryBox 组件（候选箱 + 确认沉淀）**

  **做什么**：
  - 新建 `src/components/CandidateMemoryBox.tsx`
  - Props: `projectPath: string`、`candidates: CandidateMemory[]`
  - 显示已标记的候选记忆列表（来自 TranscriptDetailView 的 onMarkMemory）
  - 每条候选显示：内容预览（前 100 字符）、来源（session_id + turn_index）、类别下拉（范围约定/产品约束/技术规范/安全约束/其他）
  - 操作按钮：
    - "✏️ 编辑" → 打开编辑框修改内容
    - "✅ 确认" → 调用 `save_candidate_memory` 写入文件，状态变为 "已沉淀"
    - "🗑️ 删除" → 从候选箱移除
  - 已沉淀的条目显示绿色对勾 + "已保存"
  - 空状态："暂无候选记忆。在对话中点击 📌 标记重要消息来添加。"
  - 确认沉淀时显示 "确认写入 .sisyphus/notepads/project-memory/decisions.md？" 二次确认

  **绝不能做**：
  - ❌ 不自动沉淀（必须用户确认）
  - ❌ 不覆盖已有记忆文件内容

  **推荐 Agent Profile**：
  - **类别**：`visual-engineering`
  - **技能**：无

  **并行化**：
  - **可并行**：YES（Wave 2，与 Task 6-9, 12 并行）
  - **阻塞**：Task 11
  - **被阻塞**：无

  **参考**：
  - `src/pages/Settings.tsx:40-80` — 表单/按钮交互模式
  - `.sisyphus/notepads/watcher-integration/decisions.md` — 决策文件格式

  **验收标准**：
  - [ ] `npm run build` 通过
  - [ ] 候选箱显示已标记的记忆
  - [ ] 确认后调用 save_candidate_memory
  - [ ] 二次确认提示出现

  **QA 场景**：
  ```
  Scenario: 标记 → 候选箱 → 确认沉淀
    Tool: Playwright
    Steps:
      1. 在对话详情中标记一条消息
      2. 切换到候选记忆箱 Tab
      3. 断言候选箱显示 1 条待确认记忆
      4. 选择类别 "范围约定"
      5. 点击 "✅ 确认"
      6. 断言二次确认弹出
      7. 点击 "确认"
      8. 断言条目变为 "已沉淀" 绿色对勾
    Expected Result: 完整候选记忆工作流正常
    Evidence: .sisyphus/evidence/task-10-candidate-flow.png
  ```

  **提交**：YES
  - 消息：`feat(frontend): 新增 CandidateMemoryBox 候选记忆箱组件`
  - 文件：`src/components/CandidateMemoryBox.tsx`

- [x] 11. **前端：ProjectMemoryPanel L2 集成 + Tab 切换**

  **做什么**：
  - 在 ProjectMemoryPanel 中新增 L2 面板子区域
  - 二级 Tab 切换：L1 "静态记忆" | L2 "对话搜索" | L3 "候选记忆 (N)"
  - L2 区域：顶部 SessionSearchView + 底部 TranscriptDetailView（分屏或折叠面板）
  - L2 工作流：搜索 → 选会话 → 查看对话 → 标记消息 → 候选箱
  - 候选记忆箱集成：TranscriptDetailView 的 onMarkMemory → 添加到候选列表
  - Tab 徽章：显示 L1 变更文件数 + L3 候选记忆数
  - Tab 切换保留各侧滚动位置

  **绝不能做**：
  - ❌ 不合并 L1/L2/L3 数据
  - ❌ 不实现跨层搜索

  **推荐 Agent Profile**：
  - **类别**：`visual-engineering`
  - **技能**：无

  **并行化**：
  - **可并行**：NO（阻塞于多个上游任务）
  - **阻塞**：Task 13, 14, 15
  - **被阻塞**：Task 5, 7, 8, 9, 10

  **参考**：
  - `src/pages/AgentMonitor.tsx:150-200` — Tab 切换模式
  - `src/components/ProjectMemoryPanel.tsx`（Task 5）— 面板结构

  **验收标准**：
  - [ ] `npm run build` 通过
  - [ ] L1/L2/L3 三级 Tab 切换正常
  - [ ] 搜索 → 查看 → 标记 → 候选箱完整工作流
  - [ ] L3 Tab 徽章显示候选记忆计数

  **QA 场景**：
  ```
  Scenario: L2 完整工作流
    Tool: Playwright
    Steps:
      1. 打开 ProjectMemoryPanel，点击 "对话搜索" Tab
      2. 搜索 "P60"，点击第一个结果
      3. 在对话详情中标记 2 条消息
      4. 切换到 "候选记忆 (2)" Tab
      5. 断言 2 条候选记忆显示
      6. 确认沉淀后，L3 徽章变为 (0)
    Expected Result: 完整工作流串联成功
    Evidence: .sisyphus/evidence/task-11-full-workflow.png
  ```

  **提交**：YES
  - 消息：`feat(frontend): L2 对话搜索 + 候选记忆集成 + 三级 Tab`
  - 文件：`src/components/ProjectMemoryPanel.tsx`

- [x] 12. **配置：tauri.conf.json fs scope 扩展**

  **做什么**：
  - 在 `tauri.conf.json` 的 `plugins.fs.scope` 中添加：
    - `"$HOME/.claude/projects/**"`
    - 确认已有项目目录 scope
  - 验证：`cargo check -p ptv` 无权限相关错误

  **推荐 Agent Profile**：
  - **类别**：`quick`
  - **技能**：无

  **并行化**：
  - **可并行**：YES（Wave 2，与 Task 6-11 并行）
  - **阻塞**：无

  **参考**：`src-tauri/tauri.conf.json`

  **验收标准**：
  - [ ] `cargo check -p ptv` 通过
  - [ ] scope 包含 `$HOME/.claude/projects/**`

  **提交**：YES
  - 消息：`config: 扩展 tauri fs scope 以读取 ~/.claude/projects/`
  - 文件：`src-tauri/tauri.conf.json`

- [x] 13. **Playwright E2E 测试**

  **做什么**：
  - 新建 `e2e/project-memory.spec.ts`，至少 10 个测试用例覆盖：L1 空状态/文件渲染/TOC/错误，L2 搜索/会话查看/消息标记，L3 候选箱确认/删除，Tab 切换/徽章
  - Mock Tauri invoke 返回值

  **推荐 Agent Profile**：`visual-engineering` + **技能**：`playwright`
  **并行化**：YES（Wave 3，与 Task 14-15 并行）| **被阻塞**：Task 5, 11
  **提交**：YES — `test(e2e): 新增项目记忆面板 E2E 测试` — `e2e/project-memory.spec.ts`

- [x] 14. **变更检测（L1 + L2）+ Tab 徽章**

  **做什么**：
  - L1：监听 template-update，对比 mtime_ms 标记变更文件
  - L2：对比会话时间戳标记变更会话
  - Tab 徽章显示 L1+L3 变更计数，查看后自动清除

  **推荐 Agent Profile**：`unspecified-low` | **并行化**：YES（Wave 3）
  **被阻塞**：Task 11 | **提交**：YES

- [x] 15. **边缘情况处理 + 错误状态 UI**

  **做什么**：
  - 文件被删除/JSONL 损坏/权限拒绝的优雅降级，超大文件截断提示，全局错误边界

  **推荐 Agent Profile**：`unspecified-low` | **并行化**：YES（Wave 3）
  **被阻塞**：Task 11, 13 | **提交**：YES

---

## Final Verification Wave（所有实现任务完成后 — 4 并行审查）

> ALL 必须 APPROVE。汇总结果呈现给用户，获取明确 "okay" 后再完成。
> F1-F4 在用户批准前不得打勾。驳回或反馈 → 修复 → 重新运行 → 再次呈现 → 等待 okay。

- [x] F1. **Plan Compliance Audit** — `oracle`
  端到端审阅计划。对每个 "必须包含"：验证实现存在（读取文件、curl 端点、运行命令）。对每个 "必须不包含"：在代码库中搜索禁止模式 — 找到则返回 `file:line`。检查 `.sisyphus/evidence/` 中是否存在证据文件。将交付物与计划对照检查。
  输出：`必须包含 [N/N] | 必须不包含 [N/N] | 任务 [N/N] | 判定: APPROVE/REJECT`

- [x] F2. **Code Quality Review** — `unspecified-high`
  运行 `cargo check -p ptv` + `npm run build` + `npm test`。检查所有变更文件是否有：`as any`/`@ts-ignore`、空 catch、console.log、注释掉的代码、未使用的 import。检查 AI slop：过度注释、过度抽象、泛型名称（data/result/item/temp）。
  输出：`构建 [PASS/FAIL] | 测试 [N pass/N fail] | 文件 [N clean/N issues] | 判定`

- [x] F3. **Real Manual QA** — `unspecified-high`（+ `playwright` skill）
  从干净状态开始。执行每个任务中的 EVERY QA 场景 — 严格遵循步骤，捕获证据。测试跨任务集成（L1+L2 一起工作）。测试边缘情况：空状态、无效输入、并发操作。保存到 `.sisyphus/evidence/final-qa/`。
  输出：`场景 [N/N pass] | 集成 [N/N] | 边缘情况 [N tested] | 判定`

- [x] F4. **Scope Fidelity Check** — `deep`
  对每个任务：阅读 "做什么"、阅读实际 diff（git log/diff）。验证 1:1 — spec 中的所有内容都已构建（无遗漏），spec 之外的内容均未构建（无 creep）。检查 "绝不能做" 合规性。检测跨任务污染：Task N 触碰了 Task M 的文件。标记未记录的变更。
  输出：`任务 [N/N compliant] | 污染 [CLEAN/N issues] | 未记录 [CLEAN/N files] | 判定`

---

## Commit Strategy

| Wave | 任务 | 提交信息 |
|:-----|:-----|:--------|
| 1 | 1 | `feat(collector): 新增 project_files.rs 静态文件收集器` |
| 1 | 2 | `feat(backend): 新增 get_project_files + get_project_file_content 命令` |
| 1 | 3 | `feat(frontend): 新增 MarkdownRenderer 组件 (react-markdown + TOC)` |
| 1 | 4 | `feat(frontend): 新增 MemoryFileTree 树形导航组件` |
| 1 | 5 | `feat(frontend): 新增 ProjectMemoryPanel L1 静态记忆面板` |
| 2 | 6 | `feat(collector): 新增 session_transcript.rs 对话收集器（仿 tui.py 解析）` |
| 2 | 7 | `feat(backend): 新增 L2 最新会话 + 搜索 + 候选记忆 Tauri 命令` |
| 2 | 8 | `feat(frontend): 新增 SessionSearchView 对话搜索组件` |
| 2 | 9 | `feat(frontend): 新增 TranscriptDetailView 对话详情（assistant Markdown 渲染）` |
| 2 | 10 | `feat(frontend): 新增 CandidateMemoryBox 候选记忆箱` |
| 2 | 11 | `feat(frontend): L2 搜索 + 候选记忆集成 + 三级 Tab` |
| 2 | 12 | `config: 扩展 tauri fs scope` |
| 3 | 13 | `test(e2e): 新增项目记忆面板 E2E 测试` |
| 3 | 14 | `feat(frontend): L1+L2 变更检测 + Tab 徽章` |
| 3 | 15 | `fix(frontend): 边缘情况处理 + 错误状态 UI` |

---

## 成功标准

### 验证命令

```bash
# Rust 后端测试
cd src-tauri && cargo test -p ptv

# 前端构建
npm run build

# E2E 测试
npm test
```

### 最终检查清单

- [x] 所有 "必须包含" 已实现
- [x] 所有 "必须不包含" 未引入
- [x] `cargo test -p ptv` 全部通过
- [x] `npm run build` 通过
- [x] `npm test` 全部通过（含 ≥10 个新 E2E 测试）
- [x] L1：ai_project_template 中可见 40+ 文件，Markdown 渲染正确，TOC 导航生效
- [x] L2：搜索 "P60" 返回匹配会话，点击查看对话详情
- [x] L3：标记消息 → 候选箱 → 确认沉淀完整工作流
- [x] 变更检测：修改文件后 Tab 徽章更新
- [x] 空状态：无记忆/无对话的项目显示友好提示
- [x] 错误状态：权限被拒绝/损坏文件时显示黄色警告


