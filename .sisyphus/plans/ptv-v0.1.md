# ptv (Project Template Visualizer) — v0.1 工作计划

## TL;DR

> **快速摘要**：构建一个 Tauri v2 桌面应用，用于跨项目监控从 `ai_project_template` 创建的多个 FPGA 项目。整合 memory_dashboard 的项目上下文能力和 abtop 的实时 Agent 监控能力。
>
> **交付物**：
> - Tauri 桌面应用（**macOS + Linux**，不支持 Windows），React/TS 前端 + Rust 后端
- Linux 测试机：`100.85.255.89` (yufei)，优先在该机器上验证
> - 项目仪表盘（项目列表 + Stage + Git 状态）
> - 项目详情面板（Stage 时间线 / 参数快照 / Memory / 规则）
> - Agent 运行时监控面板（Token / 速率 / 会话状态）
> - 独立的 Rust 库 crate：`abtop-collector`（从 abtop 提取）
>
> **预估工作量**：Large（~15 个开发任务 + 4 个最终验证）
> **并行执行**：YES — 4 波次，峰值 6 个并行任务
> **关键路径**：Task 0.1 → 1.3 → 1.4 → 2.1/2.2/2.3 → 3.1 → F1-F4

---

## Context

### 原始需求

用户管理着一组从 `ai_project_template` 创建的 FPGA 项目。当并行项目多时，难以跨项目追踪状态。需要一个可视化工具来监控：
- 各项目当前 Stage（L0→L6→Verilog→Synthesis→Hardware）
- Claude Code 的 Memory 决策和对话上下文
- AI Agent 运行时状态（Token 消耗、会话状态）
- Git 变更和项目活跃度

### 访谈摘要

**关键讨论**：
- 形式：**Tauri 桌面应用**（非 TUI，非纯 Web），React/TS 前端 + Rust 后端
- 项目名：**ptv**（Project Template Visualizer）
- 项目发现：**GUI 手动添加**（不自动扫描文件系统）
- 测试策略：v0.1 使用 **Agent QA**（Playwright E2E 验证），无需单元测试
- 技术决策 #1：abtop 代码复用以**提取独立 crate** 方式（非 vendor）
- 技术决策 #2：config/parameters.py 解析为 **shell out to Python**（非 Rust AST）
- 刷新策略：文件数据走 watchfiles，Agent 数据走 2s 轮询

### 研究结论

- **memory_dashboard**（772 行 Python TUI）：提供 Memory/History/System/Config 四视图，纯只读，项目上下文浏览器
- **abtop**（~9800 行 Rust TUI）：提供跨项目 Agent 实时监控，collector 层（~5600 行）是核心——session 发现、JSONL 增量解析、进程树/端口扫描
- **两者互补**：memory_dashboard 看项目"是什么"（上下文/规则/参数），abtop 看项目"在干什么"（实时 agent 活动）
- **abtop 是二进制 crate**：不能直接 `cargo add`，必须提取为库 crate

### Metis 审查

**已解决的缺口**：
- ✅ abtop 复用方式已决策：提取为独立 `abtop-collector` 库 crate
- ✅ parameters.py 解析已决策：Rust 端 `python3` shell out 到 JSON
- ✅ 两种项目布局（flat/namespaced）的兼容策略
- ✅ 严格只读约束（绝不写入被监控项目）
- ✅ Memory 目录不存在时的优雅降级
- ✅ 项目路径失效时的错误处理
- ✅ Tauri v2 plugin-fs watch 用于文件监听

---

## Work Objectives

### 核心目标

构建 **ptv**：一个 Tauri 桌面应用，统一监控多个基于 `ai_project_template` 的 FPGA 项目，提供"项目状态 + AI Agent 活动"双维度实时视图。

### 具体交付物

- `src-tauri/` — Rust 后端（Tauri v2）
  - `commands/` — Tauri invoke 命令（注册项目、获取数据、事件推送）
  - `collectors/` — 模板数据采集器（stage、config、memory、git）
  - `abtop-collector/` — 外部 crate 依赖（从 abtop 提取）
- `src/` — React/TS 前端
  - `panels/Dashboard.tsx` — 项目仪表盘
  - `panels/ProjectDetail.tsx` — 项目详情
  - `panels/AgentMonitor.tsx` — Agent 运行时监控
  - `panels/Settings.tsx` — 项目管理设置
- 独立的 `abtop-collector` Rust 库 crate（GitHub repo）

### 验收标准

- [ ] `cargo tauri dev` 启动后，React 前端可正常渲染
- [ ] 手动添加项目后，Dashboard 在 5 秒内显示项目名称 + Stage + 状态
- [ ] 项目详情页展示：Stage 值、config 关键参数、Memory 条目数、Git 分支名
- [ ] Agent Monitor 在有活跃 Claude 会话时显示 PID + Token + 状态
- [ ] `.current_stage` 文件变化后，Dashboard 在 3 秒内自动更新
- [ ] 被监控项目路径被删除时，显示"路径不存在"状态而非崩溃

### Must Have

- Tauri v2 + React 18+ / TypeScript 桌面应用
- **平台**：macOS + Linux（Windows 不做），**Linux 优先验证**
- Linux 测试机 `100.85.255.89`（yufei，sshpass 连接）用于最终验证
- 项目仪表盘：名称 + 当前 Stage + 最近活动 + Git 分支 + 活跃 Agent 数量
- 项目详情：Stage 时间线 + 参数快照 + Memory 条目 + Git 变更文件
- Agent 监控：实时 Token 速率 + 上下文窗口% + 会话状态
- 手动添加/移除项目的 GUI
- 文件变更驱动的自动刷新（watchfiles）
- 错误/边界情况的优雅降级

### Must NOT Have（护栏）

- ❌ 自动扫描文件系统发现项目（仅 GUI 手动添加）
- ❌ 写入/修改/删除被监控项目的任何文件
- ❌ 启动/停止/重启任何 Claude Code 进程
- ❌ 引入 abtop 的 TUI 依赖（ratatui、crossterm）
- ❌ 告警/通知/声音（v0.2 功能）
- ❌ 历史趋势图或数据分析（v0.2 功能）
- ❌ 跨项目参数对比（v0.2 功能）
- ❌ 项目创建/克隆功能（项目管理工具范畴，非监控工具）
- ❌ 远程监控或 Web 服务
- ❌ 测试报告聚合（v0.2 功能）

---

## Verification Strategy

> **零人工干预** — 所有验证由 Agent 执行。不接受需要人工操作的验收标准。

### 测试决策

- **自动化测试**：无（v0.1 MVP 阶段）
- **Agent QA**：每个任务完成后，Agent 执行具体验证场景
- **验证工具**：`cargo test`（Rust）、Playwright（React 前端）、`cargo tauri dev`（端到端）

### QA 策略

每个任务必须包含 Agent 可执行的 QA 场景：
- **Rust 后端**：`cargo test` 运行指定测试，验证断言
- **React 前端**：Playwright 打开应用 → 交互 → 断言 DOM 状态 → 截图
- **端到端**：`cargo tauri dev` 启动 → Playwright 访问 → 完整流程验证

---

## Execution Strategy

### 并行执行波次

```
Wave 0（基础建设 — 最大并行）:
├── Task 0.1: 从 abtop 提取 abtop-collector 库 crate [deep]
├── Task 0.2: Tauri + React 最小骨架 [quick]
├── Task 0.3: Shell-out config 解析器原型 [quick]
└── Task 0.4: 文件监听集成验证 [quick]

Wave 1（Rust 数据层 — 最大并行）:
├── Task 1.1: 项目注册表 [quick]（依赖: 0.2）
├── Task 1.2: 模板项目数据采集器 [deep]（依赖: 0.3, 0.4）
└── Task 1.3: Agent 运行时采集器 [deep]（依赖: 0.1）

Wave 2（集成层 + 前端基础）:
├── Task 1.4: Tauri commands + 事件层 [deep]（依赖: 1.1, 1.2, 1.3）
└── Task 2.0: 前端脚手架（types, stores, hooks）[quick]（依赖: 0.2）

Wave 3（UI 面板 — 最大并行）:
├── Task 2.1: 仪表盘面板 [visual-engineering]（依赖: 1.4, 2.0）
├── Task 2.2: 项目详情面板 [visual-engineering]（依赖: 1.4, 2.0）
├── Task 2.3: Agent 监控面板 [visual-engineering]（依赖: 1.4, 2.0）
└── Task 2.4: 设置/添加项目面板 [visual-engineering]（依赖: 1.1, 2.0）

Wave 4（集成验证）:
├── Task 3.1: 端到端数据流验证 [unspecified-high]（依赖: 2.1-2.4）
├── Task 3.2: Playwright E2E 测试 [unspecified-high]（依赖: 2.1-2.4）
└── Task 3.3: Tauri 打包构建 [quick]（依赖: 3.1, 3.2）

Wave FINAL（完成前审查 — 4 并行 → 等待用户确认）:
├── Task F1: 规划合规审计 (oracle)
├── Task F2: 代码质量审查 (unspecified-high)
├── Task F3: 真实手动 QA (unspecified-high + playwright)
└── Task F4: 范围忠实度检查 (deep)

关键路径: 0.1 → 1.3 → 1.4 → 2.1/2.2/2.3 → 3.1 → F1-F4
并行加速: 约 60% 快于纯顺序执行
最大并发: 6 (Wave 0) + 4 (Wave 3)
```

### 依赖矩阵

- **0.1**: 无 → 1.3, Wave 1
- **0.2**: 无 → 1.1, 2.0, Wave 1-2
- **0.3**: 无 → 1.2, Wave 1
- **0.4**: 无 → 1.2, Wave 1
- **1.1**: 0.2 → 1.4, 2.4, Wave 2-3
- **1.2**: 0.3, 0.4 → 1.4, Wave 2
- **1.3**: 0.1 → 1.4, Wave 2
- **1.4**: 1.1, 1.2, 1.3 → 2.1, 2.2, 2.3, Wave 3
- **2.0**: 0.2 → 2.1, 2.2, 2.3, 2.4, Wave 3
- **2.1**: 1.4, 2.0 → 3.1, 3.2, Wave 4
- **2.2**: 1.4, 2.0 → 3.1, 3.2, Wave 4
- **2.3**: 1.4, 2.0 → 3.1, 3.2, Wave 4
- **2.4**: 1.1, 2.0 → 3.1, 3.2, Wave 4
- **3.1**: 2.1, 2.2, 2.3, 2.4 → 3.3, Wave FINAL
- **3.2**: 2.1, 2.2, 2.3, 2.4 → 3.3, Wave FINAL
- **3.3**: 3.1, 3.2 → Wave FINAL

### Agent 调度摘要

- **Wave 0**: 4 — T0.1 → `deep`, T0.2 → `quick`, T0.3 → `quick`, T0.4 → `quick`
- **Wave 1**: 3 — T1.1 → `quick`, T1.2 → `deep`, T1.3 → `deep`
- **Wave 2**: 2 — T1.4 → `deep`, T2.0 → `quick`
- **Wave 3**: 4 — T2.1-2.4 → `visual-engineering`
- **Wave 4**: 3 — T3.1 → `unspecified-high`, T3.2 → `unspecified-high`, T3.3 → `quick`
- **FINAL**: 4 — F1 → `oracle`, F2 → `unspecified-high`, F3 → `unspecified-high`, F4 → `deep`

---

## TODOs

> 实施 + 测试 = 一个任务。绝不分离。
> 每个任务必须包含：推荐 Agent 配置 + 并行化信息 + QA 场景。
> **没有 QA 场景的任务是不完整的。绝不允许。**

- [ ] 0.1 从 abtop 提取 `abtop-collector` 库 crate

  **What to do**：
  - Fork abtop 仓库到本地工作区
  - 创建新的 Rust 库 crate：`abtop-collector`，Cargo.toml 含 `[lib]` 段
  - 提取以下模块到 crate 中：
    - `src/collector/claude.rs`（~3340 行）— Claude session 发现 + transcript 解析
    - `src/collector/codex.rs`（~1297 行）— Codex CLI session 发现
    - `src/collector/process.rs` — ps/lsof/git 封装
    - `src/collector/rate_limit.rs` — 速率限制读取
    - `src/collector/mod.rs` — MultiCollector 编排 + AgentCollector trait
    - `src/model/session.rs` — 数据模型（AgentSession, SessionStatus 等）
    - `src/model/mod.rs`
  - 移除对 ratatui/crossterm 的依赖（collector 层不应依赖 TUI）
  - 移除对 abtop 内部 `app.rs`/`main.rs`/`ui/` 的引用
  - 确保 crate 可独立编译：`cargo build -p abtop-collector`
  - 验证核心 API 可用：`MultiCollector::new().collect()` 返回 `Vec<AgentSession>`

  **Must NOT do**：
  - 不要引入 ratatui、crossterm 或任何 TUI 依赖
  - 不要引入 abtop 的 `app.rs`、`main.rs`、`ui/` 模块

  **Recommended Agent Profile**：
  - **Category**: `deep` — 需要对 Rust 模块系统和 crate 结构有深入理解

  **Parallelization**：
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 0（与 Task 0.2、0.3、0.4 并行）
  - **Blocks**: Task 1.3

  **References**：
  - `/Users/ckstar/Repo/abtop/src/collector/` — 要提取的 collector 源码目录
  - `/Users/ckstar/Repo/abtop/src/model/` — 要提取的 model 源码目录
  - `/Users/ckstar/Repo/abtop/Cargo.toml` — 当前依赖清单，识别哪些是 collector 需要的
  - `serde`, `serde_json`, `chrono`, `dirs` — collector 层的实际依赖

  **Acceptance Criteria**：
  - [ ] `cargo build -p abtop-collector` 编译通过（无 ratatui/crossterm）
  - [ ] `cargo test -p abtop-collector` 所有已有测试通过

  **QA Scenarios**：
  ```
  Scenario: 编译并运行测试
    Tool: Bash
    Preconditions: abtop 仓库已 fork 到本地，abtop-collector crate 已创建
    Steps:
      1. cd abtop-collector && cargo build
      2. cargo test
    Expected Result: 编译成功，所有测试通过（0 failures）
    Evidence: .sisyphus/evidence/task-0.1-build-test.txt

  Scenario: 验证无 TUI 依赖
    Tool: Bash
    Steps:
      1. cargo tree -p abtop-collector | grep -E "ratatui|crossterm"
    Expected Result: 无输出（无 TUI 依赖）
    Evidence: .sisyphus/evidence/task-0.1-deps-check.txt
  ```

- [ ] 0.2 Tauri + React 最小骨架搭建

  **What to do**：
  - 使用 `create-tauri-app` 初始化 ptv 项目（React + TypeScript 模板）
  - 项目名为 `ptv`，目录为当前工作区
  - 配置 Tailwind CSS + shadcn/ui 组件库
  - 安装 Recharts 图表库（用于 token 速率图和 stage 时间线）
  - 创建基础布局组件：侧边栏导航 + 主内容区
  - 实现 Tauri IPC 验证：前端 `invoke("greet", { name: "ptv" })` → Rust 后端返回
  - 配置 `tauri-plugin-fs`（含 `watch` feature）
  - 验证 `cargo tauri dev` 可正常启动桌面窗口
  - 创建项目记忆文件 `AGENTS.md`（内容参考 `.sisyphus/drafts/project-memory.md`）

  **Must NOT do**：
  - 不实现任何业务面板（仅骨架）
  - 不引入 Redux/MobX 等重量级状态管理
  - **不在 Windows 上构建或测试**

  **Recommended Agent Profile**：
  - **Category**: `quick` — 脚手架搭建，标准化操作

  **Parallelization**：
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 0
  - **Blocks**: Task 1.1, 2.0

  **References**：
  - Tauri v2 文档：`https://v2.tauri.app/start/create-project/`
  - shadcn/ui：`https://ui.shadcn.com/docs/installation/vite`
  - Recharts：`https://recharts.org/en-US/guide`

  **Acceptance Criteria**：
  - [ ] `cargo tauri dev` 启动后桌面窗口出现，标题显示 "ptv"
  - [ ] `invoke("greet")` 返回 Rust 后端的响应
  - [ ] Tailwind 样式生效（蓝色按钮可见）

  **QA Scenarios**：
  ```
  Scenario: 启动并验证骨架
    Tool: Bash + Playwright
    Steps:
      1. cargo tauri dev（后台启动），等待 5 秒
      2. Playwright 连接 localhost:1420
      3. 截图窗口，验证标题栏显示 "ptv"
    Expected Result: 桌面窗口出现，标题 "ptv"
    Evidence: .sisyphus/evidence/task-0.2-skeleton.png
  ```

- [ ] 0.3 Shell-out config 解析器

  **What to do**：
  - 在 Rust 端实现 `parse_parameters_py(project_path: &Path) -> Result<ProjectConfig>`
  - 使用 `std::process::Command` 执行 Python 子进程导出 JSON
  - 定义 `ProjectConfig` struct（project_name, module_name, interface_type, data_width, iterations, q_int_bits, q_frac_bits, pipeline_stages, clock_frequency, use_l0, reference_project）
  - 处理：Python 不可用、文件不存在、语法错误、JSON 解析失败

  **Must NOT do**：
  - 不尝试用 Rust 解析 Python AST
  - 不修改被监控项目的 parameters.py

  **Recommended Agent Profile**：
  - **Category**: `quick` — 单文件实现

  **Parallelization**：
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 0
  - **Blocks**: Task 1.2

  **References**：
  - `/Users/ckstar/Repo/ai_project_template/config/parameters.py` — 参数结构定义
  - 目标目录：`src-tauri/src/collectors/config.rs`

  **Acceptance Criteria**：
  - [ ] `parse_parameters_py("/path/to/project")` 返回正确的 ProjectConfig
  - [ ] 路径不存在时返回 `Err`
  - [ ] Python 不可用时返回清晰错误信息

  **QA Scenarios**：
  ```
  Scenario: 解析真实项目的 parameters.py
    Tool: Bash
    Preconditions: 存在含 config/parameters.py 的测试项目
    Steps:
      1. cargo test test_parse_parameters_real_project -- --nocapture
    Expected Result: 测试通过，JSON 字段值与 Python 端一致
    Evidence: .sisyphus/evidence/task-0.3-test-output.txt
  ```

- [ ] 0.4 文件监听集成验证

  **What to do**：
  - Rust 端测试 `tauri-plugin-fs` 的 `watch()` API
  - 验证能监听 `.current_stage`、`config/parameters.py`、`.claude/memory/` 内文件变化
  - 确认文件修改后 3 秒内触发事件
  - 创建 `FileWatcher` 抽象层，封装 plugin-fs API

  **Must NOT do**：
  - 不手动引入 `notify` crate

  **Recommended Agent Profile**：
  - **Category**: `quick`

  **Parallelization**：
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 0
  - **Blocks**: Task 1.2

  **References**：
  - Tauri plugin-fs watch：`https://v2.tauri.app/plugin/file-system/#watch`

  **Acceptance Criteria**：
  - [ ] 写入 `.current_stage` 后 3 秒内收到变更事件
  - [ ] 监听目录时子文件变化也能收到事件
  - [ ] 权限问题返回明确错误而非 panic

  **QA Scenarios**：
  ```
  Scenario: 修改 .current_stage 触发事件
    Tool: Bash
    Steps:
      1. 启动 FileWatcher 监听测试目录
      2. echo "l3" > 测试目录/.current_stage
      3. 检查控制台在 5 秒内输出变更事件
    Expected Result: 3 秒内输出 ".current_stage changed"
     Evidence: .sisyphus/evidence/task-0.4-watch-log.txt
  ```

- [ ] 1.1 项目注册表

  **What to do**：
  - Rust 端实现 `ProjectRegistry` struct
  - 功能：`add(path)` / `remove(path)` / `list()` / `get(path)`
  - 持久化到 `app_data_dir()/projects.json`
  - 去重：相同规范化绝对路径禁止重复添加
  - 路径不存在时 `get()` 返回 error variant（非 panic）
  - 暴露 Tauri commands：`add_project`, `remove_project`, `list_projects`

  **Must NOT do**：不自动扫描文件系统，不验证路径是否为模板项目

  **Recommended Agent Profile**：
  - **Category**: `quick` — 标准 CRUD 实现

  **Parallelization**：
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1（与 1.2、1.3 并行）
  - **Blocks**: Task 1.4, 2.4
  - **Blocked By**: Task 0.2

  **Acceptance Criteria**：
  - [ ] 添加/移除/列出项目功能正确，重启后数据持久化
  - [ ] 重复添加相同路径返回错误

  **QA Scenarios**：
  ```
  Scenario: 添加/列出/移除项目
    Tool: Bash (cargo test)
    Steps:
      1. cargo test test_registry_add_list_remove -- --nocapture
    Expected Result: 所有测试通过
    Evidence: .sisyphus/evidence/task-1.1-test-output.txt
  ```

- [ ] 1.2 模板项目数据采集器

  **What to do**：
  - 创建 `src-tauri/src/collectors/template/` 目录
  - `StageCollector`：读取 `.current_stage`，解析为 `Stage` enum
  - `ConfigCollector`：调用 Task 0.3 的 `parse_parameters_py()`
  - `MemoryCollector`：读取 `.claude/memory/*.md`，YAML frontmatter 解析
  - `GitCollector`：`git branch` + `git status --porcelain`
  - 集成 FileWatcher：stage/config/memory 变化时触发重新采集
  - 处理两种源码布局（flat/namespaced）

  **Must NOT do**：不写入被监控项目，不 panic 于路径不存在

  **Recommended Agent Profile**：
  - **Category**: `deep` — 多 collector 协调

  **Parallelization**：
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1
  - **Blocks**: Task 1.4
  - **Blocked By**: Task 0.3, 0.4

  **Acceptance Criteria**：
  - [ ] 所有 collector 正确处理：正常数据、空数据、不存在路径
  - [ ] 文件变更后 5 秒内触发重新采集

  **QA Scenarios**：
  ```
  Scenario: 采集所有模板数据
    Tool: Bash (cargo test)
    Steps:
      1. cargo test test_template_collectors_all -- --nocapture
    Expected Result: stage 正确，config 字段完整，memory 条目匹配
    Evidence: .sisyphus/evidence/task-1.2-test-output.txt

  Scenario: 边界 — 空项目（无 memory 目录）
    Tool: Bash (cargo test)
    Steps:
      1. cargo test test_memory_empty_dir
    Expected Result: 返回空 Vec，不 panic
    Evidence: .sisyphus/evidence/task-1.2-edge-empty.txt
  ```

- [ ] 1.3 Agent 运行时采集器

  **What to do**：
  - 在 `Cargo.toml` 添加 `abtop-collector` 依赖（Task 0.1 产出）
  - 创建包装层 `src-tauri/src/collectors/agent/mod.rs`
  - 每 2 秒调用 `MultiCollector::collect()`，将结果按 cwd 关联到注册项目
  - 通过 Tauri event `agent-update` 推送数据到前端

  **Must NOT do**：不引入 ratatui/crossterm

  **Recommended Agent Profile**：
  - **Category**: `deep` — 需要理解 abtop-collector API + Tauri 事件

  **Parallelization**：
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1
  - **Blocks**: Task 1.4
  - **Blocked By**: Task 0.1

  **Acceptance Criteria**：
  - [ ] 能检测活跃 Claude/Codex 会话并正确关联到项目
  - [ ] 前端收到 `agent-update` 事件含完整数据
  - [ ] 无活跃会话时返回空数组不报错

  **QA Scenarios**：
  ```
  Scenario: 检测活跃 Claude 会话
    Tool: Bash
    Preconditions: 至少有一个 Claude Code 进程运行中
    Steps:
      1. 启动 ptv，等待 5 秒
      2. 检查 agent-update 事件包含 pid, session_id, status, model
    Expected Result: 事件含活跃会话信息
    Evidence: .sisyphus/evidence/task-1.3-agent-output.txt
  ```

- [ ] 1.4 Tauri commands + 事件层

  **What to do**：
  - 实现完整 Tauri commands：
    - `add_project(path)` / `remove_project(path)` / `list_projects()` — 调用 ProjectRegistry
    - `get_project_detail(path)` — 聚合 Stage + Config + Memory + Git 数据
    - `get_agent_sessions()` — 返回当前缓存的 Agent 会话列表
  - 实现 Tauri 事件推送：
    - `project-update` — 文件变化时推送单项目数据
    - `agent-update` — 每 2 秒推送 Agent 运行时数据
  - 实现后台轮询 loop（tokio::spawn 方式，非 abtop 的同步 tick）
  - JSON 序列化所有数据结构（derive Serialize）

  **Must NOT do**：不在 commands 中 panic，所有错误返回 `Result::Err`

  **Recommended Agent Profile**：
  - **Category**: `deep` — 多数据源聚合 + Tauri IPC

  **Parallelization**：
  - **Can Run In Parallel**: NO（依赖 1.1、1.2、1.3 全部完成）
  - **Parallel Group**: Wave 2（与 Task 2.0 并行）
  - **Blocks**: Task 2.1, 2.2, 2.3

  **Acceptance Criteria**：
  - [ ] `invoke("list_projects")` 返回正确 JSON
  - [ ] `invoke("get_project_detail", { path })` 返回完整聚合数据
  - [ ] 事件 `project-update` 在文件变化后触发
  - [ ] 所有 command 错误返回 `{ error: "..." }` 格式

  **QA Scenarios**：
  ```
  Scenario: 验证所有 Tauri commands
    Tool: Bash
    Steps:
      1. cargo test test_commands_all -- --nocapture
      2. 验证 add_project → list_projects → get_project_detail 链
    Expected Result: 所有 command 返回正确数据
    Evidence: .sisyphus/evidence/task-1.4-test-output.txt
  ```

- [ ] 2.0 前端脚手架（类型定义、状态管理、API hooks）

  **What to do**：
  - TypeScript 类型定义（与 Rust struct 对齐）：`ProjectEntry`, `ProjectConfig`, `MemoryEntry`, `AgentSession`, `GitStatus`
  - React Context + hooks 状态管理：`useProjects()`, `useSelectedProject()`, `useAgentSessions()`
  - Tauri invoke 封装：`api.listProjects()`, `api.getProjectDetail(path)`, `api.addProject(path)`, `api.removeProject(path)`
  - Tauri event 监听：`useProjectUpdates()`, `useAgentUpdates()`
  - 安装并配置 shadcn/ui 组件（Card, Button, Badge, Tabs, Table, ScrollArea）

  **Must NOT do**：不引入 Redux/Zustand（React Context 足够）

  **Recommended Agent Profile**：
  - **Category**: `quick` — 类型定义 + hooks 封装

  **Parallelization**：
  - **Can Run In Parallel**: YES（与 Task 1.4 并行）
  - **Parallel Group**: Wave 2
  - **Blocks**: Task 2.1, 2.2, 2.3, 2.4

  **Acceptance Criteria**：
  - [ ] `npm run build` 无 TypeScript 错误
  - [ ] hooks 能正确调用 Tauri invoke 并更新状态

  **QA Scenarios**：
  ```
  Scenario: 验证 hooks 能调用后端
    Tool: Playwright
    Steps:
      1. npm run dev（前端开发模式）
      2. Playwright 打开 localhost:5173
      3. 调用 window.__PTV__.listProjects()
    Expected Result: 返回项目列表 JSON
    Evidence: .sisyphus/evidence/task-2.0-hooks-test.png
  ```

- [ ] 2.1 仪表盘面板

  **What to do**：
  - React 组件 `panels/Dashboard.tsx`
  - 主列表：表格显示每个项目的
    - 项目名称（从 ProjectConfig.project_name）
    - 当前 Stage（彩色 Badge：L1=蓝, L5=橙, Verilog=紫...）
    - 最近活动时间（从文件 mtime）
    - 活跃 Agent 数量（从 agent-update 事件计数）
    - Git 分支名 + 变更文件数（`+N ~M` 格式）
  - 点击项目行 → 选中并触发项目详情加载
  - 空状态："暂无项目，点击设置添加"引导卡片
  - 响应式：宽屏多列，窄屏自动折叠

  **Must NOT do**：不在此面板嵌入项目详情

  **Recommended Agent Profile**：
  - **Category**: `visual-engineering` — UI 组件 + Recharts

  **Parallelization**：
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3（与 2.2、2.3、2.4 并行）
  - **Blocks**: Task 3.1, 3.2

  **Acceptance Criteria**：
  - [ ] 仪表盘显示所有注册项目，每行项目名 + Stage Badge + Git 状态
  - [ ] 点击项目行后 `selectedProjectPath` 状态更新
  - [ ] 空项目列表时显示引导 UI
  - [ ] 文件变化后 Stage Badge 自动更新

  **QA Scenarios**：
  ```
  Scenario: 查看仪表盘（有项目）
    Tool: Playwright
    Preconditions: 已注册至少 2 个测试项目
    Steps:
      1. 打开 ptv，等待 Dashboard 加载
      2. 截图：验证表格包含项目名、Stage Badge、Git 分支
      3. 点击第一行项目，验证高亮
    Expected Result: 表格正确显示，Stage Badge 颜色区分正确
    Evidence: .sisyphus/evidence/task-2.1-dashboard.png

  Scenario: 空状态显示
    Tool: Playwright
    Preconditions: 移除所有已注册项目
    Steps:
      1. 打开 ptv，确认无项目
      2. 验证显示"暂无项目，点击设置添加"
    Expected Result: 空状态引导卡片可见
    Evidence: .sisyphus/evidence/task-2.1-empty.png
  ```

- [ ] 2.2 项目详情面板

  **What to do**：
  - React 组件 `panels/ProjectDetail.tsx`
  - 4 个标签页（shadcn/ui Tabs）：
    1. **Stage 时间线**：Recharts 横向步骤条，显示 L0→...→Hardware 进程，当前阶段高亮
    2. **参数快照**：Card 列表，展示 key=value（project_name, data_width, q_format 等）
    3. **Memory**：按 type 分组的条目列表（👤用户/💡反馈/📋项目/🔗参考），显示 name + description
    4. **Git**：分支名 + 未提交文件列表（新增/修改）
  - 加载状态：Spinner + "加载中..."
  - 错误状态：项目路径不存在 → "项目路径不可用"
  - 空数据状态：Memory 目录为空 → "暂无 Memory 条目"

  **Must NOT do**：不实现参数编辑功能（只读）

  **Recommended Agent Profile**：
  - **Category**: `visual-engineering` — 多标签详情页

  **Parallelization**：
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3
  - **Blocks**: Task 3.1, 3.2

  **Acceptance Criteria**：
  - [ ] 4 个标签页全部正确渲染
  - [ ] Stage 时间线高亮当前阶段
  - [ ] Memory 按类型分组显示
  - [ ] Git 标签显示分支名和变更文件

  **QA Scenarios**：
  ```
  Scenario: 查看项目详情
    Tool: Playwright
    Steps:
      1. 在 Dashboard 选中一个项目
      2. 点击进入详情
      3. 依次点击 Stage/Config/Memory/Git 标签
      4. 每页截图验证
    Expected Result: 每页数据正确渲染
    Evidence:
      - .sisyphus/evidence/task-2.2-stage.png
      - .sisyphus/evidence/task-2.2-config.png
      - .sisyphus/evidence/task-2.2-memory.png
      - .sisyphus/evidence/task-2.2-git.png
  ```

- [ ] 2.3 Agent 监控面板

  **What to do**：
  - React 组件 `panels/AgentMonitor.tsx`
  - 实时数据来自 Tauri event `agent-update`
  - 显示每个活跃会话：
    - Agent CLI 图标（CC=Claude Code / CD=Codex）
    - PID + 项目名
    - 状态指示（● 执行中 / ◌ 等待 / ⏳ 限流）
    - Token 用量（k/M 单位）
    - 上下文窗口%（进度条，80%黄色，90%红色+⚠）
    - 当前任务（tool name + first arg）
  - Token 速率图：Recharts 面积图，展示近 2 分钟趋势
  - 无活跃会话时显示"无活跃 Agent"

  **Must NOT do**：不在此面板实现 session kill 功能（v0.1 只读）

  **Recommended Agent Profile**：
  - **Category**: `visual-engineering` — 实时数据面板 + 图表

  **Parallelization**：
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3
  - **Blocks**: Task 3.1, 3.2

  **Acceptance Criteria**：
  - [ ] 活跃会话实时显示（2 秒内更新）
  - [ ] Token 速率图正确渲染
  - [ ] 上下文窗口% 进度条颜色正确切换

  **QA Scenarios**：
  ```
  Scenario: 查看 Agent 监控（有活跃会话）
    Tool: Playwright
    Preconditions: 至少有一个 Claude Code 会话活跃
    Steps:
      1. 切换到 Agent Monitor 面板
      2. 等待 5 秒让数据获取
      3. 截图：验证会话列表、token 图表、上下文%条
    Expected Result: 会话正确显示，图表渲染
    Evidence: .sisyphus/evidence/task-2.3-agent-active.png

  Scenario: 无活跃会话
    Tool: Playwright
    Preconditions: 关闭所有 Claude/Codex
    Steps:
      1. 切换到 Agent Monitor
      2. 验证显示"无活跃 Agent 会话"
    Expected Result: 正确显示空状态
    Evidence: .sisyphus/evidence/task-2.3-agent-empty.png
  ```

- [ ] 2.4 设置/添加项目面板

  **What to do**：
  - React 组件 `panels/Settings.tsx`
  - 项目列表（已注册），每行可移除（带确认对话框）
  - 添加项目：输入框（路径）+ 浏览按钮（Tauri dialog） + 添加按钮
  - 验证：路径必须存在且为目录
  - 添加成功后刷新 Dashboard
  - 移除确认：二次确认对话框（"确定移除？不会删除源文件"）

  **Must NOT do**：不实现自动扫描/发现功能

  **Recommended Agent Profile**：
  - **Category**: `visual-engineering` — 设置表单

  **Parallelization**：
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3
  - **Blocks**: Task 3.1, 3.2

  **Acceptance Criteria**：
  - [ ] 可手动输入路径添加项目
  - [ ] 添加不存在的路径时显示错误提示
  - [ ] 移除项目有二次确认
  - [ ] 添加/移除后 Dashboard 立即更新

  **QA Scenarios**：
  ```
  Scenario: 添加和移除项目
    Tool: Playwright
    Steps:
      1. 打开设置面板
      2. 输入测试项目路径 → 点击添加
      3. 验证 Dashboard 中出现新项目
      4. 点击移除 → 确认 → 验证 Dashboard 中消失
    Expected Result: 添加/移除流程正常工作
    Evidence: .sisyphus/evidence/task-2.4-settings.png
  ```

- [ ] 3.1 端到端数据流验证

  **What to do**：
  - 验证完整数据流：Rust collector → Tauri command → React state → UI 渲染
  - 测试场景：启动 ptv → 添加 2 个项目 → 验证 Dashboard → 切换详情 → 查看 Agent
  - 测试错误流：添加无效路径 → 验证错误提示 → 移除项目
  - 测试实时更新流：修改 `.current_stage` → 验证 Dashboard 在 5 秒内更新

  **Must NOT do**：不修改测试项目的实际文件

  **Recommended Agent Profile**：
  - **Category**: `unspecified-high` — 端到端集成验证

  **Parallelization**：
  - **Can Run In Parallel**: YES（与 3.2、3.3 并行）
  - **Parallel Group**: Wave 4
  - **Blocks**: Task F1-F4

  **Acceptance Criteria**：
  - [ ] 完整 happy-path 流程无报错
  - [ ] 错误路径显示合理错误信息
  - [ ] 实时更新在 5 秒内生效

  **QA Scenarios**：
  ```
  Scenario: 完整使用流程
    Tool: Playwright
    Steps:
      1. 启动 ptv
      2. 添加测试项目 A、B
      3. 验证 Dashboard 显示两个项目
      4. 选中 A → 查看详情 → 切换 4 个标签页
      5. 切换到 Agent Monitor
      6. 返回设置 → 移除项目 B → 验证 Dashboard 更新
      7. 全程截图
    Expected Result: 全部流程无崩溃，UI 状态一致
    Evidence: .sisyphus/evidence/task-3.1-e2e-flow/
  ```

- [ ] 3.2 Playwright E2E 测试套件

  **What to do**：
  - 编写 Playwright 测试脚本，覆盖所有面板
  - Dashboard：项目列表渲染、空状态、选中交互
  - 项目详情：4 个标签页内容正确
  - Agent 监控：有/无活跃会话两种状态
  - 设置：添加/移除项目流程
  - 错误状态：无效路径、缺失文件

  **Must NOT do**：不依赖真实 Claude 进程（mock agent 数据）

  **Recommended Agent Profile**：
  - **Category**: `unspecified-high` — 测试自动化

  **Parallelization**：
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 4

  **QA Scenarios**：
  ```
  Scenario: 运行全部 E2E 测试
    Tool: Bash + Playwright
    Steps:
      1. npx playwright test
    Expected Result: 所有测试通过（> 8 个 test case）
    Evidence: .sisyphus/evidence/task-3.2-playwright-results.txt
  ```

- [ ] 3.3 Tauri 打包构建（macOS .dmg + Linux AppImage）

  **What to do**：
  - 配置 `tauri.conf.json`：应用名 "ptv"、bundle identifier、图标
  - 配置 macOS entitlements（首次启动权限）
  - `cargo tauri build` 生成 macOS `.dmg` 和 Linux AppImage
  - 使用 `sshpass` 将 Linux 构建包部署到测试机 `100.85.255.89`
  - 在 Linux 测试机上验证应用可独立运行
  - 验证路径：`/home/yufei/Repo/ai_project_template_visualization`

  **Recommended Agent Profile**：
  - **Category**: `quick` — Tauri 标准打包流程

  **Parallelization**：
  - **Can Run In Parallel**: NO（依赖 3.1、3.2 完成）
  - **Parallel Group**: Wave 4（串行最后）
  - **Blocks**: Wave FINAL

  **Acceptance Criteria**：
  - [ ] macOS：`cargo tauri build` 成功生成 `.dmg`
  - [ ] Linux：`cargo tauri build` 成功生成 AppImage
  - [ ] Linux 测试机上应用可独立打开运行

  **QA Scenarios**：
  ```
  Scenario: 构建并部署到 Linux 测试机
    Tool: Bash
    Steps:
      1. cargo tauri build（Linux target）
      2. sshpass -p 'yufei' scp 构建产物到 100.85.255.89:/home/yufei/Repo/ai_project_template_visualization/
      3. sshpass -p 'yufei' ssh yufei@100.85.255.89 验证应用可启动
    Expected Result: 构建成功，Linux 上可运行
    Evidence: .sisyphus/evidence/task-3.3-linux-verify.txt
  ```

---

## Final Verification Wave (MANDATORY — after ALL implementation tasks)

> 4 个审查 agent 并行运行。全部必须 APPROVE。向用户展示汇总结果，获取明确"可以"后才完成。
> **在获取用户明确批准前，绝不标记 F1-F4 为已完成。**

- [ ] F1. **规划合规审计** — `oracle`
  通读规划全文。检查每个 "Must Have"：验证实施是否存在（读文件、curl 端点、运行命令）。检查每个 "Must NOT Have"：搜索代码库中的禁止模式 — 如发现则拒绝并标注 file:line。检查证据文件存在于 `.sisyphus/evidence/`。对比交付物与规划。
  输出：`Must Have [N/N] | Must NOT Have [N/N] | Tasks [N/N] | VERDICT: APPROVE/REJECT`

- [ ] F2. **代码质量审查** — `unspecified-high`
  运行 `cargo clippy -- -D warnings` + `npm run lint` + `cargo test`。审查所有变更文件：`unwrap()` 使用、空 catch、console.log 残留、注释掉的代码、未用 import。检查 AI 代码坏味道：过度注释、过度抽象、通用命名（data/result/item/temp）。
  输出：`Build [PASS/FAIL] | Lint [PASS/FAIL] | Tests [N pass/N fail] | Files [N clean/N issues] | VERDICT`

- [ ] F3. **真实 QA 验证** — `unspecified-high`（+ `playwright` skill）
  从干净状态开始。执行每个任务的 QA 场景——遵循精确步骤，捕获证据。测试跨任务集成（功能协作，非隔离）。测试边界：空状态、无效输入、快速操作。保存到 `.sisyphus/evidence/final-qa/`。
  输出：`Scenarios [N/N pass] | Integration [N/N] | Edge Cases [N tested] | VERDICT`

- [ ] F4. **范围忠实度检查** — `deep`
  对每个任务：读 "What to do"，读实际 diff（git log/diff）。验证 1:1 — 规范中所有内容已构建（无缺失），规范之外无构建（无蔓延）。检查 "Must NOT do" 合规。检测跨任务污染：Task N 触碰 Task M 的文件。标记未计入的变更。
  输出：`Tasks [N/N compliant] | Contamination [CLEAN/N issues] | Unaccounted [CLEAN/N files] | VERDICT`

---

## Commit Strategy

- **Wave 0**: `feat: abtop-collector crate 提取` — abtop-collector/ 目录
- **Wave 0**: `feat: Tauri + React 项目骨架` — src-tauri/, src/, package.json
- **Wave 0**: `feat: shell-out config 解析器` — src-tauri/src/collectors/config.rs
- **Wave 0**: `feat: 文件监听集成` — src-tauri/src/watcher.rs
- **Wave 1**: `feat: 项目注册表` — src-tauri/src/registry.rs
- **Wave 1**: `feat: 模板项目数据采集器` — src-tauri/src/collectors/template/
- **Wave 1**: `feat: Agent 运行时采集器` — src-tauri/src/collectors/agent/
- **Wave 2**: `feat: Tauri commands + 事件层` — src-tauri/src/commands.rs
- **Wave 2**: `feat: 前端类型定义与 hooks` — src/types/, src/hooks/
- **Wave 3**: `feat: 仪表盘面板` — src/panels/Dashboard.tsx
- **Wave 3**: `feat: 项目详情面板` — src/panels/ProjectDetail.tsx
- **Wave 3**: `feat: Agent 监控面板` — src/panels/AgentMonitor.tsx
- **Wave 3**: `feat: 设置面板` — src/panels/Settings.tsx
- **Wave 4**: `test: 端到端数据流验证` + `Playwright E2E` + `打包构建`

---

## Success Criteria

### 验证环境

| 环境 | 信息 | 用途 |
|:-----|:-----|:-----|
| **Linux 测试机**（优先） | `100.85.255.89`，用户 `yufei`，`sshpass` 已配置 | 优先验证平台 |
| macOS 开发机 | 本机 | 开发调试 |
| Windows | — | **不实现、不测试** |

### 验证命令

```bash
# Rust 后端测试
cargo test                     # 预期: 所有测试通过

# Rust 代码质量
cargo clippy -- -D warnings    # 预期: 0 warnings

# 前端构建
npm run build                  # 预期: 无 TypeScript 错误

# 前端 lint
npm run lint                   # 预期: 0 errors

# Tauri 开发模式启动
cargo tauri dev                # 预期: 桌面窗口打开，显示 ptv

# E2E 测试
npx playwright test            # 预期: > 8 test cases 通过

# 打包构建
cargo tauri build              # 预期: 生成 .dmg 文件
```

### 最终清单

- [ ] 所有 "Must Have" 存在
- [ ] 所有 "Must NOT Have" 缺失
- [ ] 所有测试通过
- [ ] Dashboard 显示注册项目及其 Stage、Git 状态
- [ ] 项目详情可切换 Stage/Config/Memory/Git 四个标签
- [ ] Agent Monitor 显示活跃会话和 token 速率图
- [ ] 文件变化后 5 秒内 UI 自动更新
- [ ] `.dmg` 安装包可独立运行

