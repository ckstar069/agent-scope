# AgentScope - AI 上下文治理 v0.1 设计草案

> 状态：设计草案
> 创建日期：2026-05-23
> 关联文档：
> - `docs/requirements-claude-memory-management.md`
> - `docs/design-claude-memory-management-v0.2.md`

---

## 1. 目标

本文档补充 Claude Memory 后续治理能力的产品骨架。它不改变当前 P1/P2 的只读实施范围，而是为健康检测、会话候选知识、审查队列和跨工具上下文资产盘点提供共同模型。

AgentScope 的治理顺序是：

```text
发现资产
    ->
解释注入
    ->
评估质量
    ->
生成待审项
    ->
用户决定写入、合并、归档或忽略
```

边界：
- 当前真源仍是各工具真实资产，Claude 资产以 Claude Code 文件系统语义为准
- AgentScope 不先发明一套统一身份层，也不在扫描时静默吸收外部工具资产
- 会话和外部资产产生的是待审项，不默认写入 `CLAUDE.md`、rules、skills 或 auto memory

---

## 2. 三个核心模型

### 2.1 Memory Asset

Memory Asset 是文件系统或工具配置中的真实上下文资产，例如：
- `CLAUDE.md`
- `.claude/rules/*.md`
- `~/.claude/projects/<project>/memory/*.md`
- `AGENTS.md`
- `.cursorrules`

它回答：
- 资产在哪里
- 属于什么作用域
- 何时加载或触发
- 是否存在风险

当前 `SerClaudeMemoryAsset` 和加载链模型已经覆盖 Claude Asset v0.1 的主干。

### 2.2 Memory Candidate

Memory Candidate 是尚未写入真实资产的候选知识。来源可以是：
- 历史会话片段
- 加载链或健康检测产生的拆分建议
- 跨工具资产盘点产生的迁移建议
- 用户手工标记

建议字段：

```ts
type MemoryCandidate = {
  id: string;
  sourceType: "session" | "asset" | "inventory" | "manual";
  sourceRef: string;
  sourceSnippet?: string;
  suggestedKind: "instruction" | "rule" | "skill" | "decision" | "lesson" | "project-note";
  suggestedTarget?: string;
  reason: string;
  status: "pending" | "accepted" | "rejected" | "merged" | "archived";
  duplicateOf?: string;
  createdAtMs: number;
};
```

Candidate 必须可追溯，且不得把“已提取”混同于“已生效”。

### 2.3 Governance Item

Governance Item 是 Review Queue 中的统一审查单元。它可以引用 Candidate，也可以引用已有 Asset。

首批类型：

| 类型 | 来源 | 典型动作 |
|------|------|----------|
| candidate | 会话或手工标记 | 接受、拒绝、合并、归档 |
| duplicate | 轻量相似度或重复段落检测 | 合并、忽略、保留并解释 |
| conflict | 结构冲突或后续语义冲突 | 选择真源、修复、延后 |
| split_suggestion | Context Pressure 或大文件检测 | 拆成 rule/skill、忽略 |
| security_blocker | Secret Scanner | 修复后继续、阻止写入/同步 |
| drift | 跨工具资产盘点 | 预览转换、保留差异、延后 |

---

## 3. Review Queue

Review Queue 是治理能力的收口点，不是单个确认弹窗。

### 3.1 必须展示

- 问题或候选摘要
- 来源与证据
- 涉及资产
- 建议动作及理由
- 风险级别
- 是否会写入真实文件

### 3.2 首批动作

| 动作 | 说明 |
|------|------|
| Accept | 接受 Candidate，进入明确的写入预览 |
| Reject | 判定不应沉淀，保留审查记录 |
| Merge | 将重复 Candidate 或 Asset 建议并入主项 |
| Archive | 归档暂不需要的治理项 |
| Defer | 延后，保留原因和下次复查入口 |

### 3.3 写入门

Accept 不是直接写盘。进入写入前仍需：
1. 展示目标文件和 diff
2. 通过 allowlist
3. 做备份和并发修改检测
4. 对 Secret blocker 保持阻断

---

## 4. Health Score v1

Health Score v1 面向 Claude 上下文资产，不评价用户画像，也不依赖 NLP 语义冲突。

### 4.1 维度

| 维度 | 关注点 | v1 信号 |
|------|--------|---------|
| Load Pressure | 启动上下文负担 | 启动链资产数、总行数、总字节、重资产占比 |
| Safety | 安全风险 | secret、私有 URL、越界 import、同步阻断项 |
| Cleanliness | 清洁度 | 重复段落、近似重复 rules、重复 import 内容 |
| Consistency | 结构一致性 | 循环 import、过深 import、失效 paths、硬编码平台路径 |
| Explainability | 可解释性 | 是否能说明加载原因、排除原因、触发范围 |

### 4.2 输出

健康页应同时返回：
- 总分和维度分
- Top issues
- Top contributing assets
- 可生成 Governance Item 的行动建议

`Context Pressure` 的首批行动建议：
- `CLAUDE.md` 过重时建议拆成 rule 或 skill
- 大量无条件 rules 时提示缩小触发范围
- 同一内容在 instruction 和 rule 中重复时生成 duplicate item

### 4.3 分阶段实现

| 阶段 | 范围 |
|------|------|
| v1 | 结构信号、轻量重复检测、加载压力 |
| v2 | 健康趋势、跨 host 漂移、复查周期 |
| v3 | 语义冲突和 AI 辅助拆分建议 |

---

## 5. 重复与冲突策略

先做无需 NLP 的 v1：
- 归一化 Markdown 段落后比对重复内容
- 对标题、摘要、frontmatter description 和 Candidate 内容做轻量相似度检测
- 对 import 图、paths、平台硬编码和覆盖范围做结构检查

v1 输出必须是候选，不自动删除资产。

语义冲突放到后续：
- 包管理器命令矛盾
- `always` / `never`、`必须` / `禁止` 等约束矛盾
- 跨工具同类规则语义漂移

---

## 6. 跨工具资产盘点

跨工具能力先从 inventory 开始，而不是直接同步。

首批盘点对象：
- Claude：`CLAUDE.md`、`.claude/`
- Codex / 通用 Agent：`AGENTS.md`
- Cursor：`.cursorrules`
- Copilot：`.github/copilot-instructions.md`
- OpenCode 或其他本地上下文文件：后续按显式规则扩展

输出：
- 工具来源
- 资产路径
- 作用域
- 是否与 Claude Asset 重复
- 是否只有单一工具可见
- 是否存在明显漂移

转换或同步前必须先给 preview；扫描本身只读。

---

## 7. 与当前路线的关系

| 当前能力 | 治理扩展 |
|----------|----------|
| Claude Memory 资产扫描 | Asset Inventory |
| 加载链模拟器 | Context Pressure |
| Secret Scanner | Safety 维度和 blocker |
| 会话历史预览 | Memory Candidate 来源 |
| 模板项目候选记忆 | Candidate v0 原型，后续脱离模板路径约束 |

建议后续实施顺序：
1. P3 健康检测 v1：Context Pressure + 重复检测 v1 + 健康维度
2. Candidate 模型泛化：从模板项目 decisions 写入流拆出候选层
3. Review Queue：先承接 Candidate、duplicate、security blocker
4. Cross-tool Asset Inventory：先只读盘点，再做 conversion preview

