# Claude Memory Phase 3：治理闭环设计文档

> **版本**: v0.1 (设计阶段，未实施)
> **状态**: 方案已定，待分批实现
> **关联文档**: [design-memory-health-phase-1.md](design-memory-health-phase-1.md) (Phase 1/2 健康诊断与资产树)

---

## 1. 背景与目标

### 1.1 已完成的能力（Phase 1 & 2）

AgentScope 的 Claude Memory 模块当前已具备：

| 能力 | 实现状态 | 说明 |
|:---|:---|:---|
| **健康评分** | ✅ Rust + UI | 五维度评分（freshness/quality/coverage/cleanliness/safety） |
| **重复检测** | ✅ Rust + UI | 精确重复 + Jaccard ≥ 0.8 近似重复，连通分量分组 |
| **过期检测** | ✅ Rust + UI | 基于文件 mtime 与内容时间戳的 staleness 标记 |
| **Secret 风险** | ✅ Rust + UI | 高熵字符串/凭证模式匹配，低误报 |
| **资产树视图** | ✅ UI | 按类型分组（instruction/rules/auto_memory/skills_agents），带 badge 标记 |
| **问题导航** | ✅ UI | Top Issues 列表点击跳转对应资产详情 |

### 1.2 Phase 3 要解决的问题

**核心痛点：诊断后没有闭环。**

用户通过健康诊断发现了问题（过期文件、重复段落、Secret 泄漏风险），但：
- 无法标记"这个问题我已经处理过了" → 每次重新加载都重复告警
- 无法区分"AgentScope 建议的处理方式"和"用户实际采取的行动" → 建议沦为噪音
- 没有历史记录证明某次诊断结果已经被审阅 → 团队协同时无法建立信任
- 缺少对记忆上下文压力的感知 → 用户不知道何时该主动清理

### 1.3 目标声明

1. **建立 Review Queue**：将健康问题转化为可逐项审阅、可标记状态的队列
2. **安全 Action Labels**：提供不误导用户的操作选项（绝不暗示已执行实际清理）
3. **轻量级 Audit Trail**：记录"何时、谁、对什么问题做了什么标记"，支持回溯
4. **Context Pressure 提示**：基于资产数量和加载链长度，给出上下文负荷预警
5. **Candidate Memory 收集**：让用户在 AgentScope 内记录"值得写入记忆"的片段，但不自动写入 .claude/

---

## 2. 非目标（产品边界）

以下能力**明确不在 Phase 3 范围内**，后续如需扩展需单独评审：

| 边界 | 说明 | 原因 |
|:---|:---|:---|
| **不写 .claude/ 目录** | AgentScope 绝不修改用户项目目录下的任何 .claude/ 文件 | 信任边界：AgentScope 是观察工具，不是 Agent 本身 |
| **不写 AGENTS.md / CLAUDE.md** | 不自动向用户项目的记忆文件追加内容 | 同上；记忆内容的取舍权属于 Agent 和用户 |
| **不使用 localStorage 作为正式持久化** | Review 状态、Audit Log 等持久化数据必须存到 App Data Dir，而非浏览器 localStorage | localStorage 随前端刷新丢失，且容量受限 (~5MB)，不适合作为正式业务状态存储 |
| **不执行实际文件操作** | 所有 Action 均为"标记状态"，绝不执行 rm/mv/edit 等文件系统操作 | 避免误删；清理动作由用户在 IDE/终端中自行执行 |
| **不引入 LLM 做决策** | Review Queue 的排序、建议生成基于确定性规则，不调用外部模型 | 保持可预测性，避免幻觉；后续如需 AI 建议可独立设计 |
| **不替代 Git** | Audit Log 不是 Git 的替代品，不记录文件内容变更 | Git 已经是内容变更的权威来源 |

---

## 3. 当前数据基础

### 3.1 已有 Rust 数据结构（简化摘录）

以下字段来自 `src-tauri/src/collectors/claude_memory/models.rs`，摘录与 Phase 3 设计相关的核心字段：

```rust
// Asset 级别字段（SerClaudeMemoryAsset）
pub struct SerClaudeMemoryAsset {
    pub id: String,                      // 资产唯一标识
    pub scope: String,                   // "global" | "project" | ...
    pub asset_type: String,              // "instruction" | "rules" | "auto_memory" | "skills_agents" | ...
    pub logical_path: String,            // 相对路径（如 .claude/instructions/onboard.md）
    pub line_count: Option<usize>,
    pub byte_size: Option<u64>,
    pub exists: bool,
    pub secret_issues: Vec<SerSecretIssue>,
}

// 健康报告（SerMemoryHealthReport）
pub struct SerMemoryHealthReport {
    pub overall_score: u8,
    pub top_issues: Vec<SerMemoryHealthIssue>,
    pub stale_assets: Vec<SerMemoryStaleness>,
    pub duplicate_groups: Vec<SerMemoryDuplicateGroup>,
    // dimensions: freshness / quality / coverage / cleanliness / safety
}

// 健康问题（SerMemoryHealthIssue）
pub struct SerMemoryHealthIssue {
    pub issue_type: String,              // "stale" | "duplicate" | "secret" | "too_long" | "quality"
    pub severity: String,                // "critical" | "warning" | "info"
    pub asset_ids: Vec<String>,          // 涉及的一个或多个资产 ID
    pub message: String,                 // 问题描述
    pub suggestion: String,              // 建议文本（Phase 3 Action Suggestions 的数据来源）
}

// 过期资产（SerMemoryStaleness）
pub struct SerMemoryStaleness {
    pub asset_id: String,
    pub asset_type: String,
    pub scope: String,
    pub logical_path: String,
    pub stale_days: Option<u64>,
    pub threshold_days: u64,
}

// 重复资产组（SerMemoryDuplicateGroup）
pub struct SerMemoryDuplicateGroup {
    pub group_id: String,                // 组唯一标识
    pub asset_ids: Vec<String>,          // 组内资产 ID 列表
    pub similarity: f64,                 // Jaccard 相似度
    pub overlap_content: String,         // 重复内容片段
    pub suggestion: String,              // 去重建议
}
```

### 3.2 已有持久化模式

项目已在 `src-tauri/src/registry.rs` 中使用 `dirs::data_local_dir()` 存储应用状态：

```rust
let app_dir = dirs::data_local_dir()
    .ok_or("无法获取数据目录")?
    .join("agent-scope");
let projects_file = app_dir.join("projects.json");
```

Phase 3 的持久化将**沿用同一目录**（`{data_local_dir}/agent-scope/`），新增以下文件：
- `memory_reviews.json` — Review Queue 状态
- `memory_audit.jsonl` — Audit Log（追加写入的 JSON Lines）
- `memory_candidates.json` — Candidate Memory 列表

### 3.3 前端状态模式

当前前端使用 React `useState` + props drilling，无全局状态库。Phase 3 的 UI 状态（如当前筛选条件、展开/折叠面板）继续沿用此模式；需要持久化的数据通过 Tauri command 读写后端文件。

---

## 4. 候选方案评估

### 4.1 候选能力总览

| 候选能力 | 价值 | 复杂度 | 风险 | 批次决策 |
|:---|:---|:---|:---|:---|
| **Review Queue（审阅队列）** | 高：诊断→闭环的核心载体 | 中 | 低 | **Batch 2** |
| **Action Suggestions（操作建议）** | 高：给用户明确的下一步指引 | 中 | 中（需防止误导） | **Batch 3** |
| **Context Pressure（上下文压力）** | 中高：帮助用户主动预防问题 | 低 | 低 | **Batch 1（先行）** |
| **Audit Log（审计日志）** | 中：建立信任和可追溯性 | 低 | 低 | **Batch 4** |
| **Candidate Memory（候选记忆）** | 中：桥接观察与行动 | 中 | 中（边界易模糊） | **Batch 5** |
| **Trust Boundary（信任边界声明）** | 高：产品立场表达 | 极低 | 极低 | **Batch 1 内嵌到 UI** |
| **Auto-fix（自动修复）** | 中：一键清理 | 高 | **极高（误删风险）** | **明确不做** |
| **Schedule Review（定时提醒）** | 低：非核心场景 | 中 | 低 | 暂不纳入 |
| **Team Sync（团队共享状态）** | 低：当前单用户场景 | 高 | 中 | 暂不纳入 |

### 4.2 决策理由

**Context Pressure 作为 Batch 1（先行）**：
- 纯计算逻辑，无需新增持久化，实现成本最低
- 可以给 Phase 1 已有的健康评分增加一个高维视角

**Review Queue 作为 Batch 2 核心**：
- Phase 1/2 已经产出大量诊断数据，但用户只能"看"不能"处理"
- Review Queue 是所有其他能力的锚点：Action Suggestions 依附于 Review Item，Audit Log 记录对 Review Item 的操作

**Action Suggestions 作为 Batch 3**：
- 需要设计安全的建议文本模板（不能让用户误以为 AgentScope 已执行操作）
- 依赖 Review Queue 的数据结构先定型

**Audit Log 作为 Batch 4**：
- 追加写入 JSON Lines，技术实现简单
- 但价值主要在团队场景和长期追溯，单用户短期使用感知不强

**Candidate Memory 作为 Batch 5**：
- 产品边界需要非常谨慎（只收集、不写入 .claude/）
- 需设计清晰的 UX 让用户理解"这里记录的内容不会自动生效"

**Auto-fix 明确排除**：
- 与"不写 .claude/"的核心边界冲突
- 即使只做本地文件操作，也存在误删用户内容的风险
- 如果未来要做，必须以"生成 diff 预览 + 用户确认"的形式单独设计

---

## 5. 第一批实现范围

### 5.1 Batch 1：Context Pressure（上下文压力提示）

**前置条件**：无（纯计算，不依赖新持久化）

**功能描述**：
基于当前项目已加载的 memory assets，计算以下指标：

| 指标 | 计算方式 | 预警阈值（可调） |
|:---|:---|:---|
| `total_assets` | memory asset 文件总数 | ≥ 20 提醒 |
| `total_lines` | 所有 asset content 行数之和 | ≥ 500 提醒 |
| `total_chars` | 所有 asset content 字符数之和 | ≥ 20,000 提醒 |
| `load_chain_depth` | 最长 load chain 深度（from SerLoadChain） | ≥ 4 提醒 |
| `auto_memory_size` | `.claude/auto_memory_index.md` 行数 | ≥ 200 警告（已存在） |

**UI 呈现（Batch 1）**：
- 在 **Claude Memory 页面顶部**增加一个轻量横幅（可关闭）
- 当任意指标超过阈值时显示对应提示
- 不阻塞用户操作，纯信息性
- **后续扩展**：Load Chain 页面可复用同一指标，展示更细的逐层加载压力

**Rust 数据结构**：
```rust
pub struct SerContextPressure {
    pub total_assets: usize,
    pub total_lines: usize,
    pub total_chars: usize,
    pub load_chain_depth: usize,
    pub auto_memory_lines: usize,
    pub alerts: Vec<SerPressureAlert>,
}

pub struct SerPressureAlert {
    pub metric: String,      // "total_assets" | "total_lines" | ...
    pub current: usize,
    pub threshold: usize,
    pub severity: String,    // "info" | "warning"
    pub message: String,     // 例如 "记忆文件数量较多 (23)，建议审查是否有冗余"
}
```

### 5.2 Batch 2：Review Queue 模型

**前置条件**：新增 `memory_reviews.json` 持久化

**核心概念**：
- 一个 **Review Item** 对应一个健康问题的一次出现
- 同一问题在不同时间扫描结果中通过 `(primary_asset_id, issue_type)` 或 `group_id` 去重

**状态机（4 状态）**：

```
         +----------------+
         |    pending     |  <-- 新发现的问题默认状态
         +--------+-------+
                  |
      +-----------+-----------+
      |           |           |
      v           v           v
+---------+ +---------+ +---------+
| reviewed| | ignored | |snoozed  |
+---------+ +---------+ +---------+
                                  |
                                  |
                                  v
                            +---------+
                            | pending |  <-- snoozed 到期后由 sync 转回
                            +---------+
```

**状态定义**：

| 状态 | 语义 | 用户可见标签 |
|:---|:---|:---|
| `pending` | 待审阅，新发现或 snoozed 到期后回到此状态 | "待处理" |
| `reviewed` | 用户已审阅，确认问题存在且已在项目外处理 | "已标记" |
| `ignored` | 用户认为此为误报或不重要 | "已忽略" |
| `snoozed` | 用户希望暂时不处理，设定时间后回到 pending | "稍后处理" |

**关键设计决策**：
- **4 状态，无 "resolved" 或 "fixed"**：AgentScope 不执行实际修复，无法确认问题是否真的已修复。"外部已处理"作为 action label 或 review_note 记录，不作为独立状态
- **"reviewed" 的含义**："我已看到这个问题，并已在外部处理（或决定不处理）"，不是"AgentScope 已修复"
- **过期机制**：snoozed 状态带 `snooze_until: Option<u64>`（Unix 时间戳），下次 `sync_review_queue` 时自动将过期项移回 pending

**Rust 数据结构**：

```rust
pub struct SerReviewItem {
    pub id: String,                    // UUID v4
    pub project_id: String,            // 关联项目（支持多项目）
    pub asset_ids: Vec<String>,        // 涉及资产 ID 列表（来自 SerMemoryHealthIssue.asset_ids）
    pub primary_asset_id: String,      // 主资产 ID（用于展示和跳转）
    pub issue_type: String,            // 同 SerMemoryHealthIssue.issue_type
    pub severity: String,
    pub message: String,               // 来自 SerMemoryHealthIssue.message
    pub suggestion: String,            // 来自 SerMemoryHealthIssue.suggestion
    pub state: SerReviewState,
    pub created_at: u64,               // Unix 时间戳（秒）
    pub updated_at: u64,
    pub snooze_until: Option<u64>,
    pub review_note: Option<String>,   // 用户可选的备注（如"已在 IDE 中人工处理，见项目提交记录"）
}

pub enum SerReviewState {
    Pending,
    Reviewed,
    Ignored,
    Snoozed,
}

pub struct SerReviewQueue {
    pub items: Vec<SerReviewItem>,
    pub pending_count: usize,
    pub reviewed_count: usize,
    pub ignored_count: usize,
    pub snoozed_count: usize,
    pub last_scan_at: Option<u64>,
}
```

**去重与匹配策略**：
- stale/secret/quality 类型：用 `(project_id, primary_asset_id, issue_type)` 查找已有 item
- duplicate 类型：用 `(project_id, group_id)` 查找（group_id 来自 SerMemoryDuplicateGroup）
- 如果找到且状态为 `pending`：更新 `message`/`suggestion` 等字段，保持 `pending`
- 如果找到且状态为 `reviewed/ignored/snoozed`：不覆盖状态，仅更新元数据（如 suggestion 文本）
- 如果没找到：新建 `pending` item

### 5.3 Batch 3：Action Suggestions（操作建议面板）

**前置条件**：Batch 2 Review Queue 已完成

**功能描述**：
在 Review Item 详情和 Asset Detail 页面中，根据 issue_type 显示建议文本。

**建议模板（安全措辞）**：

| Issue Type | 建议标题 | 建议内容 | 关联 Action |
|:---|:---|:---|:---|
| `stale` | "文件可能已过期" | "该文件 N 天未更新。建议：1) 在 IDE 中打开确认内容是否仍有效；2) 如已废弃，考虑从 load chain 中移除或归档。" | "标记已审" / "稍后处理" |
| `duplicate` | "发现重复内容" | "该片段与 X 个其他文件内容重复。建议：1) 确认哪个版本是权威的；2) 考虑将重复内容提取到公共文件并引用。" | "标记已审" / "忽略此项" |
| `secret` | "潜在凭证泄漏" | "检测到疑似凭证的高熵字符串。建议：1) 确认是否为真实凭证；2) 如是，立即从文件中移除并轮换该凭证；3) 考虑使用环境变量替代硬编码。" | "标记已审" / "忽略此项" |
| `too_long` | "文件过长" | "auto_memory_index.md 超过 200 行，可能影响加载效率。建议：1) 审查是否有冗余条目；2) 将不常用内容移至分类子文件。" | "标记已审" / "稍后处理" |
| `quality` | "内容质量待提升" | "该文件内容较简短或结构不够清晰。建议：1) 补充上下文说明；2) 检查是否符合团队记忆规范。" | "标记已审" / "忽略此项" |

**设计原则**：
- 每个建议都包含**明确的用户动作**（在 IDE 中打开、确认、移除），不是"点击修复"
- 建议文本中**不出现 "AgentScope 已..." 或 "已自动..." 的措辞**
- Action 按钮的标签**使用祈使语气描述用户的标记行为**，不描述系统行为

### 5.4 Batch 3：Review Queue UI

**页面结构**：

```
Claude Memory Page
├── Context Pressure Banner (Batch 1)
├── Stat Cards Row (已有：健康评分/重复组数/Secret 数)
├── Review Queue Panel (Batch 3)
│   ├── Filter Tabs: [全部] [待处理] [已标记] [已忽略] [稍后处理]
│   ├── Sort: [时间] [严重程度] [类型]
│   └── Review Item List
│       ├── Item Row: [severity icon] [issue_type badge] asset_path
│       │              [状态标签] [操作按钮组]
│       └── Expanded: 描述 | Snippet 预览 | 建议面板 | 备注输入
├── Memory Asset Tree (已有)
└── Memory Asset Detail (已有，增加 Review 状态叠加显示)
```

**Review Item 操作按钮**：

| 按钮 | 状态条件 | 动作 |
|:---|:---|:---|
| "标记已审" | pending / snoozed | state → reviewed，记录 audit |
| "稍后处理" | pending | 弹出选择器（1天/3天/7天/30天），state → snoozed |
| "忽略此项" | pending / snoozed | state → ignored，记录 audit |
| "取消忽略" | ignored | state → pending |
| "重新打开" | reviewed | state → pending |

### 5.5 Batch 4：Audit Log（审计日志）

**前置条件**：Batch 2 Review Queue 已完成

**功能描述**：
记录所有对 Review Item 的状态变更操作，以及系统层面的扫描事件。

**Rust 数据结构**：

```rust
pub struct SerAuditEvent {
    pub id: String,                    // UUID
    pub event_type: SerAuditEventType,
    pub review_item_id: Option<String>,
    pub project_id: String,
    pub asset_path: Option<String>,
    pub details: Option<String>,       // JSON 字符串，视 event_type 而定
    pub created_at: u64,
}

pub enum SerAuditEventType {
    ScanStarted,           // 系统：开始扫描
    ScanCompleted,         // 系统：扫描完成，包含发现/更新/关闭的统计
    ItemCreated,           // 系统：新 Review Item 创建
    ItemStateChanged,      // 用户：状态变更 (pending→reviewed 等)
    ItemSnoozed,           // 用户：设置 snooze
    ItemNoteAdded,         // 用户：添加备注
    ItemDeleted,           // 系统：过期 item 清理（如 asset 已不存在）
}
```

**存储格式**：JSON Lines（`memory_audit.jsonl`），追加写入，便于后期分析和归档。

**UI 呈现**：
- 在 Settings → Claude Memory 下增加 "Audit Log" 子页面
- 按时间倒序展示事件列表
- 支持按 event_type 筛选
- 单用户场景下默认只展示最近 100 条，避免性能问题

### 5.6 Batch 5：Candidate Memory（候选记忆）

**前置条件**：Batch 2 Review Queue 已完成

**功能描述**：
允许用户在 AgentScope 内记录"值得后续写入 .claude/ 的片段"，作为观察与行动之间的桥梁。

**Rust 数据结构**：

```rust
pub struct SerCandidateMemory {
    pub id: String,
    pub project_id: String,
    pub source: SerCandidateSource,    // 从哪里捕获的
    pub content: String,
    pub tags: Vec<String>,
    pub created_at: u64,
    pub archived_at: Option<u64>,      // 归档时间（用户确认已写入 .claude/ 后）
}

pub enum SerCandidateSource {
    Manual,              // 用户手动创建
    FromReviewItem {     // 从 Review Item 捕获
        review_item_id: String,
        asset_path: String,
    },
    FromClipboard,       // 从剪贴板捕获（未来扩展）
}
```

**关键设计**：
- **显式写入提示**：Candidate Memory 的创建 UI 必须包含提示文案："此内容仅保存在 AgentScope 中，不会自动写入您的 .claude/ 文件。请在确认后手动添加到对应记忆文件。"
- **归档操作**：用户确认已将内容写入 .claude/ 后，点击"归档"，item 移至 archived 列表（可查看历史）
- **不替代记忆文件**：Candidate Memory 列表不是 .claude/ 的镜像，只是临时收集区

---

## 6. 数据模型总览

### 6.1 实体关系

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│  Project        │────<│  ReviewItem     │────<│  AuditEvent     │
│  (已有)         │ 1:N │  (新增)         │ 1:N │  (新增)         │
└─────────────────┘     └─────────────────┘     └─────────────────┘
         │                       │
         │                       │
         v                       v
┌─────────────────┐     ┌─────────────────┐
│  HealthReport   │     │ CandidateMemory │
│  (已有)         │     │ (新增, Batch 5) │
└─────────────────┘     └─────────────────┘
         │
         v
┌─────────────────┐
│ ContextPressure │
│ (新增, Batch 1) │
└─────────────────┘
```

### 6.2 状态键设计（持久化文件）

| 文件 | 路径 | 格式 | 写入策略 |
|:---|:---|:---|:---|
| `memory_reviews.json` | `{data_local_dir}/agent-scope/memory_reviews.json` | JSON | 全量覆盖（数据量小，预计 < 10KB） |
| `memory_audit.jsonl` | `{data_local_dir}/agent-scope/memory_audit.jsonl` | JSON Lines | 追加写入 |
| `memory_candidates.json` | `{data_local_dir}/agent-scope/memory_candidates.json` | JSON | 全量覆盖 |

**状态键命名约定**：
- 持久化文件名前缀统一为 `memory_`，便于识别和后期迁移
- 所有时间戳统一使用 Unix 秒（`u64`），与现有 `last_modified_secs` 一致
- 枚举序列化使用 kebab-case（"reviewed", "ignored", "snoozed"）

---

## 7. UI 草图

### 7.1 Context Pressure Banner

```
┌─────────────────────────────────────────────────────────────┐
│ ⚡ 上下文压力提示  [关闭]                                    │
│ 记忆文件: 23 | 总行数: 1,247 | 最长加载链: 5 层             │
│ auto_memory_index.md: 312 行（超过 200 行建议拆分）         │
└─────────────────────────────────────────────────────────────┘
```

- 仅当至少一个指标超过阈值时显示
- 用户点击"关闭"后，当前会话内不再显示（但重新进入页面或刷新后会恢复）
- 不阻塞任何操作

### 7.2 Review Queue Panel

```
┌─────────────────────────────────────────────────────────────┐
│ 📋 审阅队列  (待处理: 8 | 已标记: 3 | 已忽略: 2 | 稍后: 1) │
│ [全部] [待处理] [已标记] [已忽略] [稍后处理]  排序: [▼时间] │
├─────────────────────────────────────────────────────────────┤
│ 🔴 [重复]  .claude/skills/api-design.md  ...                │
│    与 2 个其他文件存在重复内容      [标记已审] [稍后] [忽略]│
├─────────────────────────────────────────────────────────────┤
│ 🟡 [过期]  .claude/instructions/legacy-onboard.md  ...      │
│    47 天未更新                     [标记已审] [稍后] [忽略]│
├─────────────────────────────────────────────────────────────┤
│ 🔴 [Secret] .claude/auto_memory_index.md  ...               │
│    检测到疑似 API Key               [标记已审] [忽略]       │
└─────────────────────────────────────────────────────────────┘
```

**展开后的 Item 详情**：

```
┌─────────────────────────────────────────────────────────────┐
│ 🔴 [重复]  .claude/skills/api-design.md                     │
│    严重程度: 高 | 发现时间: 2026-05-20                        │
├─────────────────────────────────────────────────────────────┤
│ 问题描述：                                                   │
│ 该文件中的"REST API 设计规范"片段与以下文件重复：           │
│ - .claude/skills/backend-patterns.md                        │
│ - docs/standards/api.md                                     │
├─────────────────────────────────────────────────────────────┤
│ 💡 建议：                                                    │
│ 1. 确认哪个版本是权威来源                                    │
│ 2. 考虑将重复内容提取到公共文件，其他文件通过引用加载       │
├─────────────────────────────────────────────────────────────┤
│ 📝 备注（可选）：[____________________]                     │
│                                                             │
│ [标记已审]  [稍后处理 ▼]  [忽略此项]                        │
└─────────────────────────────────────────────────────────────┘
```

### 7.3 Asset Detail 中的 Review 状态叠加

在已有的 Memory Asset Detail 页面中，增加一个"相关审阅项"区域：

```
┌─────────────────────────────────────────────────────────────┐
│ 📄 .claude/skills/api-design.md                             │
│ [stale] [duplicate]  健康评分: 42                           │
├─────────────────────────────────────────────────────────────┤
│ 内容预览 ...                                                │
├─────────────────────────────────────────────────────────────┤
│ 🔗 相关审阅项 (2)：                                         │
│    🔴 重复内容检测 — [待处理] [标记已审] [稍后] [忽略]      │
│    🟡 文件已过期 — [已标记]                                 │
└─────────────────────────────────────────────────────────────┘
```

### 7.4 Candidate Memory 浮窗（Batch 5）

```
┌─────────────────────────────────────────────────────────────┐
│ 📝 捕获为候选记忆                                            │
│                                                             │
│ 内容：                                                       │
│ ┌─────────────────────────────────────────────────────────┐ │
│ │ 项目使用 Tauri v2 + React 19 技术栈，桌面端优先。       │ │
│ └─────────────────────────────────────────────────────────┘ │
│                                                             │
│ 标签：[技术栈] [Tauri]                                      │
│                                                             │
│ ⚠️ 此内容仅保存在 AgentScope 中，不会自动写入 .claude/     │
│                                                             │
│          [取消]  [保存到候选记忆]                            │
└─────────────────────────────────────────────────────────────┘
```

---

## 8. API 设计（Tauri Commands）

### 8.1 Batch 1: Context Pressure

```rust
#[tauri::command]
async fn get_context_pressure(
    state: tauri::State<'_, AppState>,
    project_path: String,
) -> Result<SerContextPressure, String>;
```

### 8.2 Batch 2: Review Queue

```rust
#[tauri::command]
async fn get_review_queue(
    state: tauri::State<'_, AppState>,
    project_id: String,
    filter: Option<String>,  // "all" | "pending" | "reviewed" | "ignored" | "snoozed"
) -> Result<SerReviewQueue, String>;

#[tauri::command]
async fn update_review_item_state(
    state: tauri::State<'_, AppState>,
    item_id: String,
    new_state: String,       // "pending" | "reviewed" | "ignored" | "snoozed"
    snooze_days: Option<u32>, // 仅当 new_state == "snoozed" 时有效
    note: Option<String>,
) -> Result<SerReviewItem, String>;

#[tauri::command]
async fn sync_review_queue(
    state: tauri::State<'_, AppState>,
    project_id: String,
    health_report: SerMemoryHealthReport,
) -> Result<SerReviewQueueSyncResult, String>;
```

`sync_review_queue` 由前端在获取 Health Report 后调用，将新发现的问题合并到已有 Review Queue 中。

### 8.3 Batch 4: Audit Log

```rust
#[tauri::command]
async fn get_audit_log(
    state: tauri::State<'_, AppState>,
    project_id: String,
    event_types: Option<Vec<String>>,
    limit: Option<usize>,
) -> Result<Vec<SerAuditEvent>, String>;
```

### 8.4 Batch 5: Candidate Memory

```rust
#[tauri::command]
async fn create_candidate_memory(
    state: tauri::State<'_, AppState>,
    project_id: String,
    content: String,
    tags: Vec<String>,
    source: SerCandidateSource,
) -> Result<SerCandidateMemory, String>;

#[tauri::command]
async fn archive_candidate_memory(
    state: tauri::State<'_, AppState>,
    candidate_id: String,
) -> Result<SerCandidateMemory, String>;

#[tauri::command]
async fn list_candidate_memories(
    state: tauri::State<'_, AppState>,
    project_id: String,
    include_archived: bool,
) -> Result<Vec<SerCandidateMemory>, String>;
```

---

## 9. 风险与缓解

| 风险 | 影响 | 缓解措施 |
|:---|:---|:---|
| Review Queue 数据膨胀 | 长期运行后 `memory_reviews.json` 过大 | 1) 定期清理 asset 已不存在的 orphaned items；2) 提供"清除已忽略项目"的手动操作；3) 数据量预计 < 1000 条，JSON 全量覆盖可接受 |
| 用户误以为"标记已审"=问题已修复 | 高：产品信任危机 | 1) Action 按钮使用"标记已审"而非"已修复"；2) 建议面板明确说明"请在 IDE 中执行以下操作"；3) 首次使用时有 tooltip 解释 |
| Snooze 状态丢失 | 中：用户体验受损 | Snooze 时间戳写入 `memory_reviews.json`，与项目 ID 绑定，不依赖 localStorage |
| Audit Log 写入频率过高 | 低：I/O 性能 | 采用追加写入 JSON Lines，单次写入 < 1KB；Review Item 状态变更为低频操作 |
| Candidate Memory 边界模糊 | 中：用户困惑 | UI 中强制显示警告文案；不将 Candidate Memory 与 .claude/ 文件并列展示 |
| 多项目状态隔离 | 低：数据混淆 | ReviewItem/CandidateMemory 均包含 `project_id` 字段；前端按当前选中项目过滤 |

---

## 10. 实现计划

### 10.1 批次划分（收口后）

| 批次 | 内容 | 预计文件变更 | 依赖 |
|:---|:---|:---|:---|
| **Batch 1** | Context Pressure 只读指标（Rust 计算 + Claude Memory 页面顶部 Banner） | 3-4 个文件 | 无 |
| **Batch 2** | Review Queue Rust 数据结构 + 持久化 API + sync command | 6-8 个文件 | Batch 1 可选并行 |
| **Batch 3** | Review Queue 前端 UI（列表/筛选/排序/状态切换按钮）+ Action Suggestions 面板 | 5-6 个文件 | Batch 2 |
| **Batch 4** | Audit Log 追加写入 + 查看页面 | 3-4 个文件 | Batch 2 |
| **Batch 5** | Candidate Memory 数据结构 + 浮窗 UI | 4-5 个文件 | Batch 2 |

**Batch 1 明确不做**：
- ❌ Review Queue 持久化
- ❌ Action Suggestions 交互按钮
- ❌ Audit Log 写入
- ❌ Candidate Memory 收集

### 10.2 建议实施顺序

```
Batch 1:
  └─ Context Pressure (Rust + Claude Memory 页面顶部 Banner)

Batch 2:
  ├─ SerReviewItem / SerReviewQueue / SerReviewState 数据结构
  ├─ memory_reviews.json 持久化读写
  └─ sync_review_queue command（Health Report → Review Queue 合并去重）

Batch 3:
  ├─ Review Queue Panel UI（列表 + 筛选/排序 + 状态切换按钮）
  └─ Action Suggestions 建议面板（纯文本展示，无自动执行按钮）

Batch 4:
  └─ Audit Log 追加写入 + Settings 查看页面

Batch 5:
  └─ Candidate Memory 浮窗 + 列表管理
```

### 10.3 验收标准

**Batch 1（Context Pressure）**：
- [ ] Context Pressure Banner 在 Claude Memory 页面顶部超过阈值时正确显示，可关闭
- [ ] 指标计算使用真实 asset 数据（`line_count`、`byte_size`、`exists` 等）
- [ ] 不阻塞用户操作，纯信息性展示
- [ ] 所有新增 Rust 代码通过 `cargo test` 和 `cargo clippy`
- [ ] 所有新增 UI 通过 Playwright E2E 基础测试（渲染、空状态、错误状态）

**Batch 2+（Review Queue 及后续）**：
- [ ] Review Queue 能正确从 Health Report 生成，支持去重匹配
- [ ] 状态切换（pending → reviewed/ignored/snoozed）正确持久化到文件
- [ ] Snooze 到期后自动回到 pending（通过下次 sync 触发）
- [ ] Action Suggestions 面板使用安全措辞，按钮不出现"已修复"/"已移除"等误导文案
- [ ] Audit Log 记录所有状态变更，支持按类型筛选
- [ ] Candidate Memory 创建时显示边界警告，内容不写入 .claude/

---

## 附录 A：与 Phase 1/2 的兼容性

Phase 3 的所有新增能力**不改变** Phase 1/2 的任何现有接口：

| 已有接口 | 影响 |
|:---|:---|
| `get_claude_memory_overview` | 无变化 |
| `get_memory_health_report` | 无变化；返回值被 Batch 2 `sync_review_queue` 消费 |
| `simulate_load_chain` | 无变化 |
| `MemoryAssetTree` / `MemoryAssetDetail` | 无破坏性变更；仅增加叠加显示 Review 状态 |

---

## 附录 B：术语表

| 术语 | 定义 |
|:---|:---|
| **Review Item** | 一个可审阅的健康问题实例，有独立生命周期 |
| **Review Queue** | 某项目下所有 Review Item 的集合 |
| **Context Pressure** | 基于记忆资产数量和加载链深度的上下文负荷指标 |
| **Action Suggestion** | 针对特定 issue_type 的建议文本，指导用户在项目外采取行动 |
| **Audit Log** | 记录 Review Item 状态变更和系统扫描事件的日志 |
| **Candidate Memory** | 用户在 AgentScope 内收集的、待后续手动写入 .claude/ 的记忆片段 |
| **Trust Boundary** | AgentScope 作为观察工具，绝不修改用户项目文件的产品边界 |
