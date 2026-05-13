# AgentScope

AI Agent 会话及项目状态监控桌面应用。

## 项目概述

- **名称**: AgentScope
- **定位**: Tauri v2 桌面应用，监控 AI Agent 会话及项目状态
- **前端**: React + TypeScript + Tailwind CSS + shadcn/ui + Recharts
- **后端**: Rust (Tauri v2)，复用 `abtop-collector` 库 crate
- **平台**: **macOS + Linux + Windows（测试中）**，**Linux 优先验证**
- **语言**: 中文

## 技术栈

| 层级 | 技术 | 用途 |
|------|------|------|
| 桌面框架 | Tauri v2 | Rust 后端 + WebView 前端 |
| 前端框架 | React 19 + TypeScript | UI 层 |
| 样式 | Tailwind CSS v4 + shadcn/ui (Nova) | UI 组件库 |
| 图表 | Recharts | 数据可视化 |
| Rust 库 | abtop-collector | 监控数据采集（本地路径引用） |
| 包管理 | npm | 前端依赖 |

## 目录结构

```
ai_project_template_visualization/
├── src/                    # React 前端源码
│   ├── App.tsx             # 根组件
│   ├── main.tsx            # 入口
│   ├── index.css           # 全局样式 (Tailwind + shadcn CSS vars)
│   ├── lib/
│   │   └── utils.ts        # shadcn 工具函数 (cn)
│   └── components/
│       └── ui/             # shadcn/ui 组件
├── src-tauri/              # Rust 后端
│   ├── src/
│   │   ├── main.rs         # 入口
│   │   └── lib.rs          # Tauri builder + commands
│   ├── Cargo.toml
│   └── tauri.conf.json
├── abtop-collector/        # Rust 监控数据采集库 (git submodule / local)
├── components.json         # shadcn/ui 配置
├── vite.config.ts
└── package.json
```

## 开发命令

```bash
npm run tauri dev      # 启动 Tauri 开发模式 (桌面窗口 + 热重载)
npm run dev            # 仅启动 Vite 前端 dev server
npm run build          # 构建前端
npm run tauri build    # 构建生产版本
cargo check            # 检查 Rust 后端编译 (在 src-tauri/ 下执行)
```

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
| `AgentScope`（本项目） | 当前目录 | AI Agent 监控桌面应用 |

## 关键决策

| 决策 | 方案 |
|:-----|:-----|
| abtop 代码复用 | 提取独立 `abtop-collector` 库 crate（非 vendor） |
| config 解析 | Rust 端 `python3` 子进程导出 JSON |
| 项目发现 | GUI 手动添加 |
| 测试策略 | Agent QA（Playwright E2E），v0.1 无单元测试 |
| 数据刷新 | 文件 → watchfiles，Agent → 2s 轮询 |
| Dashboard 排序 | 按项目名称字母序 |
| 平台 | macOS + Linux + Windows（测试中） |
| Windows 支持 | NSIS installer, WebView2 runtime（不捆绑） |

## 开发规范

- **语言**: TypeScript（严格模式）+ Rust
- **命名**: 小驼峰（变量/函数），大驼峰（类/组件），短横线（文件名）
- **注释**: 中文注释，复杂逻辑需说明意图
- **Git**: 中文提交描述，格式 `[模块] 动作: 描述`
