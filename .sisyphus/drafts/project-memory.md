# ptv (Project Template Visualizer)

基于 `ai_project_template` 创建的 FPGA 项目的跨项目监控桌面应用。

## 项目概述

- **名称**: ptv
- **定位**: Tauri v2 桌面应用，监控从 `ai_project_template` 创建的实际 FPGA 项目
- **前端**: React + TypeScript + Tailwind CSS + shadcn/ui + Recharts
- **后端**: Rust (Tauri v2)，复用 `abtop-collector` 库 crate
- **平台**: **macOS + Linux**（Windows 不做），**Linux 优先验证**
- **语言**: 中文

## 核心功能

1. 项目仪表盘 — 跨项目列表，显示 Stage、Git 状态、活跃 Agent 数
2. 项目详情 — Stage 时间线 / 参数快照 / Memory / Git
3. Agent 监控 — 实时 Token 速率、上下文窗口、会话状态

## 测试机器

| 项 | 值 |
|:---|:---|
| Linux 测试机（优先验证） | `100.85.255.89` |
| SSH 用户 | `yufei` |
| SSH 密码 | `yufei` |
| SSH 方式 | `sshpass` 已配置 |
| 代码路径 | `/home/yufei/Repo/ai_project_template_visualization` |
| macOS 开发机 | 本机 |

## 相关项目

| 项目 | 路径 | 关系 |
|:-----|:-----|:-----|
| `ai_project_template` | `/Users/ckstar/Repo/ai_project_template` | 模板项目，被监控的目标 |
| `abtop` | `/Users/ckstar/Repo/abtop` | Rust TUI 监控器，collector 代码复用源 |
| `ptv`（本项目） | 当前目录 | 跨项目监控桌面应用 |

## 关键文档

- 需求规格: `.sisyphus/drafts/requirements.md`
- 执行计划: `.sisyphus/plans/ptv-v0.1.md`

## 关键决策

| 决策 | 方案 |
|:-----|:-----|
| abtop 代码复用 | 提取独立 `abtop-collector` 库 crate（非 vendor） |
| config 解析 | Rust 端 `python3` 子进程导出 JSON |
| 项目发现 | GUI 手动添加 |
| 测试策略 | Agent QA（Playwright E2E），v0.1 无单元测试 |
| 数据刷新 | 文件 → watchfiles，Agent → 2s 轮询 |
| Dashboard 排序 | 按项目名称字母序 |
| 平台 | macOS + Linux only（无 Windows） |
