# ptv (Project Template Visualizer) — 需求规格文档

> **状态**: 待确认  
> **版本**: v0.1  
> **日期**: 2026-05-06

---

## 1. 项目背景

### 1.1 问题陈述

用户基于 `ai_project_template` 模板创建了多个 FPGA 项目（如 sync、cordic 等），每个项目独立开发。当并行项目增多时，出现以下痛点：

- **无法跨项目一览状态**：需要逐个 `cd` 进项目目录查看 `.current_stage`、git status
- **Claude Code 上下文分散**：每个项目的 Memory、对话历史、规则文件独立，难以跨项目回顾
- **AI Agent 活动不可见**：多个项目可能同时有 claude/codex 在运行，无法统一监控 token 消耗和会话状态
- **现有工具不满足需求**：
  - `memory_dashboard`（模板内 Python TUI）仅支持单项目
  - `abtop`（Rust TUI）侧重 AI Agent 实时监控，缺少项目层面的 Stage/Memory/Config 信息

### 1.2 目标

构建 **ptv** — 一个独立的桌面应用，统一监控所有基于 `ai_project_template` 的 FPGA 项目，提供"项目静态上下文 + AI Agent 动态活动"双维度视图。

---

## 2. 用户角色

| 角色 | 描述 |
|:-----|:-----|
| **FPGA 开发者** | 唯一用户。同时管理 3-10 个模板项目，需要快速切换上下文和监控进度 |

---

## 3. 功能需求

### 3.1 项目管理

| ID | 需求 | 优先级 |
|:---|:-----|:------:|
| PM-01 | 用户通过 GUI 手动添加项目（输入或浏览项目根目录路径） | P0 |
| PM-02 | 用户可移除已注册项目（不影响被监控项目的任何文件） | P0 |
| PM-03 | 项目列表持久化存储，应用重启后保留 | P0 |
| PM-04 | 重复添加相同路径时拒绝并提示 | P1 |
| PM-05 | 项目路径不存在时标记为"不可用"，不影响其他项目 | P1 |

### 3.2 项目仪表盘（主视图）

| ID | 需求 | 优先级 |
|:---|:-----|:------:|
| DB-01 | 列表展示所有注册项目，每行包含：项目名称、当前 Stage、最近活动时间、活跃 Agent 数、Git 分支+变更数 | P0 |
| DB-02 | Stage 使用彩色 Badge 区分（如 L1=蓝色、L5=橙色、Verilog=紫色） | P0 |
| DB-03 | 点击项目行选中，展开/跳转到项目详情 | P0 |
| DB-04 | 无注册项目时显示引导提示"添加项目" | P0 |
| DB-05 | 支持按项目名称搜索/过滤 | P2 |
| DB-06 | 支持按 Stage 排序 | P2 |

### 3.3 项目详情

| ID | 需求 | 优先级 |
|:---|:-----|:------:|
| **Stage 时间线** |
| PD-01 | 显示 L0→L1→L2→L3→L4→L5→L6→Verilog→Synthesis→Hardware 的进度步骤条 | P0 |
| PD-02 | 当前 Stage 高亮，已完成 Stage 标记为通过 | P0 |
| PD-03 | `.current_stage` 值不在已知范围内时显示"未知阶段: xxx"而非崩溃 | P1 |
| **参数快照** |
| PD-04 | 显示 `config/parameters.py` 的关键参数：project_name, module_name, interface_type, data_width, iterations, q_int_bits, q_frac_bits, pipeline_stages, clock_frequency | P0 |
| PD-05 | 参数以 key=value 卡片形式展示 | P0 |
| PD-06 | parameters.py 解析失败时显示"配置读取失败"（非崩溃） | P1 |
| **Memory** |
| PD-07 | 读取 `.claude/memory/*.md`（YAML frontmatter 格式），按类型分组展示（👤用户/💡反馈/📋项目/🔗参考） | P0 |
| PD-08 | 每个条目显示 name + description + 更新时间 | P0 |
| PD-09 | Memory 目录不存在时显示"暂无 Memory 条目"（非报错） | P0 |
| **Git** |
| PD-10 | 显示当前分支名 + 未提交文件统计（新增 N / 修改 M） | P0 |
| PD-11 | 列出具体变更的文件名 | P1 |
| PD-12 | 非 git 仓库时显示"非 Git 仓库" | P1 |

### 3.4 Agent 运行时监控

| ID | 需求 | 优先级 |
|:---|:-----|:------:|
| AG-01 | 实时显示所有活跃的 Claude Code / Codex CLI 会话 | P0 |
| AG-02 | 每个会话显示：Agent 类型图标、PID、项目名、状态（Thinking/Executing/Waiting/RateLimited/Done）、Model 名、Token 总量、上下文窗口占比 | P0 |
| AG-03 | 上下文窗口占比用进度条展示，80% 黄色警告，90% 红色+⚠图标 | P0 |
| AG-04 | Token 速率图：近 2 分钟趋势的 Recharts 面积图 | P1 |
| AG-05 | 显示当前任务（tool name + first argument） | P1 |
| AG-06 | 无活跃 Agent 时显示"无活跃 Agent 会话" | P0 |
| AG-07 | 会话状态实时更新（2 秒内反映变化） | P0 |

### 3.5 实时更新

| ID | 需求 | 优先级 |
|:---|:-----|:------:|
| RU-01 | `.current_stage` 文件变化后 3 秒内 Dashboard 自动更新 | P0 |
| RU-02 | `config/parameters.py` 变化后自动刷新参数快照 | P0 |
| RU-03 | Memory 目录文件变化后自动刷新 Memory 视图 | P0 |
| RU-04 | Git 状态变化后自动刷新 | P1 |
| RU-05 | 应用切换到后台时暂停数据采集，回到前台时恢复 | P2 |

---

## 4. 非功能需求

| ID | 需求 | 指标 |
|:---|:-----|:-----|
| NF-01 | **只读约束** | 绝不写入被监控项目的任何文件 |
| NF-02 | **性能** | 监控 ≤20 个项目时 CPU <5%（空闲）、内存 <200MB |
| NF-03 | **启动时间** | 应用冷启动 <5 秒显示 Dashboard |
| NF-04 | **平台** | **macOS + Linux**（Windows 不做）。Linux 优先验证 |
| NF-05 | **语言** | 界面中文 |
| NF-06 | **安全性** | 不发送任何数据到网络、不读取 API Key、密钥信息自动脱敏 |

---

## 5. 明确排除（v0.1 不做）

| 功能 | 原因 |
|:-----|:-----|
| 自动扫描文件系统发现项目 | 用户明确要求手动添加 |
| 启动/停止/控制 Claude Code 进程 | 监控工具，非管理工具 |
| 跨项目参数对比 | v0.2 功能 |
| 历史趋势图/数据统计 | 需要时序存储，v0.2 |
| 告警/通知 | v0.2 |
| 项目创建/克隆 | 项目管理工具范畴 |
| 远程监控 | 架构完全不同 |
| 测试报告聚合 | 模板的 scripts 已支持 |
| 暗色模式切换 | 先用明亮主题，v0.2 加 dark mode |
| Web 部署 | 定位为桌面应用 |

---

## 6. 技术方案

| 维度 | 决策 | 理由 |
|:-----|:-----|:-----|
| 前端框架 | React + TypeScript | 生态成熟，组件库丰富，Tauri 支持好 |
| UI 组件库 | shadcn/ui + Tailwind CSS | 轻量，可定制，现代风格 |
| 图表库 | Recharts | React 原生，声明式 API |
| 后端 | Rust (Tauri v2) | 与 abtop 技术栈一致，性能好 |
| Agent 数据采集 | 从 abtop 提取独立 `abtop-collector` 库 crate | 复用 ~5600 行成熟采集代码 |
| config 解析 | Rust 端 `python3` 子进程导出 JSON | 被监控项目必然有 Python 环境 |
| 文件监听 | tauri-plugin-fs (watch feature) | Tauri v2 内置功能 |
| Agent 轮询 | tokio::spawn 每 2 秒轮询 | 与 abtop 一致 |
| 项目注册存储 | `app_data_dir()/projects.json` | Tauri 标准路径 |
| 数据刷新 | 文件数据 → watchfiles（事件驱动），Agent 数据 → 2 秒轮询 | 混合策略 |

---

## 7. 数据源清单

| 数据 | 来源 | 读取方式 |
|:-----|:-----|:--------|
| 当前 Stage | `.current_stage` 文件 | 文件读取 |
| 参数配置 | `config/parameters.py` | `python3` 子进程导出 JSON |
| Memory 条目 | `.claude/memory/*.md` | YAML frontmatter 解析 |
| 规则文件 | `.claude/rules/*.md` | v0.2 |
| Git 状态 | `git branch` + `git status --porcelain` | 子进程 |
| Claude 会话 | `~/.claude/sessions/{PID}.json` + `projects/{path}/{sid}.jsonl` | abtop-collector |
| Codex 会话 | `~/.codex/sessions/{date}/rollout-*.jsonl` | abtop-collector |
| Agent token | JSONL transcript 增量解析 | abtop-collector |
| 速率限制 | `~/.claude/abtop-rate-limits.json` | abtop-collector |
| 进程信息 | `ps` + `lsof` | abtop-collector |

---

## 8. 错误处理策略

| 场景 | 行为 |
|:-----|:-----|
| 项目路径不存在 | Dashboard 标记为"路径不可用"，不阻塞其他项目 |
| `.current_stage` 不存在 | 显示"未检测到阶段文件" |
| `.current_stage` 值未知 | 显示"未知阶段: {value}" |
| config/parameters.py 不存在/损坏 | 显示"配置读取失败" |
| Memory 目录不存在 | 显示"暂无 Memory 条目" |
| Git 仓库损坏 | 显示"Git 状态不可用" |
| Python 不可用 | 配置解析降级，标记为"需要 Python 3.9+" |
| Agent 数据不可用 | 显示"无活跃 Agent 会话" |
| 权限不足（读取文件） | 显示具体错误信息 |

---

## 9. 验收标准

### 验证环境

| 环境 | 信息 |
|:-----|:-----|
| **Linux 测试机**（优先） | `100.85.255.89`，用户 `yufei`，SSH 已配置 |
| 代码路径 | `/home/yufei/Repo/ai_project_template_visualization` |
| macOS 开发机 | 本机，用于开发调试 |
| Windows | **不测试、不实现** |

### 验收项

1. ✅ 应用启动后显示 Dashboard，列出所有已注册项目
2. ✅ 每个项目可见：项目名、当前 Stage（彩色 Badge）、最近活动、Git 分支
3. ✅ 选中项目后，详情页 4 个标签页内容正确
4. ✅ Agent Monitor 在有活跃会话时正确显示
5. ✅ `.current_stage` 变化后 5 秒内 UI 自动刷新
6. ✅ 添加不存在的路径时显示错误提示
7. ✅ 项目路径被删除后优雅降级而非崩溃
8. ✅ 全程不写入被监控项目任何文件

---

## 10. 已确认的细化决策

| # | 决策点 | 确认方案 |
|:--|:-----|:--------|
| 1 | Dashboard 排序 | **按项目名称字母序** |
| 2 | 多 Agent 展示 | **Dashboard 合并计数**（如 "2 agents"），Agent Monitor 逐条列出 |
| 3 | reference_project | **显示**，作为参数快照字段 |
| 4 | Stage 时间线耗时 | **显示占位**（预留位置，v0.2 填充数据） |
| 5 | 浏览文件对话框 | **Tauri dialog plugin**，macOS 原生 NSOpenPanel |
| 6 | Memory 条目上限 | **ScrollArea 滚动**，不设上限，不分页
