# AgentScope — Claude Code 记忆管理 v0.2 技术设计草案

> 状态：技术设计草案（修订中，尚未进入实现）
> 创建日期：2026-05-21
> 修订日期：2026-05-21
> 关联文档：
> - `docs/requirements-claude-memory-management.md`
> - `docs/design-claude-memory-management-v0.1.md`
> - `docs/claude-memory-v0.1-acceptance-report.md`
> 前提：v0.1 已完成验收，代码基线已冻结

---

## 1. 设计决策总览

### 1.1 v0.2 第一实现批次范围（修订）

**第一批次只实现 P1 + P2（加载链模拟 + @import 检查），均为纯只读分析能力。**

P3（健康检测增强）和 P4（编辑能力）保留设计文档，但**不在第一批次代码中实现**。

理由：
1. P1 和 P2 是**纯只读分析能力**，与 v0.1 的只读架构一致，不引入写入风险
2. P1 需要定向解析 cwd 的文件系统（不是复用 overview 扫描结果），P2 是 P1 加载链的自然延伸
3. P3 的健康检测增强中"过长/敏感信息"已在 v0.1 实现，v0.2 只需补充"重复/冲突"的边界，可独立排期
4. P4 编辑能力涉及**文件写入、备份、并发控制**，需要独立评估安全性和复杂度，不应与 P1-P3 混排
5. 第一批次聚焦"加载可解释性"核心目标，降低实施风险

### 1.2 与 v0.1 的关系

```
v0.1 基线
├── 扫描器（scanner.rs）          → 复用，不修改
├── 路径解析（path_resolver.rs）   → 复用，不修改
├── Frontmatter 解析              → 复用，不修改
├── Secret Scanner                → 复用，不修改
├── 数据模型（models.rs）          → 扩展，新增字段/结构体
├── Service 层                     → 扩展，新增 service 函数
├── 路由（routes/claude_memory.rs） → 扩展，新增 command
└── 前端页面（features/claude-memory/） → 扩展，新增子页面

v0.2 新增
├── 加载链模拟器（load_chain.rs）   → 纯新增模块
├── @import 解析器（import_resolver.rs） → 纯新增模块
├── 健康检测增强（health_checker.rs）   → 纯新增模块
└── 编辑服务设计（edit_service.rs）     → 纯新增模块（先设计，后实现）
```

---

## 2. P1：加载可解释性（Load Chain Simulation）

### 2.1 目标

模拟从指定 cwd 启动 Claude Code 时的记忆加载链，回答用户问题："我现在启动 Claude，实际会加载哪些记忆？"

**关键边界：明确区分"启动时加载链"与"可能触发的 path-scoped rules"**

### 2.2 加载链规则（与 Claude Code 官方行为对照）

根据 Claude Code 官方文档和 `/memory` 命令输出，加载分为两个逻辑区域：

#### A. 启动时加载链（确定性的启动加载）

**Instruction 文件解析范围**（按加载顺序）：

```
1. managed CLAUDE.md（若可访问）
   - macOS: /Library/Application Support/ClaudeCode/CLAUDE.md
   - Linux: /etc/claude-code/CLAUDE.md
   - Windows: `C:\Program Files\ClaudeCode\CLAUDE.md`
2. 用户全局 ~/.claude/CLAUDE.md
3. 从根目录到 cwd 的上级目录链（逐级向下），每层检查：
   a. 该层的 CLAUDE.md（✅ 官方确认）
   b. 该层的 CLAUDE.local.md（✅ 官方确认）
   c. 该层的 .claude/CLAUDE.md（⚠️ A9 推断，第一版不纳入启动链）
4. 当前目录 ./CLAUDE.md
5. 当前目录 ./.claude/CLAUDE.md（官方明确支持的项目 instruction）
6. 当前目录 ./CLAUDE.local.md
7. 全局 rules（~/.claude/rules/**/*.md，无 paths 的 → 无条件加载）
8. 项目级 rules（./.claude/rules/**/*.md，无 paths 的 → 无条件加载）
9. Auto Memory（~/.claude/projects/<project>/memory/MEMORY.md，前 200 行或前 25KB，取先到者）
   - `<project>` 的匹配策略：
     - 普通 git repo 子目录：以 **repo root 路径的编码** 匹配（同一 repo 的不同子目录共享 auto memory）✅ 已支持
     - git worktree：官方说明与主 repo 共享 auto memory，但当前实现未解析 `.git` 文件，返回 worktree 自身路径 ⚠️ P1 limitation
     - 非 git 目录：以 **cwd 路径编码** 匹配
```

**关键细节**：
- **managed CLAUDE.md**（步骤 1）：file-based managed instruction，若存在且可读则加入启动链最前；不可读时记录 warning，不阻断模拟
- 上级目录遍历（步骤 3）：从文件系统根目录 `/` 逐级向下到 cwd，每层检查：
  - `<ancestor>/CLAUDE.md`（✅ 官方确认）
  - `<ancestor>/CLAUDE.local.md`（✅ 官方确认）
  - `<ancestor>/.claude/CLAUDE.md`（⚠️ A9 推断，第一版不纳入启动链；若 A9 验证确认，后续版本补充）
  - 同一层内顺序：CLAUDE.md → CLAUDE.local.md（A9 项验证后补充 .claude/CLAUDE.md 位置）
- 当前目录（步骤 4-6）：`./CLAUDE.md`、`./.claude/CLAUDE.md`、`./CLAUDE.local.md`
  - `./.claude/CLAUDE.md` 是官方明确支持的项目级 instruction 位置，不受 A9 推断影响
- `claudeMdExcludes`：匹配的文件从加载链中排除（见 §2.5）
- 相同 scope 的 rule 加载顺序：按文件名字母顺序
- Auto Memory：只有 `MEMORY.md` 是启动加载的；topic 文件（如 `debugging.md`）**不是**启动时自动加载
- Auto Memory 加载上限：前 200 行 **或** 前 25KB，取先达到者；超出部分不进入启动上下文

#### B. 可能触发的 path-scoped rules（运行时条件加载）

```
8. 路径触发 rules（paths 匹配后续读取的文件路径的）
```

**关键区别**：
- path-scoped rules **不在启动时无条件加载**
- 当 Claude Code 在会话中读取某个文件时，检查该文件路径是否匹配 rule 的 `paths` glob 模式
- 匹配时，该 rule 的内容被注入到当前上下文
- **设计约束**：AgentScope v0.2 的加载模拟器只展示 A 区域（启动链），B 区域单独列出为"可能触发的 rules"

### 2.3 P1 数据来源策略（修订）

**核心设计：加载链模拟从用户输入的 cwd 做定向解析，按官方规则向上遍历目录层级，不是基于 v0.1 overview 扫描结果做纯内存计算。**

原因：
1. v0.1 overview 扫描范围是**已注册项目 + 用户级配置目录**，不一定覆盖 cwd 的所有祖先目录
2. 用户可能在任意目录（未注册项目）启动 Claude，祖先目录的 CLAUDE.md 需要实时发现
3. 加载链的正确性不能依赖 overview 是否已扫描到某个路径

**实现策略**：

```rust
/// 从 cwd 定向解析加载链
///
/// 策略：
/// 1. 从 cwd 向上遍历到根目录，发现每一层的 CLAUDE.md 和 CLAUDE.local.md
/// 2. 读取 ~/.claude/CLAUDE.md（用户全局指令）
/// 3. 读取 cwd 下的 CLAUDE.md、.claude/CLAUDE.md、CLAUDE.local.md
/// 4. 读取 user 级 rules（~/.claude/rules/**/*.md，无 paths 的，递归子目录）
/// 5. 读取 project 级 rules（cwd/.claude/rules/**/*.md，无 paths 的，递归子目录）
/// 6. 读取 Auto Memory（通过 encode_cwd_path 精确匹配 ~/.claude/projects/<id>/memory/MEMORY.md）
/// 7. 应用 claudeMdExcludes 排除配置
///
/// 复用 v0.1 能力：
/// - path_resolver：解析 ~/.claude/ 路径、项目级路径
/// - frontmatter 解析：读取 rule 的 paths 字段（用于区分无条件加载 vs path-scoped）
/// - 安全读取：MAX_FILE_READ_BYTES 限制、内容预览策略
pub fn simulate_load_chain(
    cwd: &Path,
) -> Result<SerLoadChain, String>
```

**计算逻辑**：

```
输入: cwd（用户指定的启动目录）
输出: SerLoadChain

1. 启动链构建（A 区域）:
   a. managed CLAUDE.md: 检测系统级 managed instruction 路径，若存在且可读则加入
   b. 用户级 CLAUDE.md: ~/.claude/CLAUDE.md（若存在）
   c. 上级目录链：从根目录遍历到 cwd，每层检查：
      - <ancestor>/CLAUDE.md（✅ 官方确认）
      - <ancestor>/CLAUDE.local.md（✅ 官方确认）
      - <ancestor>/.claude/CLAUDE.md（⚠️ A9 推断，第一版不纳入启动链）
   d. 当前目录：
      - cwd/CLAUDE.md（✅ 官方确认）
      - cwd/.claude/CLAUDE.md（✅ 官方明确支持）
      - cwd/CLAUDE.local.md（✅ 官方确认）
   d. 全局无条件 rules: ~/.claude/rules/**/*.md（无 paths frontmatter 的，递归扫描子目录）
   e. 项目级无条件 rules: cwd/.claude/rules/**/*.md（无 paths frontmatter 的，递归扫描子目录）
   f. Auto Memory: ~/.claude/projects/<encode_cwd_path(repo_root_or_cwd)>/memory/MEMORY.md
      - git 仓库内：先定位 git repo root，再用 repo root 的编码路径匹配
      - 非 git 目录：用 cwd 的编码路径匹配
      - 受 `autoMemoryDirectory` 设置影响（见 §2.8）

2. path-scoped rules 发现（B 区域，单独列出）:
   a. 全局 path-scoped rules: ~/.claude/rules/**/*.md（有 paths 的，递归扫描子目录）
   b. 项目级 path-scoped rules: cwd/.claude/rules/**/*.md（有 paths 的，递归扫描子目录）
   c. 每条记录其 paths 模式，但不进入启动链顺序

3. 排除 claudeMdExcludes 中配置的文件（见 §2.5）
```

**registered project 与 arbitrary cwd 的行为差异**：

| 输入类型 | 行为 |
|---------|------|
| Registered project cwd | 项目级规则从 `cwd/.claude/rules/` 读取（递归子目录）；Auto Memory 通过 **repo root 编码路径** 匹配（普通 git repo） |
| Arbitrary cwd（未注册） | 项目级规则仍从 `cwd/.claude/rules/` 读取（递归子目录，Claude Code 不要求项目注册）；Auto Memory 通过 **repo root 编码路径** 匹配（普通 git repo） |
| 非 git 目录 | Auto Memory 通过 **cwd 编码路径** 匹配；若未找到匹配则记录 info warning，不阻断加载链模拟 |
| git worktree | Auto Memory 匹配行为 **未保证**：当前实现返回 worktree 自身路径，官方文档要求与主 repo 共享 ⚠️ limitation |

**Auto Memory 匹配策略（修订）**：
1. **普通 git repo 子目录**：Claude Code 按 repository identity 派生 Auto Memory 目录。同一 repo 的不同子目录共享同一 Auto Memory。AgentScope 通过向上查找 `.git` 目录定位 repo root，再用 repo root 编码匹配 ✅ 已支持
2. **git worktree**：官方文档说明 worktree 与主 repo 共享 Auto Memory，但当前实现未解析 `.git` 文件，返回 worktree 自身路径 ⚠️ P1 limitation
3. **非 git 目录**：Claude Code 使用 project root（即 cwd 本身）派生 Auto Memory 目录。AgentScope 回退到 cwd 编码路径
4. **用户自定义**：`autoMemoryDirectory` 设置可改变存储位置（见 §2.8）⚠️ P1 limitation

### 2.4 数据结构（修订）

```rust
/// 加载链模拟结果
#[derive(Debug, Clone, Serialize)]
pub struct SerLoadChain {
    pub cwd: String,                      // 模拟的启动目录
    pub host_profile: SerHostProfile,     // 当前主机信息
    pub startup_chain: Vec<SerLoadChainStep>,   // A 区域：启动时确定性加载链
    pub path_scoped_rules: Vec<SerPathScopedRule>, // B 区域：可能触发的 path-scoped rules
    pub excluded_assets: Vec<SerExcludedAsset>, // 被 claudeMdExcludes 排除的
    pub warnings: Vec<SerLoadChainWarning>,
}

/// 启动链中的单个步骤
#[derive(Debug, Clone, Serialize)]
pub struct SerLoadChainStep {
    pub order: usize,                     // 加载顺序（1-based）
    pub scope: String,                    // "user" | "project" | "local" | "auto" | "managed"
    pub asset_type: String,
    pub logical_path: String,
    pub native_path: String,
    pub load_reason: String,              // 加载原因说明
    pub line_count: Option<usize>,
    pub byte_size: Option<u64>,
    pub content_preview: Option<String>,  // 前 2KB 预览
    pub content_truncated: bool,          // 是否因大小限制截断
    pub exists: bool,
}

/// 可能触发的 path-scoped rule（不在启动链中）
#[derive(Debug, Clone, Serialize)]
pub struct SerPathScopedRule {
    pub scope: String,                    // "user" | "project"
    pub native_path: String,
    pub logical_path: String,
    pub name: Option<String>,             // frontmatter name
    pub paths: Vec<String>,               // 触发路径 glob 模式列表
    pub exists: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct SerExcludedAsset {
    pub native_path: String,
    pub logical_path: String,
    pub scope: String,                    // 被排除资产的原始 scope
    pub excluded_by: String,              // 排除来源（"user" | "project" | "local" | "managed"）
    pub pattern: String,                  // 匹配的排除模式
}

#[derive(Debug, Clone, Serialize)]
pub struct SerLoadChainWarning {
    pub level: String,                    // "warning" | "info"
    pub code: String,                     // 警告代码
    pub message: String,
}
```

### 2.5 claudeMdExcludes 设计（修订）

**来源边界**：

Claude Code 的 `claudeMdExcludes` 配置可来自多个 settings 层级。各层级的载体文件不同：

| 层级 | 来源文件 | 作用范围 | v0.2 第一阶段支持 |
|------|---------|---------|----------------|
| Managed（file-based） | 基础文件：`/Library/Application Support/ClaudeCode/managed-settings.json`<br>Drop-in 目录：`/Library/Application Support/ClaudeCode/managed-settings.d/*.json` | 整台机器/企业 | ⚠️ **仅覆盖 file-based**。检测文件存在性；若存在且可读，合并其 excludes 数组；若不可读，记录 warning |
| User | `~/.claude/settings.json` | 当前用户所有项目 | ✅ 支持读取 |
| Project | `<repo>/.claude/settings.json` | 当前项目 | ✅ 支持读取 |
| Local | `<repo>/.claude/settings.local.json` | 当前项目（本机私有） | ✅ 支持读取 |

> **重要区分**：
> 1. **managed settings 与 managed CLAUDE.md 是两种独立载体**：
>    - managed settings 是 `managed-settings.json` / `managed-settings.d/*.json` 配置文件，承载 `claudeMdExcludes` 等策略
>    - managed CLAUDE.md 是 instruction 文件（`/Library/Application Support/ClaudeCode/CLAUDE.md`），参与启动链加载，不属于 claudeMdExcludes 的排除对象
> 2. **第一批次 limitation**：server-managed / MDM / registry 等非 file-based 管理来源不可读取，标记为"不可见来源"，不假装已覆盖完整 managed tier

**读取与合并策略**：

```rust
/// 读取多层 claudeMdExcludes 并合并
/// 
/// 合并规则：
/// 1. managed 层：先读 managed-settings.json，再读 managed-settings.d/*.json
///    - 各 drop-in 文件按文件名排序后合并
/// 2. user / project / local 层：读对应 settings.json
/// 3. 每层取其 claudeMdExcludes 字符串数组
/// 4. 数组跨层合并（concat），不是覆盖：所有层的 excludes 都生效
/// 5. 每个 pattern 保留来源标注（managed / user / project / local）
/// 6. 返回合并后的排除模式列表
pub fn read_claude_md_excludes(
    cwd: &Path,
) -> Result<ClaudeMdExcludesConfig, String>;

pub struct ClaudeMdExcludesConfig {
    pub patterns: Vec<ExcludePattern>,
    pub managed_accessible: Option<bool>, // None = 无 file-based managed；Some(true/false) = 有但可读/不可读
}

pub struct ExcludePattern {
    pub pattern: String,                  // glob 模式
    pub source: String,                   // "managed" | "user" | "project" | "local"
}
```

**排除匹配策略**：
- 匹配对象：文件的**绝对路径**（canonicalized）
- 模式类型：glob 模式（如 `/Users/*/Repo/*/CLAUDE.md`）
- 匹配逻辑：使用 `glob` crate 或兼容的 glob 匹配
- 被排除文件在 UI 中展示排除来源（managed / user / project / local）

**第一阶段限制（显式声明）**：
- **仅覆盖 file-based managed settings**：基础文件 `managed-settings.json` + drop-in 目录 `managed-settings.d/*.json`
- **不覆盖非 file-based 来源**：server-managed（通过 API 推送）、MDM、registry 等管理来源不可读取，加载链模拟结果中标记为"不可见 managed 来源"
- file-based managed settings 实际可读性取决于操作系统权限
  - macOS: `/Library/Application Support/ClaudeCode/managed-settings.json`（通常需要 root 权限）
  - Linux: `/etc/claude-code/managed-settings.json`（通常需要 root 权限）
  - Windows: `C:\Program Files\ClaudeCode\managed-settings.json`（通常需要管理员权限）
  - **Legacy / unsupported**: `C:\ProgramData\ClaudeCode\managed-settings.json` 不再作为第一批实现基线，若存在可记录为发现但不纳入合并
- **第一批次实现**：尝试读取上述路径，若不可读则 `managed_accessible = Some(false)`，记录 warning："file-based managed settings 存在但无权限读取，其 claudeMdExcludes 未被纳入模拟。server-managed / MDM 等非 file-based 来源不在本次模拟范围内"
- 不将"无法读取 managed settings"视为错误，加载链模拟继续
- managed 层的 excludes 若被成功读取，**正常参与合并**
- managed CLAUDE.md（若可访问）作为 instruction 文件参与启动链（见 §2.2）

### 2.6 paths 匹配逻辑（path-scoped rules 用）

path-scoped rule 的 frontmatter 中 `paths` 是 glob 模式列表，匹配对象是**会话中读取的文件路径**（不是 cwd）：

```yaml
paths:
  - "src/**/*.rs"
  - "tests/**/*.rs"
```

**匹配逻辑**：
1. 当 Claude Code 读取某个文件时，获取其相对于项目根目录的相对路径
2. 使用 glob 模式匹配（需要引入 `glob` crate 或手写轻量匹配）
3. 若任意一个 pattern 匹配，则该 rule 被注入当前上下文
4. 无 `paths` 的 rule（paths=None）表示无条件加载，属于 A 区域

**v0.2 展示策略**：
- 在 B 区域展示每条 path-scoped rule 的 `paths` 模式
- 提供"测试匹配"功能：用户输入一个文件路径，查看哪些 rules 会触发
- 不做实时会话监控（那是运行时功能，超出 v0.2 范围）

### 2.7 Rules 递归发现

Claude Code 的 rules 目录支持递归组织：
- `~/.claude/rules/**/*.md`（用户级，递归子目录）
- `cwd/.claude/rules/**/*.md`（项目级，递归子目录）

**第一批次实现**：递归扫描 rules 子目录下的所有 `.md` 文件，不只是顶层 `*.md`。

理由：
1. 用户可能按功能将 rules 组织到子目录（如 `rules/coding/`、`rules/testing/`）
2. v0.1 scanner 已支持递归目录扫描，技术上无额外复杂度
3. 不递归会导致遗漏用户实际使用的 rules

**手动对照验证点（修订）**：
- `/memory` 命令输出为**交互式 UI**（on/off 开关 + 文件路径），不是文本列表形式的启动链报告。验证应分三个层面：
  1. **文件存在性/识别项对照**：Claude `/memory` 中显示的记忆项，AgentScope 是否也识别到？
  2. **顺序规则校验**：AgentScope 输出是否符合官方文档描述的加载规则？（独立校验，不直接对比 `/memory` UI）
  3. **差异项记录**：哪些是 `/memory` UI 不可观察但 AgentScope 可展示的（如 managed CLAUDE.md、祖先目录链）
- 验证"上级目录 CLAUDE.md"的加载顺序（从根到 cwd）：AgentScope 输出应符合官方规则
- 验证 path-scoped rules 是否在 `/memory` 中单独列出（而非混入启动链）
- 验证 Auto Memory 的 200 行 / 25KB 截断行为
- 验证 Auto Memory 匹配：同一 git repo 的不同子目录是否共享同一 Auto Memory

### 2.8 Auto Memory 目录自定义（`autoMemoryDirectory`）

Claude Code 用户可通过 `settings.json` 中的 `autoMemoryDirectory` 字段自定义 Auto Memory 的存储位置：

```json
{
  "autoMemoryDirectory": "/path/to/custom/auto-memory"
}
```

**第一批次处理策略**：
- **当前实现**：先按默认路径（`~/.claude/projects/<id>/memory/`）查找，若未找到则记录 info warning
- **Limitation**：P1 **不读取** `autoMemoryDirectory` 设置。若用户自定义了该路径，AgentScope 仅表现为默认路径 `auto_memory_not_found`（通用"未找到"提示），**不是**专门的 limitation warning。Claude Code 实际可能从自定义路径加载，但 AgentScope 不会检测或提示此差异
- **后续计划**：P2/P3 阶段增加 `autoMemoryDirectory` 读取支持，优先级低于 git repo root 匹配修复

**验证建议**：
- 若用户未设置 `autoMemoryDirectory`，默认路径查找应正常工作
- 若用户设置了 `autoMemoryDirectory`，AgentScope P1 不会检测该设置，仅表现为默认路径 `auto_memory_not_found` warning。这**不是**专门的 limitation warning，而是未找到默认路径的通用提示。需在验证日志中注明此 limitation

---

## 3. P2：@import 检查

### 3.1 目标

解析记忆文件中的 `@path/to/file` import 语法，检测加载链中的引用完整性问题。

### 3.2 @import 语法（修订）

Claude Code 支持在 CLAUDE.md 中使用 `@path/to/file` 引用其他文件，被引用的文件内容会内联展开。

**语法规则**：
- `@path/to/file.md` — **相对包含该 import 的文件**所在目录的路径
- `@~/path/to/file.md` — 相对于**用户 home 目录**（`$HOME` / `%USERPROFILE%`）的路径
- `@/absolute/path/to/file.md` — 绝对路径
- 支持嵌套：被 import 的文件中也可以包含 @import

**路径解析策略**：

```rust
/// 解析 import 文本为绝对路径
/// 
/// 规则：
/// 1. `@~/...` → resolve_claude_config_dir()?.parent() + "..."
///    注：~ 表示用户 home 目录，不是 ~/.claude/
/// 2. `@/absolute/path` → 直接使用绝对路径
/// 3. `@relative/path` → 包含 import 的文件所在目录 + relative/path
fn resolve_import_path(import_text: &str, containing_file: &Path) -> Result<PathBuf, String>;
```

**外部 import 策略（修订）**：

Claude Code 自身存在 external import approval 语义：当被 import 的文件位于当前项目目录之外时，Claude 会询问用户是否允许读取。

AgentScope v0.2 的处理策略：
- 对于解析出的 import 路径，执行与 v0.1 相同的 allowlist 校验
- 若路径在 allowlist 内（`~/.claude/` 下或已注册项目下）：正常读取并展开
- 若路径在 allowlist 外：
  - **不直接标记为"Claude 不会加载"**
  - 标记为 `external` 状态，分类为：
    - `outside-read-scope`：路径在 allowlist 外，AgentScope 无法读取内容
    - `approval-unknown`：Claude Code 可能会请求用户批准，AgentScope 无法预知结果
  - 展示 warning："此 import 指向 allowlist 外路径，Claude Code 可能请求外部读取批准"

### 3.3 解析策略（修订）

**步骤**：
1. 从加载链的每个 step 中提取 `@...` 引用
2. **忽略 Markdown 代码块（```...```）和行内 code span（`...`）中的 @ 文本**
3. 解析引用路径（按 §3.2 规则解析为绝对路径）
4. 检查被引用文件是否存在
5. 递归解析被引用文件中的 @import（构建 import 树）
6. 检测循环引用（DFS + visited set）
7. 检测深度上限（默认 5 层，可配置）

> **设计决策**：代码块和行内 code 中的 `@path` 默认被忽略。理由：Claude Code 的 @import 展开发生在发送给模型前的预处理阶段，但其语义意图是"引用记忆文件"，代码块中的 `@path` 更可能是示例/文档而非实际引用。若实测发现 Claude Code 不忽略，则回滚此决策（见验证日志 D6/D7）。

```rust
/// @import 解析结果
#[derive(Debug, Clone, Serialize)]
pub struct SerImportGraph {
    pub root_asset_id: String,            // 加载链中的起始文件
    pub root_native_path: String,         // 根文件的绝对路径
    pub nodes: Vec<SerImportNode>,
    pub edges: Vec<SerImportEdge>,
    pub issues: Vec<SerImportIssue>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SerImportNode {
    pub node_id: String,                  // 唯一标识（路径 hash + 深度）
    pub asset_id: Option<String>,         // 引用 overview 中资产的 id（None = 未扫描到）
    pub resolved_path: String,            // 解析后的绝对路径
    pub depth: usize,                     // import 深度（根文件 = 0）
    pub exists: bool,
    pub read_allowed: bool,               // 是否在 allowlist 内（AgentScope 能否读取）
    pub external_status: Option<String>,  // None | "outside-read-scope" | "approval-unknown"
}

#[derive(Debug, Clone, Serialize)]
pub struct SerImportEdge {
    pub from: String,                     // node_id
    pub to: String,                       // node_id
    pub import_text: String,              // 原始的 @... 文本
}

#[derive(Debug, Clone, Serialize)]
pub struct SerImportIssue {
    pub issue_type: String,               // "cycle" | "depth_exceeded" | "not_found" | "outside_allowlist"
    pub message: String,
    pub node_id: String,
    pub related_node_ids: Vec<String>,
}
```

### 3.4 与加载链的集成

@import 检查不是独立功能，而是**加载链模拟的下游分析**：

```
用户选择 cwd
  → P1: simulate_load_chain(cwd) → SerLoadChain
  → P2: 对 SerLoadChain 中的每个 step，解析其 @import
  → 生成 SerImportGraph（按 step 分组）
```

前端展示：
- 加载链页面中，每个 step 可展开显示其 import 树
- 循环引用用红色高亮
- 不存在的 import 用黄色警告
- 外部 import（outside allowlist）用灰色标记，附带说明

### 3.5 待验证假设

**代码块中的 @import 行为**：

当前设计选择**忽略** Markdown 代码块和行内 code span 中的 `@path`（见 §3.3）。这是一个**待验证的设计决策**：

- **若验证通过**（Claude Code 也忽略代码块中的 @）：保持当前设计
- **若验证不通过**（Claude Code 将代码块中的 @ 也视为 import）：回滚为"不忽略代码块"，parser 移除 Markdown 结构感知

**验证方法**：在 CLAUDE.md 中写入代码块包含 `@fake.md`，运行 `/memory` 查看是否尝试加载 fake.md。详见验证日志 D6/D7。

### 3.6 Allowlist 校验（复用 v0.1）

@import 解析出的文件路径必须经过与 v0.1 `get_claude_memory_file_content` 相同的 allowlist 校验：
- canonicalize 后比对 `~/.claude/`、已注册项目路径
- allowlist 外的 import 标记为 `outside-read-scope`，不阻止展示但标注风险

### 3.7 手动对照验证点

- 在包含 @import 的 CLAUDE.md 项目中运行 `/memory`，对比 import 展开结果
- 验证循环引用时 Claude Code 的行为（是报错还是静默停止？）
- 验证深度上限（Claude Code 是否有此限制？）
- 验证 `~` 前缀解析为 home 目录（不是 ~/.claude/）
- 验证代码块中的 `@path` 是否被展开

---

## 4. P3：记忆健康检测增强（保留设计，不在第一批次实现）

### 4.1 设计保留说明

P3 的设计内容（重复段落检测、健康评分、冲突检测占位）保留在本文档中作为未来实现参考，但**不在 v0.2 第一批次代码中实现**。

第一批次完成后，根据 P1+P2 的实际进度评估是否进入 P3。

---

## 5. P4：编辑能力设计（保留设计，不在第一批次实现）

### 5.1 设计保留说明

P4 的设计内容（可编辑范围、写入 allowlist、备份策略、原子写入、编辑器选型）保留在本文档中作为未来实现参考，但**不在 v0.2 第一批次代码中实现**。

编辑相关 command（save/create/delete/backup/restore）**不作为第一批次实现清单的一部分**。

### 5.2 设计要点摘要（供未来参考）

| 资产类型 | 允许编辑 | 允许创建 | 允许删除 |
|----------|---------|---------|---------|
| `user_claude_md` / `project_claude_md` / `local_md` | ✅ | ❌ | ❌ |
| `global_rule` / `project_rule` | ✅ | ✅ | ✅ |
| `global_skill` / `project_skill` / `global_agent` / `project_agent` | ✅ | ❌ | ❌ |
| `auto_memory_*` | ❌ | ❌ | ❌ |

**安全策略**：
- 复用 v0.1 allowlist + 额外校验（禁止写入 auto_memory 目录）
- 每次写入前自动备份到 `.backups/` 目录
- 原子写入（先写 .tmp 再重命名）
- mtime 并发修改检测
- 大文件（> 1 MiB）禁止编辑

**前端**：先用 textarea，后续评估 Monaco/CodeMirror

---

## 5.3 Claude 资产安全原则（P1-P4 通用）

> 以下原则适用于 AgentScope 所有与 Claude Code 资产交互的功能。

**只读优先原则**：
- P1（加载链模拟）和 P2（@import 解析）为纯只读功能：只扫描、不写入、不修改、不删除任何文件
- 加载链模拟器读取用户真实 `~/.claude/` 和项目目录，但绝不回写
- 模拟器结果仅用于展示和分析，不触发任何文件系统变更

**写操作隔离原则（P4 及以后）**：
- 任何写入操作必须通过明确的 allowlist 校验
- 禁止写入 `~/.claude/projects/` 下的 Auto Memory 目录（Claude Code 独占区域）
- 禁止修改 managed settings（`/Library/Application Support/ClaudeCode/`、`/etc/claude-code/` 等系统级配置）
- 所有写入操作前先自动备份原文件到 `.backups/` 目录
- 原子写入：先写 `.tmp` 临时文件，验证成功后重命名

**测试隔离原则**：
- 单元测试和 E2E 测试绝不触碰真实 `~/.claude/` 目录
- 测试使用临时目录 + `CLAUDE_CONFIG_DIR` 环境变量隔离
- 并发测试使用全局锁（`ENV_LOCK`）防止环境变量竞争

**验证阶段安全约束**：
- P1 语义验证（与 `/memory` 命令对照）为纯观察行为，不修改任何真实资产
- 验证过程中若发现需要创建测试文件，必须在临时目录或专用测试项目中进行
- 验证完成后清理所有临时测试数据

---

## 6. 新增后端 Command（第一批次）

### 6.1 第一批次命令集（P1 + P2 只读）

```rust
/// P1: 加载链模拟
/// 
/// 参数：
///   - cwd: String — 模拟启动目录（必填）
/// 返回：SerLoadChain（含 startup_chain + path_scoped_rules）
#[tauri::command(rename = "simulate_claude_memory_load_chain")]
pub fn simulate_load_chain_cmd(
    cwd: String,
    state: State<'_, AppState>,
) -> Result<SerLoadChain, String>;

/// P2: @import 解析
/// 
/// 参数：
///   - native_path: String — 要解析 @import 的根文件绝对路径
///   - max_depth: Option<usize> — 最大解析深度（默认 5）
/// 返回：SerImportGraph
#[tauri::command(rename = "get_claude_memory_import_graph")]
pub fn get_import_graph_cmd(
    native_path: String,
    max_depth: Option<usize>,
    state: State<'_, AppState>,
) -> Result<SerImportGraph, String>;
```

### 6.2 命令分类

| 批次 | 命令 | 对应 P | 依赖 |
|------|------|--------|------|
| 第一批次 | `simulate_claude_memory_load_chain` | P1 | v0.1 path_resolver + frontmatter + 安全读取 |
| 第一批次 | `get_claude_memory_import_graph` | P2 | P1 + v0.1 allowlist |

**不在第一批次的命令（P3/P4 保留设计）**：
- `get_claude_memory_health_report`（P3）
- `save_claude_memory_file` / `create_claude_memory_file` / `delete_claude_memory_file`（P4）
- `list_claude_memory_backups` / `restore_claude_memory_backup`（P4）

---

## 7. 前端页面/子导航扩展（第一批次）

### 7.1 Claude 记忆域子导航扩展

当前 v0.1 结构：
```
Claude 记忆
└── 记忆资产
```

第一批次扩展后（只增加 P1 + P2）：
```
Claude 记忆
├── 记忆资产          (v0.1 现有，资产树 + 详情)
└── 加载模拟器         (v0.2 P1 + P2 新增)
    └── Import 展开面板 (嵌入在加载模拟器中)
```

**不在第一批次的子导航**：
- 健康检测页面（P3）：保留设计，后续评估
- Import 检查独立页面：不增加，import 作为加载模拟器的展开面板集成

### 7.2 加载模拟器页面设计

- **输入区**：
  - 目录路径输入框（默认当前项目路径或 home 目录）
  - "模拟加载"按钮
  - 选项：是否同时解析 @import（复选框）

- **启动链结果区**（A 区域）：
  - 按加载顺序排列的列表
  - 每行显示：order / scope / 文件名 / 加载原因 / 行数 / 大小
  - Auto Memory 显示截断状态（若 > 200 行或 > 25KB）
  - 被排除文件：单独列表，灰色标记，显示排除来源（user/project/local/managed）
  - 每个 item 可点击展开 import 树（P2）

- **Path-scoped Rules 区**（B 区域）：
  - 独立列表，标题为"可能触发的 Path-scoped Rules"
  - 每行显示：rule 名 / paths 模式列表
  - 提供"测试匹配"输入框：用户输入文件路径，查看哪些 rules 会触发
  - **不与启动链混排**

- **警告区**：
  - 缺失 CLAUDE.md（启动链中无 instruction 文件）
  - 加载链过长（总文件数 > 阈值）
  - Auto Memory 截断提示
  - Managed policy 存在提示

### 7.3 Import 展开面板设计

**方案：Import 作为加载模拟器中每个 step 的展开面板**

- 在加载模拟器结果中，每个 step 右侧增加"展开 import"按钮
- 展开后显示该文件的 import 树（类似文件树）
- 循环引用用红色标记
- 不存在的 import 用黄色标记
- 外部 import（outside allowlist）用灰色标记，附带说明
- 不新增独立子导航项

**理由**：@import 的语义是"加载链中某个文件的展开内容"，独立页面会割裂上下文。

---

## 8. 数据结构复用与扩展（修订）

### 8.1 v0.1 数据模型的复用策略

v0.1 数据模型在第一批次中的复用方式：

| 数据结构 | 复用方式 | 说明 |
|----------|---------|------|
| `SerClaudeMemoryAsset` | **不直接使用** | P1 定向解析独立构造 `SerLoadChainStep`，不依赖 overview 的 asset 数据。`SerClaudeMemoryAsset` 为 v0.1 scanner 专属结构，P1 不参与其生命周期。 |
| `SerHostProfile` | **复用** | P1 独立构造 host_profile，构造逻辑与 v0.1 一致 |
| `SerFrontmatter` | **复用解析逻辑** | P1 读取 rule 的 paths 字段时使用 v0.1 frontmatter 解析器 |
| `SerSecretIssue` | **不用于第一批次** | P3 保留设计时复用 |
| `SerMemorySummary` | **不用于第一批次** | P3 保留设计时复用 |

### 8.2 新增数据结构（第一批次）

| 数据结构 | 说明 | 关联 P |
|----------|------|--------|
| `SerLoadChain` | 加载链模拟核心输出 | P1 |
| `SerLoadChainStep` | 启动链中的单个步骤 | P1 |
| `SerPathScopedRule` | 可能触发的 path-scoped rule | P1 |
| `SerExcludedAsset` | 被排除的资产（含排除来源） | P1 |
| `SerLoadChainWarning` | 加载链警告 | P1 |
| `ClaudeMdExcludesConfig` | claudeMdExcludes 配置 | P1 |
| `ExcludePattern` | 单个排除模式 | P1 |
| `SerImportGraph` | @import 解析核心输出 | P2 |
| `SerImportNode` | import 图中的节点 | P2 |
| `SerImportEdge` | import 图中的边 | P2 |
| `SerImportIssue` | import 问题（cycle/depth/not_found/outside_allowlist） | P2 |

### 8.3 既有模型字段扩展评估

| 模型 | 是否新增字段 | 字段名 | 说明 |
|------|-------------|--------|------|
| `SerClaudeMemoryAsset` | ❌ 不新增 | — | 第一批次不需要额外字段。P2 中 import 关联使用 asset_id 查询，不缓存 import_count。 |
| `SerClaudeMemoryScanResult` | ❌ 不新增 | — | P1 不将其作为输入依赖，保留为独立数据结构。 |

### 8.4 兼容性约束

**v0.1 已有数据模型不修改已有字段的类型或含义，只新增独立结构体。**

原因：
1. v0.1 E2E 测试（63 passed）和前端代码依赖现有数据结构
2. 新增结构体独立序列化，不影响 v0.1 API 的返回格式
3. 若未来需要给 `SerClaudeMemoryAsset` 新增字段，使用 `Option<T>` 并同步更新前端类型定义

---

## 9. 手动对照验证计划（修订）

### 9.1 验证文档

验证结果记录到 `docs/claude-memory-v0.2-validation-log.md`，模板：

```markdown
## 验证记录 YYYY-MM-DD

### 环境
- Claude Code 版本: x.x.x
- AgentScope 版本: v0.2.x
- 操作系统: macOS/Linux/Windows

### 用例: {名称}

#### 预期行为（来自官方文档 / 推断）
...

#### /memory 命令输出
```
...
```

#### AgentScope 模拟输出
...

#### 对照结果
- [ ] 完全一致
- [ ] 有差异（差异说明: ...）
- [ ] 无法验证（原因: ...）

#### 假设更新
若验证结果与预期不符，更新设计文档相应章节。
```

### 9.2 必须验证的假设清单

#### A. 启动链顺序（来自官方文档 + 需实测对照）

| # | 验证项 | 来源 | 验证方法 | 状态 |
|---|--------|------|---------|------|
| A1 | 用户全局 ~/.claude/CLAUDE.md 始终最先加载 | 官方文档 | /memory 观察 | ⬜ |
| A2 | 从根目录到 cwd 的上级 CLAUDE.md 逐级加载 | 官方文档 | 深层目录启动，/memory 观察 | ⬜ |
| A3 | 上级目录加载顺序：根 → cwd（从上到下） | 推断 | /memory 观察 | ⬜ |
| A4 | cwd/CLAUDE.md 在上级目录之后加载 | 官方文档 | /memory 观察 | ⬜ |
| A5 | CLAUDE.local.md 在 CLAUDE.md 之后加载 | 官方文档 | /memory 观察 | ⬜ |
| A6 | 无 paths 的 rule 无条件加载 | 官方文档 | /memory 观察 | ⬜ |
| A7 | 同 scope rule 按文件名排序 | 推断 | 多 rule 测试 | ⬜ |
| A8 | Auto Memory (MEMORY.md) 在 rules 之后加载 | 推断 | /memory 观察 | ⬜ |

#### B. Path-scoped Rules（需实测对照）

| # | 验证项 | 来源 | 验证方法 | 状态 |
|---|--------|------|---------|------|
| B1 | path-scoped rules 不在启动链中加载 | 推断 | /memory 观察（确认不显示在启动列表） | ⬜ |
| B2 | paths 匹配对象是"会话中读取的文件路径" | 官方文档 | 创建 paths rule，读取匹配/不匹配文件，观察行为 | ⬜ |
| B3 | paths 使用 glob 模式（minimatch 风格） | 推断 | 测试 `**/*.rs` 等模式 | ⬜ |
| B4 | 多个 paths 模式是 OR 关系 | 推断 | 一个匹配即触发 | ⬜ |

#### C. claudeMdExcludes（需实测对照）

| # | 验证项 | 来源 | 验证方法 | 状态 |
|---|--------|------|---------|------|
| C1 | 排除模式匹配绝对路径 | 官方文档 | 配置 excludes 后 /memory 观察 | ⬜ |
| C2 | 排除模式是 glob 语法 | 推断 | 测试通配符匹配 | ⬜ |
| C3 | user/project/local 多层设置合并 | 推断 | 多层配置测试 | ⬜ |
| C4 | managed policy 不可被用户覆盖 | 推断 | 若可访问 managed settings 测试 | ⬜ |

#### D. @import（需实测对照）

| # | 验证项 | 来源 | 验证方法 | 状态 |
|---|--------|------|---------|------|
| D1 | `@path` 相对包含 import 的文件解析 | 推断 | 测试不同目录文件互相 import | ⬜ |
| D2 | `@~/path` 解析为用户 home 目录 | 推断 | 测试 `@~/test.md` | ⬜ |
| D3 | `@/absolute/path` 为绝对路径 | 推断 | 测试绝对路径 import | ⬜ |
| D4 | 循环引用处理行为 | 未知 | 创建循环 import，/memory 观察 | ⬜ |
| D5 | 深度上限（若存在） | 未知 | 创建 5+ 层嵌套 import 测试 | ⬜ |
| D6 | 代码块中的 `@path` 是否被展开（设计文档 §3.3 假设：默认忽略） | 设计文档 | CLAUDE.md 中写代码块含 @fake.md，/memory 观察 | ⬜ |
| D7 | 行内 code span 中的 `@path` 是否被展开（设计文档 §3.3 假设：默认忽略） | 设计文档 | 同上，行内 code 形式 | ⬜ |
| D8 | 外部 import（项目外路径）的 approval 语义 | 官方文档 | import 项目外文件，观察是否请求批准 | ⬜ |

#### E. Auto Memory（来自官方文档）

| # | 验证项 | 来源 | 验证方法 | 状态 |
|---|--------|------|---------|------|
| E1 | MEMORY.md 启动加载上限：200 行或 25KB | 官方文档 | 创建超大 MEMORY.md，/memory 观察截断位置 | ⬜ |
| E2 | topic 文件（非 MEMORY.md）不自动加载 | 官方文档 | /memory 观察是否有 topic 文件 | ⬜ |
| E3 | Auto Memory 是否按 **repo identity / project root** 语义匹配（而非 cwd 精确匹配） | 官方文档 | 同一 git repo 的不同子目录下对比 `/memory` 与 AgentScope 模拟结果，确认共享同一 Auto Memory | ⬜ |
| E4 | `autoMemoryDirectory` 自定义路径是否被支持 | 官方文档 | 若用户设置了 `autoMemoryDirectory`，观察 AgentScope 是否能正确查找 | ⬜ |

### 9.3 验证优先级

**第一批验证（P1 只读语义验证）**：
- A 区域启动链顺序（A0-A8 中通过现有真实资产即可观察的项）
- B 区域 path-scoped rules 展示
- warnings / excluded assets（若环境自然存在）
- Auto Memory 匹配（若当前 cwd 有可观察 Auto Memory）
- E1-E2（Auto Memory 规则）

**第一批不强制覆盖（需创建真实资产才能验证）**：
- A9 / A10 / A11：若真实 cwd 天然存在对应文件则记录，否则本批不覆盖
- E4：若使用临时隔离目录验证则允许，否则标记"本批未覆盖"

**第二批验证（P2 开发前完成）**：
- D1-D5（@import 基本行为）
- D6-D7（代码块中的 @ 处理）

**第三批验证（P1+P2 联调时完成）**：
- C1-C6（claudeMdExcludes 多层设置 + managed 载体区分）
- B3-B4（paths glob 匹配细节）
- D8（外部 import approval）
- A9-A11（祖先目录 .claude/CLAUDE.md、当前目录 .claude/CLAUDE.md、祖先 CLAUDE.local.md）

---

## 10. 第一批次实施顺序与里程碑

### 10.1 第一批次开发阶段划分

**只包含 P1 + P2（加载链模拟 + @import 检查）**

```
Phase 1：加载链模拟（P1）
├── Step 1
│   ├── 后端：claudeMdExcludes 多层读取（user/project/local）
│   ├── 后端：从 cwd 定向解析启动链（向上遍历目录层级）
│   ├── 后端：区分无条件 rules 与 path-scoped rules
│   ├── 后端：Auto Memory 匹配（200行/25KB 截断）
│   └── 单元测试：加载链顺序、排除逻辑、路径解析
│
├── Step 2
│   ├── 前端：加载模拟器页面（输入区 + 启动链结果区 + path-scoped rules 区）
│   └── E2E：加载链基础测试
│
Phase 2：@import 检查（P2）
├── Step 3
│   ├── 后端：@import 解析器（相对路径/~家目录/绝对路径）
│   ├── 后端：循环引用检测（DFS）
│   ├── 后端：深度上限检查（默认 5 层）
│   ├── 后端：外部 import 标记（outside_allowlist/approval-unknown）
│   └── 单元测试：import 解析、循环检测、深度检测
│
├── Step 4
│   ├── 前端：import 展开面板（嵌入加载模拟器）
│   ├── 前端：issue 可视化（cycle/not_found/outside_allowlist）
│   └── E2E：import 检查测试
│
Phase 3：手动对照验证
├── 持续进行（与开发并行）
│   ├── 启动链顺序验证（§9.2 A 组）
│   ├── path-scoped rules 验证（§9.2 B 组）
│   ├── @import 行为验证（§9.2 D 组）
│   └── 记录到 docs/claude-memory-v0.2-validation-log.md
```

### 10.2 第一批次退出条件

**P1 阶段退出条件（当前，修订）**：
1. P1 加载链模拟功能完整，E2E 测试通过
2. 与 Claude Code `/memory` 命令在 2 个以上 cwd 完成手动对照验证；验证口径为：
   - **文件存在性/识别项对照**：Claude `/memory` 中显示的记忆项，AgentScope 是否也识别到？
   - **顺序规则独立校验**：AgentScope 输出是否符合官方文档描述的加载规则？（不直接对比 `/memory` UI）
   - **差异项记录**：记录 `/memory` UI 不可观察但 AgentScope 可展示的项（如 managed CLAUDE.md、祖先目录链）
   - 可优先使用"项目根目录 + 同项目深层子目录"，不强制必须是 2 个不同项目
3. Auto Memory 匹配策略已按 repo identity / project root 语义修复（同一 git repo 的不同子目录共享 Auto Memory）
4. 不引入 SQLite 等重型依赖（保持 v0.1 的架构约束）
5. 三平台 CI 构建通过
6. §9.3 中列为"第一批验证"的现有资产可观察项已完成验证（C 组保留在后续批次/条件满足时验证）

**P2 阶段退出条件（后续）**：
1. @import 功能完整，E2E 测试通过
2. @import 行为手动对照验证通过
3. D 组假设项（D1-D8）已完成验证

### 10.3 P3/P4 排期

P3（健康检测增强）和 P4（编辑能力）在第一批次完成后评估：
- 若第一批次按时完成，P3 作为 v0.2 第二批次
- 若第一批次超预期，P3/P4 拆分为 v0.2.5 或 v0.3

---

## 11. 风险与应对（第一批次）

| 风险 | 影响 | 应对 |
|------|------|------|
| Claude Code 加载链规则未文档化 | 高 | 以 `/memory` 命令输出为 ground truth，§9 验证计划中所有假设需实测验证 |
| paths 匹配行为与 Claude 不一致 | 中 | path-scoped rules 单独展示（B 区域），不与启动链混排，降低误报影响 |
| @import 语法有多变体 | 中 | 先支持 `@path`、`@~/path`、`@/absolute` 三种形式，代码块中的 @ 处理列为待验证项 |
| 代码块中的 @ 被误解析为 import | 中 | 标记为 §3.5 限制说明，验证后再决定是否增加 Markdown 结构感知 |
| claudeMdExcludes glob 匹配与 Claude 不一致 | 中 | 使用标准 glob crate，记录与 Claude 行为的偏差 |
| file-based managed settings 不可访问导致信息缺失 | 低 | 检测到存在但不可读时展示 warning，不阻断加载链模拟。server-managed / MDM 等非 file-based 来源标记为 limitation |
| 加载链模拟性能差 | 低 | 定向解析（只读 cwd 祖先目录 + ~/.claude/），不是全量扫描 |

---

## 12. 附录：第一批次新增/修改文件清单（预估）

### 新增文件

| 文件 | 说明 |
|------|------|
| `src-tauri/src/collectors/claude_memory/load_chain.rs` | 加载链模拟器（定向解析、启动链构建、path-scoped 分离） |
| `src-tauri/src/collectors/claude_memory/import_resolver.rs` | @import 解析器（相对路径/~家目录/绝对路径、循环检测、深度检查） |
| `src-tauri/src/collectors/claude_memory/settings_reader.rs` | settings 多层读取：user/project/local 的 settings.json + file-based managed 的 managed-settings.json / managed-settings.d/*.json |
| `src/features/claude-memory/pages/LoadChainSimulator.tsx` | 加载模拟器页面（输入区 + 启动链结果 + path-scoped 区 + import 展开） |
| `src/features/claude-memory/components/ImportTree.tsx` | Import 展开树组件 |
| `src/features/claude-memory/components/LoadChainStep.tsx` | 加载链步骤项组件 |
| `e2e/claude-memory-load-chain.spec.ts` | 加载链 E2E 测试 |
| `docs/claude-memory-v0.2-validation-log.md` | 手动验证日志（按 §9.1 模板） |

### 修改文件

| 文件 | 变更 |
|------|------|
| `src-tauri/src/collectors/claude_memory/models.rs` | 新增 SerLoadChain、SerLoadChainStep、SerPathScopedRule、SerImportGraph 等结构体 |
| `src-tauri/src/collectors/claude_memory/mod.rs` | 导出新增模块 |
| `src-tauri/src/services/claude_memory_service.rs` | 新增 simulate_load_chain、get_import_graph service 函数 |
| `src-tauri/src/routes/claude_memory.rs` | 注册 simulate_claude_memory_load_chain、get_claude_memory_import_graph 命令 |
| `src-tauri/src/lib.rs` | 注册新命令到 generate_handler |
| `src/App.tsx` | ClaudeMemoryPage 扩展子页面类型（增加"加载模拟器"） |
| `src/components/Sidebar.tsx` | Claude 记忆域增加"加载模拟器"子导航项 |
| `src/features/claude-memory/index.tsx` | 扩展为主页面路由分发（增加加载模拟器路由） |
| `src/features/claude-memory/hooks/useClaudeMemory.ts` | 新增 P1 + P2 的 API 调用 |
| `src/features/claude-memory/types.ts` | 新增前端类型（LoadChain、ImportGraph 等） |

### 不在第一批次的文件（P3/P4 保留设计）

| 文件 | 说明 | 排期 |
|------|------|------|
| `src-tauri/src/collectors/claude_memory/health_checker.rs` | 健康检测增强 | P3 |
| `src-tauri/src/collectors/claude_memory/edit_service.rs` | 编辑服务 | P4 |
| `src/features/claude-memory/pages/HealthCheck.tsx` | 健康检测页面 | P3 |
| `src/features/claude-memory/components/HealthScoreCard.tsx` | 健康评分卡 | P3 |
| `src/features/claude-memory/components/EditPanel.tsx` | 编辑面板 | P4 |
| `e2e/claude-memory-health.spec.ts` | 健康检测 E2E | P3 |
