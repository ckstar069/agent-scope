# AgentScope 第一阶段重构方案

> 本文档汇总第一阶段重构的背景、目的及详细方案。
> 最后更新：2026-05-13

---

## 一、重构背景

### 1.1 项目现状

AgentScope（原名 PTV）是一款跨平台 Tauri v2 桌面应用，用于实时监控和管理 AI Agent 会话及项目状态。当前版本 v0.2.0 已实现核心功能：

- 项目仪表盘（Stage、Git 状态、Agent 活跃度）
- 项目详情（时间线、参数快照、Memory 浏览、Git 状态）
- Agent 监控（实时会话追踪、Token 速率、工具调用）
- Claude 历史（会话搜索、导出、预览）
- 记忆标记（对话标记 → 沉淀为项目文档）

| 部分 | 规模 | 技术栈 |
|------|------|--------|
| 前端 | ~14,600 行 | React 19 + TypeScript + Tailwind CSS v4 + shadcn/ui |
| 后端 | ~3,000+ 行 | Rust (Tauri v2) |
| abtop-collector | ~1,500+ 行 | 本地 Rust crate，支持 Claude / Codex / MCP 多源采集 |

### 1.2 当前代码诊断

#### 后端核心问题

| 文件 | 行数 | 问题 |
|------|------|------|
| `commands.rs` | **907 行** | God File — 包含 AppState、17 个序列化结构体、16 个命令函数、状态初始化逻辑，新增功能需不断膨胀 |
| `lib.rs` | 97 行 | 命令注册列表与 `commands.rs` 重复维护，新增命令需改两处 |
| `collectors/template/mod.rs` | 410 行 | 混合了数据结构定义、采集逻辑、watched 事件推送 |
| `collectors/agent/mod.rs` | 1091 行 | 体量过大，包含进程监控、会话映射、事件发射 |

#### 前端核心问题

| 方面 | 现状 |
|------|------|
| 组织方式 | 按文件类型（`pages/`、`components/`、`hooks/`），非按业务领域 |
| 类型定义 | 散落在各组件中（如 `Dashboard.tsx` 内定义 `TemplateDataPayload`），无共享 |
| 样式体系 | shadcn/ui 默认主题（base-nova），与 `DESIGN.md` 的 Linear 设计系统不一致 |
| 组件边界 | `components/` 目录下业务组件与通用组件混放，新增功能时难以定位应放置位置 |

#### 构建与分发现状

| 方面 | 现状 | 问题 |
|------|------|------|
| CI/CD 配置 | **完全缺失** | 每次发版需手动在三台机器上分别构建 |
| 构建目标 | `tauri.conf.json` 中 `"targets": "all"` | 理论支持全平台，但未经自动化验证 |
| 产物签名 | 未配置 | macOS 需 Notarization，Windows 需证书签名 |
| 版本管理 | 手动修改 `package.json` + `Cargo.toml` + `tauri.conf.json` 三处 | 易遗漏，版本不一致 |

### 1.3 测试状态

| 测试类型 | 结果 | 说明 |
|----------|------|------|
| Rust 单元测试 | **120 passed, 0 failed** | 当前全部通过 |
| E2E 测试 | 大部分已修复，剩 1 个文本断言失步 | UI 副标题文本变更后测试未同步更新 |

### 1.4 技术债务

| 债务项 | 位置 | 严重程度 | 处理时机 |
|--------|------|----------|----------|
| E2E 测试与 UI 不同步（剩余 1 个） | `e2e/agent-monitor.spec.ts:18` | 高 | **Phase 1 — Pipeline #248 失败根因** |
| CI job artifact 无限堆积 | `.gitlab-ci.yml` | 中 | **Phase 1 — 影响外部软件发布页项目** |
| 编译器 warnings | `src-tauri/` 多处 | 中 | 随重构逐步清理 |
| ~~文档仍用旧名"PTV"~~ | ~~`docs/distribution.md`~~ | ~~中~~ | ~~已验证无残留~~ |

---

## 二、重构目的

本阶段重构**不新增任何功能**，仅对现有已实现内容进行结构性调整，核心目的有三：

### 2.1 降低代码耦合，建立独立维护边界

当前 `commands.rs` 是 907 行的 God File，所有命令、模型、状态混杂。新增任何一个命令都需修改此文件，极易引入冲突。目标是通过 **models / app_state / routes / services** 拆分，让每个功能域拥有独立的文件边界，后续扩展只需新增文件而非修改现有大文件。

**Phase 1 范围**：只拆分文件和收敛边界，不强行抽象通用 Collector trait（三种采集器形态差异过大，统一时机不成熟）。

### 2.2 前端按业务领域重组，消除类型与组件的重复定义

当前类型定义散落在各页面组件中，同一数据结构（如 `ProjectConfig`、`GitStatus`）在 Dashboard 和 ProjectDetail 中各定义一次。目标是通过 **Feature 垂直切片**，将组件、hooks、类型按业务领域内聚，外部通过 `index.ts` 暴露接口，禁止跨 feature 直接引用。

**Phase 1 范围**：只做目录迁移和共享类型/API 封装，UI 视觉保持当前主题不大改（视觉 token 化放入后续阶段）。

### 2.3 建立 CI/CD 基础验证能力

当前无自动化构建和验证能力。目标是建立 **CI 流水线**，确保每次 push 都能在 Ubuntu 上完成前端构建、Rust check、Rust test，为后续三平台发布建立基础。

**Phase 1 范围**：只补 `ci.yml`，确保 `npm run build`、`cargo check`、`cargo test` 可跑；release 流水线、多平台产物、签名/Notarization 放到下一步。

---

## 三、重构方案

### 3.1 后端模块化重构

#### 3.1.1 目标架构

```
src-tauri/src/
├── main.rs                 # 入口（不变）
├── lib.rs                  # Tauri builder（精简到 ~40 行）
├── app_state.rs            # AppState 独立提取
├── routes/                 # Tauri 命令 = HTTP 路由层（只做参数解析 + 调用 service）
│   ├── mod.rs              # 路由注册汇总（替代 lib.rs 中的命令列表）
│   ├── project.rs          # 项目注册、列表、数据获取
│   ├── agent.rs            # Agent 监控（启动/停止、事件推送）
│   ├── history.rs          # 会话历史（搜索、导出、预览）
│   ├── memory.rs           # 记忆标记（读写 decisions.md）
│   └── settings.rs         # 设置（模板路径等）
├── services/               # 业务逻辑层（命令的实际实现）
│   ├── project_service.rs
│   ├── agent_service.rs
│   ├── history_service.rs
│   ├── memory_service.rs
│   └── settings_service.rs
├── models/                 # 共享序列化结构体（替代 commands.rs 中的 Ser* 结构）
│   ├── mod.rs
│   ├── project.rs          # SerStage, SerProjectConfig, SerGitStatus 等
│   ├── agent.rs
│   └── history.rs
├── collectors/             # 数据采集器（保持现有目录结构，Phase 1 不做通用 trait）
│   ├── template/           # 模板项目采集
│   ├── agent/              # Agent 实时监控
│   └── claude_history/     # 历史会话扫描
├── registry.rs             # ProjectRegistry（已有，保持独立）
└── watcher.rs              # FileWatcher（已有，保持独立）
```

#### 3.1.2 重构原则

- **分层架构**：`routes/`（接口层）→ `services/`（业务逻辑）→ `collectors/`（数据采集）→ `models/`（数据结构）
- **命令即路由**：`routes/*.rs` 只做参数解析和调用 service，不包含业务逻辑
- **状态集中**：`AppState` 独立为 `app_state.rs`，各 service 通过依赖注入获取所需状态
- **平台无关**：路径操作必须使用 `std::path::PathBuf`，禁止字符串拼接路径
- **暂不做通用 trait**：`template`（一次性采集）、`agent`（轮询事件推送）、`claude_history`（文件扫描）三种形态差异过大，Phase 1 只收敛文件边界，不强行统一接口

#### 3.1.3 渐进拆分策略

避免大爆炸改动，按命令域逐步迁移，期间保留兼容 re-export：

| 批次 | 命令域 | 涉及的 commands.rs 函数 | 策略 |
|------|--------|------------------------|------|
| 1 | settings / history | `get_template_path`, `set_template_path`, `list_claude_sessions_cmd`, `search_claude_history_cmd`, `export_claude_session_cmd`, `delete_claude_session_cmd` | 独立为 routes/settings.rs + services/settings_service.rs，原 commands.rs 保留 re-export |
| 2 | project / memory | `add_project`, `remove_project`, `list_projects`, `get_project_data`, `get_project_files`, `save_candidate_memory` | 独立为 routes/project.rs + routes/memory.rs，原 commands.rs 保留 re-export |
| 3 | agent / watcher | `start_watching`, `stop_watching`, `get_latest_session` | 独立为 routes/agent.rs，移除旧 re-export |
| 4 | 收尾 | 清理 commands.rs 中的 re-export，验证 lib.rs 命令注册 | 确认 `cargo check` / `cargo test` 通过 |

#### 3.1.4 关键文件变动

| 原文件 | 新位置 | 说明 |
|--------|--------|------|
| `commands.rs:35-200` | `models/project.rs` | `SerStage`, `SerProjectConfig`, `SerGitStatus` 等序列化结构体 |
| `commands.rs:200-500` | `models/history.rs` | Claude 历史相关结构体 |
| `commands.rs:1-30` | `app_state.rs` | `AppState` 结构体定义 |
| `commands.rs:settings 相关` | `services/settings_service.rs` | 设置业务逻辑 |
| `commands.rs:history 相关` | `services/history_service.rs` | Claude 历史业务逻辑 |
| `commands.rs:project 相关` | `services/project_service.rs` | 项目数据获取逻辑 |
| `lib.rs:71-94` | `routes/mod.rs` | 命令注册列表 |

---

### 3.2 前端功能归类重构

#### 3.2.1 目标架构

```
src/
├── features/               # 按业务领域垂直切片
│   ├── dashboard/          # 仪表盘
│   │   ├── components/     # 仪表盘专属组件
│   │   ├── hooks/          # 仪表盘专属 hooks
│   │   ├── types.ts        # 领域类型定义
│   │   └── index.tsx       # 页面入口（原 pages/Dashboard.tsx）
│   ├── project-detail/     # 项目详情
│   │   ├── components/
│   │   ├── hooks/
│   │   ├── types.ts
│   │   └── index.tsx
│   ├── agent-monitor/      # Agent 实时监控
│   │   ├── components/
│   │   ├── hooks/
│   │   ├── types.ts
│   │   └── index.tsx
│   ├── claude-history/     # Claude 会话历史
│   │   ├── components/     # 原 components/claude-history/*
│   │   ├── hooks/          # 原 hooks/useClaudeHistory.ts
│   │   ├── types.ts
│   │   └── index.tsx       # 原 pages/ClaudeHistory.tsx
│   └── settings/           # 设置
│       ├── components/
│       ├── hooks/
│       ├── types.ts
│       └── index.tsx
├── components/ui/          # shadcn/ui 基础组件（保持不变）
├── lib/
│   ├── utils.ts
│   ├── api.ts              # Tauri invoke 统一封装
│   └── types.ts            # 跨 feature 共享类型
├── App.tsx                 # 路由入口（更新导入路径）
└── main.tsx                # 应用入口
```

#### 3.2.2 重构原则

- **按功能垂直切片**：每个 feature 目录包含该功能所需的组件、hooks、类型，外部通过 `index.ts` 暴露接口
- **禁止跨 feature 直接引用**：如需共享，提取到 `src/lib/` 或建立明确的共享模块
- **类型内聚**：每个 feature 的 `types.ts` 定义该领域的数据结构，避免在组件中重复定义
- **API 封装**：`lib/api.ts` 统一封装所有 Tauri invoke 调用，避免各组件直接依赖 `useTauri`
- **Phase 1 不做视觉改造**：当前 shadcn/ui 主题保持不动，`theme/` 目录在后续阶段建立

#### 3.2.3 关键文件变动

| 原文件 | 新位置 | 说明 |
|--------|--------|------|
| `pages/Dashboard.tsx` | `features/dashboard/index.tsx` | 仪表盘页面 |
| `pages/ProjectDetail.tsx` | `features/project-detail/index.tsx` | 项目详情页面 |
| `pages/AgentMonitor.tsx` | `features/agent-monitor/index.tsx` | Agent 监控页面 |
| `pages/ClaudeHistory.tsx` | `features/claude-history/index.tsx` | Claude 历史页面 |
| `pages/Settings.tsx` | `features/settings/index.tsx` | 设置页面 |
| `components/claude-history/*` | `features/claude-history/components/` | Claude 历史组件 |
| `hooks/useClaudeHistory.ts` | `features/claude-history/hooks/` | Claude 历史 hook |
| `hooks/useTauri.ts` | `lib/api.ts` | Tauri invoke 封装 |

---

### 3.3 CI/CD 基础流水线

#### 3.3.1 方案选择：GitHub Actions

理由：
- Tauri 官方提供成熟的 [tauri-action](https://github.com/tauri-apps/tauri-action)
- 生态完善，后续扩展 Release 流水线成本低

**Phase 1 只做 CI（验证），不做 Release（发布）**。多平台构建矩阵、签名、Notarization、GitHub Release 集成放到下一步。

#### 3.3.2 CI 配置

```yaml
# .github/workflows/ci.yml
name: CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  test:
    runs-on: ubuntu-22.04
    steps:
      - uses: actions/checkout@v4

      - name: Setup Node
        uses: actions/setup-node@v4
        with:
          node-version: 20

      - name: Setup Rust
        uses: dtolnay/rust-action@stable

      - name: Install system dependencies
        run: |
          sudo apt-get update
          sudo apt-get install -y libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev patchelf

      - name: Install frontend dependencies
        run: npm ci

      - name: Install Playwright
        run: npx playwright install --with-deps chromium

      - name: Build frontend
        run: npm run build

      - name: Check Rust
        run: cd src-tauri && cargo check

      - name: Run Rust tests
        run: cd src-tauri && cargo test

      - name: Run E2E tests
        run: npm test
```

#### 3.3.3 后续阶段待补充（Release 流水线）

以下内容**不纳入 Phase 1**，作为已知待办：

- **多平台构建矩阵**：macOS (x86_64 + aarch64)、Linux、Windows
- **Universal Binary**：macOS 需 `lipo` 合并 x86_64 和 aarch64 产物
- **目标平台安装**：`rustup target add aarch64-apple-darwin x86_64-apple-darwin`
- **代码签名降级方案**：
  - macOS：无证书时构建 `*.app` 但不 Notarize，用户需在系统设置中手动允许
  - Windows：无证书时构建未签名 `*.exe` 和 `*.msi`
- **版本管理自动化**：`scripts/bump-version.sh` 统一修改 `package.json` + `Cargo.toml`

#### 3.3.4 GitLab 集成策略

如果主仓库在 GitLab：

- **代码主仓**：GitLab
- **镜像同步**：配置 GitLab Mirror 同步到 GitHub
- **构建执行**：GitHub Actions 负责构建和发布
- **产物回传**：构建产物通过 GitHub Release API 上传，GitLab 侧可通过 webhook 获取

---

## 四、执行顺序

### 主线一：后端模块化

| 阶段 | 内容 | 关键产出 | 验证方式 |
|------|------|----------|----------|
| B1 | 提取 `models/` + `app_state.rs` | 序列化结构体独立 | `cargo check` 通过 |
| B2 | 迁移 settings + history 命令域到 routes/services | 两个域独立文件 | `cargo check` + `cargo test` 通过 |
| B3 | 迁移 project + memory 命令域 | 两个域独立文件 | `cargo check` + `cargo test` 通过 |
| B4 | 迁移 agent + watcher 命令域，清理 `commands.rs` | God File 消失 | `cargo check` + `cargo test` 通过 |

### 主线二：前端功能归类

| 阶段 | 内容 | 关键产出 | 验证方式 |
|------|------|----------|----------|
| F1 | 创建 `features/` 目录结构，迁移 Dashboard | 页面可正常渲染 | `npm run build` 通过 |
| F2 | 迁移 ProjectDetail + AgentMonitor | 页面可正常渲染 | `npm run build` 通过 |
| F3 | 迁移 ClaudeHistory + Settings，提取 `lib/api.ts` + `lib/types.ts` | 所有页面归位 | `npm run build` + `npm run tauri dev` 正常 |
| F4 | 删除 `pages/`、`components/claude-history/`、`hooks/useClaudeHistory.ts` | 旧文件清理 | 全量功能手动验证 |

### 主线三：工程化

| 阶段 | 内容 | 关键产出 | 验证方式 |
|------|------|----------|----------|
| E1 | 创建 `.github/workflows/ci.yml` | CI 配置就绪 | GitHub Actions 执行成功 |
| E2 | 修复 CI 中的环境依赖问题 | 稳定 green build | 多次 push 验证 |

**预估工时**：
- 后端主线：6-8 小时
- 前端主线：4-6 小时
- 工程化主线：2-3 小时
- **总计：约 12-17 小时**

---

## 五、明确不做的事（Phase 1 范围外）

以下事项已识别但**不纳入第一阶段**，避免范围膨胀：

| 事项 | 原因 | 计划时机 |
|------|------|----------|
| 通用 `Collector` trait | 三种采集器形态差异大（一次性/轮询/文件扫描），统一接口需要关联类型或泛型，时机不成熟 | Phase 2 |
| Linear 视觉主题改造 | 涉及所有组件样式替换，回归面过大，且需要自定义标题栏组件 | Phase 1.5 或 Phase 2 |
| 自定义窗口标题栏 | 不是纯样式改动，影响拖拽、窗口控制、平台一致性 | Phase 2 |
| Release 流水线 | 需要签名证书、Notarization、多平台 Runner 配置，依赖 CI 先稳定 | Phase 2 |
| E2E 测试修复 | 测试失步是独立问题，与代码结构重构无关 | Phase 1 之后单独排期 |

---

## 六、验收标准（硬门槛）

以下标准为**不可降低的硬门槛**，每批次迁移完成后必须全部满足才能进入下一批次。

### 6.1 后端每批次硬门槛

每完成一个后端批次（B1–B4），在提交前必须执行：

```bash
cd src-tauri && cargo check
```
**预期结果**：零 error，warnings 数量不增加（与重构前基线对比）。

```bash
cd src-tauri && cargo test
```
**预期结果**：`120 passed, 0 failed`。任何失败必须在本批次内修复，不得带到下一批次。

### 6.2 前端每批次硬门槛

每完成一个前端批次（F1–F4），在提交前必须执行：

```bash
npm run build
```
**预期结果**：零 TypeScript 编译错误，零 Vite 构建错误。

```bash
npm run tauri dev
```
**预期结果**：应用正常启动，当前批次涉及的所有页面可正常访问，无白屏或崩溃。

### 6.3 CI 变更硬门槛

完成工程化批次（E1–E2）后，必须触发 GitHub Actions 实际执行：

```bash
git push origin <branch>
```
**预期结果**：`.github/workflows/ci.yml` 的 workflow run 状态为 **✅ Success**（green），且所有步骤（Build frontend / Check Rust / Run Rust tests / Run E2E tests）均通过。任何失败必须在本批次内修复。

### 6.4 批次间依赖

| 批次 | 前置条件 |
|------|----------|
| B2 | B1 硬门槛全部通过 |
| B3 | B2 硬门槛全部通过 |
| B4 | B3 硬门槛全部通过 |
| F2 | F1 硬门槛全部通过 |
| F3 | F2 硬门槛全部通过 |
| F4 | F3 硬门槛全部通过 |
| E2 | E1 硬门槛全部通过 |

三条主线之间可并行推进，但同一条主线内的批次必须串行执行。

---

## 七、相关文档

| 文档 | 路径 | 说明 |
|:-----|:-----|:-----|
| 开发指南 | `CLAUDE.md` | Tauri 命令、架构、数据流、测试环境 |
| 软件分发 | `docs/distribution.md` | Linux/macOS/Windows 分发形式 |
| 设计系统 | `DESIGN.md` | Linear 风格设计令牌（颜色、字体、间距），Phase 2 使用 |
| 项目路线图 | `docs/roadmap.md` | 项目状态与扩展路线 |
| 本文件 | `docs/refactor-phase-1-plan.md` | 第一阶段重构方案（本文档） |
