# AgentScope

AgentScope 是一款跨平台的桌面监控应用，用于实时追踪和管理 AI Agent 会话及项目状态。提供项目仪表盘、Stage 时间线、参数快照、Git 状态、Memory 文件浏览和 Agent 监控等功能。

## 功能特性

- **项目仪表盘**：一览所有注册项目的 Stage、Git 状态和 Agent 活跃度
- **项目详情**：
  - Stage 时间线 — 可视化 L0 → L6 → Verilog → Synthesis → Hardware 开发阶段
  - 参数快照 — 实时解析 config/parameters.py
  - Memory 浏览 — 读取 .claude/memory/*.md 文件
  - 项目记忆 — 查看 CLAUDE.md、规则文档、设计笔记
  - Git 状态 — 分支、变更、暂存、冲突实时监控
- **Agent 监控**：追踪 AI Agent 会话、Token 速率、工具调用历史
- **记忆标记**：从对话中标记重要知识片段，沉淀为项目文档
- **字号调节**：4 档字号预设（紧凑/标准/大/超大），自动适配布局
- **主题切换**：浅色 / 深色 / 跟随系统

## 技术栈

| 层级 | 技术 | 说明 |
|------|------|------|
| 桌面框架 | Tauri v2 | Rust 后端 + WebView 前端 |
| 前端框架 | React 19 + TypeScript | UI 层 |
| 样式 | Tailwind CSS v4 + shadcn/ui | UI 组件库 |
| 图表 | Recharts | 数据可视化 |
| Rust 库 | abtop-collector | 监控数据采集 |
| 包管理 | npm / pnpm | 前端依赖 |

## 快速开始

### 环境要求

- Node.js ≥ 18
- Rust 工具链（cargo、rustc）
- 系统依赖（Linux / Windows）：
  ```bash
  sudo apt-get install libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev patchelf
  ```
  - Windows 依赖：
    - WebView2 Runtime（Windows 11 / 较新 Windows 10 已预装）
    - Visual Studio Build Tools 或 Visual Studio 2022（用于 Rust 编译）

### 安装依赖

```bash
npm install
```

### 开发模式

```bash
# 启动 Tauri 开发模式（桌面窗口 + 热重载）
npm run tauri dev

# 仅启动 Vite 前端 dev server
npm run dev
```

### 构建生产版本

```bash
# 构建前端
npm run build

# 构建桌面应用（生成 AppImage / deb / rpm）
npm run tauri build

# Windows 构建（交叉编译，需安装目标）
rustup target add x86_64-pc-windows-msvc
cargo tauri build --target x86_64-pc-windows-msvc
```

### 检查 Rust 后端

```bash
cd src-tauri
cargo check
```

## 项目结构

```
agent-scope/
├── src/                          # React 前端源码
│   ├── App.tsx                   # 根组件 + 路由
│   ├── pages/                    # 页面组件
│   │   ├── Dashboard.tsx         # 项目仪表盘
│   │   ├── ProjectDetail.tsx     # 项目详情
│   │   ├── AgentMonitor.tsx      # Agent 监控
│   │   └── Settings.tsx          # 设置
│   ├── components/               # 可复用组件
│   │   ├── ui/                   # shadcn/ui 组件
│   │   ├── Sidebar.tsx           # 侧边栏导航
│   │   ├── ProjectMemoryPanel.tsx # 项目记忆面板
│   │   └── CandidateMemoryBox.tsx # 记忆标记管理
│   ├── hooks/                    # 自定义 Hooks
│   │   ├── useTheme.ts           # 主题管理
│   │   └── useFontSize.ts        # 字号管理
│   └── index.css                 # 全局样式
├── src-tauri/                    # Rust 后端
│   ├── src/
│   │   ├── main.rs               # 入口
│   │   ├── lib.rs                # Tauri builder + commands
│   │   └── collectors/           # 数据采集器
│   │       ├── template/         # 模板项目采集
│   │       └── agent/            # Agent 监控采集
│   └── Cargo.toml
├── abtop-collector/              # Rust 监控库（子模块）
└── package.json
```

## 核心概念

### 项目注册

在「设置」中添加项目路径（如 `/Users/me/Repo/my-project`），AgentScope 会自动读取项目中的：

- `.stage` 文件 — 当前开发阶段
- `config/parameters.py` — 项目参数
- `.claude/memory/*.md` — AI 记忆文件
- `.git/` — Git 仓库状态

### 记忆标记

在「项目详情 → 项目记忆 → 对话搜索」中浏览历史对话，点击消息旁的「标记」按钮将重要信息加入记忆标记。确认后沉淀为项目文档（写入 `.sisyphus/notepads/project-memory/decisions.md`）。

## 平台支持

- macOS（开发机）
- Linux（优先验证，生成 AppImage）
- Windows 10+（测试中，NSIS 安装包）

## 开发规范

- **语言**：TypeScript（严格模式）+ Rust
- **命名**：小驼峰（变量/函数）、大驼峰（类/组件）、短横线（文件名）
- **注释**：中文注释，复杂逻辑需说明意图
- **Git**：中文提交描述，格式 `[模块] 动作: 描述`

## 相关项目

| 项目 | 路径 | 关系 |
|:-----|:-----|:-----|
| `ai_project_template` | `/Users/ckstar/Repo/ai_project_template` | 模板项目，被监控的目标 |
| `abtop` | `/Users/ckstar/Repo/abtop` | Rust TUI 监控器，collector 代码复用源 |
| `AgentScope`（本项目） | 当前目录 | AI Agent 监控桌面应用 |

## License

MIT
