# AgentScope 项目状态与扩展路线

> 本文档汇总项目当前状态、技术债务、重构计划和功能扩展方向，作为团队迭代的核心参考。
> 最后更新：2026-05-13

---

## 一、项目定位

AgentScope（原名 PTV）是一款跨平台 Tauri v2 桌面应用，用于实时监控和管理 AI Agent 会话及项目状态。

| 里程碑 | 状态 |
|:---|:---|
| FPGA 模板项目监控（Stage、Git、参数） | ✅ 已完成 |
| Claude Code 会话管理（实时追踪、Token 速率、上下文窗口） | ✅ 已完成 |
| 品牌升级（PTV → AgentScope） | ✅ 已完成 |
| 重构为通用 AI Agent 生态监控平台 | 🔄 当前阶段 |

---

## 二、当前状态速览

### 2.1 功能现状

| 模块 | 状态 | 说明 |
|------|------|------|
| 项目仪表盘 | ✅ 稳定 | Stage、Git 状态、Agent 活跃度卡片展示 |
| 项目详情 | ✅ 稳定 | 时间线、参数快照、Memory 浏览、Git 状态 |
| Agent 监控 | ✅ 稳定 | 实时会话追踪、Token 速率、工具调用 |
| Claude 历史 | ✅ 稳定 | 会话搜索、导出（Markdown/JSON）、预览 |
| 记忆标记 | ✅ 稳定 | 对话标记 → 沉淀为 `.sisyphus/notepads/project-memory/decisions.md` |
| 设置 | ✅ 稳定 | 项目注册、模板路径配置 |

### 2.2 代码规模

| 部分 | 规模 | 技术栈 |
|------|------|--------|
| 前端 | ~14,600 行 | React 19 + TypeScript + Tailwind CSS v4 + shadcn/ui |
| 后端 | ~3,000+ 行 | Rust (Tauri v2) |
| abtop-collector | ~1,500+ 行 | 本地 Rust crate，支持 Claude / Codex / MCP 多源采集 |

### 2.3 测试状态

| 测试类型 | 结果 | 问题 |
|----------|------|------|
| Rust 单元测试 | 116 通过 / 7 失败 | `claude_history/scanner.rs` 6 个失败（JSONL 导出格式断言不同步）；`collectors/agent/mod.rs` 1 个失败（浮点数精度） |
| E2E 测试 | 大量失败 | UI 已改为中文，但测试仍期望英文按钮名（如 "Dashboard" 实际为"仪表盘"），测试与 UI 严重失步 |

### 2.4 遗留问题

| 问题 | 位置 | 严重程度 |
|------|------|----------|
| ~~文档仍用旧名"PTV"~~ | ~~`docs/distribution.md` 全文~~ | ~~中~~ | ~~已验证：文档标题和内容均已改为 AgentScope，无残留~~ |
| Sisyphus 配置指向旧路径 | `.sisyphus/boulder.json` | 低 |
| E2E 测试与 UI 不同步 | `e2e/navigation.spec.ts` 等 | **高** |
| Rust 单元测试失败 | `scanner.rs`, `agent/mod.rs` | **高** |

---

## 三、迭代路线

### Phase 1：立即处理（1 周内）

目标：恢复测试可靠性，清理遗留债务，修复 CI 配置缺口。

- [x] 修复 Rust 单元测试（7 个）
  - `scanner.rs`：同步 JSONL 清洗/导出逻辑与测试断言
  - `agent/mod.rs`：浮点数比较改用近似相等
- [x] 同步 E2E 测试 — 全面审计 `e2e/` 目录，更新选择器匹配当前中文 UI（43 passed / 0 failed，无剩余不同步项）
- [x] ~~清理遗留名称 — 更新 `docs/distribution.md` 中"PTV"为"AgentScope"~~（已验证：文档无 PTV 残留，roadmap 记录已更新）
- [x] 修复 `.sisyphus/boulder.json` 中的旧路径引用（`ai_project_template_visualization` → `agent-scope`）
- [x] **优化 CI artifact 生命周期，补充 artifact 查询规范**
  - `.gitlab-ci.yml`：`build:linux` / `build:windows` 的 `expire_in` 从 `1 month` → `1 day`
  - `docs/desktop-ci-cd-lessons.md`：新增 5.11 踩坑条目（artifact 堆积 + 外部系统误用）
  - `docs/ci-cd-setup.md`：新增 9.7.1 Artifact 生命周期管理章节
- [x] **修复 Pipeline #248 E2E 副标题文本不同步**
  - 根因：`src/features/agent-monitor/index.tsx` 副标题从 `"实时 Token 速率"` 改为 `"Token 消耗速率（burn rate）"`，且句末加了句号，但 `e2e/agent-monitor.spec.ts:18` 未同步更新
  - 修复：更新测试断言匹配当前 UI 文本
  - 验证：`43 passed / 0 failed`

---

### Phase 2：重构（2–3 周）→ ✅ 已完成

目标：为后续多 Agent 支持、Token 统计等扩展建立清晰的架构基础。

#### 2.1 后端模块化重构 ✅

- `commands.rs`（907 行 God File）已拆分为：
  - `app_state.rs` — AppState 集中定义
  - `models/` — `project.rs`, `history.rs`, `mod.rs`（序列化结构体）
  - `routes/` — `project.rs`, `agent.rs`, `history.rs`, `memory.rs`, `settings.rs`, `mod.rs`（Tauri 命令接口层）
  - `services/` — `project_service.rs`, `agent_service.rs`, `history_service.rs`, `memory_service.rs`, `settings_service.rs`, `mod.rs`（业务逻辑层）
- 验证：`cargo check` ✅，`cargo test` ✅ `123 passed / 0 failed`

#### 2.2 前端功能归类重构 ✅

- 按业务领域垂直切片为 `features/`：
  - `features/dashboard/` — 仪表盘
  - `features/project-detail/` — 项目详情
  - `features/agent-monitor/` — Agent 监控
  - `features/claude-history/` — Claude 会话历史（含 `components/`, `hooks/`）
  - `features/settings/` — 设置
- 旧目录已清理：`pages/`、`components/claude-history/`、`hooks/useClaudeHistory.ts` 已删除
- `lib/api.ts` — Tauri invoke 统一封装已创建
- 验证：`npm run build` ✅

#### 2.3 未实施项（Phase 2.1 设计 Token 系统）

- `src/theme/` 设计 token 目录 **尚未创建**
- 当前仍使用 shadcn/ui 默认主题 + `index.css`
- 原因：视觉主题改造涉及全组件样式替换，回归面大，与功能扩展优先级权衡后暂缓
- 计划时机：Phase 3 功能稳定后，或独立排期

目标结构：

```
src/
├── features/
│   ├── dashboard/          # 仪表盘（项目卡片、总览）
│   │   ├── components/
│   │   ├── hooks/
│   │   ├── types.ts
│   │   └── index.ts
│   ├── project-detail/     # 项目详情（Stage、Git、Memory、参数）
│   │   ├── components/
│   │   ├── hooks/
│   │   ├── types.ts
│   │   └── index.ts
│   ├── agent-monitor/      # Agent 实时监控
│   │   ├── components/
│   │   ├── hooks/
│   │   ├── types.ts
│   │   └── index.ts
│   ├── claude-history/     # Claude 会话历史
│   │   ├── components/
│   │   ├── hooks/
│   │   ├── types.ts
│   │   └── index.ts
│   └── settings/           # 设置（项目注册、模板路径）
│       ├── components/
│       ├── hooks/
│       ├── types.ts
│       └── index.ts
├── components/ui/          # shadcn/ui 基础组件（保持不变）
├── theme/                  # 设计令牌、主题配置
├── lib/                    # 通用工具（utils、api 封装）
├── App.tsx                 # 路由入口
└── main.tsx                # 应用入口
```

重构原则：
- **按功能垂直切片**：每个 feature 目录包含该功能所需的组件、hooks、类型，外部通过 `index.ts` 暴露接口
- **禁止跨 feature 直接引用**：如需共享，提取到 `src/lib/` 或建立明确的共享模块
- **类型内聚**：每个 feature 的 `types.ts` 定义该领域的数据结构，避免在组件中重复定义

#### 2.3 后端模块化解耦重构

当前问题：
- `commands.rs` 是 God File（600+ 行），所有 Tauri 命令集中在一处
- `collectors/` 虽有目录划分，但 `template/` 和 `agent/` 的数据流未完全隔离
- `claude_history` 采集逻辑与 `agent` 采集逻辑有潜在重叠，但未抽象出通用接口
- 新增采集器（如 Codex）时需要修改多处文件

目标结构：

```
src-tauri/src/
├── main.rs                 # 入口
├── lib.rs                  # Tauri Builder、插件注册、状态初始化
├── app_state.rs            # AppState 定义（替代 commands 中的内联定义）
├── routes/                 # 按领域拆分的 Tauri 命令
│   ├── mod.rs              # 路由注册汇总
│   ├── project.rs          # 项目注册、列表、数据获取
│   ├── agent.rs            # Agent 监控（启动/停止、事件推送）
│   ├── claude_history.rs   # 会话历史（搜索、导出、预览）
│   ├── memory.rs           # 记忆标记（读写 decisions.md）
│   └── settings.rs         # 设置（模板路径等）
├── services/               # 业务逻辑层（命令的实现）
│   ├── project_service.rs
│   ├── agent_service.rs
│   ├── history_service.rs
│   └── memory_service.rs
├── collectors/             # 数据采集器（保持现有目录结构，提炼通用接口）
│   ├── mod.rs              # Collector trait 定义
│   ├── template/           # 模板项目采集（Stage、Git、Config、Files）
│   ├── agent/              # Agent 实时监控（abtop-collector 封装）
│   └── claude_history/     # 历史会话扫描
├── models/                 # 共享数据结构（替代 commands.rs 中的 Ser* 结构）
│   ├── mod.rs
│   ├── project.rs
│   ├── agent.rs
│   └── history.rs
├── registry.rs             # ProjectRegistry（已有，保持独立）
└── watcher.rs              # FileWatcher（已有，保持独立）
```

重构原则：
- **分层架构**：`routes/`（HTTP/Tauri 接口）→ `services/`（业务逻辑）→ `collectors/`（数据采集）→ `models/`（数据结构）
- **Collector Trait**：定义通用采集接口 `trait Collector { fn collect(&self) -> Result<Data, Error>; }`，所有采集器实现该接口
- **命令即路由**：`routes/*.rs` 只做参数解析和调用 service，不包含业务逻辑
- **状态集中**：`AppState` 独立为 `app_state.rs`，各 service 通过依赖注入获取所需状态

---

### Phase 3：短期迭代（1–2 月）

目标：按 roadmap 推进核心功能扩展。

| 优先级 | 方向 | 依赖 | 价值 |
|--------|------|------|------|
| ~~P1~~ | ~~**Token 用量统计**~~ | ~~依赖 abtop-collector 已有数据，前端统计卡片 + Agent 详情 Token 展示~~ | ~~高~~ | ✅ 已完成 |
| P2 | **多 Agent 支持** | abtop-collector 已有 Codex/MCP 采集器；依赖 Phase 2 的 Collector Trait 和前端 feature 结构 | 高 |
| P3 | **Dashboard 增强** | 活跃度评分、异常告警、跨项目对比；依赖 Token 统计和 Agent 数据 | 中 |
| P4 | **会话历史深度分析** | 热点文件识别、工具调用频率；依赖 Phase 2 的 history_service 模块化 | 中 |

---

## 四、潜在探索方向（长期）

| 方向 | 场景描述 |
|:---|:---|
| Agent 协作监控 | 同一项目中多 Agent（Claude + Codex）资源协调与冲突可视化 |
| 团队 Agent 面板 | 多开发者场景下汇总团队 Agent 使用效率 |
| 远程 Agent 监控 | 通过 SSH/API 监控云环境 / CI 流水线中的 Agent |
| Agent 行为审计 | 完整记录文件操作序列，生成可回溯的变更审计日志 |
| 智能提醒/干预 | Agent 陷入循环、上下文溢出、长时间无进展时主动推送 |
| 模型效率对比 | 同一任务用不同模型（Claude vs GPT-4o）对比 Token 效率、耗时、质量 |
| Agent 会话回放 | 时间轴形式回放会话过程，支持快进/回退 |
| 自定义监控指标 | 用户自定义指标（特定文件变更次数、测试通过率变化）接入面板 |
| 项目知识图谱 | 从会话中提取关键概念和文件关系，自动生成知识结构图 |

---

## 五、相关文档

| 文档 | 路径 | 说明 |
|:-----|:-----|:-----|
| 开发指南 | `CLAUDE.md` | Tauri 命令、架构、数据流、测试环境 |
| 软件分发 | `docs/distribution.md` | Linux/macOS/Windows 分发形式（⚠️ 仍含旧名"PTV"） |
| 本文件 | `docs/roadmap.md` | 项目状态与扩展路线（本文档） |
