# AgentScope v0.3.7 Usage Analytics 补强调研与规划

> 建议存放路径：`docs/research/agent-scope-v0.3.7-usage-analytics-research.md`  
> 目的：为 AgentScope v0.3.7 规划“usage analytics / token 统计 / 额度窗口 / 成本估算 / 数据源可信度”相关能力，供后续 Agent 读取和拆分任务。  
> 当前前提：v0.3.6 已完成性能、会话预览、Token 速率估算、Tooltip、UI 语义收口，应进入正式发布；本文不应再塞入 v0.3.6。

---

## 1. 背景

AgentScope v0.3.6 已经完成以下核心收口：

- Agent 监控首屏快照与会话管理加载性能优化；
- 记忆资产 dashboard 合并扫描、异步化与刷新去重；
- Token 速率计算语义修正：不再使用 `active_tokens / session.elapsed()` 伪全程均速；
- 窗口速率采用 1 分钟、5 分钟窗口内 token delta；
- 速率 UI 明确为估算性质；
- 会话预览去除 `last-prompt` 造成的伪重复；
- 默认折叠工具调用，避免工具噪音淹没用户/助手正文；
- 关键指标增加轻量 Tooltip；
- 速率统计按钮合并为单组：5分钟平均、1分钟平均、监控期平均、最近2秒。

在人工审核中发现，当前 Token 速率仍会出现“跳变/飘忽”的观感。经排查，这主要不是 AgentScope 计算 bug，而是数据源机制决定的：当前通过 `abtop-collector` 读取 Claude Code 本地 session/transcript 文件，Token usage 通常来自 Claude Code JSONL 中 assistant 消息的 `usage` 字段，而该字段往往在回复完成后才写入。因此流式输出过程中，本地 usage 不一定实时增长。

---

## 2. 当前 AgentScope 机制

当前 AgentScope 的 Agent runtime 采集路径是：

```text
Claude Code 正常运行
  -> 写入 ~/.claude/sessions/*.json
  -> 写入 ~/.claude/projects/*/*.jsonl
  -> abtop-collector 周期性扫描进程和落盘文件
  -> 解析 AgentSession、token usage、工具调用、文件访问、上下文等
  -> AgentScope 后端每约 2 秒通过 Tauri event 推送给前端
```

### 2.1 数据来源

当前 Token usage 主要来自 Claude Code transcript JSONL 的 assistant message usage 字段：

```json
{
  "type": "assistant",
  "message": {
    "usage": {
      "input_tokens": 123,
      "output_tokens": 456,
      "cache_read_input_tokens": 789,
      "cache_creation_input_tokens": 10
    }
  }
}
```

AgentScope / abtop-collector 从这些字段累加：

- input tokens；
- output tokens；
- cache read input tokens；
- cache creation input tokens；
- token_history；
- context_history；
- tool calls；
- file accesses。

### 2.2 当前机制优点

- 非侵入，不需要修改 Claude Code；
- 不需要拦截模型 API；
- 可兼容官方 Claude Code 与部分第三方 Anthropic-compatible endpoint；
- 适合最终 usage 汇总、会话历史分析、工具调用审计；
- 可以与会话预览、文件访问、Claude Memory 等 AgentScope 桌面能力结合。

### 2.3 当前机制限制

- 不是实时 token stream；
- 流式输出中 token usage 可能不更新；
- 速率本质是基于已写入 usage 的采样估算；
- 状态是 best-effort 推断，不是 Claude Code 官方状态 API；
- 依赖 Claude Code 本地文件结构，后续 Claude Code 结构变化可能导致解析适配成本；
- 只靠 transcript 文本不能精确重建 provider-side usage、hidden system context、cache 计费和工具 overhead。

---

## 3. 社区工具对标

### 3.1 ccusage

项目：`ryoppippi/ccusage`  
地址：`https://github.com/ryoppippi/ccusage`

公开定位：从本地 coding agent CLI 数据中分析 token usage 与 cost。其 README 描述支持 Claude Code、Codex、OpenCode、Kimi、Qwen、Gemini CLI 等多个来源，并提供 daily、weekly、monthly、session、blocks、statusline、JSON output、model breakdown、cost tracking、cache token support、custom paths 等能力。

对 AgentScope 的启发：

- 本地 usage 文件解析是主流路线，不是错误路线；
- 需要补齐 historical usage analytics，而不是只做实时监控；
- 需要支持更完整的数据目录发现与自定义路径；
- 需要提供 JSON/CSV 导出；
- 需要支持按日期、项目、模型、会话聚合；
- 需要支持成本估算与 cache token 分项。

### 3.2 Claude-Code-Usage-Monitor

项目：`Maciek-roboblog/Claude-Code-Usage-Monitor`  
地址：`https://github.com/Maciek-roboblog/Claude-Code-Usage-Monitor`

公开定位：面向 Claude Code token usage 的实时终端监控工具，提供 burn rate、cost analysis、session limit prediction、progress bars、daily/monthly views、P90 预测、5-hour window 等能力。

其文档说明数据流为：

```text
Claude Config Files -> Data Layer -> Analysis Engine -> UI Components -> Terminal Display
```

这说明它同样不是 API streaming proxy，而是基于 Claude 本地配置/usage 文件构建实时刷新和统计预测层。

对 AgentScope 的启发：

- “real-time monitoring”可以基于本地文件轮询，但应明确为刷新已知数据，不等于 streaming token 精确计数；
- Claude Code 的 5 小时 rolling window 是使用者真正关心的额度管理视角；
- burn rate 不应只看最近 2 秒，应有更稳健的 1 小时、P90、窗口预测等统计层；
- 成本、额度、预测、告警比瞬时 token/s 更有实际价值。

---

## 4. 与主流工具相比，AgentScope 当前不足

### 4.1 数据目录覆盖不足

主流工具倾向支持多个本地数据路径和自定义路径。AgentScope 当前需要重点补齐：

- 默认支持 `~/.claude`；
- 默认支持 `~/.config/claude`；
- 支持 `CLAUDE_CONFIG_DIR`；
- 支持 `CLAUDE_CONFIG_DIR` 逗号分隔多个目录；
- UI 中展示当前识别到的数据目录；
- 对不可读、缺失、空目录给出诊断提示。

### 4.2 缺少历史 usage 报表

AgentScope 当前偏实时桌面面板，缺少：

- 今日、本周、本月 usage；
- 按项目聚合；
- 按模型聚合；
- 按会话聚合；
- 按 5 小时 block 聚合；
- date range 过滤；
- JSON / CSV 导出；
- 与 ccusage 类似的 usage report 视图。

### 4.3 缺少 Claude Code 5 小时额度窗口

用户真正关心的是：

- 当前 5 小时窗口从什么时候开始；
- 什么时候结束；
- 已消耗多少；
- 当前 burn rate 下还能撑多久；
- 是否会接近限额；
- 多个 session 是否重叠消耗。

AgentScope 当前速率显示是 session-level 短窗口估算，不是 quota window 视角。

### 4.4 缺少成本估算

AgentScope 已展示 input/output/cache read/cache create，但没有成本估算。

后续可补：

- 模型价格表；
- 自定义价格表；
- 第三方 endpoint 价格配置，如 Kimi、DeepSeek、讯飞星辰等；
- input/output/cache read/cache create 分项成本；
- 按日、项目、会话、模型成本；
- 明确标注 estimated cost。

### 4.5 burn rate 统计稳健性不足

当前 v0.3.6 已修复错误计算，但仍是短窗口展示：

- 最近2秒；
- 1分钟平均；
- 5分钟平均；
- 监控期平均。

后续需要补：

- 最近15分钟平均；
- 最近1小时平均；
- 活跃期平均，排除 Waiting 时间；
- P50 / P90 burn rate；
- 预计耗尽时间；
- 额度窗口内 burn rate；
- 按项目/模型分组 burn rate。

### 4.6 缺少 usage source / confidence 标识

用户需要知道某个指标的可信度。建议增加：

- 数据源：Claude JSONL usage；
- 实时性：延迟更新；
- 精度：最终 usage 较可靠，实时速率为估算；
- 最近一次 JSONL 写入时间；
- 最近一次 usage 更新时间；
- 当前是否处于“等待计数更新”。

### 4.7 缺少与 ccusage 的交叉校验

可增加开发/诊断工具：

```text
agent-scope usage audit
```

用于对比：

- AgentScope 解析结果；
- ccusage JSON 输出；
- session 数量；
- input/output/cache token totals；
- 缺失 session；
- 重复 session；
- model/project 分组差异。

这将提高 AgentScope usage 统计可信度。

### 4.8 缺少轻量外部消费出口

可后续提供：

- local HTTP endpoint；
- JSON export；
- CLI statusline；
- 当前项目 usage summary；
- 当前窗口 burn rate。

这不是 v0.3.7 必须项，但可进入后续路线。

---

## 5. v0.3.7 推荐目标

v0.3.7 不建议一次性追平 ccusage / Claude-Code-Usage-Monitor。建议定位为：

```text
Usage Analytics Foundation
```

即先补基础数据源、可信度和最小历史报表，为 v0.4/v0.5 的 5 小时窗口、成本估算、预测打基础。

### 5.1 v0.3.7 P0 必做

#### P0-1：Usage 数据源兼容增强

目标：补齐 Claude 数据目录发现能力。

验收：

- 支持 `~/.claude`；
- 支持 `~/.config/claude`；
- 支持 `CLAUDE_CONFIG_DIR`；
- 支持多个目录；
- UI 或诊断面板能显示已识别目录；
- 不可读目录不会导致崩溃；
- 有测试覆盖目录解析。

#### P0-2：Usage Source / Confidence 标识

目标：让用户理解速率是估算，不是 streaming 精确值。

验收：

- Agent 监控页面展示 usage source 信息；
- Tooltip 说明“基于 Claude Code 已写入 usage”；
- 显示最近 usage 更新时间；
- active 会话 token 计数未刷新时显示“等待计数更新”；
- 不再让用户误解为 API-level realtime token stream。

#### P0-3：最小 Usage Analytics 页面或面板

目标：提供基本历史用量视图。

最小视图：

- 今日 usage；
- 最近 7 天 usage；
- 按项目聚合；
- 按模型聚合；
- 按会话聚合；
- input/output/cache read/cache create 分项；
- total tokens；
- JSON 导出。

可以先不做成本估算。

#### P0-4：ccusage 对账诊断脚本/命令

目标：验证 AgentScope usage 解析结果与主流工具是否一致。

验收：

- 可以输出 AgentScope 解析 totals；
- 可选读取 ccusage JSON 结果进行 diff；
- 报告缺失 session、重复 session、token totals 差异；
- 不强依赖 ccusage 安装；
- 作为开发诊断工具，不必在 UI 暴露。

### 5.2 v0.3.7 P1 可选

- CSV 导出；
- date range filter；
- Usage Analytics 页面图表；
- 最近15分钟/1小时 burn rate；
- 活跃期平均；
- 按模型 breakdown；
- Usage 诊断页。

### 5.3 暂缓到 v0.4+

- Claude Code 5 小时 rolling window 完整支持；
- 额度耗尽预测；
- P90 limit detection；
- Cost estimation；
- 第三方模型价格配置；
- 本地 API Proxy / streaming usage 捕获；
- statusline / HTTP endpoint。

---

## 6. v0.3.7 建议阶段拆分

### Phase 1：需求澄清与数据模型

输出：

- `docs/specs/v0.3.7-usage-analytics-spec.md`；
- Usage 数据模型；
- 数据源可信度定义；
- UI 信息架构。

不要直接编码。

### Phase 2：Claude 数据目录兼容

输出：

- 数据目录发现逻辑；
- 多目录支持；
- 目录诊断；
- 单元测试。

### Phase 3：Usage 聚合服务

输出：

- Rust 后端 usage aggregation service；
- 按 day/session/project/model 聚合；
- JSON export；
- 测试样本。

### Phase 4：Usage Analytics UI

输出：

- 最小页面/面板；
- today / 7 days / project / model / session；
- source/confidence 标识；
- 不影响 Agent Monitor。

### Phase 5：ccusage 对账工具

输出：

- `scripts/usage-audit` 或 Tauri command；
- 可输出 JSON；
- 可对比外部 ccusage JSON；
- 差异报告。

### Phase 6：验证与文档

输出：

- README 或 docs/user-guide usage analytics 文档；
- 测试报告；
- v0.3.7 release notes draft。

---

## 7. v0.3.7 非目标

v0.3.7 不做：

- API Proxy；
- streaming token 精确计数；
- 完整成本估算系统；
- 复杂 quota prediction；
- P90 自适应限额；
- 大规模 UI 重构；
- 替代 ccusage 的完整 CLI；
- 对 v0.3.6 Agent Monitor 速率算法进行大改。

---

## 8. 关键验收标准

v0.3.7 完成时，至少应满足：

1. AgentScope 能识别主流 Claude Code 数据目录；
2. 用户能看到当前 usage 数据来源与可信度；
3. 用户能查看今日/近7天/项目/模型/会话 usage；
4. usage 统计分 input/output/cache read/cache create；
5. 能导出 JSON；
6. 能用诊断脚本与 ccusage 结果做基本对账；
7. 所有新增聚合逻辑有单元测试；
8. UI 文案明确“估算/延迟/最终 usage”的边界；
9. 不破坏 v0.3.6 的 Agent Monitor、会话管理和 Claude Memory 功能。

---

## 9. 推荐给 Agent 的执行原则

- 先写 spec，不要直接编码；
- 先做数据模型和边界定义；
- 每个阶段独立 commit；
- 每次修改后先 push github，gitlab 等人工确认；
- 不把 v0.3.7 功能混入 v0.3.6 release tag；
- 对 usage 统计保持谦虚口径：最终总量较可靠，实时速率为估算；
- 与 ccusage 对齐优先看 totals 和 grouping，不追求 UI 一致。

---

## 10. 参考项目

- ccusage: https://github.com/ryoppippi/ccusage
- Claude-Code-Usage-Monitor: https://github.com/Maciek-roboblog/Claude-Code-Usage-Monitor

