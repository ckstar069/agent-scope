# AgentScope — Claude Code 记忆管理需求草案

> 状态：草案 v0.2
> 创建日期：2026-05-21
> 关联需求：通用 Claude Code 记忆管理能力（非模板项目专属）

---

## 1. 产品定位

AgentScope 的记忆管理功能不是"Claude 文件浏览器"，而是 **Claude Code context injection chain 治理平台**。

核心价值在于回答以下问题：
1. 我现在启动 Claude，实际会加载哪些记忆？
2. 哪些是全局的，哪些是项目的，哪些是本机私有的？
3. 哪些内容重复、冲突、过期、太长？
4. 哪些内容应该从 CLAUDE.md 拆成 rules？
5. 哪些内容应该从 CLAUDE.md 拆成 skill？
6. 哪些内容是 Claude 自动写入的 auto memory？
7. 哪些 subagent 有自己的 memory？
8. 哪些记忆没有同步到另一台主力机？
9. 哪些记忆里可能泄露了 secret？

**三端同步（Windows / macOS / Linux）不是附加功能，而是底座设计。** 从第一天起，所有模块必须按跨平台架构设计，避免后期重写。

---

## 2. 背景与问题

### 2.1 当前现状

AgentScope 现有的"记忆"功能**仅针对模板项目**（ai_project_template 创建的项目）：

| 现有功能 | 实际范围 | 限制 |
|---------|---------|------|
| L1 静态记忆浏览 | 扫描已注册项目的白名单目录（CLAUDE.md、.claude/rules、.sisyphus/notepads） | 只显示已注册项目内的文件 |
| L2 记忆标记沉淀 | 写入 `{project_path}/.sisyphus/notepads/project-memory/decisions.md` | 仅适用于 ai_project_template 结构 |
| 文件树展示 | 通过 `ProjectFilesCollector` 收集 | 白名单硬编码模板项目路径 |

**核心问题**：AgentScope 无法管理通用 Claude Code 的记忆。用户在任意目录启动 Claude Code 时产生的记忆文件（`~/.claude/` 下的全局配置、项目记忆、规则等）完全不可见。

### 2.2 用户痛点

1. **记忆分散不可见**：`~/.claude/CLAUDE.md`、`.claude/rules/`、`~/.claude/projects/<id>/memory/` 等文件分散在不同层级，用户难以概览全部记忆
2. **加载链不透明**：从某个目录启动 Claude 时，实际加载了哪些 CLAUDE.md、哪些 rules，用户只能依赖 `/memory` 命令的文本输出
3. **记忆质量无检测**：CLAUDE.md 过长（超过 200 行）、不同文件间冲突（如一个说 pnpm 一个说 npm）、敏感信息泄露等问题无工具检测
4. **Auto Memory 不可控**：Claude 自动写入的 `~/.claude/projects/<id>/memory/` 内容不断增长，用户无法有效归档和清理
5. **双机同步困难**：Auto Memory 是本机级的，不会在 Mac 和 PC 之间同步

---

## 3. 核心目标

> **一句话目标**：让 AgentScope 成为 Claude Code 记忆的可视化治理平台，回答"我现在启动 Claude，实际会加载哪些记忆？"

具体目标：

| 目标 | 说明 |
|------|------|
| **可视化** | 显示 Claude Code 记忆的完整层级结构和加载链 |
| **可管理** | 支持对记忆文件的增删改查 |
| **可分析** | 检测记忆健康问题（过长、冲突、敏感信息） |
| **可模拟** | 模拟从任意目录启动 Claude 时的记忆加载情况 |
| **可同步** | 跨 Windows / macOS / Linux 三端同步记忆资产 |

---

## 4. 术语定义

| 术语 | 定义 |
|------|------|
| **Instruction Memory** | 用户显式编写的指令文件：CLAUDE.md、CLAUDE.local.md、.claude/CLAUDE.md |
| **Rule Memory** | `.claude/rules/*.md` 文件，支持全局加载或路径触发 |
| **Auto Memory** | Claude 自动写入的记忆：`~/.claude/projects/<project>/memory/` 下的文件 |
| **记忆加载链** | 从某个 cwd 启动 Claude 时，按加载顺序拼接的所有记忆文件 |
| **作用域** | 记忆生效的范围：managed（组织级）、user（全局）、project（项目级）、local（本机私有）、auto（自动写入）、runtime（运行期） |
| **Host Profile** | 一台运行 AgentScope 的主机身份（host_id, hostname, os, home_dir, claude_config_dir） |
| **Project Identity** | 项目的逻辑身份。Claude Code 的 Auto Memory 匹配策略：普通 git repo 内以 **repo root 路径编码** 识别（子目录共享 Auto Memory）；非 git 目录以 **cwd 路径编码** 识别。用户可通过 `autoMemoryDirectory` 自定义存储位置。AgentScope P1 已支持普通 repo 子目录共享；git worktree 共享与 `autoMemoryDirectory` 为 limitation |
| **Logical Asset** | 跨平台抽象后的记忆资产，同一资产在不同 OS 上路径不同但逻辑身份相同 |
| **Native Path** | 某平台上该记忆资产的实际文件系统路径 |
| **Normalized Path** | 统一使用 `/` 分隔符的平台无关路径表示 |

---

## 5. 架构设计

### 5.1 核心流程

记忆管理功能的整体数据流：

```
Host Profile
    ↓
Path Resolver（优先 CLAUDE_CONFIG_DIR，回退平台默认值）
    ↓
Memory Scanner（扫描文件系统，生成 logical asset）
    ↓
Project Identity Matcher（普通 git repo：编码 repo root 路径；非 git：编码 cwd 路径）
    ↓
Conflict Detector（内容冲突 / 路径冲突 / 语义冲突 / 安全冲突）
    ↓
Sync / Backup / Restore
    ↓
Cross-platform Path Mapping（native_path ↔ normalized_path ↔ logical asset）
```

### 5.2 模块职责

| 模块 | 职责 |
|------|------|
| **Platform Adapter** | 识别当前操作系统，提供平台特定的路径常量 |
| **Path Resolver** | 解析 CLAUDE_CONFIG_DIR 环境变量，生成各类型记忆的标准路径 |
| **Memory Scanner** | 遍历文件系统，发现所有记忆资产，计算 content hash |
| **Project Identity Matcher** | 普通 git repo：编码 **repo root** 路径匹配 Auto Memory 目录（子目录共享同一 repo 的 Auto Memory）；非 git 目录：编码 **cwd** 路径匹配；git worktree 与 `autoMemoryDirectory` 自定义路径为 P1 limitation |
| **Load Chain Simulator** | 模拟从任意 cwd 启动 Claude 时的记忆加载顺序（只读模拟，不修改任何文件） |
| **Conflict Detector** | 检测内容冲突、路径冲突、语义冲突、安全冲突 |
| **Sync Policy Engine** | 执行语义同步：扫描 → hash → 解析 → 对比 → merge / conflict / backup |
| **Secret Scanner** | 本地正则匹配，检测 API key、token、密码等敏感信息 |

---

## 6. 路径解析与平台适配

### 6.1 路径解析规则

1. **优先读取环境变量**：若设置了 `CLAUDE_CONFIG_DIR`，所有 `~/.claude` 路径替换为该目录
2. **回退平台默认值**：
   - macOS / Linux: `~/.claude/` → `$HOME/.claude/`
   - Windows: `~/.claude/` → `%USERPROFILE%\.claude\`
3. **项目级路径**：以项目根目录为基准，三端路径结构一致，仅分隔符不同
4. **组织级 managed 路径**：
   - macOS: `/Library/Application Support/ClaudeCode/`
   - Linux / WSL: `/etc/claude-code/`
   - Windows: `C:\Program Files\ClaudeCode\`

### 6.2 用户级路径对照表

| 类型 | macOS / Linux | Windows |
|------|--------------|---------|
| 用户 Claude 配置根目录 | `~/.claude/` | `%USERPROFILE%\.claude\` |
| 用户全局指令 | `~/.claude/CLAUDE.md` | `%USERPROFILE%\.claude\CLAUDE.md` |
| 用户全局 rules | `~/.claude/rules/*.md` | `%USERPROFILE%\.claude\rules\*.md` |
| 用户全局 skills | `~/.claude/skills/*/SKILL.md` | `%USERPROFILE%\.claude\skills\*\SKILL.md` |
| 用户全局 subagents | `~/.claude/agents/*.md` | `%USERPROFILE%\.claude\agents\*.md` |
| 用户全局 agent memory | `~/.claude/agent-memory/<agent>/` | `%USERPROFILE%\.claude\agent-memory\<agent>\` |
| auto memory（默认路径） | `~/.claude/projects/<project>/memory/` | `%USERPROFILE%\.claude\projects\<project>\memory\` |
| auto memory（用户自定义 `autoMemoryDirectory`） | 以 `settings.json` 中 `autoMemoryDirectory` 字段为准 | 同上 |
| 全局 settings | `~/.claude/settings.json` | `%USERPROFILE%\.claude\settings.json` |
| 会话历史 | `~/.claude/history.jsonl` | `%USERPROFILE%\.claude\history.jsonl` |

### 6.3 项目级路径对照表

| 类型 | 路径（统一结构，仅分隔符不同） |
|------|------------------------------|
| 项目主记忆 | `<repo>/CLAUDE.md` |
| 项目 Claude 目录 | `<repo>/.claude/` |
| 项目 rules | `<repo>/.claude/rules/*.md` |
| 项目 settings | `<repo>/.claude/settings.json` |
| 本地私有 settings | `<repo>/.claude/settings.local.json` |
| 项目 skills | `<repo>/.claude/skills/*/SKILL.md` |
| 项目 commands | `<repo>/.claude/commands/*.md` |
| 项目 subagents | `<repo>/.claude/agents/*.md` |
| 项目 subagent memory | `<repo>/.claude/agent-memory/<agent>/` |
| 本地私有指令 | `<repo>/CLAUDE.local.md` |

### 6.4 组织级 managed 路径对照表

| 系统 | Managed CLAUDE.md / settings 路径 |
|------|----------------------------------|
| macOS | `/Library/Application Support/ClaudeCode/` |
| Linux / WSL | `/etc/claude-code/` |
| Windows | `C:\Program Files\ClaudeCode\` |

> 注：Windows 旧路径 `C:\ProgramData\ClaudeCode\managed-settings.json` 从 Claude Code v2.1.75 起不再支持，管理员应迁移到 `C:\Program Files\ClaudeCode\`。

### 6.5 跨平台路径处理要求

- 后端使用 `std::path::PathBuf`，禁止硬编码路径分隔符
- 前端显示路径时统一使用 `/` 分隔符（用户层面），底层读写按平台原生格式处理
- Windows 下 Registry 加载自动剥离 `\\?\` 前缀（复用现有逻辑）
- WSL 与 Windows 原生视为独立运行时，各自有独立的 Host Profile

---

## 7. 数据模型

### 7.1 设计原则

数据库从 **v0.1 第一天起按多主机设计**，预留跨设备同步所需字段。v0.1 只操作当前主机一条记录，v0.3 跨设备同步时无需修改表结构。

### 7.2 表结构

#### hosts

```sql
CREATE TABLE hosts (
  id TEXT PRIMARY KEY,
  hostname TEXT NOT NULL,
  os TEXT NOT NULL,              -- macos | linux | windows
  os_version TEXT,
  home_dir TEXT NOT NULL,
  claude_config_dir TEXT NOT NULL,
  user_name TEXT,
  last_seen_at TEXT,
  is_local INTEGER DEFAULT 1
);
```

#### projects

```sql
CREATE TABLE projects (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  remote_url TEXT,               -- git remote URL（v0.3 跨主机同步/项目关联候选字段；P1 Auto Memory 使用 encode_cwd_path 匹配，不依赖此字段）
  repo_hash TEXT,                -- git repo 特征哈希
  created_at TEXT,
  updated_at TEXT
);
```

#### project_paths

```sql
CREATE TABLE project_paths (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  host_id TEXT NOT NULL,
  native_path TEXT NOT NULL,     -- 该平台上的实际路径
  normalized_path TEXT NOT NULL, -- 统一 / 分隔符的路径
  is_active INTEGER DEFAULT 1
);
```

#### memory_assets

```sql
CREATE TABLE memory_assets (
  id TEXT PRIMARY KEY,
  host_id TEXT NOT NULL,
  project_id TEXT,
  scope TEXT NOT NULL,           -- managed | user | project | local | auto | runtime
  asset_type TEXT NOT NULL,      -- 见下表
  logical_path TEXT NOT NULL,    -- 跨平台逻辑路径
  native_path TEXT NOT NULL,     -- 该平台上的实际路径
  content_hash TEXT,
  last_scanned_at TEXT,
  last_modified_at TEXT,
  sync_policy TEXT               -- auto_sync | manual | exclude
);
```

#### asset_type 枚举

| 值 | 说明 |
|----|------|
| `claude_md` | CLAUDE.md（项目级） |
| `claude_local_md` | CLAUDE.local.md |
| `user_claude_md` | `~/.claude/CLAUDE.md` |
| `rule` | `.claude/rules/*.md` |
| `global_rule` | `~/.claude/rules/*.md` |
| `skill` | `skills/*/SKILL.md` |
| `command` | `commands/*.md` |
| `agent` | `agents/*.md` |
| `agent_memory` | `agent-memory/<name>/` |
| `auto_memory_index` | `MEMORY.md` |
| `auto_memory_topic` | topic 文件（debugging.md 等） |
| `settings` | `settings.json` |
| `transcript` | 会话历史 `.jsonl` |
| `history` | `history.jsonl` |

---

## 8. Claude Code 记忆层级（完整参考）

按作用域从大到小：

| 层级 | 路径/机制 | 类型 | 作用范围 | v0.1-v0.4 支持计划 |
|------|----------|------|---------|-------------------|
| 0 | System Prompt / `--append-system-prompt` | 启动级约束 | 当前 invocation | 否 |
| 1 | Managed CLAUDE.md（组织级） | 组织指令 | 整台机器/企业 | 企业版支持 |
| 2 | `~/.claude/CLAUDE.md` | 用户全局指令 | 当前用户所有项目 | **v0.1** |
| 3 | `~/.claude/rules/*.md` | 用户全局规则 | 当前用户所有项目 | **v0.1** |
| 4 | `./CLAUDE.md` | 项目指令 | 当前项目 | **v0.1** |
| 5 | `./.claude/CLAUDE.md` | 项目指令 | 当前项目 | **v0.1** |
| 6 | `./CLAUDE.local.md` | 本地私有指令 | 当前项目 | **v0.1** |
| 7 | `.claude/rules/*.md` | 项目规则 | 当前项目 | **v0.1** |
| 8 | `@path/to/file` imports | 记忆拆分/复用 | 由引用关系决定 | **v0.2** |
| 9 | `~/.claude/projects/<project>/memory/` | Auto memory | 每个 git repo | **v0.1** |
| 10 | `.claude/agents/*.md` / `~/.claude/agents/*.md` | Subagent 定义 | 项目或用户 | **v0.1**（只读展示），v0.2 支持编辑 |
| 11 | `agent-memory/<name>/` | Subagent 独立记忆 | user/project/local | **v0.2** |
| 12 | `.claude/skills/*/SKILL.md` | 流程记忆 | 项目或用户 | **v0.1**（只读展示），v0.2 支持编辑 |
| 13 | `~/.claude/projects/<project>/<session>.jsonl` | 会话历史 | 单会话 | **v0.4**（只读审计） |
| 14 | `~/.claude/history.jsonl` | Prompt 历史 | 当前用户 | 谨慎支持 |

---

## 9. 功能需求

### 9.1 v0.1 — 本机三端兼容扫描器

#### FR-01：Instruction Memory 扫描与查看

| 项 | 内容 |
|----|------|
| 功能 | 扫描并查看 CLAUDE.md 系列文件 |
| 支持文件 | `~/.claude/CLAUDE.md`、`./CLAUDE.md`、`./.claude/CLAUDE.md`、`./CLAUDE.local.md` |
| 操作（v0.1 必需） | 扫描发现、查看内容、统计行数/大小 |
| 操作（v0.1 非目标） | 编辑内容、创建新文件、删除文件 |
| 操作（v0.2 增强） | 编辑内容、创建新文件 |
| 约束 | 编辑前自动备份原文件；限制可编辑文件类型为 `.md` |

**验收标准**：
- 能正确扫描并读取上述 4 个位置的 CLAUDE.md 文件（如存在）
- 能显示文件行数、字节数、最后修改时间

#### FR-02：Auto Memory 扫描与查看

| 项 | 内容 |
|----|------|
| 功能 | 扫描并查看 Claude 自动写入的记忆 |
| 路径 | 默认 `~/.claude/projects/<project>/memory/`。普通 git repo 由 repo/project identity 派生，AgentScope P1 用 repo root 路径编码近似匹配；非 git 目录回退 cwd 路径编码；worktree 与 `autoMemoryDirectory` 为 P1 limitation |
| 操作（v0.1 必需） | 扫描发现、查看列表、查看内容、统计行数/大小 |
| 操作（v0.1 非目标） | 编辑内容、删除文件 |
| 操作（v0.2 增强） | 编辑内容、删除文件 |
| 特殊处理 | MEMORY.md 显示行数和大小（提示用户前 200 行/25KB 限制） |

**验收标准**：
- 列出指定项目的 auto memory 目录下所有 `.md` 文件
- 显示每个文件的行数和字节数
- MEMORY.md 超出 200 行时给出警告提示

#### FR-03：Rule Memory 浏览

| 项 | 内容 |
|----|------|
| 功能 | 浏览 `.claude/rules/*.md` 文件 |
| 路径 | `~/.claude/rules/*.md`（全局）、`.claude/rules/*.md`（项目级） |
| 操作 | 查看规则列表、查看规则内容、识别 paths frontmatter |
| 展示 | 带 paths 的规则显示触发路径；无 paths 的规则显示"全局加载" |

**验收标准**：
- 正确列出全局和项目级 rules 文件
- 解析并显示 frontmatter 中的 `paths` 字段
- 区分"启动加载"和"路径触发"两类规则

#### FR-04：Skills 与 Agents 只读扫描

| 项 | 内容 |
|----|------|
| 功能 | 扫描并只读展示 Skills 和 Agents 定义文件 |
| 支持文件 | `~/.claude/skills/*/SKILL.md`（用户级）、`<repo>/.claude/skills/*/SKILL.md`（项目级）、`~/.claude/agents/*.md`（用户级）、`<repo>/.claude/agents/*.md`（项目级） |
| 操作（v0.1） | 扫描发现、查看列表、查看内容、解析并显示元数据（name、description、trigger、memory scope 等） |
| 操作（v0.2 增强） | 编辑、创建、删除 |

**frontmatter 缺失策略**：
- 若 skill / agent 文件没有 frontmatter：
  - `name` 使用文件夹名（skill）或文件名（agent）兜底
  - `description`、`trigger`、`memory scope` 等字段显示为「未声明」
  - 不把缺少 frontmatter 视为扫描错误

**验收标准**：
- 正确扫描用户级和项目级的 skills 目录，列出所有 `SKILL.md`
- 正确扫描用户级和项目级的 agents 目录，列出所有 `.md`
- 解析并显示 skill 的 name、description、trigger 等元数据
- 解析并显示 agent 的 name、description、memory scope 等元数据
- 文件不存在时显示空状态，不报错

#### FR-05：基础敏感信息扫描

| 项 | 内容 |
|----|------|
| 功能 | 本地正则匹配扫描敏感信息 |
| 检测项 | API key、token、密码、私有 URL、.env 内容 |
| 约束 | 不将内容发送到外部服务，纯本地正则匹配 |

**验收标准**：
- 扫描所有记忆文件内容
- 命中敏感模式时高亮提示
- 提供忽略误报的机制

### 9.2 v0.2 — 项目级加载模拟与规则增强

#### FR-06：记忆加载链模拟

| 项 | 内容 |
|----|------|
| 功能 | 模拟从指定目录启动 Claude Code 时的记忆加载情况 |
| 输入 | 任意用户可访问目录（不限于已注册项目，不做系统目录黑名单） |
| 输出 | 按加载顺序排列的记忆文件列表 |
| 逻辑 | 从输入目录向上遍历，收集 CLAUDE.md 链；收集 rules 文件；排除 `claudeMdExcludes` 中配置的文件 |
| 错误处理 | 目录不存在、无权限、不是目录时返回友好错误；扫描过程不因单个不可读文件中断整体模拟 |

**加载顺序**（与 Claude Code 官方行为一致）：

**A. 启动时确定性加载链**：
1. managed CLAUDE.md（若可访问）
   - file-based managed instruction，路径因平台而异
   - 若存在且可读则加入启动链；不可读时记录 warning
2. 用户全局 `~/.claude/CLAUDE.md`
3. 从根目录到 cwd 的上级目录链（逐级向下），每层检查：
   - `CLAUDE.md`（✅ 官方确认）
   - `CLAUDE.local.md`（✅ 官方确认）
   - `.claude/CLAUDE.md`（⚠️ A9 推断，第一版不纳入启动链；A9 验证确认后补充）
4. 当前目录 `./CLAUDE.md`（✅ 官方确认）
5. 当前目录 `./.claude/CLAUDE.md`（✅ 官方明确支持的项目 instruction）
6. 当前目录 `./CLAUDE.local.md`（✅ 官方确认）
7. 全局无条件 rules（`~/.claude/rules/**/*.md`，无 paths 的，递归子目录）
8. 项目级无条件 rules（`./.claude/rules/**/*.md`，无 paths 的，递归子目录）
9. Auto Memory（`~/.claude/projects/<project>/memory/MEMORY.md`，前 200 行或前 25KB，取先到者）
   - `<project>` 匹配：普通 git repo 用 **repo root 路径编码**；非 git 目录用 **cwd 路径编码**；worktree / `autoMemoryDirectory` 为 P1 limitation

**B. 运行时条件加载（path-scoped rules）**：
- 有 paths 的 rules 不在启动时加载，而是在会话中读取匹配文件时触发
- 启动模拟结果中 path-scoped rules 应与启动链分开展示

**Auto Memory 限制**：
- 仅 `MEMORY.md` 启动加载，topic 文件不自动加载
- 加载上限：200 行 **或** 25KB，取先达到者

**claudeMdExcludes 来源**：
- user 层：`~/.claude/settings.json`
- project 层：`<repo>/.claude/settings.json`
- local 层：`<repo>/.claude/settings.local.json`
- managed 层（file-based）：`managed-settings.json` + `managed-settings.d/*.json`
  - 仅覆盖 file-based managed settings；server-managed / MDM 等非 file-based 来源标记为 limitation
  - 若存在但不可读，记录 warning 并继续模拟

**验收标准**：
- 输入任意可访问目录路径（已注册项目或任意目录），输出完整的启动加载链
- 正确显示每个文件的绝对路径和作用域
- path-scoped rules 单独列出，不与启动链混排
- 被 `claudeMdExcludes` 排除的文件标记为"已排除"，并显示排除来源
- 显示 rules 是否 path-scoped（无条件加载 vs 路径触发）

#### FR-07：@import 解析

- 解析 CLAUDE.md 中的 `@path/to/file` import 语法
- `@path` 相对**包含 import 的文件**所在目录解析
- `@~/path` 解析为用户 home 目录（`$HOME` / `%USERPROFILE%`）
- `@/absolute/path` 为绝对路径
- 显示 import 关系图
- 检测循环引用和超过 5 层深度的问题
- 外部 import（allowlist 外路径）标记为 `outside-read-scope` 或 `approval-unknown`，不直接断言"Claude 不会加载"

#### FR-08：Rule Memory 编辑

- 支持创建、编辑、删除 rules 文件
- 编辑 frontmatter（paths、description 等）
- 路径触发规则的测试（验证 paths 模式是否匹配目标文件）

#### FR-09：记忆健康检测

| 检测项 | 说明 | 优先级 |
|--------|------|--------|
| 过长检测 | CLAUDE.md > 200 行，MEMORY.md > 200 行 或 > 25KB | 高 |
| 敏感信息扫描 | 正则匹配 API key、token、密码、私有 URL 等模式 | 中 |
| 冲突检测 | 语义层面的冲突识别（如 pnpm vs npm） | 低（需 NLP） |

#### FR-10：Settings 可视化

- 读取 `~/.claude/settings.json`、`.claude/settings.json`、`.claude/settings.local.json`
- 显示 `autoMemoryEnabled`、`claudeMdExcludes`、`hooks`、`permissions` 等关键字段
- 只读展示，暂不支持编辑

#### FR-11：Subagent 与 Skills 编辑管理

- 在 v0.1 只读扫描基础上，增加创建、编辑、删除能力
- 支持编辑 `.claude/agents/*.md` 和 `~/.claude/agents/*.md`
- 支持编辑 `agent-memory/<name>/` 下的记忆文件
- 支持编辑 `.claude/skills/*/SKILL.md`

### 9.3 v0.3 — 跨设备同步

#### FR-12：多主机注册

- 注册多台 Host Profile
- 用户全局记忆同步（`~/.claude/CLAUDE.md`、`~/.claude/rules/`）
- Auto memory 同步
- Project identity 绑定：跨设备同步阶段需显式处理 project identity。不同场景下的 identity 规则不同：普通 git repo 以 repo root 路径编码派生；非 git 目录以 cwd 路径编码派生；worktree / `autoMemoryDirectory` 自定义路径需单独处理。不再把单一 cwd 编码写成 Claude Code 全部场景的 identity 规则

#### FR-13：语义同步

推荐同步模型：

```
扫描文件
    ↓
计算 hash
    ↓
解析 frontmatter / imports / paths
    ↓
生成 logical asset
    ↓
与数据库上次版本对比
    ↓
判断变更来源
    ↓
执行 merge / conflict / backup
    ↓
写回目标平台真实路径
```

#### FR-14：冲突检测与处理

| 冲突级别 | 示例 | 处理方式 |
|---------|------|---------|
| 无冲突 | Mac 修改，Windows 未修改 | 自动同步 |
| 内容冲突 | Mac 和 Windows 都改了同一个 CLAUDE.md | 人工合并 |
| 路径冲突 | rule 中写死 `/Users/ckstar/Repo` | 提示改成变量 |
| 语义冲突 | 一个说用 npm，一个说用 pnpm | 高亮提示 |
| 安全冲突 | 记忆里出现 API key | 阻止同步 |

#### FR-15：同步前自动备份

- 同步前自动备份目标文件到 `~/.claude/backups/`
- 备份失败时阻止同步操作

### 9.4 v0.4 — 治理能力

#### FR-16：高级冲突语义检测

- 重复记忆检测
- 大文件拆分建议（CLAUDE.md → rules / skills）
- 记忆健康评分（重复、冲突、过长、过期、无引用）

#### FR-17：会话 Transcript 分析

- 只读分析 `~/.claude/projects/<project>/<session>.jsonl`
- 从历史会话中提取候选记忆建议
- 安全审计：扫描是否泄露 API key、token、.env 内容

#### FR-18：自动建议写入位置

- 判断内容该放 CLAUDE.md、rules、skill、auto memory 还是 agent memory
- 基于内容类型和项目结构给出建议

#### FR-19：版本管理

- 对记忆变更做 diff、rollback、tag

#### FR-20：多工具兼容

- 同步生成 AGENTS.md、Codex、OpenCode 适配文件

---

## 10. 非功能需求

### 10.1 性能

- 记忆加载链模拟应在 500ms 内完成（普通 SSD 环境）
- 文件列表加载应在 200ms 内完成
- 扫描整台主机的全部记忆资产应在 3 秒内完成

### 10.2 安全

- 编辑前自动备份原文件到 `~/.claude/backups/`
- 敏感信息扫描不将内容发送到外部服务（本地正则匹配）
- 不暴露 SSH 密钥、.env 文件等敏感路径
- 同步前必须完成备份，备份失败阻止操作

### 10.3 跨平台兼容性（必须支持 Windows / macOS / Linux）

记忆管理功能必须同时支持三个桌面平台，具体要求：

| 平台 | `~/.claude/` 实际路径 | 特殊处理 |
|------|----------------------|---------|
| **macOS** | `/Users/<user>/.claude/` | 标准 Unix 路径 |
| **Linux** | `/home/<user>/.claude/` | 标准 Unix 路径 |
| **Windows** | `C:\Users\<user>\.claude\` | 路径分隔符转换；自动处理 `\\?\` 前缀 |

**实现要求**：
- 后端使用 `std::path::PathBuf`，禁止硬编码路径分隔符
- 前端显示路径时统一使用 `/` 分隔符（用户层面），底层读写按平台原生格式处理
- Windows 下 Registry 加载自动剥离 `\\?\` 前缀（复用现有逻辑）
- 三平台 CI 构建均通过（Linux Docker、Windows Shell、macOS 本机）

**测试覆盖**：
- macOS：本机开发验证
- Linux：3.50 实体机验证
- Windows：3.10 实体机验证

### 10.4 错误处理

- 文件不存在时显示空状态，不报错
- 权限不足时显示友好提示
- 备份失败时阻止编辑操作
- 扫描到损坏文件时跳过并记录，不中断整体扫描

### 10.5 Claude 资产安全原则（P1-P4 通用）

> 以下原则适用于 AgentScope 所有与 Claude Code 资产交互的功能，无论当前实现阶段。

**只读优先原则**：
- P1（加载链模拟）为纯只读功能：只扫描、不写入、不修改、不删除任何文件
- 加载链模拟器读取用户真实 `~/.claude/` 和项目目录，但绝不回写

**写操作隔离原则**：
- 任何写入操作（P4 及以后）必须通过明确的 allowlist 校验
- 禁止写入 `~/.claude/projects/` 下的 Auto Memory 目录（Claude Code 独占区域）
- 禁止修改 managed settings（`/Library/Application Support/ClaudeCode/`、`/etc/claude-code/` 等）
- 所有写入操作前先自动备份原文件到 `.backups/` 目录

**测试隔离原则**：
- 单元测试和 E2E 测试绝不触碰真实 `~/.claude/` 目录
- 测试使用临时目录 + `CLAUDE_CONFIG_DIR` 环境变量隔离
- 测试目录命名包含进程 ID 和时间戳，避免并发冲突

**验证阶段安全约束**：
- P1 语义验证（与 `/memory` 命令对照）为纯观察行为，不修改任何真实资产
- 验证过程中若发现需要创建测试文件，必须在临时目录或专用测试项目中进行
- 验证完成后清理所有临时测试数据

---

## 11. 界面位置

Claude 记忆管理作为 **Claude Code** 大域的子功能，与 Agent 监控、会话管理并列：

```
顶部大域导航
├── 模板项目        (现有)
├── Claude Code     (通用监控 + 记忆管理合并)
│   ├── Agent 监控
│   ├── 会话管理
│   ├── 记忆资产
│   └── 加载链模拟器
└── 设置            (现有)
    ├── 项目设置    (现有)
    └── 通用设置    (现有)
```

Claude Code 域内部结构：

```
Claude Code
├── 监控
│   ├── Agent 监控      (实时 Token 消耗、上下文窗口、会话状态)
│   └── 会话管理        (Claude Code 会话历史列表)
└── 记忆
    ├── 记忆资产         (记忆文件扫描、详情、风险检测)
    └── 加载链模拟器      (模拟启动时的记忆加载顺序)
```

合并理由：
1. Agent 监控、会话管理、记忆资产均围绕 Claude Code 运行时的上下文输入展开，属于同一功能域
2. 减少顶部大域数量（从 4 个合并为 3 个），降低认知负担
3. 侧边栏子导航已支持分组（监控 / 记忆），扩展性足够承载 4 个子页

---

## 12. 技术约束

### 12.1 后端约束

- 新增命令需遵循现有分层架构：`routes/` → `services/` → 文件系统操作
- 路径处理使用 `std::path::PathBuf`，自动处理跨平台路径分隔符
- 文件读写使用原子操作（先写临时文件再重命名），避免写入中断导致文件损坏
- 数据库从 v0.1 起按多主机设计，预留 host_id、sync_policy 等字段

### 12.2 前端约束

- v0.1 不实现编辑器，仅复用 MarkdownRenderer 做只读展示
- v0.2 若实现编辑能力，先使用简单 textarea，后续可升级为 Monaco Editor
- 复用现有的文件树组件模式（参考 MemoryFileTree）

### 12.3 与现有架构的关系

- 新增功能**不依赖** `ProjectRegistry`，支持任意目录
- 与现有的模板项目记忆功能**并行存在**，不冲突
- 后端新增 `claude_memory` 模块，与现有的 `memory` 模块（模板项目候选记忆）区分

---

## 13. 验收标准汇总

### 13.1 v0.1 验收标准

| 编号 | 验收项 | 验证方式 |
|------|--------|---------|
| AC-01 | Windows / macOS / Linux 三端路径解析正确 | 三端分别验证 |
| AC-02 | 设置 CLAUDE_CONFIG_DIR 后路径正确切换 | 手动测试 |
| AC-03 | 能扫描并读取 `~/.claude/CLAUDE.md` 内容 | 手动测试 |
| AC-04 | 能扫描并读取指定项目的 `CLAUDE.md`、`CLAUDE.local.md` 内容 | 手动测试 |
| AC-05 | 能列出 auto memory 目录下的所有文件并显示行数/大小 | 手动测试 |
| AC-06 | MEMORY.md 超过 200 行时显示警告 | 手动测试 |
| AC-07 | 能正确解析并显示 rules 的 paths frontmatter | 手动测试 |
| AC-08 | 能扫描并展示用户级和项目级的 skills（SKILL.md） | 手动测试 |
| AC-09 | 能扫描并展示用户级和项目级的 agents（*.md） | 手动测试 |
| AC-10 | 敏感信息扫描命中常见模式 | 手动测试 |
| AC-11 | 构建无 TypeScript 错误 | `npm run build` |
| AC-12 | E2E 测试通过 | `npm test` |

### 13.2 v0.2 验收标准

| 编号 | 验收项 | 验证方式 |
|------|--------|---------|
| AC-13 | 加载模拟器输出正确的记忆加载顺序 | 手动测试 + 与 `/memory` 命令对比 |
| AC-14 | 加载模拟器显示 rules 是否 path-scoped | 手动测试 |
| AC-15 | @import 关系图正确显示 | 手动测试 |
| AC-16 | 循环引用和超过 5 层深度被检测 | 手动测试 |
| AC-17 | Rule Memory 可创建/编辑/删除 | 手动测试 |
| AC-18 | Settings 可视化显示关键字段 | 手动测试 |
| AC-19 | Subagent / Skills 可编辑管理 | 手动测试 |

### 13.3 v0.3 验收标准

| 编号 | 验收项 | 验证方式 |
|------|--------|---------|
| AC-20 | 多主机注册成功 | 手动测试 |
| AC-21 | 跨设备同步成功 | 手动测试 |
| AC-22 | 冲突检测正确分类 | 手动测试 |
| AC-23 | 同步前自动备份 | 手动测试 |

### 13.4 v0.4 验收标准

| 编号 | 验收项 | 验证方式 |
|------|--------|---------|
| AC-24 | 记忆健康评分准确 | 手动测试 |
| AC-25 | Transcript 安全审计命中泄露 | 手动测试 |
| AC-26 | 版本管理 diff / rollback 可用 | 手动测试 |

---

## 14. 风险与应对

| 风险 | 影响 | 应对 |
|------|------|------|
| Claude Code 官方机制变更 | 高 | 关注官方 release note，设计时留出扩展空间；加载链模拟逻辑与官方行为解耦 |
| 用户 `~/.claude/` 目录结构非标准 | 中 | 文件不存在时显示空状态，不阻塞；支持 CLAUDE_CONFIG_DIR 覆盖 |
| 编辑大文件时性能问题 | 低 | v0.2 若引入编辑功能，先使用 textarea，限制单次加载文件大小（如 1MB） |
| Windows 路径权限问题 | 中 | 权限不足时显示友好错误，提供手动操作指引 |
| 多主机数据库 Schema 过早设计 | 低 | 预留字段不影响单主机功能；Schema 变更通过 migration 管理 |
| WSL 与 Windows 原生路径混淆 | 中 | WSL 视为独立 host，独立 Host Profile |

---

## 15. 附录

### 15.1 参考文档

- [Claude Code 官方 .claude 目录说明](https://docs.anthropic.com/en/docs/claude-code/settings)
- [Claude Code Auto Memory 文档](https://docs.anthropic.com/en/docs/claude-code/memory)
- [Claude Code CLAUDE.md 文档](https://docs.anthropic.com/en/docs/claude-code/claude-md)

### 15.2 相关文件

| 文件 | 说明 |
|------|------|
| `docs/roadmap.md` | 项目整体路线图 |
| `CLAUDE.md` | 项目开发指南 |
| `src-tauri/src/services/memory_service.rs` | 现有记忆服务（模板项目） |
| `src/components/ProjectMemoryPanel.tsx` | 现有记忆面板（模板项目） |

### 15.3 记忆对象模型

```
ClaudeMemoryAsset
├── InstructionMemory
│   ├── ManagedClaudeMd
│   ├── UserClaudeMd
│   ├── ProjectClaudeMd
│   ├── LocalClaudeMd
│   └── ImportedInstructionFile
│
├── RuleMemory
│   ├── GlobalRule
│   ├── ProjectRule
│   └── PathScopedRule
│
├── AutoMemory
│   ├── ProjectMemoryIndex (MEMORY.md)
│   └── TopicMemoryFile
│
├── AgentMemory
│   ├── AgentDefinition
│   ├── AgentUserMemory
│   ├── AgentProjectMemory
│   └── AgentLocalMemory
│
├── WorkflowMemory
│   ├── Skill
│   └── Command
│
├── BehaviorConfig
│   ├── Settings
│   ├── Hooks
│   ├── Permissions
│   └── OutputStyle
│
└── RuntimeTrace
    ├── SessionTranscript
    ├── PromptHistory
    ├── PlanFile
    └── TaskFile
```
