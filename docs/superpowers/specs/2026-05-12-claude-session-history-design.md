# Claude Code 会话管理模块设计文档

## 日期

2026-05-12

---

## 1. 概述

### 1.1 目标

为 PTV 新增独立的 Claude Code 会话管理模块，能够扫描本地 `~/.claude/` 目录，收集并按项目组织展示所有历史会话，支持删除非活跃会话。

### 1.2 范围

**包含：**
- 扫描 `~/.claude/sessions/`、`~/.claude/history.jsonl`、`~/.claude/projects/` 三个数据源
- 按项目分组展示会话列表
- 搜索过滤（session 名称 + 项目路径）
- 区分活跃/非活跃会话
- 删除非活跃会话（仅删除 `projects/{编码路径}/{sessionId}.jsonl`）

**不包含：**
- 实时监听 `~/.claude/` 变化
- 渲染会话完整 message 内容
- 会话恢复/重放
- 跨设备同步
- 修改 `abtop-collector/`

---

## 2. 架构设计

### 2.1 后端模块结构

```
src-tauri/src/
├── collectors/
│   ├── claude_history/
│   │   ├── mod.rs          # 模块入口，暴露 Scanner
│   │   ├── scanner.rs      # 目录扫描 + 数据聚合 + 删除逻辑
│   │   ├── models.rs       # 数据结构定义
│   │   └── path_codec.rs   # 共享：encode_cwd_path + decode_project_dir（跨平台）
│   └── template/
│       └── session_transcript.rs  # 复用 path_codec.rs，移除内联 encode_cwd_path
├── commands.rs             # 新增 Tauri 命令
└── lib.rs                  # 注册新命令
```

### 2.2 前端组件结构

```
src/
├── pages/
│   └── ClaudeHistory.tsx           # 主页面，管理左右分栏布局
├── components/
│   └── claude-history/
│       ├── ProjectList.tsx         # 左侧项目列表
│       ├── SessionTimeline.tsx     # 右侧会话时间线
│       └── SearchBar.tsx           # 顶部搜索过滤
├── hooks/
│   └── useClaudeHistory.ts         # 数据获取 + 缓存
└── App.tsx                         # 新增 "claude-history" 路由
```

### 2.3 路由与导航

- `AppRoute` 扩展：`"dashboard" | "agents" | "settings" | "claude-history"`
- `Sidebar` 新增导航项：图标 `MessageSquare`（或 `History`），标签"会话历史"

---

## 3. 数据设计

### 3.1 核心数据结构

```rust
// collectors/claude_history/models.rs

#[derive(Debug, Clone, Serialize)]
pub struct SerClaudeSession {
    pub session_id: String,
    pub name: Option<String>,
    pub cwd: String,
    pub status: SerSessionStatus,
    pub started_at: Option<u64>,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize)]
pub enum SerSessionStatus {
    Active,
    Idle,
    Exited,
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
pub struct SerProjectSessionGroup {
    pub project_path: String,
    pub project_name: String,
    pub sessions: Vec<SerClaudeSession>,
    pub session_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct SerHistoryEntry {
    pub display: String,
    pub timestamp: u64,
    pub session_id: String,
}
```

### 3.2 API 接口

```rust
#[tauri::command]
pub fn list_claude_sessions() -> Result<Vec<SerProjectSessionGroup>, String>;

#[tauri::command]
pub fn get_claude_session_detail(
    session_id: String,
) -> Result<Option<SerClaudeSession>, String>;

#[tauri::command]
pub fn search_claude_history(query: String) -> Result<Vec<SerHistoryEntry>, String>;

#[tauri::command]
pub fn delete_claude_session(session_id: String) -> Result<(), String>;
```

---

## 4. 数据流

### 4.1 扫描流程

```
list_claude_sessions()
    │
    ▼
┌─────────────────┐
│ 1. 扫描 sessions/ │ ← 读取 {pid}.json → 活跃会话元数据
└────────┬────────┘
         │
    ┌────▼────┐
    │ 合并     │ ← 以 sessionId 为 key，活跃会话数据覆盖历史
    └────┬────┘
         │
┌────────▼────────┐
│ 2. 扫描 projects/ │ ← 每个编码目录 = 一个项目
│    └── 列出 .jsonl │ ← 每个 .jsonl = 一个会话
└────────┬────────┘
         │
┌────────▼────────┐
│ 3. 按项目分组    │ → Vec<SerProjectSessionGroup>
│    └── 活跃会话置顶 │
└─────────────────┘
```

### 4.2 删除流程

```
delete_claude_session(session_id)
    │
    ▼
┌─────────────────┐
│ 1. 检查活跃状态  │ ← 若 session 在 sessions/ 中存在 → 拒绝删除
└────────┬────────┘
         │
    ┌────▼────┐
    │ 2. 查找 .jsonl │ ← 遍历 projects/ 各目录
    └────┬────┘
         │
    ┌────▼────┐
    │ 3. 删除文件  │ ← std::fs::remove_file()
    └────┬────┘
         │
    ┌────▼────┐
    │ 4. 返回结果  │ ← Ok(()) 或 Err
    └─────────┘
```

---

## 5. 路径编解码设计

### 5.1 编码规则

| 平台 | 原始路径 | 编码结果 | 规则 |
|------|----------|----------|------|
| macOS/Linux | `/Users/name/Repo` | `-Users-name-Repo` | 首 `/` 保留为 `-`，其余 `/` 替换为 `-` |
| Windows | `C:\Repo` | `C--Repo` | `\` 替换为 `--` |

### 5.2 解码规则

| 编码目录名 | 解码结果 | 规则 |
|------------|----------|------|
| `-Users-name-Repo` | `/Users/name/Repo` | 首 `-` 替换为 `/`，其余 `-` 替换为 `/` |
| `C--Repo` | `C:\Repo` | `--` 替换为 `\`，单 `-` 替换为 `\` |

### 5.3 共享模块

```rust
// collectors/claude_history/path_codec.rs

pub fn encode_cwd_path(cwd: &str) -> String;
pub fn decode_project_dir(encoded: &str) -> String;
```

**共享策略**：
- `session_transcript.rs` 移除内联的 `#[cfg(not(windows))] encode_cwd_path`，改为 `use crate::collectors::claude_history::path_codec::encode_cwd_path`
- `session_transcript.rs` 的 `sessions_dir` 移除 `#[cfg(windows)]` 过时分支，统一使用 `encode_cwd_path`

---

## 6. 前端设计

### 6.1 页面布局

```
┌─────────────────────────────────────────────────────────────┐
│  会话历史                                   [🔍 搜索...]    │
├──────────────────┬──────────────────────────────────────────┤
│                  │                                          │
│  📁 Project A    │  🔵 Session 1 (运行中)                   │
│  📁 Project B    │     2026-05-12 10:30                     │
│  📁 Project C    │                                          │
│                  │  ⚪ Session 2                            │
│                  │     2026-05-11 15:20     [删除]          │
│                  │                                          │
│                  │  ⚪ Session 3                            │
│                  │     2026-05-10 09:15     [删除]          │
│                  │                                          │
├──────────────────┴──────────────────────────────────────────┤
│  共 3 个项目，12 个会话                         [刷新 🔄]    │
└─────────────────────────────────────────────────────────────┘
```

### 6.2 交互设计

| 交互 | 行为 |
|------|------|
| 点击左侧项目 | 右侧展示该项目的会话列表 |
| 搜索输入 | 实时过滤（session 名称 + 项目路径），debounce 200ms |
| 删除按钮 | 确认对话框 → 删除 → 列表自动移除 |
| 刷新按钮 | 重新调用 `list_claude_sessions`，更新缓存 |
| 活跃会话 | 绿色圆点标记，置顶显示，无删除按钮 |

### 6.3 状态管理

使用 React `useState` + `useCallback`（无全局状态库）：

```typescript
interface ClaudeHistoryState {
  projectGroups: ProjectSessionGroup[];
  selectedProject: string | null;
  searchQuery: string;
  isLoading: boolean;
  error: string | null;
}
```

---

## 7. 错误处理

| 场景 | 后端行为 | 前端行为 |
|------|----------|----------|
| `~/.claude/` 不存在 | `Err("Claude Code 配置目录未找到")` | 空状态 + 提示安装 Claude Code |
| `~/.claude/` 无读取权限 | `Err("无法读取 Claude Code 配置目录")` | 权限错误提示 |
| `history.jsonl` 损坏行 | 跳过，继续扫描 | 正常展示，损坏数据自动忽略 |
| 删除活跃会话 | `Err("无法删除正在运行的会话")` | 按钮禁用，hover 显示"运行中" |
| 删除时文件已不存在 | `Err("会话文件不存在或已被删除")` | 自动从列表移除 |

---

## 8. 性能与缓存

### 8.1 缓存策略

- 首次进入页面时全量扫描
- 扫描结果缓存在 `useClaudeHistory` hook 的 state 中
- 页面切换时复用缓存，不重新扫描
- 手动刷新按钮触发重新扫描

### 8.2 性能优化

- `history.jsonl` 流式读取（`BufReader` + 逐行解析）
- 每个项目默认展示最近 50 条会话，提供"加载更多"
- 首次实现不做虚拟滚动，如数据量极大后续优化

---

## 9. 安全与隐私

- **只读扫描**：不写入 `~/.claude/` 任何文件（删除操作除外）
- **仅展示元数据**：session 名称、时间、状态，不渲染完整 message 内容
- **删除不可逆**：删除后 Claude Code CLI 的 `/resume` 无法恢复该会话
- **仅删除非活跃会话**：避免破坏正在运行的 CLI 实例

---

## 10. 测试策略

### 10.1 Rust 单元测试

| 测试项 | 覆盖内容 |
|--------|----------|
| `path_codec::encode_cwd_path` | macOS/Linux/Windows 路径编码 |
| `path_codec::decode_project_dir` | 编码目录名还原为原始路径 |
| `scanner::list_sessions` | 扫描 `projects/` 正确分组 |
| `scanner::merge_active_sessions` | 活跃会话数据覆盖历史数据 |
| `scanner::delete_session` | 仅允许删除非活跃会话 |
| JSONL 容错解析 | 损坏行跳过不中断 |

### 10.2 Playwright E2E

| 测试项 | 覆盖内容 |
|--------|----------|
| 页面渲染 | ClaudeHistory 路由加载，Sidebar 导航项存在 |
| 空状态 | `~/.claude/` 不存在时的 UI 提示 |
| 搜索过滤 | 输入关键词后列表正确过滤 |
| 删除交互 | 删除按钮 → 确认对话框 → 列表更新 |

---

## 11. 风险评估

| 风险 | 可能性 | 影响 | 缓解措施 |
|------|--------|------|----------|
| `history.jsonl` 过大 | 中 | 高 | 流式读取，不一次性加载 |
| Windows 路径解码歧义 | 中 | 中 | 单元测试覆盖多种路径场景 |
| 删除活跃会话 | 低 | 高 | 仅允许删除非活跃会话 |
| 隐私泄露 | 中 | 高 | 仅展示元数据，不渲染 message 内容 |
| Claude Code 格式变更 | 低 | 中 | 忽略未知字段，损坏行跳过 |

---

## 12. 任务分解

```
Wave 1（后端核心）：
├── Task 1: path_codec.rs + 修复 session_transcript.rs
├── Task 2: claude_history 模块（scanner + models）
└── Task 3: commands.rs 新增命令 + lib.rs 注册

Wave 2（前端 + 集成）：
├── Task 4: 前端页面组件（ClaudeHistory + 子组件 + hook）
└── Task 5: 路由/导航扩展 + 集成验证 + Playwright 测试
```
