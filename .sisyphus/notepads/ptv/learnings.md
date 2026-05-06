# abtop-collector 提取任务总结

## 完成状态
- [x] 新 crate 目录：`/Users/ckstar/Repo/ai_project_template_visualization/abtop-collector/`
- [x] Cargo.toml 含 `[lib]` 段，可独立编译
- [x] 提取模块：claude.rs、codex.rs、mcp.rs、process.rs、rate_limit.rs、mod.rs、model/session.rs、model/mod.rs
- [x] 移除 ratatui、crossterm 依赖
- [x] 验证：`cargo build` + `cargo test` 通过（86 个测试全部通过）

## 关键发现

### 1. 源文件无 ratatui/crossterm 引用
提取的 collector 和 model 模块中原本就不包含任何 ratatui 或 crossterm 的引用，因此无需额外清理代码。

### 2. 需要包含 mcp.rs
虽然任务列表未明确列出 mcp.rs，但 `collector/mod.rs` 中直接引用了 `mcp::detect()` 和 `mcp::McpServer`，因此必须一并提取，否则编译失败。

### 3. crate 根模块声明
新 crate 需要 `src/lib.rs` 声明顶层模块：
```rust
pub mod collector;
pub mod model;
```
这样 `collector/` 内的 `use crate::model::...` 引用才能正确解析。

### 4. 依赖分析结果
提取后 Cargo.toml 仅保留以下依赖（对比原 abtop 的 Cargo.toml 移除了 ratatui、crossterm）：
- `serde` (+ derive)
- `serde_json`
- `dirs`
- `chrono` (+ serde)
- `tempfile`
- 平台特定：`proc_pidinfo` (apple)、`sysinfo` (windows)、`libc` (linux)

### 5. 测试覆盖
`cargo test` 运行 86 个单元测试，全部通过，包括：
- collector::claude 测试（约 40+）
- collector::codex 测试（约 15+）
- collector::mcp 测试（约 7）
- collector::process 测试（约 3）
- collector::mod 测试（约 5）
- model::session 测试（约 2）

## 目录结构
```
abtop-collector/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── collector/
    │   ├── mod.rs
    │   ├── claude.rs
    │   ├── codex.rs
    │   ├── mcp.rs
    │   ├── process.rs
    │   └── rate_limit.rs
    └── model/
        ├── mod.rs
        └── session.rs
```

## 注意事项
- 原始 abtop 仓库未被修改
- 未引入 app.rs、main.rs、ui/ 等 TUI 相关代码
- 未添加新功能，仅做代码提取和结构调整

---

# Task 1.1: ProjectRegistry 实现总结

## 完成状态
- [x] `src-tauri/src/registry.rs` — 包含 `ProjectRegistry` struct
- [x] API: `add(path)`, `remove(path)`, `list()`, `get(path)`
- [x] 持久化到 `app_data_dir()/projects.json`（Tauri setup 中通过 `app.path().app_data_dir()` 获取）
- [x] 路径去重（`canonicalize()` 规范化绝对路径作为 HashMap key）
- [x] `get()` 返回 `Result<ProjectEntry, RegistryError>` 而非 panic
- [x] 暴露 Tauri commands: `add_project`, `remove_project`, `list_projects`
- [x] 14 个单元测试全部通过（34 总测试 = 14 registry + 20 watcher）

## 关键发现

### 1. `tauri::Manager` trait 必须显式导入
在 `lib.rs` 中使用 `app.path()` 和 `app.manage()` 时，需要 `use tauri::Manager;`。该 trait 提供了 Tauri App 的 path 和 state 管理方法。
```rust
use tauri::Manager; // 必须导入，否则 app.path() 和 app.manage() 不可用
```

### 2. Registry 状态管理方式
使用 `Mutex<ProjectRegistry>` 包装并通过 `app.manage()` 注册为 Tauri 全局状态：
- 在 `.setup()` 回调中初始化并注册
- 在 Tauri commands 中通过 `tauri::State<'_, Mutex<ProjectRegistry>>` 获取
- 每个 command 加锁后操作

### 3. 路径去重设计
- `canonicalize()` 解析符号链接和 `./`/`../`，生成规范绝对路径
- 以规范化路径作为 `HashMap<String, ProjectEntry>` 的 key
- `remove()` 和 `get()` 在 canonicalize 失败时回退到使用原始路径字符串

### 4. 持久化策略
- `load_or_default()` 静默处理加载失败（文件不存在/格式错误时返回空注册表）
- 每个 add/remove 操作自动触发 `save()`
- 使用 `serde_json::to_string_pretty` 生成可读 JSON
- 保存前自动创建父目录（通过 `fs::create_dir_all`）

### 5. `dirs` crate 使用
- 添加 `dirs = "5"` 依赖
- 提供 `ProjectRegistry::default_data_dir()` 方法返回平台数据目录
- 在 Tauri setup 中使用 `app.path().app_data_dir()` 而非 `dirs`（更符合 Tauri 惯例）

## 目录结构
```
src-tauri/src/
├── lib.rs          # 入口：模块注册 + Tauri commands + Builder setup
├── main.rs         # 二进制入口
├── registry.rs     # [新] ProjectRegistry 实现
└── watcher.rs      # 文件监听器
```

## 测试覆盖
共 14 个单元测试（`registry::tests` 模块）：
- 添加操作：test_add_project, test_add_duplicate_rejected, test_add_duplicate_normalized, test_add_nonexistent_path_fails
- 移除操作：test_remove_project, test_remove_nonexistent_returns_not_found
- 列出操作：test_list_empty, test_list_sorted, test_list_after_add
- 获取操作：test_get_project, test_get_nonexistent_returns_not_found
- 持久化：test_persistence, test_persistence_after_remove, test_load_or_default_nonexistent_file, test_load_or_default_corrupted_file
- 辅助功能：test_default_data_dir
- 集成场景：test_full_lifecycle, test_multiple_projects

---

# Task 1.3: Agent 运行时采集器实现总结

## 完成状态
- [x] `src-tauri/src/collectors/agent/mod.rs` — AgentCollector 实现（701 行，含 5 个单元测试）
- [x] `src-tauri/src/collectors/mod.rs` — collectors 模块入口
- [x] `src-tauri/Cargo.toml` — 添加 `abtop-collector` path 依赖
- [x] `src-tauri/src/lib.rs` — setup 中启动 AgentCollector
- [x] `cargo check` 通过，`cargo test` 22 个测试全部通过（5 agent + 17 existing）

## 关键发现

### 1. `tauri::Emitter` trait 必须显式导入
在 Rust 后端调用 `app_handle.emit("event", payload)` 时，必须 `use tauri::Emitter;`。
该 trait 提供 `emit` 方法，虽然 `AppHandle` 已实现此 trait，但不导入则方法不可见。
```rust
use tauri::{AppHandle, Emitter}; // 两者都需要
```

### 2. AgentSession 不可直接 Serialize
`abtop-collector` 的 `AgentSession` 未实现 `Serialize`，且包含 `&'static str` 和自定义枚举 `SessionStatus` 等不可序列化字段。
解决方案：创建镜像结构体 `AgentInfo`（仅含可序列化字段），在采集时手动从 `AgentSession` 转换。

### 3. SessionStatus 映射策略
由于不能修改 `abtop-collector`，创建 `SerializableStatus` 枚举并手动映射：
```rust
pub enum SerializableStatus {
    Thinking, Executing, Waiting, RateLimited, Done,
}
impl From<&SessionStatus> for SerializableStatus { ... }
```

### 4. token_rate 计算方式
利用 `last_tokens: HashMap<String, (u64, Instant)>` 记录每个 session 上一次的 active_tokens 和时间戳：
- 首次采集：token_rate = 0.0
- 后续采集：delta_tokens / delta_seconds
- 每 2 秒采样一次，因此速率单位为 token/秒

### 5. 按 cwd 关联注册项目的匹配逻辑
使用前缀匹配（去除末尾斜杠后比对）：
```rust
session_cwd == project_path || session_cwd.starts_with(&format!("{}/", project_path))
```
未匹配的 session 放入 `unmapped` 列表，确保所有采集数据都可见。

### 6. 错误降级设计
- 使用 `std::panic::catch_unwind` 包裹 `MultiCollector::collect()` 调用
- 采集 panic 时记录错误日志，不中断轮询线程
- Tauri event 发送失败也仅记录日志，不影响下一轮
- 空 session 列表是正常的（无活跃 Agent 时），仍发送空 payload

### 7. 线程生命周期管理
`AgentCollector` 使用 `Arc<AtomicBool>` 作为运行状态信号：
- `start()` 创建后台线程，传入 running 的 clone
- `stop()` 设置 `running = false`
- 线程在每次 sleep 后检查信号，安全退出
- 不持有 JoinHandle（Tauri app 生命周期内持续运行）

### 8. lib.rs 历史代码清理
lib.rs 中存在重复的 `run()` 函数定义和未实现的 `registry` 模块引用（来自之前任务的残留），导致编译失败。重写 lib.rs 为干净版本。

## 数据结构

### AgentUpdatePayload（Tauri event payload）
```rust
pub struct AgentUpdatePayload {
    pub projects: Vec<ProjectAgents>,    // 按项目分组
    pub unmapped: Vec<AgentInfo>,        // 未匹配 session
    pub timestamp_ms: u64,
    pub total_sessions: usize,
}
```

### AgentInfo（单个 session）
包含 20+ 字段：session_id, agent_type, status, model, context_percent, context_window,
total_input_tokens, total_output_tokens, token_rate, git_branch, git_added, git_modified,
children, current_tasks, initial_prompt 等。

## 测试覆盖
5 个单元测试：
- `test_agent_collector_new` — 构造验证
- `test_register_unregister_project` — 注册/注销项目路径
- `test_serializable_status_from` — SessionStatus 映射
- `test_session_to_info_basic` — AgentSession → AgentInfo 转换
- `test_build_payload_mapping` — cwd 前缀匹配 + unmapped
- `test_build_payload_empty_projects` — 空 session 边界情况

## 注意事项
- 不引入 ratatui、crossterm、log crate（使用 eprintln/println）
- 未修改 abtop-collector crate（仅使用其 API）
- 无新增 HTTP/WebSocket 连接（纯 Tauri event 机制）
- 轮询间隔固定 2 秒，使用精确时间补偿（sleep 剩余时间）


---

# Task 1.2: 模板项目数据采集器实现总结

## 完成状态
- [x] `src-tauri/src/collectors/template/` 目录含 4 个 collector + 编排层
- [x] `stage.rs` — StageCollector + Stage enum（L0-L6, Verilog, Synthesis, Hardware）
- [x] `config.rs` — ConfigCollector（复用 parse_parameters_py）+ ProjectConfig
- [x] `memory.rs` — MemoryCollector + YAML frontmatter 手动解析
- [x] `git.rs` — GitCollector（git branch + git status --porcelain）
- [x] `mod.rs` — TemplateDataCollector 编排层 + WatchedCollector FileWatcher 集成
- [x] 源码布局检测：flat（src/python_model/）vs namespaced（src/<name>/python_model/）
- [x] 错误处理：路径不存在返回空数据/Err，不 panic
- [x] `cargo test` 56 个测试全部通过（22 template + 6 agent + 28 watcher）

## 关键发现

### 1. Task 0.3 的 config.rs 在模板项目自身中
`parse_parameters_py()` 原始实现在 `ai_project_template/src-tauri/src/collectors/config.rs`（模板项目自己的 Tauri 后端），而非 ptv 项目中。直接复制并适配到 `collectors/template/config.rs`，保持 API 兼容。

### 2. YAML frontmatter 无需外部依赖
使用手动行解析替代 regex/yaml crate：
- 检测文件是否以 `---` 开头
- 查找第二个 `---` 作为 frontmatter 结束标记
- 逐行解析 `key: value` 格式
- 无 frontmatter 时返回空 HashMap + 完整内容
这样可以避免引入 `regex`、`yaml-rust` 等额外依赖。

### 3. GitCollector 的多层降级策略
```
git 不可用 → GitStatus::git_not_available()
非 git 仓库 → GitStatus::no_repo()
git 命令失败 → 返回 Ok（空状态）而非 Err
成功执行 → 完整 GitStatus
```
这样前端始终能收到可序列化的状态对象，无需处理 Err 分支。

### 4. macOS 文件系统时间戳精度问题
在测试 `WatchedCollector` 时，文件创建后立即修改可能导致 mtime 不变（1秒精度）。解决方案：
- 测试中创建文件后 sleep 100ms
- FileWatcher 轮询间隔 500ms
- 给初始快照预留 600ms 时间
- 总超时设置为 10 秒

### 5. WatchedCollector 的防抖实现
使用 `Arc<Mutex<Option<(Instant, Vec<PathBuf>)>>>` 作为待处理队列：
- FileWatcher 回调中记录变化路径和时间戳
- 独立循环每 100ms 检查是否超过 debounce 间隔
- 超时后执行完整采集并通过 channel 发送事件
- 默认防抖 5000ms，测试中调整为 300ms

### 6. 源码布局检测逻辑
```rust
pub fn detect(path: &Path) -> SourceLayout {
    if path.join("src/python_model").is_dir() {
        return SourceLayout::Flat;
    }
    // 遍历 src/ 下的子目录，查找 */python_model/
    for entry in fs::read_dir(path.join("src"))? {
        if entry.join("python_model").is_dir() {
            return SourceLayout::Namespaced(module_name);
        }
    }
    SourceLayout::Unknown
}
```

### 7. 路径不存在的统一处理策略
| 采集器 | 路径不存在时的行为 |
|:-------|:-------------------|
| StageCollector | Err(StageError::FileNotFound) |
| ConfigCollector | Err(ParameterError::FileNotFound) |
| MemoryCollector | Ok(Vec::new)（返回空列表） |
| GitCollector | Ok(GitStatus::no_repo()) |

### 8. Stage 解析的兼容性设计
支持多种输入格式：
- 大小写不敏感：`L1`, `l1`, `L1_prototype` 都能解析
- 带下划线前缀：取第一个 `_` 前的部分作为阶段标识
- 空内容/未知内容：返回明确的错误枚举

## 目录结构
```
src-tauri/src/collectors/
├── mod.rs              # collectors 模块入口（agent + template）
├── agent/
│   └── mod.rs          # AgentCollector（Task 1.3）
└── template/
    ├── mod.rs          # 编排层：TemplateDataCollector + WatchedCollector
    ├── stage.rs        # StageCollector + Stage enum
    ├── config.rs       # ConfigCollector + parse_parameters_py
    ├── memory.rs       # MemoryCollector + YAML frontmatter 解析
    └── git.rs          # GitCollector + GitStatus
```

## 测试覆盖
22 个 template 相关单元测试：
- stage: 11 个（解析、采集、错误边界）
- config: 3 个（文件不存在、导出失败、Collector 接口）
- memory: 7 个（frontmatter 分割、空目录、多文件、跳过非 md）
- git: 5 个（无仓库、干净仓库、未追踪、已修改、已暂存）
- mod（编排层）: 6 个（布局检测、空项目、阶段采集、FileWatcher 集成）

## 注意事项
- 未修改被监控项目的任何文件（只读采集）
- 未引入新的外部依赖（仅添加 `tempfile` 作为 dev-dependency）
- ConfigCollector 调用 `python3` 子进程，要求系统已安装 Python3
- MemoryCollector 跳过无法解析的文件，继续处理其他文件

---

# 前端脚手架实现总结

## 完成状态
- [x] `src/App.tsx` 改为前端路由状态入口，使用条件渲染切换页面
- [x] `src/components/Layout.tsx` 提供侧边栏 + 主内容 ScrollArea 布局
- [x] `src/components/Sidebar.tsx` 提供 Dashboard / Projects / Agents / Settings 导航和展开切换
- [x] `src/pages/` 下新增 Dashboard、ProjectDetail、AgentMonitor、Settings 占位页
- [x] `src/hooks/useTauri.ts` 封装 Tauri `invoke()` 和 `listen()`
- [x] `src/components/ui/scroll-area.tsx` 补齐项目内缺失的 ScrollArea 基础组件

## 关键发现
- 当前 shadcn/ui Nova 主题已在 `src/index.css` 配好 CSS 变量，但实际落地组件只有 `Button`，没有 Card 或 ScrollArea。
- 侧边栏宽度按任务要求抽为 CSS 变量：`--sidebar-width-collapsed: 4rem` 与 `--sidebar-width-expanded: 12.5rem`。
- 项目尚未引入 React Router，因此本轮按要求使用 React state + 条件渲染，避免新增路由依赖。
- Tauri 窗口尺寸为 1280x800，最小 960x600，当前布局使用固定侧边栏 + `flex-1` 主区可适配该尺寸。

## 2025-05-06: Tauri Commands + Events Layer

### 实现内容
- 创建 `src-tauri/src/commands.rs`，暴露 6 个 Tauri commands
- `lib.rs` 中注册全局状态 `AppState`（包含 ProjectRegistry、watchers、AgentCollector）
- 所有 commands 通过 `State<AppState>` 访问共享状态

### 命令列表
| Command | 功能 |
|---------|------|
| `add_project` | 添加项目到注册表，同时注册到 AgentCollector |
| `remove_project` | 移除项目，同时注销 AgentCollector 并停止监听 |
| `list_projects` | 返回按字母序排列的项目列表 |
| `get_project_data` | 一次性采集并返回 TemplateDataPayload |
| `start_watching` | 启动 WatchedCollector，通过 `template-update` event 推送变化 |
| `stop_watching` | 停止监听，清理 watcher 线程 |

### 事件设计
- `template-update`: 由 WatchedCollector 的后台线程 emit，payload 为 `TemplateDataPayload`
- `agent-update`: 已由 AgentCollector 直接 emit（Task 1.3），lib.rs 中确认集成

### 序列化方案
Collector 内部类型（Stage, ProjectConfig, MemoryEntry, GitStatus, SourceLayout）不直接实现 Serialize，而是在 commands.rs 中创建对应的 Ser* 包装类型，通过 `From` trait 转换。这样避免修改 collector 逻辑，同时保证所有返回值 JSON 可序列化。

### 关键决策
1. **错误处理**: 所有命令返回 `Result<T, String>`，错误信息 human-readable
2. **锁管理**: `AppState` 内部分别使用 `Mutex<ProjectRegistry>` 和 `Mutex<HashMap>`，避免长时间持有锁
3. **AgentCollector 同步**: add/remove 时同步注册/注销到 AgentCollector，确保 cwd 前缀匹配正确
4. **Watcher 线程命名**: 每个项目监听线程使用 `ptv-watcher-{path}` 命名，便于调试

### 编译验证
- `cargo check` 通过
- `cargo test` 74 tests 全部通过

---

# AgentMonitor 前端实现总结

## 完成状态
- [x] `src/pages/AgentMonitor.tsx` 替换占位页，接入 `listen("agent-update")` 实时快照。
- [x] 按 `projects[].agents` 分组展示项目卡片，`unmapped` 统一显示在“未关联”区域。
- [x] 每个 session 展示 `session_id`、`agent_type`、`token_rate`、上下文窗口和 Active / Idle / Offline 标签。
- [x] Token 速率支持 token/s 与 token/min 切换，使用 CSS 条形进度显示当前快照。
- [x] 上下文窗口使用率按 `context_percent` 与 `context_window` 推导 current/max，并显示进度条。
- [x] 前端每 2 秒刷新相对时间，与后端 AgentCollector 轮询节奏一致。

## 关键发现
- AgentCollector 推送的 `status` 原始值为 `Thinking | Executing | Waiting | RateLimited | Done`，前端面向用户映射为 `Active | Idle | Offline`：Waiting → Idle，Done → Offline，其余视为 Active。
- `context_window` 是最大窗口，当前占用需要结合 `context_percent` 推导；若窗口为 0，则降级显示 `total_input_tokens`。
- 项目主题已使用 shadcn Nova CSS 变量和 stage 色阶，AgentMonitor 继续使用 `Card`、`Button`、`bg-muted`、`stage-l*` 等既有 token，未新增图表库。

## 验证
- `lsp_diagnostics`：`src/pages/AgentMonitor.tsx` 无诊断。
- `npm run dev -- --host 127.0.0.1`：Vite dev server 启动成功。
- `npm run build`：前端构建通过。

---

# Task 3.0: ProjectDetail 项目详情面板实现总结

## 完成状态
- [x] `src/pages/ProjectDetail.tsx` 替换占位，实现 Stage 时间线、参数快照、Memory、Git 四个只读面板
- [x] 组件通过 `projectPath` props 接收项目路径，空路径显示友好空状态
- [x] 使用 `useTauri().invoke("get_project_data", { path })` 获取 `TemplateDataPayload`
- [x] 使用 `useTauri().listen("template-update")` 接收实时更新，并按 `project_path` 匹配当前项目
- [x] 调用 `start_watching` / `stop_watching` 管理当前项目监听生命周期
- [x] 路径不存在时将后端错误转换为中文友好提示
- [x] 验证：`lsp_diagnostics` 清洁，`npm run build` 通过，`npm run dev` 可启动 Vite

## 关键发现
- `TemplateDataPayload` 中 `config` 和 `stage` 均为可空值，并分别带有 `*_error` 字段；前端应按面板局部降级展示，而不是让整个详情页失败。
- `SerMemoryEntry` 已在后端将正文截断为 `content_preview`，前端无需再限制条目数量；Memory 面板只限制视口滚动，不截断 entries。
- `template-update` 事件 payload 的 `project_path` 可能来自 watcher 的规范化路径，因此前端至少需要去除尾部斜杠后再比较。
- Vite `npm run dev` 是长驻进程；本轮通过短超时确认服务输出 `ready` 后终止。

## Review 修复
- 修复 `App.tsx` 中 `currentProjectPath` 固定为空的问题：改为 `useState` 保存选中项目路径，并允许 `handleRouteChange(route, projectPath?)` 接收 Dashboard 传入的路径。
- 修复 `Dashboard.tsx` 项目卡片点击只切换路由、不传项目路径的问题：`ProjectCard.onOpen` 现在调用 `onRouteChange("projects", project.path)`。
- 优化 `ProjectDetail.tsx` 实时监听顺序：先注册 `template-update` listener，再启动 `start_watching`，降低初始事件丢失风险。
- 优化 `ProjectDetail.tsx` 细节：图标类型改为通用 `DetailIcon`，`template-update` 收到数据时停止 loading，顶层错误且无数据时不再重复渲染子面板 notice，`formatTimestamp` 使用 `timestampMs == null` 判空。
- 修复后验证：`lsp_diagnostics` 对 `ProjectDetail.tsx`、`App.tsx`、`Dashboard.tsx` 均无诊断；`npm run build` 通过。

---

## 2026-05-06: Dashboard 项目列表实现

### 实现内容
- `Dashboard` 通过 `useTauri().invoke("list_projects")` 获取注册项目，再用 `get_project_data` 补齐 Stage、Git、项目名快照。
- `listen("agent-update")` 监听实时事件，按 `project_path` 更新每个项目的活跃 Agent 数。
- 项目卡片按展示名使用 `Intl.Collator("zh-CN", { numeric: true })` 字母序排序。
- 空状态提供“添加项目”按钮，跳转到 Settings；项目卡片点击/键盘 Enter/Space 跳转到 ProjectDetail 路由。

### UI 约定
- 新增 `src/components/ui/card.tsx` 作为 shadcn 风格 Card 基础组件，沿用 `border-border`、`bg-card`、`text-card-foreground`、`shadow-sm` 等现有 token。
- Stage 标签和顶部进度条使用现有 CSS 变量 token（chart/sidebar/destructive 等）组合渐变，避免新增依赖和硬编码色值。

### 验证
- `lsp_diagnostics`：`Dashboard.tsx`、`App.tsx`、`components/ui/card.tsx` 均无诊断。
- `npm run build` 通过。
- `npm run dev -- --host 127.0.0.1` 成功启动 Vite（超时停止前显示 ready）。

---

## 2026-05-06: Settings 项目配置面板实现

### 实现内容
- `src/pages/Settings.tsx` 替换占位页，使用 `useTauri().invoke()` 接入 `list_projects`、`add_project`、`remove_project`。
- 添加项目表单使用本地 `Input` + `Button`，前端先校验非空、macOS/Linux 绝对路径、不能为根目录。
- 添加成功后清空输入并刷新列表；失败时按后端错误文案区分“路径不存在/无法访问”“已注册”“添加失败”。
- 已注册项目列表展示规范化路径、添加时间和移除按钮；无项目时显示引导空状态。
- 移除项目使用 Base UI 封装的 shadcn 风格 `Dialog` 确认框，确认后调用 `remove_project(path)` 并刷新列表。

### 关键发现
- 当前项目实际只有 `Button`、`Card`、`ScrollArea` 基础 UI 组件；本轮补齐 `src/components/ui/input.tsx` 和 `src/components/ui/dialog.tsx`，沿用 shadcn Nova token。
- `add_project` 后端签名为 `{ path: string } -> ProjectEntry`，`remove_project` 为 `{ path: string } -> void`，`list_projects` 无参数返回 `ProjectEntry[]`。
- `ProjectRegistry` 通过 `canonicalize()` 去重，重复路径错误为 `项目已存在: ...`；不存在/无权限路径会返回 `路径规范化失败: ...`。

### 验证
- `lsp_diagnostics`：`Settings.tsx`、`components/ui/input.tsx`、`components/ui/dialog.tsx` 均无诊断。
- `npm run build` 通过。
- `npm run dev -- --host 127.0.0.1` 成功启动 Vite（显示 ready 后终止）。
