# Task 1 学习记录：真实 JSONL 格式分析

## 数据来源
- Linux 测试机 (100.85.255.89)
- 文件：`d907e492-446c-4f75-8bbf-d1170e9c572b.jsonl`
- 路径：`~/.claude/projects/-home-yufei-Repo-fpga-project-coarse-cfo/`
- 大小：233 KB (74 行)

## 关键发现

### 用户消息格式
- **ALL** `type="user"` 条目使用 `message.content`（字符串或 blocks 数组）
- **NONE** 使用 top-level `content`
- 但 `queue-operation` 条目使用 top-level `content`，因此 parser 应具备防御性支持
- 当前 parser 只读取 `message.content`，这是 Task 3 的修复点

### Assistant 消息格式
- **ALL** `type="assistant"` 条目使用 `message.content` blocks 数组
- 包含 `type: "text"` 和 `type: "tool_use"` 块
- Model 名位于 `message.model`（如 "kimi-for-coding"）

### Tool Use 模式
- **NO** top-level `tool_use` 或 `tool_result` 条目
- Tool use 嵌入在 assistant 的 `message.content` blocks 中
- Tool result 表现为 `type="user"` 条目，带 `toolUseResult` 和 `sourceToolAssistantUUID` 字段

### 特殊模式
- `$@`：出现在 user message content 字符串（line 4，2 字符）和 assistant blocks 中（共 6 次）
- `[Request interrupted by user]`：出现在 user message blocks[1]（line 9）
- custom-title：**ZERO** 出现在任何 JSONL 文件中

### 其他条目类型
- `permission-mode`：会话权限状态
- `file-history-snapshot`：文件备份跟踪
- `attachment`：Skill/附件数据（含 `attachment.content`）
- `queue-operation`：队列状态（可能含 top-level `content`）
- `system`：错误/重试元数据
- `last-prompt`：会话叶提示跟踪

## 测试覆盖
新增 5 个测试，验证以下真实/合成场景：
1. `test_user_top_level_content_not_parsed_yet` — 记录当前 parser 不支持 top-level content 的局限性
2. `test_user_dollar_at_pattern` — `$@` 模式解析
3. `test_user_interrupted_request` — `[Request interrupted by user]` 解析
4. `test_user_with_tool_use_result` — 带 toolUseResult 的 user 条目解析
5. `test_assistant_multiple_tool_use_blocks` — 多 tool_use 块的 assistant 消息

## 注意事项
- 真实 JSONL 中没有 custom-title，但 parser 已做防御性处理
- 不要假设所有 user 消息都用 `message.content` — top-level `content` 可能在其他 Claude 版本中出现
- 工具名已验证去重排序（Bash, Read）
- 文件路径提取覆盖 `file_path`, `path` 等多个键名

## 测试状态（Task 1）
- `cargo test session_transcript`：19 tests passed
- `cargo test encode_cwd_path`：4 tests passed

---

# Task 3 学习记录：Parser 清理与噪声过滤

## 变更摘要

### Parser 改动
1. `extract_text_from_content` 不变，继续支持 string 和 blocks 数组两种格式。
2. 新增 `is_noisy_text()` 辅助函数，统一过滤不应成为 `initial_prompt` 或 `custom_title` 的占位/噪声内容。
3. `process_jsonl_entry` 中 user 条目现在优先读取 top-level `content`，再回退到 `message.content`。
4. `process_jsonl_entry` 中 custom-title 条目现在也经过 `is_noisy_text()` 过滤。

### 噪声过滤规则（按添加顺序）
- 空字符串或仅空白字符
- 精确等于 `$@`
- 以 `[SYSTEM DIRECTIVE:` 开头
- 以 `[Request interrupted by user]` 开头
- 包含 `<!-- OMO_INTERNAL_INITIATOR -->`
- 包含 "You are a conversation title generator"

### 关键设计决策
- **turn 文本保留**：噪声内容仍然注册为 `SessionTurn`，只是不用于 `initial_prompt`/`custom_title`。这让 UI 层可以完整渲染对话流（包括 `$@` 占位符和 `[Request interrupted by user]` 状态提示）。
- **不回退到助手内容**：如果所有 user 条目都是噪声，`initial_prompt` 保持为空，不会错误地回退到 assistant 文本。
- **不删除助手内容**：assistant 消息即使包含状态标记 + 有意义文本，也不会被删除。

### 测试变更
- `test_user_top_level_content_not_parsed_yet` → `test_top_level_user_content`，断言 top-level `content` 现在能正确解析出 `分析当前项目`。
- 新增 `test_transcript_noise_filters`：验证 `$@`、 `[SYSTEM DIRECTIVE:`、`<!-- OMO_INTERNAL_INITIATOR -->`、空白、title-generator 全部被过滤，但真实用户请求能成为 `initial_prompt`；同时验证噪声文本仍保留在 turn 中。
- 新增 `test_custom_title_noise_filtered`：验证 custom-title 条目中的噪声被跳过，正常标题被接受。
- 更新 `test_user_dollar_at_pattern` 和 `test_user_interrupted_request`：新增 `initial_prompt.is_empty()` 断言，确保这些模式不会成为 prompt。

### 测试状态
- `cargo test session_transcript`：21 tests passed（原 19 + 2 新增）
- `cargo test encode_cwd_path`：4 tests passed
- 零失败、零跳过、零忽略
## 2026-05-07
- transcript 清理应在 Rust 解析层完成：先移除 local-command-caveat / command-name / system-reminder / work_context 块，再剥离 XML-like 标签，保留 command-message 中的真实用户输入。
- L2 对话详情更适合按 user→assistant 合并为 Q&A 卡片；标记动作使用用户 turn 的原始 index，候选记忆内容组合用户问题与助手回答。
- 工具调用不再展示 details，最多显示 muted chip，避免工具参数噪声干扰阅读。
- L2 对话搜索的视觉分区应优先复用 shadcn/Tailwind token：外层 `border border-border bg-card/70 shadow-sm`、左右面板用 `lg:border-r` 分隔，Q&A 卡片用 `ring-border/40`、`bg-muted/25` 标题条和角色块左侧 token 色强调，避免硬编码视觉值。

## 2026-05-08
- 滚动中的分界线不要依赖 `shadow-sm` / `ring` 这类合成层效果；L2 面板与 Q&A 卡片应以 `border`、`divide-*` 和 `bg-*` token 的背景对比作为主视觉锚点。
- L2 左右分栏滚动时保持可辨识：外层使用 `overflow-hidden rounded-xl border border-border bg-card`，左侧 `bg-muted/40`，右侧 `bg-background`，面板标题使用 `sticky top-0 z-10 bg-inherit border-b border-border`。
- Q&A 卡片在长列表滚动时更适合 `border-2 border-border/60` + `bg-muted/40` 标题条，角色内容块继续使用 `border border-border/80`，避免微弱阴影在滚动合成期间消失。
