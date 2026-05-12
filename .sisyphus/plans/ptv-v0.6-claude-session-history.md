# PTV v0.6 — Claude Code 会话管理

## TL;DR

> **快速摘要**：新增独立模块，扫描本地 `~/.claude/` 目录，收集所有 Claude Code 历史会话并按项目组织展示。后端新建 `collectors/claude_history/` 模块负责数据扫描与解析，同时修复 `session_transcript.rs` 中已过时的 Windows 路径编码逻辑；前端新增独立路由 `"claude-history"` 及对应页面，Sidebar 新增导航入口。与现有 Dashboard/Agent/Settings 功能零耦合。
>
> **交付物**：
> - Rust 后端：`collectors/claude_history/` 模块（scanner + models + path_codec）+ 新增 Tauri 命令
> - Rust 后端：共享路径编解码模块 `path_codec.rs`（替换 `session_transcript.rs` 中内联的 Unix-only 实现）
> - 前端：`pages/ClaudeHistory.tsx` + `components/claude-history/` 子组件
> - Sidebar：`AppRoute` 扩展 + 导航项新增
> - 跨平台：macOS/Linux/Windows 路径编码解码支持
>
> **预估工作量**：中等（后端 3 任务 + 前端 2 任务 = 5 任务）
> **并行执行**：YES — 2 个 Wave
> **关键路径**：Task 1 → Task 2 → Task 3 → Task 4 → Task 5

---

## 上下文

### 原始需求
用户希望 PTV 能够管理本地环境中所有的 Claude Code 历史会话：
1. 获取软件运行本地环境中的所有 Claude Code 历史会话
2. 按项目进行历史会话的展示组织
3. 不和当前已实现的功能混在一起，作为独立模块实现
4. 客户端界面上新开菜单用于此功能

### 调研总结

**Claude Code 本地数据结构**（macOS/Linux/Windows 结构一致）：

| 位置 | 内容 | 用途 |
|------|------|------|
| `~/.claude/sessions/{pid}.json` | 活跃会话元数据 | pid、sessionId、cwd、status、name、startedAt、updatedAt |
| `~/.claude/history.jsonl` | 历史命令记录 | display(输入内容)、timestamp、project(cwd)、sessionId |
| `~/.claude/projects/{编码路径}/{sessionId}.jsonl` | 按项目存储的完整 message 历史 | 项目级会话详情 |

**路径编码规则**（基于对 `session_transcript.rs` 现有实现及 Windows 实测的推导）：
- macOS/Linux: `/Users/name/Repo` → `-Users-name-Repo`（首 `/` 保留为前缀 `-`，其余 `/` 替换为 `-`）
- Windows: `C:\Repo` → `C--Repo`（`\` 替换为 `--`）

**⚠️ 重要发现**：现有 `session_transcript.rs` 第 690-695 行的 `#[cfg(windows)]` 分支注释声称"Claude Code 不在 Windows 上运行"，但 Windows 测试环境（`192.168.3.10`）已确认：
- 配置目录：`%USERPROFILE%\.claude` 存在且结构与 macOS 完全一致
- `history.jsonl` 格式相同
- `projects/` 目录编码规则确认（`C--Repo`）

**结论**：Claude Code 在 Windows 上实际运行，现有代码的 Windows 分支注释已过时，需一并修复。

**现有功能区分**：
- 现有 `session_transcript.rs`：读取项目内部的 `.sisyphus/sessions/`（项目自身会话记录）
- 新增功能：读取全局 `~/.claude/`（所有 Claude Code 会话）
- 两者数据源完全不重叠，天然适合独立模块

**用户决策**：
- 作为独立模块实现，不与现有 Dashboard/Agent/Settings 功能耦合
- 新开菜单路由，Sidebar 新增导航项
- 按项目组织展示（项目列表 + 该项目下的会话列表）
- 支持搜索过滤
- 路径编解码逻辑提取为共享模块（避免 `claude_history` 与 `session_transcript.rs` 重复实现）

### Metis 审查要点

**已识别的缺口**：
- `history.jsonl` 可能很大，需要流式/逐行读取而非一次性加载
- Windows 路径解码需处理 `--`（双横线）vs `-`（单横线）的歧义：`C--Users--name` 解码为 `C:\Users\name`
- 活跃会话（`sessions/*.json`）与历史会话（`history.jsonl`、`projects/`）的数据合并逻辑
- 项目目录可能已删除（cwd 不存在），需处理 orphaned session
- 前端需要处理长列表的性能（虚拟滚动或分页）
- **隐私风险**：Claude Code 历史可能包含敏感信息（API 密钥、密码等），PTV 仅展示元数据（session 名称、时间、命令摘要），不渲染完整 message 内容
- **缓存策略**：首次扫描后结果缓存，页面切换时复用；提供手动刷新按钮；不做实时监听
- **并发冲突**：Claude Code 写入文件时 PTV 读取可能失败，需容错处理（跳过损坏行）
- **权限错误**：`~/.claude/` 存在但无读取权限时，与"目录不存在"区分提示

**Metis 锁定的防护栏**：
- ❌ 不修改现有 Dashboard/Agent/Settings 页面
- ❌ 不做 Claude Code 会话的实时监听（仅按需扫描 + 缓存复用）
- ❌ 不做会话内容的完整 message 渲染（仅展示元数据和摘要）
- ❌ 不做会话恢复/重放功能
- ❌ 不做跨设备会话同步
- ❌ 不修改 `abtop-collector/` 任何文件
- ❌ 不新增外部依赖（优先用标准库 + 已有 crates）
- ❌ 不写入 `~/.claude/` 任何文件

---

## 工作目标

### 核心目标
为 PTV 新增独立的 Claude Code 会话管理模块，能够扫描本地 `~/.claude/` 目录，收集并按项目组织展示所有历史会话。

### 具体交付物

**后端（Rust）**：
- `src-tauri/src/collectors/claude_history/mod.rs` — 模块入口
- `src-tauri/src/collectors/claude_history/scanner.rs` — 目录扫描与数据收集
- `src-tauri/src/collectors/claude_history/models.rs` — 数据结构定义
- `src-tauri/src/collectors/claude_history/path_codec.rs` — 路径编码/解码（跨平台）
- `src-tauri/src/commands.rs` — 新增 Tauri 命令（`list_claude_sessions`、`get_claude_session_detail`、`search_claude_history`）
- `src-tauri/src/lib.rs` — 注册新命令
- `src-tauri/src/collectors/template/session_transcript.rs` — 复用 `path_codec.rs`，移除内联的 Unix-only `encode_cwd_path`，修复 `sessions_dir` 的 Windows 分支

**前端（React/TypeScript）**：
- `src/App.tsx` — 新增 `"claude-history"` 路由
- `src/components/Sidebar.tsx` — 新增导航项
- `src/pages/ClaudeHistory.tsx` — 主页面（单文件）
- `src/components/claude-history/ProjectList.tsx` — 项目列表侧边栏
- `src/components/claude-history/SessionTimeline.tsx` — 会话时间线
- `src/components/claude-history/SearchBar.tsx` — 搜索过滤
- `src/hooks/useClaudeHistory.ts` — 数据获取 hook

### 完成定义
- [ ] `cargo test -p ptv` 全部通过（无回归）
- [ ] `npm run build` 通过
- [ ] Playwright E2E 全部通过
- [ ] 新页面能正确列出所有项目的 Claude Code 会话
- [ ] 搜索功能能按 session 名称/内容过滤
- [ ] Windows 路径编码解码正确（通过代码审查 + 单元测试）
- [ ] 空状态处理（无会话、无项目、扫描失败、权限不足）
- [ ] `session_transcript.rs` 的 Windows 分支修复后行为正确

### 必须包含
- `claude_config_dir()` 跨平台辅助函数（`~/.claude` / `%USERPROFILE%\.claude`）
- `encode_cwd_path()` / `decode_project_dir()` 路径编解码（macOS/Linux/Windows）
- `SerClaudeSession` 结构体：sessionId、name、cwd、status、startedAt、isActive
- `SerProjectSessionGroup` 结构体：projectPath、projectName、sessions[]、sessionCount
- `SerHistoryEntry` 结构体：display、timestamp、sessionId
- 流式读取 `history.jsonl`（`BufReader` + 逐行解析）
- 活跃会话与历史会话数据合并（同一 sessionId 去重，活跃会话优先）
- 新增 Tauri 命令：`list_claude_sessions`、`get_claude_session_detail`、`search_claude_history`
- 前端：项目列表（可折叠）+ 会话时间线 + 搜索框 + 手动刷新按钮
- 空状态 UI（无会话、扫描失败、权限不足提示）
- Rust 单元测试：路径编解码（3 平台场景）、scanner 基础功能、JSONL 容错解析
- `session_transcript.rs` 复用 `path_codec.rs`，移除内联 `encode_cwd_path`

### 必须不包含（护栏）
- ❌ 修改现有 Dashboard/Agent/Settings 页面
- ❌ 实时监听 `~/.claude/` 目录变化
- ❌ 渲染会话完整 message 内容
- ❌ 会话恢复/重放功能
- ❌ 跨设备同步
- ❌ 修改 `abtop-collector/`
- ❌ 新增外部 crate 依赖
- ❌ 写入 `~/.claude/` 任何文件

---

## 验证策略

> **零人工干预** — 所有验证均由 agent 执行。

### 测试决策
- **基础设施存在**：YES（Rust `#[cfg(test)]` + Playwright E2E）
- **自动化测试**：Rust 新增单元测试 + Playwright 新增页面渲染测试
- **框架**：Rust: `cargo test`；前端: Playwright

### QA 策略
每个任务包含 Agent-Executed QA Scenarios：
- **Rust 任务**：`cargo test` 验证 + 代码审查
- **前端任务**：页面渲染验证 + Playwright 基础测试
- **集成任务**：端到端流程验证（后端命令 → 前端渲染）

---

## 执行策略

### 并行执行 Waves

```
Wave 1（立即开始 — 后端核心）：
├── Task 1: 共享路径编解码模块 `path_codec.rs` + 修复 `session_transcript.rs`
│   └── 包含：跨平台 encode/decode、单元测试（3 平台场景）、替换现有内联实现
├── Task 2: collectors/claude_history 模块（scanner + models）
│   └── 包含：目录扫描、数据结构、流式 JSONL 解析、活跃/历史会话合并、单元测试
└── Task 3: commands.rs 新增 Tauri 命令 + lib.rs 注册
    └── 包含：list_claude_sessions、get_claude_session_detail、search_claude_history

Wave 2（Wave 1 完成后 — 前端 + 集成）：
├── Task 4: 前端页面组件（ClaudeHistory + ProjectList + SessionTimeline + SearchBar）
│   └── 包含：UI 布局、数据展示、搜索过滤、空状态、手动刷新
├── Task 5: App.tsx 路由扩展 + Sidebar 导航新增 + 集成验证 + Playwright 测试
    └── 包含：AppRoute 扩展、Sidebar 更新、端到端验证、E2E 基础测试、回归检查
```

### 依赖关系

```
Task 1 ──→ Task 2 ──→ Task 3 ──→ Task 5
                          ↑
Task 4 ───────────────────┘
```

### 快速通道
- 如时间紧张，可跳过 `search_claude_history` 命令（前端本地过滤即可）
- 如时间紧张，可简化前端为单列表视图（不做项目侧边栏 + 时间线分栏）
- 如时间紧张，`session_transcript.rs` 的复用改造可延后（先让 `claude_history` 独立实现，后续再提取共享模块）

---

## 数据结构参考

### Rust Models

```rust
// collectors/claude_history/models.rs

#[derive(Debug, Clone, Serialize)]
pub struct SerClaudeSession {
    pub session_id: String,
    pub name: Option<String>,    // 用户命名的 session 名称
    pub cwd: String,             // 工作目录
    pub status: SerSessionStatus, // active | idle | exited
    pub started_at: Option<u64>, // timestamp ms
    pub is_active: bool,         // 是否当前在 sessions/ 中存在
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
    pub project_name: String, // 路径最后一段
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

### API 接口

```rust
// commands.rs

#[tauri::command]
pub fn list_claude_sessions() -> Result<Vec<SerProjectSessionGroup>, String>;

#[tauri::command]
pub fn get_claude_session_detail(
    session_id: String,
) -> Result<Option<SerClaudeSession>, String>;

#[tauri::command]
pub fn search_claude_history(query: String) -> Result<Vec<SerHistoryEntry>, String>;
```

---

## 边界情况处理

| 场景 | 处理策略 |
|------|----------|
| `history.jsonl` 正在被写入 | 流式读取，遇到无法解析的行跳过并记录警告（与 `session_transcript.rs` 的 `parse_jsonl_file` 一致） |
| `~/.claude/` 不存在 | 返回空列表，UI 显示"未找到 Claude Code 配置目录" |
| `~/.claude/` 存在但无读取权限 | 返回权限错误，UI 显示"无法读取 Claude Code 配置目录" |
| 项目目录已删除（orphaned session） | 仍然展示，但 `project_path` 标注为不存在（灰色显示或添加 🚫 图标） |
| 同名项目不同路径 | `project_name` 显示最后一段，鼠标悬停显示完整路径 |
| 会话数量极大 | 首次实现不做虚拟滚动，每个项目默认展示最近 50 条，提供"加载更多" |
| `sessions/*.json` 与 `projects/` 中同一 sessionId | 以 `sessions/*.json` 为准（活跃会话优先），合并时更新 `is_active = true` |

---

## 风险与缓解

| 风险 | 可能性 | 影响 | 缓解措施 |
|------|--------|------|----------|
| `history.jsonl` 过大导致内存问题 | 中 | 高 | 流式读取（`BufReader`），不一次性加载 |
| Windows 路径解码歧义 | 中 | 中 | 单元测试覆盖多种 Windows 路径场景；`--` 对应 `\`、`-` 对应 `/` |
| 用户无 `~/.claude/` 目录 | 低 | 中 | 优雅处理：返回空列表 + UI 提示 |
| 项目目录已删除（orphaned session） | 中 | 低 | 标记为 orphaned，仍然展示但标注 |
| 前端长列表性能 | 低 | 中 | 每个项目默认展示最近 50 条，提供"加载更多" |
| **隐私泄露** | 中 | 高 | 仅展示元数据（session 名称、时间），不渲染完整 message 内容；不暴露 `history.jsonl` 原始内容 |
| **Claude Code 格式变更** | 低 | 中 | 解析时忽略未知字段，损坏行跳过不中断整体扫描 |
| 扫描频率过高 | 中 | 低 | 结果缓存复用，页面切换不重新扫描；手动刷新按钮 |
| 与现有 `session_transcript.rs` 改造冲突 | 低 | 中 | 提取共享模块后统一调用，改造范围限制在 `encode_cwd_path` 和 `sessions_dir` |
