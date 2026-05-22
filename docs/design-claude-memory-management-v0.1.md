# AgentScope — Claude Code 记忆管理 v0.1 技术设计

> 状态：技术设计草案  
> 创建日期：2026-05-21  
> 关联需求：`docs/requirements-claude-memory-management.md`（v0.1）  
> 目标范围：只读扫描器（本机三端兼容），无编辑、无加载链模拟、无同步

---

## 1. 当前实现约束

### 1.1 技术栈约束

| 层级 | 约束 |
|------|------|
| 后端 | Tauri v2（Rust edition 2021），无 SQLite / rusqlite / sqlx / YAML 库，持久化仅用 JSON 文件；**v0.1 仅新增 `regex = "1"`**（本地敏感信息扫描） |
| 前端 | React 19 + TypeScript，双层路由（domain + page），无全局状态库 |
| 构建 | Linux Docker + Windows Shell + macOS 本机三平台 CI |

### 1.2 已有架构模式（复用约束）

**后端分层**：`routes/*.rs` → `services/*.rs` → `collectors/<domain>/`

- `routes/`：Tauri command 入口，只做参数透传和 `State<'_, AppState>` 提取
- `services/`：业务逻辑编排，调用 collector 并处理错误
- `collectors/<domain>/`：文件系统扫描、数据解析、模型构造

**持久化**：`ProjectRegistry` 使用 JSON 文件（`{data_local_dir}/agent-scope/projects.json`），v0.1 不复用 SQLite（需求文档第 7 章的 SQL schema 为 v0.3+ 预留）。

**状态管理**：`AppState` 结构体通过 Tauri `.manage()` 注入，包含：

```rust
pub struct AppState {
    pub registry: Mutex<ProjectRegistry>,
    pub watchers: Mutex<HashMap<String, Arc<AtomicBool>>>,
    pub agent_collector: Mutex<AgentCollector>,
    pub template_path: Mutex<Option<PathBuf>>,
    pub template_fingerprint: Mutex<Option<TemplateFingerprintCache>>,
    // v0.1 不新增字段：扫描为实时执行，无服务端缓存
}
```

**前端 API 模式**：`src/lib/api.ts` 中封装 `invoke` 调用，按功能域组织（已有 `claudeHistoryApi`）。

### 1.3 现有代码的复用与局限

| 现有模块 | 复用方式 | 局限 |
|---------|---------|------|
| `collectors/claude_history/path_codec.rs` | 参考 `claude_config_dir()` 实现，但需重写 | 当前仅回退 `dirs::home_dir().join(".claude")`，**不读取 `CLAUDE_CONFIG_DIR`** |
| `collectors/template/project_files.rs` | 参考扫描逻辑（`MAX_FILE_SIZE`、错误跳过） | 白名单硬编码模板项目路径，不适用于通用记忆 |
| `components/MemoryFileTree.tsx` | 不直接复用，参考分组展示模式 | 按 `source_group` 分组，v0.1 需要按 scope 分组 |
| `components/MarkdownRenderer.tsx` | **直接复用** | 无局限 |
| `components/ProjectMemoryPanel.tsx` | 参考 Tab 切换布局 | 模板项目专属，不通用 |

### 1.4 v0.1 边界

- **不做**：编辑/创建/删除、加载链模拟、@import 解析、Settings 可视化、跨设备同步
- **只做**：扫描发现、查看列表、查看内容、统计元数据（行数/大小/mtime）、frontmatter 解析、敏感信息检测

---

## 2. 后端模块设计

### 2.1 模块结构

```
src-tauri/src/
├── collectors/claude_memory/
│   ├── mod.rs              # 模块入口，导出 public API
│   ├── path_resolver.rs    # 路径解析（CLAUDE_CONFIG_DIR 优先）
│   ├── scanner.rs          # 文件系统扫描器
│   ├── frontmatter.rs      # 轻量 frontmatter 解析器
│   ├── secret_scanner.rs   # 敏感信息扫描器
│   └── models.rs           # 数据模型（SerClaudeMemoryScanResult 等）
├── services/
│   └── claude_memory_service.rs  # 业务编排层
└── routes/
    └── claude_memory.rs    # Tauri 命令入口
```

### 2.2 各模块职责

#### `collectors/claude_memory/mod.rs`

- 导出 `scan_claude_memory()` 入口函数
- 协调 path_resolver → scanner → frontmatter → secret_scanner 的调用链
- 提供 `get_file_content()` 单文件读取（带 allowlist 校验）

#### `collectors/claude_memory/path_resolver.rs`

- 解析 `CLAUDE_CONFIG_DIR` 环境变量
- 生成各类型记忆的标准 `PathBuf`
- 提供 `resolve_claude_config_dir()` 和各类型的 `resolve_*_path()` 函数

#### `collectors/claude_memory/scanner.rs`

- 遍历文件系统，发现记忆资产
- 按 scope 分类（user / project / local / auto）
- v0.1 不计算 content_hash（留空 None），以 mtime_ms + byte_size 作为轻量 fingerprint
- 错误跳过策略：单个文件不可读不中断整体扫描

#### `collectors/claude_memory/frontmatter.rs`

- 解析 YAML frontmatter（轻量实现，无外部 YAML 依赖）
- 提取 `name`, `description`, `trigger`, `paths`, `memory_scope` 等字段
- 缺失字段回退策略

#### `collectors/claude_memory/secret_scanner.rs`

- 本地正则匹配，检测 API key、token、密码等模式
- 返回命中位置和类型，不修改文件

#### `collectors/claude_memory/models.rs`

- 所有数据结构定义，derive `Debug, Clone, Serialize`

### 2.3 与现有代码的关系

```
lib.rs
  ├── 新增 route: claude_memory.rs（2 个命令）
  ├── 新增 service: claude_memory_service.rs
  └── AppState 不新增字段（v0.1 实时扫描，无缓存）

collectors/
  ├── claude_history/        # 已有，v0.1 不改
  ├── template/              # 已有，v0.1 不改
  └── claude_memory/         # 新增 v0.1 模块
```

---

## 3. Path Resolver 设计

### 3.1 设计原则

1. **环境变量优先**：`CLAUDE_CONFIG_DIR` 存在时，所有 `~/.claude` 路径替换为该目录
2. **dirs crate 回退**：未设置环境变量时使用 `dirs::home_dir()`
3. **PathBuf 全程**：禁止字符串拼接路径，使用 `PathBuf::join()`
4. **平台无关**：不硬编码 `/` 或 `\`，依赖 `std::path` 自动处理

### 3.2 核心函数

```rust
/// 解析 Claude Code 配置根目录
/// 优先级：CLAUDE_CONFIG_DIR > dirs::home_dir().join(".claude")
pub fn resolve_claude_config_dir() -> Result<PathBuf, String>;

/// 解析用户级路径（基于 claude_config_dir）
pub fn resolve_user_claude_md() -> Result<PathBuf, String>;
pub fn resolve_user_rules_dir() -> Result<PathBuf, String>;
pub fn resolve_user_skills_dir() -> Result<PathBuf, String>;
pub fn resolve_user_agents_dir() -> Result<PathBuf, String>;
pub fn resolve_auto_memory_dir() -> Result<PathBuf, String>;

/// 解析项目级路径（基于给定项目根目录）
pub fn resolve_project_claude_md(project_root: &Path) -> PathBuf;
pub fn resolve_project_rules_dir(project_root: &Path) -> PathBuf;
pub fn resolve_project_local_md(project_root: &Path) -> PathBuf;
pub fn resolve_project_skills_dir(project_root: &Path) -> PathBuf;
pub fn resolve_project_agents_dir(project_root: &Path) -> PathBuf;

/// 解析组织级 managed 路径（v0.1 扫描但不显示内容，仅记录存在性）
pub fn resolve_managed_dir() -> Option<PathBuf>;
```

### 3.3 与现有 `path_codec.rs` 的区别

| 方面 | 现有 `path_codec.rs` | v0.1 Path Resolver |
|------|---------------------|-------------------|
| `CLAUDE_CONFIG_DIR` | **不读取** | **优先读取** |
| 用途 | claude_history 专用 | 通用记忆管理 |
| 函数粒度 | 单一 `claude_config_dir()` | 按类型细分的 resolve 函数族 |
| 返回值 | `Option<PathBuf>` | `Result<PathBuf, String>`（带错误信息） |

### 3.4 跨平台处理

- **Windows**：`dirs::home_dir()` 返回 `C:\Users\<user>`，join `.claude` 得到标准路径
- **WSL**：视为独立运行时，各自有独立的 `home_dir()` 和 `.claude` 目录
- **`\\?\` 前缀**：扫描结果返回前自动剥离（复用 registry.rs 的 strip_prefix 逻辑）

---

## 4. Memory Scanner 设计

### 4.1 数据模型

```rust
// models.rs —— 所有结构体 derive Debug, Clone, Serialize

/// 单次扫描结果（v0.1 实时生成，无服务端缓存）
#[derive(Debug, Clone, Serialize)]
pub struct SerClaudeMemoryScanResult {
    pub scanned_at_ms: u64,           // Unix epoch milliseconds（std::time::SystemTime）
    pub host_profile: SerHostProfile,
    pub assets: Vec<SerClaudeMemoryAsset>,
    pub summary: SerMemorySummary,
    pub errors: Vec<SerMemoryScanError>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SerHostProfile {
    pub host_id: String,              // v0.1 轻量 hash(os + home_dir + user_name + hostname)
    pub hostname: String,             // 环境变量：Windows=COMPUTERNAME，macOS/Linux=HOSTNAME，取不到则 "unknown"
    pub os: String,                   // "macos" | "linux" | "windows"
    pub home_dir: String,
    pub claude_config_dir: String,
    pub user_name: String,            // 环境变量：USER / USERNAME，取不到则 "unknown"
}

#[derive(Debug, Clone, Serialize)]
pub struct SerClaudeMemoryAsset {
    pub id: String,                   // hash(native_path) 作为唯一标识
    pub scope: String,                // "user" | "project" | "local" | "auto"
    pub asset_type: String,           // 见 asset_type 枚举（v0.1 实际扫描范围见 §4.2）
    pub logical_path: String,         // 跨平台逻辑路径（统一 / 分隔符）
    pub native_path: String,          // 本机实际路径
    pub content_hash: Option<String>, // v0.1 留空 None，v0.3 引入持久化后再计算稳定 hash
    pub content_preview: Option<String>, // 前 2KB 文本预览（.md 文件），overview 接口专用
    pub content_truncated: bool,      // preview 是否因文件过大（> 1 MiB）被截断
    pub line_count: Option<usize>,
    pub byte_size: Option<u64>,
    pub mtime_ms: Option<u64>,
    pub frontmatter: Option<SerFrontmatter>,
    pub secret_issues: Vec<SerSecretIssue>,
    pub exists: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct SerFrontmatter {
    pub name: Option<String>,
    pub description: Option<String>,
    pub trigger: Option<String>,
    pub paths: Option<Vec<String>>,   // rules 的 paths 触发条件
    pub memory_scope: Option<String>, // "user" | "project" | "local"
    pub raw: String,                  // 完整原始 frontmatter 文本
}

#[derive(Debug, Clone, Serialize)]
pub struct SerSecretIssue {
    pub issue_type: String,           // "api_key" | "token" | "password" | "private_url"
    pub line_number: usize,
    pub column_start: usize,
    pub column_end: usize,
    pub matched_text: String,         // 脱敏后的匹配文本（如 "sk-***"）
}

#[derive(Debug, Clone, Serialize)]
pub struct SerMemorySummary {
    pub total_assets: usize,
    pub total_existing: usize,
    pub by_scope: HashMap<String, usize>,
    pub by_type: HashMap<String, usize>,
    pub total_secret_issues: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct SerMemoryScanError {
    pub scope: String,          // "user" | "project" | "auto"
    pub path: String,
    pub message: String,
}

/// v0.1 实际扫描的 asset_type（§4.2）
/// "user_claude_md" | "project_claude_md" | "project_dot_claude_md" | "local_md"
/// "global_rule" | "project_rule" | "global_skill" | "project_skill"
/// "global_agent" | "project_agent" | "auto_memory_index" | "auto_memory_topic"
///
/// v0.1 不扫描但预留的 asset_type（后续版本实现）：
/// "settings" | "managed_claude_md" | "transcript" | "history"
```

### 4.2 扫描范围（v0.1）

| 类型 | scope | asset_type | 扫描路径 | 说明 |
|------|-------|-----------|---------|------|
| 用户全局 CLAUDE.md | user | `user_claude_md` | `~/.claude/CLAUDE.md` | 存在即扫描 |
| 项目级 CLAUDE.md | project | `project_claude_md` | `<repo>/CLAUDE.md` | 仅限已注册项目 |
| 项目级 .claude/CLAUDE.md | project | `project_dot_claude_md` | `<repo>/.claude/CLAUDE.md` | 仅限已注册项目 |
| 本地私有 CLAUDE.local.md | local | `local_md` | `<repo>/CLAUDE.local.md` | 仅限已注册项目 |
| 全局 rules | user | `global_rule` | `~/.claude/rules/*.md` | 所有 `.md` 文件 |
| 项目级 rules | project | `project_rule` | `<repo>/.claude/rules/*.md` | 仅限已注册项目 |
| 全局 skills | user | `global_skill` | `~/.claude/skills/*/SKILL.md` | 只扫描 SKILL.md |
| 项目级 skills | project | `project_skill` | `<repo>/.claude/skills/*/SKILL.md` | 只扫描 SKILL.md |
| 全局 agents | user | `global_agent` | `~/.claude/agents/*.md` | 所有 `.md` 文件 |
| 项目级 agents | project | `project_agent` | `<repo>/.claude/agents/*.md` | 仅限已注册项目 |
| Auto Memory | auto | `auto_memory_index` | `~/.claude/projects/<id>/memory/MEMORY.md` | 存在即扫描 |
| Auto Memory topics | auto | `auto_memory_topic` | `~/.claude/projects/<id>/memory/*.md` | 排除 MEMORY.md |

> 注：v0.1 中"项目级"记忆仅扫描已注册项目（`ProjectRegistry` 中的项目）。未注册项目不扫描。

### 4.3 扫描流程

```
get_claude_memory_overview_service(project_path, force, state)
  ├── 1. 构建 HostProfile
  ├── 2. 解析路径（Path Resolver）
  ├── 3. 扫描用户级资产（~/.claude/）
  │     ├── 读取目录 → 遍历文件
  │     ├── 对每个文件：先检查 metadata.len()
  │     │   ├── 若 > MAX_FILE_SIZE（1 MiB）：只读前 2KB preview，truncated=true
  │     │   └── 若 <= MAX_FILE_SIZE：正常 read_to_string
  │     ├── 解析 frontmatter（基于实际读取内容）
  │     └── 运行 secret_scanner（基于实际读取内容）
  ├── 4. 扫描 Auto Memory（~/.claude/projects/*/memory/）
  │     └── projects 目录不存在时静默返回（空状态，不记录 error）
  ├── 5. 若 project_path = Some(path)
  │     ├── 校验 path 存在且是目录
  │     ├── canonicalize path 并加入 scanned_paths HashSet
  │     ├── 扫描该目录下的项目级资产（同第 3 步逻辑）
  │     └── 若不可访问：在 scan result 的 project-level errors 中记录，不中断整体扫描
  ├── 6. 遍历已注册项目列表
  │     └── 对每个项目：canonicalize 路径
  │         ├── 若已在 scanned_paths 中：跳过（去重）
  │         └── 否则：扫描项目级资产，加入 scanned_paths
  ├── 7. 汇总统计
  └── 8. 返回 SerClaudeMemoryScanResult
```

**project_path 参数说明**：
- `None`：扫描用户级配置目录 + ProjectRegistry 中所有已注册项目
- `Some(path)`：在 None 的基础上，额外扫描指定目录作为项目级资产来源
- 指定目录不要求已注册，但必须存在且是目录；不可访问时记录错误但不中断整体扫描
- **project_path 与 registry 去重**：`project_path` 和 registry 中的项目路径均 canonicalize 后比对，已扫描的 canonical 路径放入 HashSet，避免同一目录被扫描两次
- **理由**：返回 `Ok` 并记录错误，而非直接 `Err`，确保用户仍能看到其他可用资产，符合 v0.1 "扫描过程不因单个不可读文件中断整体扫描" 的约束

### 4.4 错误处理

**单文件级别**（不中断整体扫描）：
- **文件不存在**：标记 `exists: false`，其他字段为 `None`
- **权限不足**：跳过该文件，记录 `exists: false`，继续扫描
- **文件过大**：超过 `MAX_FILE_SIZE`（1 MiB）时截断读取 `content_preview`，标记 `content_truncated: true`
- **损坏文件 / 无法读取**：跳过，记录 `exists: false`，继续扫描

**目录/项目级别**（仍不中断整体扫描，但记录到 `errors` 字段）：
- **指定 project_path 不可访问**：在 `SerClaudeMemoryScanResult.errors` 中记录 `SerMemoryScanError`
- **用户级配置目录不可访问**：同样在 `errors` 中记录
- **已注册项目路径不存在或不可访问**：在 `errors` 中记录，但继续扫描其他项目

**Auto Memory 空状态（特殊处理）**：
- **`~/.claude/projects/` 目录不存在**：静默返回，不产生任何 `SerMemoryScanError`，result 中 `assets` 列表不包含 auto memory 资产
- **理由**：Auto Memory 是用户会话触发的自动产物，并非所有用户都会创建。目录不存在是正常状态，不应显示为错误。前端在 auto memory 分组中显示"无记忆文件"或完全隐藏该分组即可

**前端展示**：`errors` 列表在 OverviewStats 中以警告卡片形式展示，不阻断用户查看可用资产。

### 4.5 性能目标

- 扫描整台主机全部记忆资产 ≤ 3 秒（普通 SSD）
- v0.1 不做服务端缓存，每次调用命令实时扫描，保证结果始终与文件系统一致

**大文件扫描策略**：
- **阈值**：`MAX_FILE_SIZE = 1_048_576` bytes（1 MiB）
- **大于阈值的文件**：仅读取前 2KB 作为 `content_preview`，`content_truncated` 标记为 `true`，`line_count` 设为 `None`，不计算完整 content_hash
- **实现方式**：使用 `std::io::Read::read()` 直接读取前 2KB 到缓冲区，在 UTF-8 字符边界安全截断，避免 `read_to_string` 整文件加载
- **理由**：记忆文件（CLAUDE.md、rules、skills）通常为文本文件，正常大小在 KB 级别；1 MiB 以上多为异常情况（日志堆积、误存二进制）。读取 preview 足以完成 frontmatter 解析和 secret scanner 检测，同时避免内存峰值和扫描延迟

---

## 5. Frontmatter 策略

### 5.1 解析范围

仅解析 `.md` 文件。扫描器读取文件前 2KB，提取 `---` 包围的 YAML frontmatter。

### 5.2 轻量解析器（无 YAML 依赖）

不引入 `serde_yaml` 等重型依赖。使用基于行的简单解析器：

```rust
/// 提取文件中的 frontmatter
/// 返回 (frontmatter_text, content_offset)
pub fn extract_frontmatter(content: &str) -> Option<(&str, usize)>;

/// 解析 frontmatter 文本为键值对
/// 支持 "key: value" 和 "key:\n  - item1\n  - item2" 两种形式
pub fn parse_frontmatter(raw: &str) -> HashMap<String, String>;
```

**解析规则**：
- 只解析一级键值对（不支持嵌套）
- `paths` 字段按行分割为数组
- 遇到不认识的字段直接忽略

### 5.3 字段映射

| 文件类型 | 关注的字段 | 缺失回退 |
|---------|-----------|---------|
| Skills (`SKILL.md`) | `name`, `description`, `trigger`, `memory_scope` | `name` → 目录名；其他 → `None` |
| Agents (`*.md`) | `name`, `description`, `memory_scope` | `name` → 文件名（不含扩展名）；其他 → `None` |
| Rules (`*.md`) | `paths`, `description` | `paths` → `None`（表示全局加载）；其他 → `None` |

### 5.4 示例

```markdown
---
name: git-commit-helper
description: 帮助生成符合规范的 git commit message
trigger: "用户要求生成 commit message"
memory_scope: project
---

# Git Commit Helper
...
```

解析结果：
- `name`: `"git-commit-helper"`
- `description`: `"帮助生成符合规范的 git commit message"`
- `trigger`: `"用户要求生成 commit message"`
- `memory_scope`: `"project"`
- `paths`: `None`

---

## 6. Secret Scanner 策略

### 6.1 设计原则

- **纯本地**：所有正则匹配在 Rust 后端执行，不发送任何内容到外部服务
- **只读检测**：v0.1 仅检测并报告，不修改文件
- **误报容忍**：宁可误报，不可漏报；提供 issue 级别分类

**v0.1 依赖决策**：允许新增 `regex = "1"` 依赖。理由：
- 是实现 secret scanner 的直接需求，不涉及 SQLite/YAML/hash 等重型架构依赖
- 仍保持纯本地扫描，不发送任何内容到外部服务
- regex crate 是 Rust 生态标准库，构建开销可控

### 6.2 检测模式

```rust
const SECRET_PATTERNS: &[(&str, &str)] = &[
    ("api_key", r"(?i)(api[_-]?key|apikey)\s*[:=]\s*[\"']?([a-zA-Z0-9_-]{16,})[\"']?"),
    ("token", r"(?i)(token|bearer)\s*[:=]\s*[\"']?([a-zA-Z0-9_-]{16,})[\"']?"),
    ("password", r"(?i)(password|passwd|pwd)\s*[:=]\s*[\"']?([^\"'\s]{8,})[\"']?"),
    ("private_url", r"(?i)(https?://[^:]+:[^@]+@[^\s]+)"),
    ("env_content", r"(?i)(DATABASE_URL|SECRET_KEY|PRIVATE_KEY|AWS_ACCESS_KEY)\s*[:=]\s*[\"']?([^\"'\n]+)"),
];
```

### 6.3 脱敏策略

匹配结果中 `matched_text` 字段进行脱敏：
- API key / token：保留前 4 字符，其余替换为 `***`
- Password：全部替换为 `***`
- URL：保留协议和域名，凭据部分替换为 `***`

### 6.4 输出级别

| 级别 | 说明 | 前端展示 |
|------|------|---------|
| `warning` | 疑似敏感信息（正则命中） | 黄色警告图标 + 行号提示 |
| `info` | 仅为参考（如 `.env` 文件名出现） | 灰色提示 |

### 6.5 扩展预留

v0.2+ 可增加：
- `.claudeignore` 风格的忽略规则
- 用户手动标记误报
- entropy-based 检测（高随机性字符串）

---

## 7. 持久化策略决策

### 7.1 方案对比

| 方案 | 实现 | 优点 | 缺点 |
|------|------|------|------|
| **A：实时扫描（推荐 v0.1）** | 每次调用命令时扫描文件系统 | 零持久化代码、始终最新、无数据不一致 | 扫描耗时（目标 3s 内） |
| **B：JSON 缓存** | 扫描结果序列化为 JSON，按 host_id 存储 | 快速读取、可离线查看 | 需要缓存失效策略、增加代码复杂度 |
| **C：SQLite（需求文档 schema）** | 按第 7 章 SQL schema 建表 | 支持复杂查询、为 v0.3 同步做准备 | 需引入 rusqlite/sqlx，改变现有架构 |

### 7.2 v0.1 决策：方案 A（实时扫描）

**理由**：
1. 现有代码库无 SQLite 依赖，引入会改变架构并增加构建复杂度
2. v0.1 只读场景下，扫描结果不需要长期持久化
3. 记忆文件数量预期在数百个以内，SSD 上 3 秒扫描目标可达
4. 减少 v0.1 代码量，聚焦核心扫描逻辑

**无缓存策略（v0.1）**：
- 每次调用命令实时扫描文件系统，不保留任何服务端缓存
- 前端通过 `force: true` 参数语义保留（向后兼容），但后端无缓存可刷新，始终执行完整扫描
- 如未来引入缓存，key 必须包含 `claude_config_dir + project_path + registered_project_count` 等状态维度，避免跨环境缓存污染

### 7.3 v0.3 迁移点

当实现跨设备同步时，必须引入 SQLite（或类似持久化）：
- `SerClaudeMemoryScanResult` 的数据结构可直接映射到需求文档第 7 章的 SQL schema
- `host_id` 字段已预留，v0.1 只生成单条本机记录
- 迁移路径：v0.1 实时扫描 → v0.2 仍实时扫描 + 可选缓存 → v0.3 SQLite 持久化 + 后台同步任务

---

## 8. Tauri Command 设计

### 8.1 v0.1 最小命令集

仅 2 个命令，覆盖全部只读需求：

```rust
/// 扫描 Claude Code 记忆资产
///
/// 参数：
///   - project_path: Option<String> — 额外扫描的指定项目目录（None = 仅扫用户级 + 已注册项目）
///   - force: bool — 是否强制重新扫描（保留向后兼容语义，v0.1 始终实时扫描）
/// 返回：SerClaudeMemoryScanResult
#[tauri::command(rename = "get_claude_memory_overview")]
pub fn get_claude_memory_overview_cmd(
    project_path: Option<String>,
    force: bool,
    state: State<'_, AppState>,
) -> Result<SerClaudeMemoryScanResult, String>;

/// 读取指定记忆文件的内容
///
/// 参数：
///   - native_path: String — 文件的本机绝对路径
///   - project_path: Option<String> — 本次扫描时指定的额外项目目录（允许该目录下的文件读取）
/// 返回：文件内容字符串
/// 
/// 安全校验：
///   1. 路径必须是绝对路径（拒绝相对路径）
///   2. 路径必须在 allowlist 内：
///      - canonicalized Claude config dir 下
///      - 已注册项目路径下
///      - 若 project_path = Some(path) 且存在/可 canonicalize，该目录下也允许
///   3. 文件扩展名必须是 .md 或 .json（v0.1 限制）
///   4. 拒绝符号链接逃逸（native_path 和 allowlist root 均 canonicalize 后比对）
#[tauri::command(rename = "get_claude_memory_file_content")]
pub fn get_claude_memory_file_content_cmd(
    native_path: String,
    project_path: Option<String>,
    state: State<'_, AppState>,
) -> Result<String, String>;
```

### 8.2 Allowlist 校验（`get_claude_memory_file_content`）

**安全校验步骤**（必须全部通过）：

1. **必须是绝对路径**：拒绝相对路径（如 `../../etc/passwd`）
2. **扩展名限制**：仅允许 `.md`、`.json`（v0.1），拒绝其他扩展名
3. **canonicalize 比对**（防符号链接逃逸）：
   - 对待读取文件执行 `std::fs::canonicalize`
   - 对 allowlist 根目录（`~/.claude/` 或已注册项目路径）也执行 `canonicalize`
   - 使用 `canonicalized_file.starts_with(canonicalized_root)` 判断归属
   - 若 `canonicalize` 失败（文件不存在、权限不足、broken symlink），返回友好错误
4. **展示与校验分离**：前端展示使用用户传入的原始 `native_path`，后端安全校验使用 `canonicalize` 后的路径

```rust
fn is_allowed_path(
    native_path: &Path,
    project_path: Option<&str>,
    state: &AppState,
) -> Result<bool, String> {
    // 1. 必须是绝对路径
    if !native_path.is_absolute() {
        return Ok(false);
    }

    // 2. 扩展名校验
    let ext = native_path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    if !matches!(ext, "md" | "json") {
        return Ok(false);
    }

    // 3. canonicalize 待读取文件
    let canonical_file = std::fs::canonicalize(native_path)
        .map_err(|e| format!("无法解析路径: {}", e))?;

    // 4. 检查是否在 ~/.claude/ 下（canonicalize 后比对）
    if let Ok(claude_dir) = resolve_claude_config_dir() {
        if let Ok(canonical_claude) = std::fs::canonicalize(&claude_dir) {
            if canonical_file.starts_with(&canonical_claude) {
                return Ok(true);
            }
        }
    }

    // 5. 检查是否在已注册项目路径下
    // 依据：现有 ProjectRegistry 暴露的是 registry.list()（见 src-tauri/src/registry.rs）
    let registry = state.registry.lock().unwrap();
    for project in registry.list() {
        let project_path = Path::new(&project.path);
        if let Ok(canonical_project) = std::fs::canonicalize(project_path) {
            if canonical_file.starts_with(&canonical_project) {
                return Ok(true);
            }
        }
    }
    drop(registry); // 显式释放锁

    // 6. 检查是否在额外指定的 project_path 下
    if let Some(extra_path) = project_path {
        let extra = Path::new(extra_path);
        if extra.exists() && extra.is_dir() {
            if let Ok(canonical_extra) = std::fs::canonicalize(extra) {
                if canonical_file.starts_with(&canonical_extra) {
                    return Ok(true);
                }
            }
        }
    }

    Ok(false)
}
```

**符号链接逃逸防护说明**：
- 若 `~/.claude/xxx` 是指向 `/etc` 的 symlink，`canonicalize` 后路径变为 `/etc/...`，不会 `starts_with` 真实的 `~/.claude/` 目录
- 若已注册项目路径本身包含 symlink，`canonicalize` 后得到真实路径，确保被引用的文件确实位于该项目目录内

### 8.3 错误返回规范

| 场景 | 返回 |
|------|------|
| 路径不在 allowlist | `Err("路径不在允许范围内".to_string())` |
| 文件不存在 | `Err("文件不存在".to_string())` |
| 权限不足 | `Err("权限不足，无法读取文件".to_string())` |
| 文件过大（> 1 MiB） | `Err("文件过大，无法读取".to_string())` |
| 扫描过程中部分文件失败 | 在 `SerClaudeMemoryScanResult` 中标记 `exists: false`（单文件级别），或记录到 `errors` 字段（目录/项目级别），整体返回 `Ok` |

### 8.4 注册位置

在 `lib.rs` 的 `tauri::generate_handler!` 宏中新增两个命令：

```rust
tauri::generate_handler![
    // ... 现有命令 ...
    get_claude_memory_overview,
    get_claude_memory_file_content,
]
```

---

## 9. 前端集成设计

### 9.1 路由扩展

**`src/App.tsx`**：

Claude 记忆已提升为顶部一级域，与项目监控、通用监控、设置并列：

```typescript
export type AppDomain = "projects" | "monitoring" | "claude-memory" | "settings";
```

设置域不再承载 Claude 记忆入口。`SettingsPage` 仅保留：

```typescript
export type SettingsPage = "project" | "general";
```

**`src/components/Sidebar.tsx`**：

Claude 记忆域有独立的侧边栏子导航：

```typescript
// Claude 记忆域侧边栏
const items = [
  { id: "assets", label: "记忆资产", icon: Brain },
] as const;

// 设置域侧边栏（Claude 记忆已移除）
const items = [
  { id: "project", label: "项目设置", icon: FolderKanban },
  { id: "general", label: "通用设置", icon: Settings },
] as const;
```

> `Brain` 图标从 `lucide-react` 导入。

### 9.2 新增页面

**`src/features/claude-memory/index.tsx`**：

```typescript
// ClaudeMemory 主组件
// 内部子组件：
//   - InstructionTab: CLAUDE.md 系列
//   - RulesTab: rules 文件
//   - AutoMemoryTab: auto memory
//   - SkillsAgentsTab: skills 和 agents
//   - OverviewStats: 顶部统计卡片
```

组件结构：

```
ClaudeMemory
├── OverviewStats（统计卡片：总文件数、secret issues、按 scope 分布）
├── Tabs（Instruction / Rules / Auto Memory / Skills & Agents）
│   ├── InstructionTab
│   │   └── MemoryAssetTree（按 scope 分组：user → project → local）
│   ├── RulesTab
│   │   └── MemoryAssetTree（区分"全局加载"和"路径触发"）
│   ├── AutoMemoryTab
│   │   └── MemoryAssetTree（按 project 分组）
│   └── SkillsAgentsTab
│       └── MemoryAssetTree（按 type 分组：skill / agent）
└── FileDetailPanel（选中文件后展示内容）
    ├── MarkdownRenderer（.md 文件）
    └── SecretWarning（如有敏感信息 issue）
```

### 9.3 API 封装

**`src/lib/api.ts`** 新增：

```typescript
export interface ClaudeMemoryOverview {
  scanned_at_ms: number;
  host_profile: HostProfile;
  assets: ClaudeMemoryAsset[];
  summary: MemorySummary;
  errors: MemoryScanError[];
}

export interface HostProfile {
  host_id: string;
  hostname: string;
  os: string;
  home_dir: string;
  claude_config_dir: string;
  user_name: string;
}

export interface ClaudeMemoryAsset {
  id: string;
  scope: "user" | "project" | "local" | "auto";
  asset_type: string;
  logical_path: string;
  native_path: string;
  content_hash: string | null;
  content_preview: string | null;
  content_truncated: boolean;
  line_count: number | null;
  byte_size: number | null;
  mtime_ms: number | null;
  frontmatter: Frontmatter | null;
  secret_issues: SecretIssue[];
  exists: boolean;
}

export interface MemoryScanError {
  scope: string;
  path: string;
  message: string;
}

export const claudeMemoryApi = {
  getOverview: (projectPath?: string, force = false): Promise<ClaudeMemoryOverview> =>
    invoke("get_claude_memory_overview", { projectPath, force }),

  getFileContent: (nativePath: string, projectPath?: string): Promise<string> =>
    invoke("get_claude_memory_file_content", { nativePath, projectPath }),
};
```

### 9.4 新增/改造组件

| 组件 | 来源 | 说明 |
|------|------|------|
| `MemoryAssetTree` | 新建 | 参考 `MemoryFileTree`，但按 scope 分组而非 source_group |
| `SecretBadge` | 新建 | 显示敏感信息警告的小徽章 |
| `FrontmatterCard` | 新建 | 展示解析后的 frontmatter 字段 |
| `MarkdownRenderer` | 复用 | 直接复用现有组件 |
| `ThemeToggle` | 复用 | 已有组件 |

### 9.5 状态管理

使用 React `useState` + `useEffect`（不引入全局状态库）：

```typescript
// ClaudeMemory 组件内部状态
const [scanResult, setScanResult] = useState<ClaudeMemoryOverview | null>(null);
const [selectedAsset, setSelectedAsset] = useState<ClaudeMemoryAsset | null>(null);
const [fileContent, setFileContent] = useState<string>("");
const [activeTab, setActiveTab] = useState<string>("instruction");
const [isLoading, setIsLoading] = useState(false);
```

---

## 10. 验证计划

### 10.1 后端验证

| 验证项 | 方法 | 通过标准 |
|--------|------|---------|
| Path Resolver | Rust 单元测试 | `CLAUDE_CONFIG_DIR` 优先、回退正确、PathBuf 无字符串拼接 |
| Scanner | Rust 单元测试 | 所有 asset_type 正确识别、空目录不报错、大文件截断 |
| Frontmatter | Rust 单元测试 | 标准 YAML 解析正确、缺失字段回退、异常格式不 panic |
| Secret Scanner | Rust 单元测试 | 命中所有 5 种模式、脱敏正确、无 false negative |
| Allowlist | Rust 单元测试 | 相对路径拒绝、符号链接逃逸拒绝、非 allowlist 路径拒绝 |
| 跨平台 | CI 三平台构建 | Linux / Windows / macOS 均通过 `cargo test` |

### 10.2 前端验证

| 验证项 | 方法 | 通过标准 |
|--------|------|---------|
| 路由 | E2E 测试 | 顶部大域导航显示 "Claude 记忆"，点击正常切换；设置域侧边栏不再显示 Claude 记忆 |
| API 调用 | E2E 测试 | `get_claude_memory_overview` 返回正确结构 |
| 组件渲染 | E2E 测试 | 文件树正确分组、选中文件显示内容 |
| 空状态 | E2E 测试 | 无记忆文件时显示友好空状态 |
| 错误状态 | E2E 测试 | 权限不足时显示友好错误提示 |

### 10.3 集成验证（三平台实体机）

| 平台 | 机器 | 验证内容 |
|------|------|---------|
| macOS | 本机 | 完整功能验证（扫描、查看、敏感信息检测） |
| Linux | 3.50 实体机 | 完整功能验证 |
| Windows | 3.10 实体机 | 完整功能验证 + 路径分隔符处理 + `\\?\` 前缀 |

### 10.4 性能验证

| 指标 | 目标 | 验证方法 |
|------|------|---------|
| 整主机扫描 | ≤ 3 秒 | 在实体机上用 `std::time::Instant` 测量 |
| 文件列表加载 | ≤ 200ms | 前端计时 |
| 单文件内容读取 | ≤ 100ms | 前端计时 |

### 10.5 安全验证

| 验证项 | 方法 |
|--------|------|
| 路径逃逸防护 | 尝试传入 `../../../etc/passwd`，验证被拒绝 |
| 符号链接逃逸 | 在 `~/.claude/` 下创建指向 `/etc` 的 symlink，验证被拒绝 |
| 敏感信息不泄露 | 抓包验证无网络请求 |
| 大文件保护 | 传入超过 1 MiB 的文件路径，验证被拒绝 |

---

## 附录 A：文件变更清单

### 新建文件

| 文件 | 说明 |
|------|------|
| `src-tauri/src/collectors/claude_memory/mod.rs` | 模块入口 |
| `src-tauri/src/collectors/claude_memory/path_resolver.rs` | 路径解析器 |
| `src-tauri/src/collectors/claude_memory/scanner.rs` | 扫描器 |
| `src-tauri/src/collectors/claude_memory/frontmatter.rs` | Frontmatter 解析器 |
| `src-tauri/src/collectors/claude_memory/secret_scanner.rs` | 敏感信息扫描器 |
| `src-tauri/src/collectors/claude_memory/models.rs` | 数据模型 |
| `src-tauri/src/services/claude_memory_service.rs` | 业务服务层 |
| `src-tauri/src/routes/claude_memory.rs` | Tauri 命令 |
| `src/features/claude-memory/index.tsx` | 前端主页面 |
| `src/features/claude-memory/components/MemoryAssetTree.tsx` | 资产树组件 |
| `src/features/claude-memory/components/SecretBadge.tsx` | 敏感信息徽章 |
| `src/features/claude-memory/components/FrontmatterCard.tsx` | Frontmatter 卡片 |

### 修改文件

| 文件 | 变更 |
|------|------|
| `src-tauri/src/lib.rs` | 注册 2 个新命令；AppState **不新增**缓存字段（v0.1 实时扫描） |
| `src-tauri/src/app_state.rs` | **不修改**（v0.1 实时扫描，无需缓存字段） |
| `src-tauri/src/routes/mod.rs` | 导出 `claude_memory` 模块 |
| `src-tauri/Cargo.toml` | **新增 `regex = "1"`**（仅用于本地敏感信息扫描） |
| `src/App.tsx` | `AppDomain` 扩展 `"claude-memory"`；`SettingsPage` 移除 `"claude-memory"` |
| `src/components/Sidebar.tsx` | 新增 Claude 记忆域侧边栏；设置域移除 Claude 记忆项 |
| `src/lib/api.ts` | 新增 `claudeMemoryApi` |
| `e2e/navigation.spec.ts` | 更新导航测试（新增设置子页面） |

---

## 附录 B：与需求文档的追溯矩阵

| 需求文档 FR/AC | 本设计对应章节 | 说明 |
|---------------|---------------|------|
| FR-01（Instruction Memory） | §4.2（扫描范围）、§8（命令） | 扫描 4 个位置的 CLAUDE.md |
| FR-02（Auto Memory） | §4.2（扫描范围） | 扫描 `~/.claude/projects/<id>/memory/` |
| FR-03（Rule Memory） | §4.2、§5（Frontmatter） | 解析 `paths` frontmatter |
| FR-04（Skills & Agents） | §4.2、§5 | 扫描 SKILL.md 和 agents/*.md，解析 frontmatter |
| FR-05（Secret Scanner） | §6 | 本地正则匹配，脱敏输出 |
| AC-01 ~ AC-04 | §4.4（错误处理） | 文件不存在显示空状态 |
| AC-05 ~ AC-08 | §4.2 | 各类 asset 的扫描覆盖 |
| AC-09 ~ AC-12 | §6、§10 | 敏感信息扫描 + 验证 |
