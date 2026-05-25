# AgentScope — Memory Health Phase 1 实施设计

> 状态：已实施 + review gate 完成
> 创建日期：2026-05-25
> 关联文档：`docs/piia-engram-borrowing-analysis.md`、`docs/design-ai-context-governance-v0.1.md`

---

## 1. 当前 AgentScope memory 数据流

```
前端 invoke("get_claude_memory_overview")
  → routes::get_claude_memory_overview_cmd()
    → services::get_claude_memory_overview_service()
      → scanner::scan_claude_memory(project_path)       // baseline scan (user-level)
      → scanner::scan_project_level() × N               // registered projects
      → secret_scanner::SecretScanner::scan() per file
      → frontmatter::extract/parse per .md file
      → 返回 SerClaudeMemoryScanResult
        → 前端 ClaudeMemoryOverview 类型
          → ClaudeMemoryAssets 页面渲染
            → StatCard × 4 (total_assets, total_existing, total_secret_issues, claude_config_dir)
            → MemoryAssetTree (分组展示)
            → MemoryAssetDetail (内容详情)
```

**涉及文件和结构体**：

| 层 | 文件 | 核心结构体 |
|----|------|-----------|
| Collector | `scanner.rs` | `SerClaudeMemoryScanResult`, `SerClaudeMemoryAsset` |
| Collector | `secret_scanner.rs` | `SecretScanner` → `Vec<SerSecretIssue>` |
| Collector | `load_chain.rs` | `SerLoadChain`, `SerLoadChainStep` |
| Collector | `models.rs` | 全部 `Ser*` 定义 |
| Service | `claude_memory_service.rs` | `get_claude_memory_overview_service()` |
| Route | `routes/claude_memory.rs` | 3 个 `#[tauri::command]` |
| 前端 | `types.ts` | `ClaudeMemoryOverview`, `ClaudeMemoryAsset` |
| 前端 | `useClaudeMemory.ts` | `useClaudeMemory()` hook |
| 前端 | `index.tsx` | `ClaudeMemoryAssets` 页面 |
| 前端 | `MemoryAssetTree.tsx` | 资产树分组展示 |
| 前端 | `MemoryAssetDetail.tsx` | 资产详情面板 |

---

## 2. 新增数据模型

### 2.1 SerMemoryHealthReport

```rust
#[derive(Debug, Clone, Serialize)]
pub struct SerMemoryHealthReport {
    pub overall_score: u8,                    // 0-100 加权汇总
    pub freshness: SerHealthDimension,        // 新鲜度
    pub quality: SerHealthDimension,          // 质量
    pub coverage: SerHealthDimension,         // 覆盖度
    pub cleanliness: SerHealthDimension,      // 清洁度
    pub safety: SerHealthDimension,           // 安全性
    pub top_issues: Vec<SerMemoryHealthIssue>,// Top 5 问题
    pub stale_assets: Vec<SerMemoryStaleness>,// 过期资产列表
    pub duplicate_groups: Vec<SerMemoryDuplicateGroup>,// 重复组列表
}
```

### 2.2 SerHealthDimension

```rust
#[derive(Debug, Clone, Serialize)]
pub struct SerHealthDimension {
    pub name: String,         // "freshness" | "quality" | "coverage" | "cleanliness" | "safety"
    pub score: u8,            // 0-100
    pub reason: String,       // 评分理由（人可读）
    pub contributing_assets: Vec<String>,  // 贡献最大的 asset id（最多 3 个）
}
```

### 2.3 SerMemoryHealthIssue

```rust
#[derive(Debug, Clone, Serialize)]
pub struct SerMemoryHealthIssue {
    pub issue_type: String,     // "stale" | "too_long" | "secret_risk" | "duplicate" | "missing_instruction" | "context_pressure"
    pub severity: String,       // "critical" | "warning" | "info"
    pub asset_ids: Vec<String>, // 关联的 asset id
    pub message: String,        // 人可读描述
    pub suggestion: String,     // 建议动作
}
```

### 2.4 SerMemoryStaleness

```rust
#[derive(Debug, Clone, Serialize)]
pub struct SerMemoryStaleness {
    pub asset_id: String,
    pub asset_type: String,
    pub scope: String,
    pub logical_path: String,
    pub mtime_ms: Option<u64>,
    pub stale_days: Option<u64>,    // 自 mtime 起的天数（None 表示无 mtime）
    pub threshold_days: u64,        // 使用的阈值天数
}
```

### 2.5 SerMemoryDuplicateGroup

```rust
#[derive(Debug, Clone, Serialize)]
pub struct SerMemoryDuplicateGroup {
    pub group_id: String,
    pub asset_ids: Vec<String>,      // 组内所有 asset id
    pub similarity: f64,             // 0.0-1.0 组平均相似度
    pub overlap_content: String,     // 共现内容摘要（前 200 字）
    pub suggestion: String,          // "merge" | "review" | "ignore"
}
```

---

## 3. 健康评分规则

五个维度，每个 0-100，加权汇总：

**overall = freshness×0.2 + quality×0.2 + coverage×0.2 + cleanliness×0.25 + safety×0.15**

### 3.1 Freshness（新鲜度）

| 信号 | 计算来源 | 规则 |
|------|---------|------|
| 过期资产占比 | `mtime_ms` vs 当前时间 | stale_days > threshold 的资产数 / 总 existing 资产数。占比 0% → 100，占比 >50% → 0，中间线性插值 |
| 阈值 | 配置常量 | 默认 30 天。auto_memory 专项 14 天（Auto Memory 预期高频更新） |
| mtime 不可用 | `mtime_ms = None` | 视为 "unknown"，不算 stale 也不算 fresh，该资产不参与新鲜度计算 |

**公式**：`freshness = 100 × (1 - stale_ratio)`，其中 `stale_ratio = stale_count / eligible_count`

**contributing_assets**：stale_days 最长的 3 个 asset

### 3.2 Quality（质量）

| 信号 | 计算来源 | 规则 |
|------|---------|------|
| 过长资产占比 | `line_count` | instruction（CLAUDE.md 等）或 auto_memory_index（MEMORY.md）`line_count > 200` → "too_long"。过长占比 0% → 100，>30% → 0 |
| frontmatter 缺失率 | `frontmatter` | rule/skill/agent 没有 frontmatter 的占比。缺失率 0% → 100，>50% → 0 |
| 大文件占比 | `byte_size > 100KB` | 大文件占比 >20% → 扣分 |

**公式**：`quality = 100 × (1 - (too_long_ratio×0.5 + no_frontmatter_ratio×0.3 + large_file_ratio×0.2))`

**contributing_assets**：line_count 最长的 3 个 asset

### 3.3 Coverage（覆盖度）

| 信号 | 计算来源 | 规则 |
|------|---------|------|
| instruction 文件覆盖 | `exists` + `scope` | 缺少 user_claude_md 扣 20 分，缺少 project_claude_md 扣 20 分，缺少 local_md 扣 10 分 |
| rules 覆盖 | `assets` 中 rules 数 | 有至少 1 条 unconditional rule → 不扣分；0 条 → 扣 30 分 |
| auto memory 覆盖 | `exists` | auto_memory_index 存在 → 不扣分；缺失 → 扣 20 分 |

**公式**：`coverage = 100 - penalty`（penalty 为各缺失项的固定扣分之和，上限 100）

**contributing_assets**：缺失的 instruction asset id

### 3.4 Cleanliness（清洁度）

| 信号 | 计算来源 | 规则 |
|------|---------|------|
| 重复组数 | `duplicate_groups` | 重复组数 0 → 100，>5 组 → 0，中间线性 |
| 重复资产占比 | `duplicate_groups` 中涉及的 asset 数 / total | 占比 0% → 不扣分，>20% → 扣 50 分 |

**公式**：`cleanliness = 100 × (1 - (duplicate_ratio×0.6 + duplicate_group_penalty×0.4))`

**contributing_assets**：相似度最高的 3 个 duplicate 组中的 asset id

### 3.5 Safety（安全性）

| 信号 | 计算来源 | 规则 |
|------|---------|------|
| secret_issues 总数 | `SerSecretIssue` | 0 → 100，>5 → 0，中间线性 |
| secret_issues 严重分布 | `issue_type` | env_content/private_url 视为 critical，权重更高 |

**公式**：`safety = 100 × (1 - min(1.0, (total_secrets×0.7 + critical_secrets×0.3) / 10))`

**contributing_assets**：secret_issues 最多的 3 个 asset id

---

## 4. 过期检测规则

| 项 | 规则 |
|----|------|
| 默认阈值 | 30 天（2592000000 ms） |
| Auto Memory 专项阈值 | 14 天（1209600000 ms）—— Auto Memory 预期高频更新，14 天未修改更可能过期 |
| mtime 不可用时 | `mtime_ms = None` → `stale_days = None`，该资产不计入 stale，也不计入 fresh，在 UI 中标记为 "unknown freshness" |
| 不同 memory 类型 | 不区分 instruction/rule/skill/agent 的阈值，统一 30 天；仅 auto_memory 用 14 天 |
| 过期检测范围 | 只检测 `exists = true` 的资产 |

---

## 5. 重复检测 v1 规则

### 5.1 段落切分

- 按 Markdown 标题（`#`、`##`、`###`）切分
- 没有标题的文件视为一个整段落
- 切分后每个段落保留：标题文本 + 正文文本
- 跳过 frontmatter（`---...---` 包裹的内容）

### 5.2 文本归一化

```
原始文本 → 去除 Markdown 语法标记 → 去除空白/换行 → 小写化 → 去除标点
```

具体步骤：
1. 去除 `#`、`*`、`-`、`>` 等 Markdown 标记
2. 去除连续空白，合并为单空格
3. 转小写
4. 去除 `。`、`，`、`！`、`？`、`.`、`,`、`!`、`?`、`;`、`:` 等标点

### 5.3 hash / lightweight similarity

**策略：两阶段检测**

**阶段 1 — 精确 hash 匹配**：
- 对归一化后的段落文本计算 `DefaultHasher`（64-bit）hash
- hash 完全相同的段落 → 精确重复（similarity = 1.0）
- 精确重复直接归入 duplicate_group
- 注：采用 `DefaultHasher` 而非 SHA-256，≤50 资产碰撞概率可忽略，避免引入 crypto 依赖

**阶段 2 — Jaccard 相似度**：
- 对归一化后的文本按空格分词（word tokenization）
- 计算两个段落的词集合 Jaccard：`|A∩B| / |A∪B|`
- Jaccard ≥ 0.8 → 近似重复（similarity = jaccard 值）
- 近似重复归入 duplicate_group，suggestion = "review"（不是 merge，需要人工确认）

### 5.4 group 输出格式

```
SerMemoryDuplicateGroup {
  group_id: "dup_<hash_prefix>",
  asset_ids: ["asset_id_1", "asset_id_2"],
  similarity: 0.85,
  overlap_content: "归一化后的共现内容前 200 字",
  suggestion: "review" | "merge"  // merge=精确重复, review=近似重复
}
```

### 5.5 性能上限

- **只对 exists=true 且 line_count > 0 的资产做检测**
- **只对 content_preview 可用的资产做检测**（content_truncated=true 的大文件只基于前 2KB 检测）
- **资产数 ≤ 50 时全量比对；> 50 时只比对同 scope 内的资产**
- **时间上限**：重复检测应在 500ms 内完成（普通 SSD 环境）

---

## 6. UI 最小展示方案

### 6.1 设计原则

- 不做复杂 Review Queue
- 只在现有 memory 面板展示评分、过期、重复提示
- 复用现有 Card/Collapsible/Switch 等组件
- 评分数据跟随 overview 返回，不需要单独 API 调用

### 6.2 修改点

**StatCard 区域扩展**：

当前 4 个 StatCard：total_assets, total_existing, total_secret_issues, claude_config_dir

新增 1 个：**Health Score**（0-100，颜色：≥80 绿、≥60 黄、<60 红）

**AssetTree 过期标记**：

在 asset 按钮中，stale 资产增加淡色/删除线样式，tooltip 显示 "stale for N days"

**AssetDetail 过期/重复信息**：

在 AssetMetaHeader 中增加：
- stale badge：过期天数
- duplicate badge：如果该 asset 在某个 duplicate_group 中

**底部新增 Collapsible "Health Details" 面板**：

展示 5 维度雷达图（简化为 5 个进度条）+ Top Issues 列表 + 重复组列表

---

## 7. 测试计划

### 7.1 Rust 单元测试

| 测试 | 覆盖 | 文件 |
|------|------|------|
| 健康评分计算 | 5 维度分数计算正确 | `health_checker.rs` |
| 过期检测 | stale_days 计算正确，阈值区分 auto/其他 | `health_checker.rs` |
| 重复检测 v1 | 精确 hash 匹配 + Jaccard 近似匹配 | `dedup.rs` |
| 归一化文本 | Markdown 去标记 + 去标点 + 小写 | `dedup.rs` |
| 段落切分 | 按标题切分 + 无标题整段 | `dedup.rs` |
| 集成测试 | 从 scan result 计算完整 health report | `health_checker.rs` |
| 边界测试 | 无资产 / 全 stale / 全 secret | `health_checker.rs` |

### 7.2 前端最小测试

| 测试 | 覆盖 |
|------|------|
| 类型定义编译 | `types.ts` 新增字段无 TypeScript 错误 |
| npm run build | 前端构建通过 |

### 7.3 不做的测试

- E2E 测试不在本次范围（UI 变动小，可后续补充）

---

## 8. 分步实施清单

### Step 1：Rust 数据结构（models.rs 扩展）

| 文件 | 变更 |
|------|------|
| `models.rs` | 新增 SerMemoryHealthReport、SerHealthDimension、SerMemoryHealthIssue、SerMemoryStaleness、SerMemoryDuplicateGroup |

验证：`cd src-tauri && cargo check`

### Step 2：文本归一化和段落切分（dedup.rs 新增）

| 文件 | 变更 |
|------|------|
| `collectors/claude_memory/dedup.rs` | 新增 normalize_text()、split_paragraphs()、compute_content_hash()、compute_jaccard_similarity()、find_duplicates() |
| `collectors/claude_memory/mod.rs` | 新增 `pub mod dedup;` |

验证：`cd src-tauri && cargo test`（含 dedup 单元测试）

### Step 3：过期检测和健康评分计算（health_checker.rs 新增）

| 文件 | 变更 |
|------|------|
| `collectors/claude_memory/health_checker.rs` | 新增 compute_staleness()、compute_health_report()、各维度计算函数 |
| `collectors/claude_memory/mod.rs` | 新增 `pub mod health_checker;` |

验证：`cd src-tauri && cargo test`（含 health_checker 单元测试）

### Step 4：Service 和 Route 扩展

| 文件 | 变更 |
|------|------|
| `services/claude_memory_service.rs` | 新增 get_memory_health_report_service() |
| `routes/claude_memory.rs` | 新增 get_memory_health_report_cmd Tauri command |
| `lib.rs` | 注册新命令到 generate_handler |

验证：`cd src-tauri && cargo check`

### Step 5：前端类型和 API 扩展

| 文件 | 变更 |
|------|------|
| `types.ts` | 新增 MemoryHealthReport、HealthDimension、MemoryHealthIssue、MemoryStaleness、MemoryDuplicateGroup 类型 |
| `hooks/useClaudeMemory.ts` | 新增 useMemoryHealth hook |
| `lib/api.ts` | 新增 getMemoryHealthReport invoke 封装 |

验证：`npm run build`

### Step 6：前端 UI 最小展示

| 文件 | 变更 |
|------|------|
| `index.tsx` | StatCard 区域新增 Health Score 卡；底部新增 Health Details Collapsible 面板 |
| `MemoryAssetTree.tsx` | stale 资产样式增加 |
| `MemoryAssetDetail.tsx` | AssetMetaHeader 增加 stale/duplicate badge |

验证：`npm run build`

### Step 7：集成验证

| 验证项 | 命令 |
|--------|------|
| Rust 编译 | `cd src-tauri && cargo check` |
| Rust 测试 | `cd src-tauri && cargo test` |
| 前端构建 | `npm run build` |
| Tauri 构建 | `npm run tauri dev`（手动验证 UI 展示） |

---

## 附录 A：实施状态

### 已实现功能

| 功能 | 后端 | 前端 | 测试 |
|:---|:---|:---|:---|
| 健康评分（5 维度加权） | `health_checker.rs::compute_health_report` | `index.tsx` StatCard + 折叠面板 | 12 个单元测试 |
| 过期检测（mtime） | `health_checker.rs::compute_staleness` | 同上（freshness 维度） | 3 个边界测试 |
| 重复检测 v1 | `dedup.rs::find_duplicates` | 同上（cleanliness 维度） | 13 个单元测试 |

### 已实现字段

**SerMemoryHealthReport**
- `overall_score: u8` — 加权总分 (0..=100)
- `freshness/quality/coverage/cleanliness/safety: SerHealthDimension` — 各含 `name, score, reason`
- `top_issues: Vec<SerMemoryHealthIssue>` — 含 `severity, message, suggestion`
- `stale_assets: Vec<SerMemoryStaleness>` — 含 `asset_id, stale_days, threshold_days`（无 `reason` 字段）
- `duplicate_groups: Vec<SerMemoryDuplicateGroup>` — 含 `group_id, asset_ids, similarity, overlap_content, suggestion`

### 评分规则详解

| 维度 | 权重 | 计算方式 | 数据来源 |
|:---|:---|:---|:---|
| freshness | 0.20 | `(1 - stale_ratio) × 100` | `mtime_ms` + 类型阈值 |
| quality | 0.20 | `(1 - (too_long×0.5 + no_fm×0.3 + large×0.2)) × 100` | `line_count`, `frontmatter`, `byte_size` |
| coverage | 0.20 | `100 - penalty`（固定扣分规则） | `exists` + `scope` + `asset_type` |
| cleanliness | 0.25 | `(1 - (dup_ratio×0.6 + group_penalty×0.4)) × 100` | `find_duplicates` 结果 |
| safety | 0.15 | `(1 - severity_weighted_ratio) × 100` | `secret_issues` 字段 |

所有维度分数经过 `.clamp(0.0, 100.0)` 保证 u8 安全转换。

### 当前评分限制

1. **freshness**：依赖 `mtime_ms`。若文件无 mtime（旧扫描数据），该文件不参与 freshness 计算。无 mtime 不等于"过期"。
2. **quality**：`too_long` 阈值固定 200 行，`large` 阈值固定 100KB，未做类型差异化。
3. **coverage**：仅统计 `exists` 布尔值，不衡量内容深度或完整度。
4. **cleanliness**：重复检测基于词集合 Jaccard 相似度，不理解语义。阈值 0.8 可能对短文本敏感。
5. **safety**：仅检测 `secret_issues` 中已标记的问题，不主动扫描密钥。

### 未采用方案及理由

| 方案 | 不采用理由 |
|:---|:---|
| Embedding 向量相似度 | 需要外部模型服务，桌面离线应用无法保证可用性 |
| LLM 辅助评分 | 同上，且评分延迟高、成本不可控 |
| 持久化评分历史 | Phase 1 聚焦当前快照诊断，历史趋势属于 Phase 2 |
| SHA-256 内容 hash | 采用 `DefaultHasher`（64-bit），≤50 资产碰撞概率可忽略，避免引入 crypto 依赖 |
| 语义级别重复检测 | 依赖 embedding，超出 Phase 1 范围 |

### 未实施功能（留待未来）

- **Context Pressure**：计算加载链中有效 token 估算，需要实现 tokenizer 集成
- **Memory Candidate**：基于规则生成候选记忆条目，需要定义规则引擎
- **Review Queue**：需要持久化存储 + 交互式确认流程
- **评分历史趋势**：需要持久化存储 + 时序数据管理
- **类型差异化质量阈值**：不同 asset_type 使用不同的 line/size 阈值
- **Health Report 与 Overview 双扫描优化**：当前前端调用 overview 和 health_report 会触发两次独立扫描（overview service 中 force 参数实际被忽略），Phase 1 后续可改为共享缓存或合并为单次扫描