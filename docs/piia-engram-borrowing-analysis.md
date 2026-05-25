# piia-engram 借鉴分析

> 状态：第一阶段分析完成
> 创建日期：2026-05-25
> 分析对象：[piia-engram](https://github.com/Patdolitse/piia-engram) v3.29.0（Apache 2.0）
> 分析目的：判断哪些能力可借鉴、哪些遗漏需补齐、哪些可复用，不直接接入

---

## 1. piia-engram 摘要

### 1.1 解决什么问题

piia-engram 是一个**本地 AI 身份层**（Personal Intelligence Identity Asset），核心解决：

- **冷启动问题**：每次新对话 AI 不认识你，需要重复自我介绍
- **跨工具失忆**：Claude Code → Cursor → Codex 切换时上下文丢失
- **经验不沉淀**：踩过的坑下次还会踩，决策没有跨会话留存
- **平台锁定**：身份数据存在某个工具里，换工具就丢

### 1.2 核心模块/能力

| 模块 | 文件 | 大小 | 职责 |
|------|------|------|------|
| MCP Server | `mcp_server.py` | 104KB | 43 个 MCP 工具注册与分发 |
| Core | `core.py` | 74KB | 身份/知识 CRUD、搜索、健康检查、重复检测 |
| Context | `context.py` | 50KB | 冷启动上下文组装、信任边界过滤 |
| Setup Wizard | `setup_wizard.py` | 104KB | 安装向导、AI 工具发现、种子导入 |
| Retrieval | `retrieval.py` | 27KB | 多词加权搜索、项目关联检索 |
| Reconcile | `reconcile.py` | 18KB | 知识合并、重复检测、关联建议 |
| Crypto | `crypto.py` | 7KB | AES-256-GCM 字段级加密 |
| Audit | `audit.py` | 1.7KB | JSONL 审计日志 |
| Storage | `storage.py` | 10KB | JSON 文件读写、portalocker 文件锁 |
| Compat | `compat.py` | 12KB | 跨工具配置发现与注入 |

### 1.3 数据模型

```text
~/.engram/
├── schema_version.json
├── identity/              # 你是谁
│   ├── profile.json       # 角色、技术栈、语言
│   ├── preferences.json   # 偏好（简洁注释、TDD 等）
│   ├── quality_standards.json  # 质量标准
│   └── trust_boundaries.json   # 信任边界（restricted_fields）
├── knowledge/             # 你学过什么
│   ├── lessons.json       # 经验教训（踩坑记录）
│   ├── decisions.json     # 关键决策及理由
│   └── domains.json       # 领域知识
├── projects/              # 项目快照
└── exports/               # 备份导出
```

核心数据结构：
- **Lesson**：`{id, title, lesson, context, domains[], severity, created_at, updated_at, review_count, last_reviewed_at}`
- **Decision**：`{id, title, decision, rationale, alternatives, domains[], created_at, updated_at, review_count}`
- **Profile**：`{name, role, experience_level, primary_languages[], frameworks[], specializations[], work_style, communication_style}`
- **Preferences**：`{coding_style, documentation_style, testing_approach, commit_style, ...}`
- **QualityStandards**：`{code_quality, testing_standards, documentation_standards, ...}`
- **TrustBoundaries**：`{restricted_fields[], public_context_fields[], ...}`

### 1.4 运行方式

- **MCP Server**（stdio）：`pip install piia-engram` → `engram setup` → 自动配置 Claude Code/Cursor/Codex 的 MCP 连接
- **远程部署**（SSE + uvicorn）：`python -m piia_engram.mcp_server --transport sse`
- **身份卡导出**：`get_identity_card` → Markdown，粘贴给无 MCP 工具
- **CLI**：`piia-engram doctor` / `piia-engram stats`

### 1.5 依赖和 license 风险

| 项 | 详情 |
|----|------|
| License | Apache 2.0 — 允许商用、修改、分发，需保留 NOTICE |
| 核心依赖 | `mcp>=1.0`、`portalocker>=2.0` |
| 可选依赖 | `cryptography>=41.0`（加密）、`uvicorn>=0.20`（远程） |
| 语言 | Python 3.10+ |
| 风险 | Python 生态与 AgentScope 的 Rust/Tauri 架构不匹配，无法直接复用代码；portalocker 是 Python 独有文件锁方案 |

### 1.6 与 AgentScope 的重合点和差异点

| 维度 | piia-engram | AgentScope |
|------|-------------|------------|
| **定位** | AI 身份层，跨工具保持"你是谁" | AI 上下文治理平台，看懂 Claude Code 的记忆加载链 |
| **数据源** | 自建 `~/.engram/` JSON 仓库 | 读取 Claude Code 真实文件系统（`~/.claude/`、项目目录） |
| **存储** | 自有 JSON + 文件锁 | 无持久化，实时扫描文件系统 |
| **交互方式** | MCP Server（AI 调用工具） | Tauri 桌面应用（人看面板） |
| **身份/画像** | ✅ 核心能力（profile、preferences、quality_standards） | ❌ 无，只看 Claude Code 的配置文件 |
| **知识管理** | ✅ lessons/decisions/domains，有 CRUD、搜索、合并、归档 | ❌ 仅有模板项目的 `decisions.md` 单文件写入 |
| **健康检查** | ✅ 0-100 健康度（新鲜度、质量、覆盖度、清洁度） | ⚠️ v0.1 有 Secret Scanner，v0.2 有加载链模拟，健康评分待实现 |
| **重复检测** | ✅ `suggest_merges` 全库近似重复 | ❌ 无（需求文档中规划为 P3） |
| **知识关联** | ✅ `link_knowledge` 知识网络 | ❌ 无 |
| **搜索/检索** | ✅ 多词加权搜索、项目关联检索 | ❌ 无（只按目录结构浏览） |
| **加密** | ✅ AES-256-GCM 字段级加密 | ❌ 无（Secret Scanner 只检测，不加密） |
| **审计日志** | ✅ JSONL 审计 | ❌ 无 |
| **跨工具适配** | ✅ Claude/Cursor/Codex/Windsurf MCP + 身份卡 | ⚠️ 规划中（跨工具资产盘点为 P2） |
| **加载链模拟** | ❌ 无（不关心 Claude Code 怎么加载） | ✅ 核心能力 |
| **Context Pressure** | ❌ 无 | ⚠️ 规划中（P3 健康检测） |
| **Review Queue** | ❌ 无（知识审查是 inline 的 review_knowledge） | ⚠️ 规划中（P2） |
| **跨平台** | ✅ Python 跨平台 | ✅ Tauri 跨 macOS/Linux/Windows |

---

## 2. 借鉴矩阵

| # | piia-engram 能力 | AgentScope 已有 | 对应文件/模块 | 可借鉴方式 | 价值 | 风险 | 优先级 |
|---|------------------|-----------------|--------------|-----------|------|------|--------|
| 1 | **知识健康度评分**（0-100，新鲜度/质量/覆盖度/清洁度） | ❌ 无 | — | 理念 + 数据结构 | 高：给用户量化记忆质量 | 低：纯计算，无外部依赖 | **P0** |
| 2 | **重复检测**（suggest_merges 全库扫描近似重复） | ❌ 无 | — | 算法 | 高：记忆膨胀是真实痛点 | 中：需定义相似度阈值和去重策略 | **P0** |
| 3 | **知识关联网络**（link_knowledge 可导航关联） | ❌ 无 | — | 理念 | 中：提升知识可发现性 | 低：数据结构简单 | **P1** |
| 4 | **过期检测**（get_stale_knowledge 30天未复查） | ❌ 无 | — | 理念 + 算法 | 高：Auto Memory 膨胀无人管 | 低：基于 mtime | **P0** |
| 5 | **会话知识提取**（wrap_up_session / extract_session_insights） | ⚠️ 有会话历史，无提取 | `claude-history/` | 理念 | 高：从会话中提炼候选知识是治理闭环核心 | 中：需定义提取策略，不能自动写入 | **P1** |
| 6 | **多词加权搜索**（search_knowledge） | ❌ 无（只按目录浏览） | — | 算法 | 高：记忆资产数增长后必须可搜索 | 中：需选搜索引擎（本地全文 vs 轻量索引） | **P1** |
| 7 | **身份画像**（profile/preferences/quality_standards） | ❌ 无 | — | 不建议 | 低：AgentScope 定位是治理平台不是身份层 | 低 | **不做** |
| 8 | **字段级加密**（AES-256-GCM） | ❌ 无 | — | 理念 | 中：敏感记忆资产保护 | 高：引入 cryptography 依赖，密钥管理复杂 | **P2** |
| 9 | **审计日志**（JSONL audit.log） | ❌ 无 | — | 数据结构 + 工具链 | 高：治理操作可追溯 | 低：JSONL 追加写入 | **P1** |
| 10 | **信任边界**（restricted_fields 过滤冷启动上下文） | ⚠️ 有 Secret Scanner | `secret_scanner.rs` | 理念 | 中：比正则匹配更灵活的敏感控制 | 低 | **P2** |
| 11 | **知识合并**（merge_knowledge 合并近似重复） | ❌ 无 | — | 算法 | 高：重复检测后的执行路径 | 中：需 diff 和冲突处理 | **P1** |
| 12 | **知识归档**（archive_knowledge） | ❌ 无 | — | 理念 | 中：替代删除，保留历史 | 低 | **P2** |
| 13 | **跨项目知识继承**（get_knowledge_inheritance） | ❌ 无 | — | 理念 | 中：新项目启动时推荐历史教训 | 中：需定义相关性匹配策略 | **P2** |
| 14 | **项目快照**（save/get_project_context） | ⚠️ 有项目数据采集 | `collectors/template/` | 理念 | 低：AgentScope 已有更细粒度的项目数据 | 低 | **不做** |
| 15 | **安装向导/工具发现**（setup_wizard） | ❌ 无 | — | 不建议 | 低：AgentScope 是桌面应用，不需要 CLI 安装向导 | 低 | **不做** |
| 16 | **身份卡导出**（get_identity_card → Markdown） | ❌ 无 | — | 不建议 | 低：与 AgentScope 治理定位不符 | 低 | **不做** |
| 17 | **跨工具配置注入**（compat.py） | ❌ 无 | — | 理念 | 中：AgentScope 跨工具资产盘点的执行路径 | 中：需逐工具适配 | **P2** |
| 18 | **知识审查流**（review_knowledge） | ❌ 无 | — | 理念 + UI | 高：Review Queue 的 inline 版本 | 低 | **P1** |

---

## 3. AgentScope 遗漏项

### 3.1 记忆/上下文资产发现

| 遗漏 | 现状 | piia-engram 参考 | 建议 |
|------|------|-------------------|------|
| 跨工具资产发现 | 仅扫描 Claude Code 资产 | compat.py 发现 Claude/Cursor/Codex 配置 | P2：只读盘点 AGENTS.md、.cursorrules、.github/copilot-instructions.md |
| subagent memory 发现 | v0.1 扫描 agents/*.md，不扫描 agent-memory/ | — | P1：扫描 `.claude/agent-memory/<name>/` 和 `~/.claude/agent-memory/<name>/` |
| managed CLAUDE.md 发现 | v0.2 load_chain 检测但不可读时只 warning | — | 已规划，保持 |

### 3.2 来源追踪（Provenance）

| 遗漏 | 现状 | piia-engram 参考 | 建议 |
|------|------|-------------------|------|
| 候选知识来源追踪 | 模板项目 decisions.md 无来源字段 | Lesson/Decision 有 created_at、review_count | **P0**：Memory Candidate 必须带 sourceType + sourceRef |
| 治理操作审计 | 无 | audit.py JSONL 审计 | **P1**：Review Queue 操作写入审计日志 |
| 变更来源判断 | 无 | — | P2：跨设备同步时判断变更来自哪台主机 |

### 3.3 session 与 memory 的关联

| 遗漏 | 现状 | piia-engram 参考 | 建议 |
|------|------|-------------------|------|
| 会话 → 候选知识提取 | 无 | wrap_up_session / extract_session_insights | **P1**：从 JSONL 会话中提取 lesson/decision 候选 |
| 会话 → 记忆关联展示 | 无 | — | P2：在记忆资产详情中展示"来自哪个会话" |

### 3.4 embedding / index / retrieval

| 遗漏 | 现状 | piia-engram 参考 | 建议 |
|------|------|-------------------|------|
| 记忆资产全文搜索 | 无（只按目录树浏览） | search_knowledge 多词加权搜索 | **P1**：轻量全文搜索（Rust tantivy 或自建倒排索引） |
| 语义检索 | 无 | — | P3：需 embedding，引入重依赖，暂不做 |

### 3.5 自动提炼候选记忆

| 遗漏 | 现状 | piia-engram 参考 | 建议 |
|------|------|-------------------|------|
| 会话结束自动提取 | 无 | wrap_up_session | **P1**：不自动写入，提取后进入 Review Queue |
| 加载链分析自动建议 | 无（v0.2 只展示加载链） | — | **P0**：Context Pressure 超阈值时生成 split_suggestion |

### 3.6 用户确认与审核流

| 遗漏 | 现状 | piia-engram 参考 | 建议 |
|------|------|-------------------|------|
| Review Queue | 无 | review_knowledge（inline 审查） | **P1**：集中审查工作台，Accept/Reject/Merge/Archive/Defer |
| 写入预览 | 无 | — | P2：Accept 后展示 diff，确认后才写盘 |

### 3.7 冲突、过期、去重、压缩

| 遗漏 | 现状 | piia-engram 参考 | 建议 |
|------|------|-------------------|------|
| 重复检测 | 无 | suggest_merges 近似重复扫描 | **P0**：段落级归一化 + 轻量相似度 |
| 过期检测 | 无 | get_stale_knowledge（30天未复查） | **P0**：基于 mtime 标记 stale 资产 |
| 冲突检测 | 无（需求规划 P3） | — | **P1**：结构冲突（循环 import、硬编码路径）先行，语义冲突后续 |
| 记忆压缩 | 无 | — | P3：大文件拆分建议（已有 Context Pressure 基础） |

### 3.8 隐私、secret、敏感信息处理

| 遗漏 | 现状 | piia-engram 参考 | 建议 |
|------|------|-------------------|------|
| Secret Scanner | ✅ 已有（5 种模式） | — | 已满足 v0.1 |
| 信任边界 | 无（只有全量扫描） | trust_boundaries（restricted_fields） | P2：按字段/路径配置扫描策略 |
| 加密存储 | 无 | AES-256-GCM 字段级加密 | P3：复杂度高，暂不做 |

### 3.9 可观察性和调试面板

| 遗漏 | 现状 | piia-engram 参考 | 建议 |
|------|------|-------------------|------|
| 审计日志 | 无 | audit.py JSONL | **P1**：治理操作写入审计日志 |
| 健康仪表盘 | 无（规划 P3） | get_knowledge_overview（0-100 健康度） | **P0**：Health Score v1 面板 |
| 操作历史 | 无 | — | P2：Review Queue 操作历史可回溯 |

### 3.10 eval / 回归验证方式

| 遗漏 | 现状 | piia-engram 参考 | 建议 |
|------|------|-------------------|------|
| 健康评分回归测试 | 无 | 678 个测试，96% 覆盖率 | **P1**：健康评分计算有单元测试 |
| 加载链对照验证 | v0.2 有手动验证计划 | — | 已规划，保持 |
| 去重精度测试 | 无 | — | **P1**：重复检测有精确率/召回率测试 |

---

## 4. 可直接复用评估

### 4.1 候选复用项总览

| # | 候选项 | piia-engram 路径 | License | 结论 |
|---|--------|-------------------|---------|------|
| R1 | 知识健康度评分算法 | `core.py` `_calculate_health_score` | Apache 2.0 | ❌ 不复用代码，借鉴算法 |
| R2 | 重复检测算法 | `reconcile.py` `_find_similar` | Apache 2.0 | ❌ 不复用代码，借鉴策略 |
| R3 | 多词加权搜索 | `retrieval.py` `_weighted_search` | Apache 2.0 | ❌ 不复用代码，自建 Rust 实现 |
| R4 | 审计日志格式 | `audit.py` | Apache 2.0 | ⚠️ 借鉴 JSONL 格式，自建 Rust 写入 |
| R5 | 信任边界数据模型 | `core.py` TrustBoundaries | Apache 2.0 | ❌ 不复用代码，借鉴数据结构 |
| R6 | 跨工具配置发现 | `compat.py` | Apache 2.0 | ❌ 不复用代码，借鉴工具发现逻辑 |

### 4.2 详细评估

#### R1：知识健康度评分算法

- **文件路径**：`src/piia_engram/core.py` → `_calculate_health_score`
- **License**：Apache 2.0，允许使用算法思想
- **依赖**：无特殊依赖，纯 Python 计算
- **跨平台**：算法本身跨平台
- **架构匹配**：❌ Python → Rust 需完全重写
- **复用成本 vs 重写成本**：算法约 50 行 Python，Rust 重写约 80 行；复用需引入 Python 运行时，不可接受
- **结论**：**借鉴算法，Rust 自建**。四个维度（新鲜度、质量、覆盖度、清洁度）的加权评分模型直接移植

#### R2：重复检测算法

- **文件路径**：`src/piia_engram/reconcile.py` → `_find_similar`
- **License**：Apache 2.0
- **依赖**：纯 Python 字符串比对
- **跨平台**：是
- **架构匹配**：❌ Python → Rust
- **复用成本 vs 重写成本**：算法约 100 行 Python；Rust 重写约 150 行
- **结论**：**借鉴策略，Rust 自建**。采用"归一化 + 编辑距离/余弦相似度"策略，阈值可配置

#### R3：多词加权搜索

- **文件路径**：`src/piia_engram/retrieval.py`
- **License**：Apache 2.0
- **依赖**：纯 Python
- **跨平台**：是
- **架构匹配**：❌ Python → Rust
- **复用成本 vs 重写成本**：约 200 行 Python；Rust 可用 tantivy 或自建轻量倒排
- **结论**：**借鉴策略，Rust 自建**。优先考虑 tantivy crate（Rust 原生全文搜索），或自建简单 TF-IDF

#### R4：审计日志格式

- **文件路径**：`src/piia_engram/audit.py`
- **License**：Apache 2.0
- **依赖**：无
- **跨平台**：是
- **架构匹配**：✅ JSONL 格式通用，Rust 写入简单
- **复用成本 vs 重写成本**：15 行 Python → 30 行 Rust，重写更划算
- **结论**：**借鉴 JSONL 格式，Rust 自建**。字段：`{timestamp, action, target, actor, detail}`

#### R5：信任边界数据模型

- **文件路径**：`src/piia_engram/core.py` → TrustBoundaries
- **License**：Apache 2.0
- **依赖**：无
- **跨平台**：是
- **架构匹配**：❌ 数据模型是 Python dict，需转为 Rust struct
- **复用成本 vs 重写成本**：数据结构定义约 20 行，Rust 重写约 30 行
- **结论**：**借鉴数据结构，Rust 自建**。`restricted_fields` 和 `public_context_fields` 概念可复用

#### R6：跨工具配置发现

- **文件路径**：`src/piia_engram/compat.py`
- **License**：Apache 2.0
- **依赖**：无
- **跨平台**：是（但路径因平台而异）
- **架构匹配**：❌ Python 路径发现逻辑需转为 Rust
- **复用成本 vs 重写成本**：约 200 行 Python → 150 行 Rust
- **结论**：**借鉴发现逻辑，Rust 自建**。AgentScope 已有 path_resolver 基础，扩展即可

### 4.3 复用结论

**没有任何 piia-engram 代码可以直接复用**。原因：
1. 语言不匹配（Python vs Rust），引入 Python 运行时不可接受
2. 架构不匹配（MCP Server vs Tauri 桌面应用）
3. 存储不匹配（自建 ~/.engram/ vs 读取 Claude Code 真实文件系统）
4. 定位不匹配（身份层 vs 治理平台）

**但算法思想、数据结构、策略模式有较高借鉴价值**，特别是：
- 健康度评分的维度划分和加权模型
- 重复检测的归一化 + 相似度策略
- 审计日志的 JSONL 格式
- 信任边界的字段级控制理念

---

## 5. 分阶段实施计划

### Phase 0：文档和设计对齐（1 周）

| # | 目标 | 涉及文件 | 验证命令 | 风险 | 需确认 |
|---|------|---------|---------|------|--------|
| 0.1 | 补量健康评分维度定义，与 governance-v0.1 对齐 | `docs/design-ai-context-governance-v0.1.md` | 文档评审 | 低 | 是 |
| 0.2 | 定义 Memory Candidate 数据模型（含 sourceType/sourceRef） | `docs/design-ai-context-governance-v0.1.md` | 文档评审 | 低 | 是 |
| 0.3 | 定义审计日志格式和存储位置 | `docs/design-audit-log.md`（新增） | 文档评审 | 低 | 是 |
| 0.4 | 定义重复检测 v1 策略（段落归一化 + 相似度阈值） | `docs/design-dedup-v1.md`（新增） | 文档评审 | 低 | 是 |

### Phase 1：低风险补齐（2-3 周）

| # | 目标 | 涉及文件 | 验证命令 | 风险 | 需确认 |
|---|------|---------|---------|------|--------|
| 1.1 | **健康评分 v1**：Load Pressure + Safety + Cleanliness + Consistency + Explainability 五维度评分 | `src-tauri/src/collectors/claude_memory/health_checker.rs`（新增）、`src/features/claude-memory/components/HealthScoreCard.tsx`（新增） | `cargo test`、`npm run build` | 低：纯计算，无写入 | 否 |
| 1.2 | **过期检测**：基于 mtime 标记 30 天未修改的资产为 stale | `health_checker.rs` | `cargo test` | 低 | 否 |
| 1.3 | **重复检测 v1**：段落归一化 + 轻量相似度（Jaccard 或编辑距离） | `src-tauri/src/collectors/claude_memory/dedup.rs`（新增） | `cargo test` | 中：阈值需调优 | 是 |
| 1.4 | **Memory Candidate 模型**：定义 Rust struct + 前端类型，支持 sourceType/sourceRef/status | `models.rs`（扩展）、`types.ts`（扩展） | `cargo check`、`npm run build` | 低 | 否 |
| 1.5 | **审计日志**：治理操作写入 JSONL 审计文件 | `src-tauri/src/services/audit_service.rs`（新增） | `cargo test` | 低 | 否 |
| 1.6 | **subagent memory 扫描**：发现 `.claude/agent-memory/<name>/` 和 `~/.claude/agent-memory/<name>/` | `scanner.rs`（扩展） | `cargo test` | 低 | 否 |
| 1.7 | **Context Pressure 指标**：加载链结果中增加总行数/总字节/重资产占比 | `load_chain.rs`（扩展）、`LoadChainSimulator.tsx`（扩展） | `cargo test`、`npm run build` | 低 | 否 |

### Phase 2：核心能力（3-4 周）

| # | 目标 | 涉及文件 | 验证命令 | 风险 | 需确认 |
|---|------|---------|---------|------|--------|
| 2.1 | **Review Queue UI**：集中审查工作台，Accept/Reject/Merge/Archive/Defer | `src/features/claude-memory/pages/ReviewQueue.tsx`（新增） | `npm run build`、E2E | 中：需定义交互流程 | 是 |
| 2.2 | **候选知识提取**：从 JSONL 会话中提取 lesson/decision 候选，进入 Review Queue | `src-tauri/src/collectors/claude_memory/candidate_extractor.rs`（新增） | `cargo test` | 中：提取策略需定义 | 是 |
| 2.3 | **全文搜索**：记忆资产内容搜索（轻量倒排或 tantivy） | `src-tauri/src/collectors/claude_memory/search.rs`（新增） | `cargo test` | 中：tantivy 是新依赖 | 是 |
| 2.4 | **知识合并**：重复检测后提供合并操作（diff + 选择保留） | `dedup.rs`（扩展）、`ReviewQueue.tsx`（扩展） | `cargo test` | 中：合并冲突处理 | 是 |
| 2.5 | **知识关联**：标记资产间关联关系 | `models.rs`（扩展） | `cargo test` | 低 | 否 |
| 2.6 | **跨工具资产盘点**：只读发现 AGENTS.md、.cursorrules、copilot-instructions.md | `src-tauri/src/collectors/claude_memory/cross_tool_inventory.rs`（新增） | `cargo test` | 低：只读 | 否 |

### Phase 3：自动化/eval/长期治理（4+ 周）

| # | 目标 | 涉及文件 | 验证命令 | 风险 | 需确认 |
|---|------|---------|---------|------|--------|
| 3.1 | **写入预览与版本管理**：Accept 后展示 diff，备份后写入 | `edit_service.rs`（新增） | `cargo test` | 高：涉及文件写入 | 是 |
| 3.2 | **语义冲突检测**：NLP 辅助检测 pnpm vs npm 等矛盾 | — | — | 高：需 NLP 依赖 | 是 |
| 3.3 | **健康趋势**：跨次扫描的健康评分变化 | — | — | 中 | 是 |
| 3.4 | **跨设备同步**：多主机注册 + 语义同步 | — | — | 高 | 是 |
| 3.5 | **eval 回归**：健康评分/去重精度/加载链正确性的自动化验证 | `e2e/` | `npm test` | 低 | 否 |

---

## 6. 第一批建议实施项

从 Phase 1 中挑选 **5 个最值得做、风险低、能提升 AgentScope 价值** 的任务：

### 6.1 健康评分 v1（P0）

**为什么做**：用户打开 AgentScope 第一眼就应该知道记忆资产健不健康。piia-engram 的 0-100 健康度模型已验证可行，我们借鉴维度划分，基于已有数据（加载链、Secret Scanner、资产元数据）计算。

**做什么**：
- 五维度评分：Load Pressure / Safety / Cleanliness / Consistency / Explainability
- 每个维度 0-100，加权汇总
- 前端：HealthScoreCard 组件，总分 + 雷达图 + Top Issues

**涉及文件**：
- 新增：`src-tauri/src/collectors/claude_memory/health_checker.rs`
- 新增：`src/features/claude-memory/components/HealthScoreCard.tsx`
- 扩展：`models.rs`（SerHealthReport 结构体）
- 扩展：`routes/claude_memory.rs`、`services/claude_memory_service.rs`

**验证**：`cargo test` + `npm run build`

**风险**：低。纯计算，无写入，不引入新依赖。

### 6.2 过期检测（P0）

**为什么做**：Auto Memory 持续增长但无人管理，30 天未修改的资产可能已过期。piia-engram 的 `get_stale_knowledge` 验证了基于 mtime 的过期检测足够实用。

**做什么**：
- 在 health_checker 中增加 stale 检测：mtime 超过 30 天的资产标记为 stale
- stale 资产在 UI 中用淡色/删除线展示
- 可配置阈值（默认 30 天）

**涉及文件**：
- 扩展：`health_checker.rs`
- 扩展：`MemoryAssetTree.tsx`（stale 标记展示）

**验证**：`cargo test`

**风险**：低。基于已有 mtime_ms 字段。

### 6.3 重复检测 v1（P0）

**为什么做**：记忆资产膨胀后重复是最大痛点。piia-engram 的 `suggest_merges` 证明了轻量相似度检测（无需 embedding）就有实用价值。

**做什么**：
- 段落归一化：按 Markdown 标题拆分，去空白/标点后比对
- 轻量相似度：Jaccard（词集合交集/并集）或 normalized edit distance
- 阈值可配置（默认 0.8）
- 输出：重复对列表，每对含相似度和共现内容摘要

**涉及文件**：
- 新增：`src-tauri/src/collectors/claude_memory/dedup.rs`
- 扩展：`models.rs`（SerDuplicatePair 结构体）
- 扩展：`routes/claude_memory.rs`、`services/claude_memory_service.rs`

**验证**：`cargo test`（含精确率测试用例）

**风险**：中。阈值需调优，但检测本身只读，不影响资产。

### 6.4 Memory Candidate 模型（P0）

**为什么做**：当前模板项目的 `decisions.md` 写入流没有候选层，直接写盘。piia-engram 的 Lesson/Decision 模型证明了"先提取、后审查、再写入"的流程更安全。这是 Review Queue 的前置依赖。

**做什么**：
- 定义 Rust struct：`SerMemoryCandidate`（id, source_type, source_ref, suggested_kind, suggested_target, reason, status, created_at_ms）
- 定义前端 TypeScript 类型
- 不实现提取逻辑（Phase 2），只建立数据模型和 API 骨架

**涉及文件**：
- 扩展：`models.rs`
- 扩展：`types.ts`
- 扩展：`routes/claude_memory.rs`（list_candidates 命令骨架）

**验证**：`cargo check` + `npm run build`

**风险**：低。只定义模型，不改变现有行为。

### 6.5 Context Pressure 指标（P0）

**为什么做**：加载链模拟器已能展示加载顺序，但缺少量化指标。用户需要知道"我的启动上下文有多重"。piia-engram 没有此能力（它不关心加载链），但 AgentScope 的需求文档已规划 Context Pressure。

**做什么**：
- 在 `SerLoadChain` 中增加 `context_pressure` 字段：总资产数、总行数、总字节数、最重资产 Top 3、重资产占比
- 前端：LoadChainSimulator 页面顶部增加压力指示条

**涉及文件**：
- 扩展：`models.rs`（SerContextPressure 结构体）
- 扩展：`load_chain.rs`（计算 context_pressure）
- 扩展：`LoadChainSimulator.tsx`（压力指示条）

**验证**：`cargo test` + `npm run build`

**风险**：低。基于已有加载链数据，纯计算。

---

## 附录 A：piia-engram 工具全表

### Tier-1（10 个，默认加载）

| 工具 | 功能 | AgentScope 对应 |
|------|------|----------------|
| `get_user_context` | 冷启动上下文 | 加载链模拟器（不同定位） |
| `wrap_up_session` | 会话结束提取知识 | 无（规划：候选知识提取） |
| `add_lesson` | 记录经验教训 | 无（规划：Memory Candidate） |
| `add_decision` | 记录关键决策 | 模板项目 decisions.md（受限） |
| `search_knowledge` | 多词加权搜索 | 无（规划：全文搜索） |
| `get_relevant_knowledge` | 项目关联检索 | 无 |
| `get_identity_card` | 导出 Markdown 身份卡 | 不适用 |
| `update_identity` | 更新身份画像 | 不适用 |
| `get_project_context` | 读取项目快照 | 项目详情（已有） |
| `save_project_snapshot` | 保存项目状态 | 无 |

### Tier-2（33 个，ENGRAM_TOOLS=all 开启）

身份管理（5）：get_profile, get_work_style, get_preferences, get_trust_boundaries, get_quality_standards
知识管理（6）：get_lessons, get_decisions, get_domains, extract_session_insights, bulk_add_knowledge, ingest_notes
知识审查（5）：update_knowledge, archive_knowledge, review_knowledge, merge_knowledge, link_knowledge
知识发现（5）：get_knowledge_overview, get_related_knowledge, find_similar_knowledge, suggest_merges, get_stale_knowledge
导入导出（4）：export_engram, import_engram, export_knowledge_report, export_engram_to_openclaw
项目工作（3）：start_project, list_projects, get_knowledge_inheritance
安全审查（3）：request_outline_review, apply_review, get_audit_log

---

## 附录 B：AgentScope 当前 Claude Memory 模块清单

### 后端（Rust）

| 文件 | 行数 | 职责 |
|------|------|------|
| `scanner.rs` | 741 | 全量扫描（用户级 + 项目级 + auto memory） |
| `load_chain.rs` | ~1800 | 加载链模拟（P1 核心实现） |
| `path_resolver.rs` | ~230 | 路径解析（CLAUDE_CONFIG_DIR、平台适配） |
| `frontmatter.rs` | ~140 | YAML frontmatter 解析 |
| `secret_scanner.rs` | 245 | 敏感信息正则扫描 |
| `settings_reader.rs` | ~430 | settings.json 多层读取 + claudeMdExcludes |
| `models.rs` | 150 | 数据结构定义 |

### 前端（React/TypeScript）

| 文件 | 职责 |
|------|------|
| `MemoryAssetTree.tsx` | 资产树展示 |
| `MemoryAssetDetail.tsx` | 资产详情 |
| `LoadChainSimulator.tsx` | 加载链模拟器页面 |
| `IssueBadge.tsx` | 问题标记组件 |
| `useClaudeMemory.ts` | API 调用 hook |
| `useLoadChain.ts` | 加载链 API hook |
| `types.ts` | 前端类型定义 |

### 缺失模块（与 piia-engram 对比后确认）

| 模块 | 对应 piia-engram 能力 | 建议优先级 |
|------|----------------------|-----------|
| `health_checker.rs` | get_knowledge_overview | P0 |
| `dedup.rs` | suggest_merges | P0 |
| `candidate_extractor.rs` | extract_session_insights | P1 |
| `search.rs` | search_knowledge | P1 |
| `audit_service.rs` | audit.py | P1 |
| `cross_tool_inventory.rs` | compat.py | P2 |
| `edit_service.rs` | update_knowledge | P3 |
