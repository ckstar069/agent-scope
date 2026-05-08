# ptv — 对话搜索文本阅读器与 Claude Code JSONL 解析修复

## TL;DR

> **快速摘要**：将“项目记忆 → 对话搜索”从气泡聊天布局改为格式化文本转录阅读器，并修复 Claude Code JSONL 解析对真实 user content、标题、系统噪声、工具调用的处理问题。
>
> **交付物**：
> - 修复 `session_transcript.rs` 对 Claude Code JSONL 的 user content 解析与噪声过滤。
> - 重构 `TranscriptDetailView` 为文档式 transcript reader，不再嵌入 `MarkdownRenderer` 气泡。
> - 优化 `SessionSearchView` 的标题、摘要、选中态和元信息层级。
> - 保留候选记忆标记能力，并修复标记状态显示。
> - 在 Linux 测试机真实 Claude Code 会话上验证。
>
> **预估工作量**：中等
> **并行执行**：YES — 3 个 Wave
> **关键路径**：Task 1 → Task 3/4 → Task 6 → Final QA

---

## Context

### Original Request
用户指出“项目记忆 → 对话搜索”Tab 展示存在很大问题：会话列表和详情可见，但右侧大面积空白、消息窄条、标题显示 `$@`、内容片段如 `[Request interrupted by user]` 直接作为主要内容展示。用户明确表示：**不需要气泡对话形式，应直接把读取到的会话美化/格式化为文本形式展示**。

### Interview Summary
**已确认决策**：
- 对话详情改成“格式化文本阅读器 / 会话转录文档”，不做聊天气泡 UI。
- 工具调用默认折叠，只显示 `Bash / Read / Write` 等标签，点击展开。
- `[Request interrupted by user]` 保留为灰色状态提示，不作为正文主内容。
- `thinking` / `redacted_thinking` 默认隐藏，后续如需要再做调试模式。
- 不做 AI 自动总结或自动提取记忆。

**Research Findings**：
- `ProjectMemoryPanel.tsx` L2 使用 `lg:grid-cols-[1fr_1fr]`，详情区过窄。
- `TranscriptDetailView.tsx` 使用气泡式 `TurnBubble` + `max-w-[85%]`，进一步压缩正文。
- `TranscriptDetailView.tsx` 内嵌 `MarkdownRenderer`，而 `MarkdownRenderer` 是带 16rem TOC 的完整文档阅读器，导致巨大空白和正文被挤窄。
- `SessionSearchView.tsx` 缺少选中态，标题/摘要/元信息视觉层级弱。
- `ProjectMemoryPanel.tsx` 传入 `markedTurns={new Set()}`，标记状态无法保留。
- `session_transcript.rs` 当前 user 消息解析读取 `message.content`，但真实 Claude Code JSONL 常用顶层 `content`。
- 缺少 `$@`、`[SYSTEM DIRECTIVE: ...]`、`<!-- OMO_INTERNAL_INITIATOR -->`、标题生成器提示、tool_result wrapper 等过滤策略。

### Metis Review
**已纳入的缺口**：
- 第一任务必须验证测试机真实 JSONL 格式，避免基于推断改 parser。
- 需要明确不改 L1 静态记忆、L3 候选记忆核心持久化、实时更新等非目标范围。
- 接受标准必须覆盖：不显示 `$@`、不使用 `MarkdownRenderer` 渲染 turn、工具折叠、系统中断状态提示、标记状态保留、真实 JSONL 验证。
- parser 改动必须保持 string content 与 blocks content 双格式兼容。
- 转录视图需要处理长代码块横向滚动、空会话、空标题回退、正在写入/不完整 JSONL。

---

## Work Objectives

### Core Objective
让“对话搜索”成为可读、稳定、可信的 Claude Code 会话转录阅读器：能找到真实会话、正确提取用户/助手文本、清理系统噪声，并以文本/Markdown 文档形式展示。

### Concrete Deliverables
- `src-tauri/src/collectors/template/session_transcript.rs`：真实 JSONL 格式支持、标题/用户内容提取、系统噪声过滤、相关单元测试。
- `src/components/TranscriptDetailView.tsx`：从气泡视图改为文本转录阅读器。
- `src/components/SessionSearchView.tsx`：增强会话列表标题、摘要、选中态。
- `src/components/ProjectMemoryPanel.tsx`：L2 布局和 marked turn 状态管理。
- 可选新增轻量组件：`src/components/TranscriptMarkdown.tsx` 或等价文本渲染子组件。

### Definition of Done
- [ ] Linux 测试机 `/home/yufei/Repo/fpga_project_coarse_cfo` 能显示真实 Claude Code 会话。
- [ ] 会话标题不再显示 `$@`，空标题有合理 fallback。
- [ ] 右侧对话详情不再是气泡布局，不再出现 16rem TOC 空白。
- [ ] 工具调用默认折叠，展开后可读。
- [ ] `[Request interrupted by user]` 仅作为状态提示。
- [ ] `cargo test encode_cwd_path session_transcript` 通过；前端 `npm run build` 通过。

### Must Have
- 文本式 transcript reader，不是聊天气泡。
- Parser 同时支持顶层 `content` 和 `message.content`。
- 候选记忆标记能力保留。
- 真实测试机数据验证。

### Must NOT Have (Guardrails)
- 不改 L1 静态记忆的 `MarkdownRenderer` 用途。
- 不做 AI 自动总结/提取。
- 不实现实时 transcript streaming。
- 不改 L3 候选记忆写入格式，除非仅为兼容标记入口。
- 不引入新的大型 UI/Markdown 库。

---

## Verification Strategy

> **ZERO HUMAN INTERVENTION** - ALL verification is agent-executed. No acceptance criterion may require manual confirmation.

### Test Decision
- **Infrastructure exists**: YES
- **Automated tests**: Tests-after
- **Framework**: Rust `cargo test`; frontend `npm run build`; E2E/Agent QA on Linux AppImage where applicable.
- **Agent-Executed QA**: ALWAYS

### QA Policy
每个任务必须包含 agent-executed QA。证据保存到 `.sisyphus/evidence/task-{N}-{scenario}.{ext}`。

---

## Execution Strategy

### Parallel Execution Waves

```
Wave 1 (Research validation + parser/test foundation):
├── Task 1: Capture and codify real Claude JSONL fixtures [deep]
├── Task 2: Define transcript text rendering contract [visual-engineering]
├── Task 3: Parser cleanup and Rust tests [deep]
└── Task 4: Session list metadata and active state design [visual-engineering]

Wave 2 (UI refactor, after Wave 1 contracts):
├── Task 5: Replace bubble transcript view with text reader (depends: 2,3) [visual-engineering]
├── Task 6: L2 layout and selected/marked state wiring (depends: 2,4) [visual-engineering]
├── Task 7: Tool call folding and status hint rendering (depends: 2,3) [visual-engineering]
└── Task 8: Frontend integration build fixes (depends: 5,6,7) [quick]

Wave 3 (Linux packaging + real-data QA):
├── Task 9: Linux test-machine real session verification [unspecified-high]
├── Task 10: Regression tests for static memory and candidate memory [unspecified-high]
└── Task 11: Release AppImage rebuild and artifact verification [quick]

Wave FINAL:
├── F1: Plan Compliance Audit (oracle)
├── F2: Code Quality Review (unspecified-high)
├── F3: Real Manual QA by agent on Linux (unspecified-high)
└── F4: Scope Fidelity Check (deep)
```

### Dependency Matrix

| Task | Depends On | Blocks | Wave |
|------|------------|--------|------|
| 1 | None | 3, 5, 7, 9 | 1 |
| 2 | None | 5, 6, 7 | 1 |
| 3 | 1 | 5, 7, 9 | 1 |
| 4 | None | 6 | 1 |
| 5 | 2, 3 | 8, 9 | 2 |
| 6 | 2, 4 | 8, 9 | 2 |
| 7 | 2, 3 | 8, 9 | 2 |
| 8 | 5, 6, 7 | 9, 11 | 2 |
| 9 | 1, 3, 5, 6, 7, 8 | FINAL | 3 |
| 10 | 8 | FINAL | 3 |
| 11 | 8, 9 | FINAL | 3 |

### Agent Dispatch Summary
- Wave 1: 4 agents — T1/T3 deep parser; T2/T4 visual-engineering UI contract.
- Wave 2: 4 agents — text reader, layout wiring, tool folding, integration build.
- Wave 3: 3 agents — Linux QA, regression QA, release packaging.
- FINAL: 4 review agents.

---

## TODOs

- [x] 1. **Capture and codify real Claude JSONL fixtures**

  **What to do**:
  - SSH to `100.85.255.89` and inspect `/home/yufei/.claude/projects/-home-yufei-Repo-fpga-project-coarse-cfo/*.jsonl`.
  - Capture representative JSONL snippets for user top-level `content`, assistant `message.content`, tool_use/tool_result, interrupted entries, custom-title entries.
  - Add fixture-oriented tests or inline test cases in `session_transcript.rs` covering the discovered real format.

  **Must NOT do**:
  - Do not hardcode the specific user’s absolute path except in QA commands.
  - Do not copy sensitive full transcript content into docs beyond minimal sanitized fixtures.

  **Recommended Agent Profile**:
  - **Category**: `deep` — requires careful data-format validation before parser changes.
  - **Skills**: []
  - **Skills Evaluated but Omitted**: `playwright` not needed; no browser interaction.

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1
  - **Blocks**: 3, 5, 7, 9
  - **Blocked By**: None

  **References**:
  - `src-tauri/src/collectors/template/session_transcript.rs:26-29` — path encoding already fixed; verify unchanged.
  - `src-tauri/src/collectors/template/session_transcript.rs:427-441` — current user parsing logic to validate against real JSONL.
  - `/home/yufei/.claude/projects/-home-yufei-Repo-fpga-project-coarse-cfo/*.jsonl` — real test data source.

  **Acceptance Criteria**:
  - [ ] A sanitized fixture/test covers top-level `content` user messages.
  - [ ] A fixture/test covers assistant `message.content` block messages.
  - [ ] A fixture/test covers custom-title fallback and `$@` filtering.

  **QA Scenarios**:
  ```
  Scenario: Real JSONL directory exists and is sampled safely
    Tool: Bash
    Preconditions: SSH access to 100.85.255.89 as yufei
    Steps:
      1. Run `ls /home/yufei/.claude/projects/-home-yufei-Repo-fpga-project-coarse-cfo/*.jsonl`.
      2. Run a JSONL sampler that prints only `type`, top-level keys, and content field location, not full transcript text.
      3. Save output to `.sisyphus/evidence/task-1-real-jsonl-shape.txt`.
    Expected Result: At least one `.jsonl` file is found and sampled shapes show where user content resides.
    Evidence: .sisyphus/evidence/task-1-real-jsonl-shape.txt

  Scenario: No sensitive transcript dump in fixtures
    Tool: Bash
    Preconditions: Fixture/test file exists
    Steps:
      1. Search fixture for long contiguous user text over 500 chars.
      2. Verify fixtures are minimal/sanitized.
    Expected Result: Fixtures contain minimal synthetic or redacted examples.
    Evidence: .sisyphus/evidence/task-1-fixture-sanitization.txt
  ```

  **Evidence to Capture**:
  - [ ] JSONL shape sample
  - [ ] Fixture sanitization proof

  **Commit**: NO (group with parser changes)

- [x] 2. **Define transcript text rendering contract**

  **What to do**:
  - Define the UI contract for document-style transcript rendering: session summary header, user/assistant sections, status hint blocks, tool call folding blocks, mark action placement.
  - Decide whether to create `TranscriptMarkdown.tsx` / `TranscriptTextRenderer.tsx` or inline renderers.
  - Keep `MarkdownRenderer` reserved for L1 only.

  **Must NOT do**:
  - Do not keep chat bubble UI.
  - Do not reuse `MarkdownRenderer` inside transcript turns.

  **Recommended Agent Profile**:
  - **Category**: `visual-engineering` — UI structure and readability.
  - **Skills**: []
  - **Skills Evaluated but Omitted**: `frontend-ui-ux` not loaded unless available in executor context.

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1
  - **Blocks**: 5, 6, 7
  - **Blocked By**: None

  **References**:
  - `src/components/TranscriptDetailView.tsx` — current bubble implementation to replace.
  - `src/components/MarkdownRenderer.tsx:37-80` — full document renderer to avoid embedding.
  - `src/components/ProjectMemoryPanel.tsx:320-350` — current L2 panel layout.

  **Acceptance Criteria**:
  - [ ] UI contract specifies no bubble layout and no TOC inside transcript turns.
  - [ ] Contract includes tool default-collapsed behavior and status hint rendering.

  **QA Scenarios**:
  ```
  Scenario: Contract rejects MarkdownRenderer-in-turn usage
    Tool: Bash
    Preconditions: Contract or implementation draft exists
    Steps:
      1. Search for `MarkdownRenderer` usage in transcript turn renderer.
      2. Confirm use is absent or limited to L1 static memory.
    Expected Result: No `MarkdownRenderer` import/use in transcript turn rendering.
    Evidence: .sisyphus/evidence/task-2-no-markdownrenderer-in-turns.txt

  Scenario: Text reader layout contract includes tools and status hints
    Tool: Bash
    Preconditions: Contract or implementation notes exist
    Steps:
      1. Verify terms `tools collapsed`, `status hint`, `section heading` appear.
      2. Save relevant excerpt.
    Expected Result: Contract captures user-approved display strategy.
    Evidence: .sisyphus/evidence/task-2-contract-excerpt.txt
  ```

  **Evidence to Capture**:
  - [ ] Renderer contract excerpt
  - [ ] Search proof excluding MarkdownRenderer in turns

  **Commit**: NO (group with UI implementation)

- [x] 3. **Parser cleanup and Rust tests**

  **What to do**:
  - Update `session_transcript.rs` to extract user text from top-level `content` first, falling back to `message.content`.
  - Add filtering for `$@`, empty strings, `[SYSTEM DIRECTIVE: ...]`, `<!-- OMO_INTERNAL_INITIATOR -->`, title generator prompts.
  - Render `[Request interrupted by user]` as a status-like turn marker or tagged text, not primary title/prompt.
  - Add/adjust Rust tests for real/synthetic Claude JSONL shapes.
  - Consider matching parser behavior in `abtop-collector/src/collector/claude.rs` only if affected code path feeds this UI; otherwise note as follow-up.

  **Must NOT do**:
  - Do not delete entire assistant content when it contains one status string plus meaningful text.
  - Do not break string content and block content dual support.

  **Recommended Agent Profile**:
  - **Category**: `deep` — parser correctness and regression tests.
  - **Skills**: []
  - **Skills Evaluated but Omitted**: `playwright` not relevant for Rust parser unit tests.

  **Parallelization**:
  - **Can Run In Parallel**: YES after Task 1 fixture findings are available.
  - **Parallel Group**: Wave 1
  - **Blocks**: 5, 7, 9
  - **Blocked By**: 1

  **References**:
  - `src-tauri/src/collectors/template/session_transcript.rs:237-254` — text extraction from content.
  - `src-tauri/src/collectors/template/session_transcript.rs:427-441` — user parsing path.
  - `abtop-collector/src/collector/claude.rs` — existing prompt/tool filtering patterns to compare.

  **Acceptance Criteria**:
  - [ ] Top-level user `content` parses into `SessionTurn` text.
  - [ ] Nested `message.content` user format still parses.
  - [ ] `$@` never becomes `initial_prompt` or session title.
  - [ ] System directive/internal initiator text is filtered.
  - [ ] `cargo test encode_cwd_path session_transcript` passes with zero skipped/ignored failures; if `test_list_sessions_basic` is flaky due to mtime resolution, stabilize ordering instead of skipping.

  **QA Scenarios**:
  ```
  Scenario: Top-level user content parses correctly
    Tool: Bash
    Preconditions: Rust test added for top-level user content
    Steps:
      1. Run `cd src-tauri && cargo test top_level_user_content -- --nocapture`.
      2. Confirm parsed turn text equals `分析当前项目`.
    Expected Result: Test passes and evidence shows expected text.
    Evidence: .sisyphus/evidence/task-3-top-level-user-content.txt

  Scenario: Noise filters protect title extraction
    Tool: Bash
    Preconditions: Rust tests added for `$@` and system directive filters
    Steps:
      1. Run `cd src-tauri && cargo test transcript_noise_filters -- --nocapture`.
      2. Confirm `$@` and `[SYSTEM DIRECTIVE:` are skipped as titles.
      3. Run `cargo test session_transcript` and confirm all relevant tests pass without skipping `test_list_sessions_basic`.
    Expected Result: Tests pass; fallback title logic is exercised; mtime-sensitive ordering is stabilized if needed.
    Evidence: .sisyphus/evidence/task-3-noise-filter-tests.txt
  ```

  **Evidence to Capture**:
  - [ ] Rust parser test output
  - [ ] Real-data sampled parse output

  **Commit**: YES (with Task 1)
  - Message: `fix(memory): 修复 Claude 会话解析`
  - Files: `session_transcript.rs` and parser tests
  - Pre-commit: `cargo test encode_cwd_path session_transcript`

- [x] 4. **Session list metadata and active state design**

  **What to do**:
  - Update `SessionSearchView` contract/implementation so current session is visibly selected.
  - Improve title fallback order: valid custom_title → valid initial_prompt → `无标题会话 {session_id_prefix}`.
  - Demote model/session id/time/file chips to metadata level.

  **Must NOT do**:
  - Do not add date/model/file advanced filters.
  - Do not make session list consume full width.

  **Recommended Agent Profile**:
  - **Category**: `visual-engineering`
  - **Skills**: []
  - **Skills Evaluated but Omitted**: None.

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1
  - **Blocks**: 6
  - **Blocked By**: None

  **References**:
  - `src/components/SessionSearchView.tsx` — session list UI and search logic.
  - `src/components/ProjectMemoryPanel.tsx` — selected session state ownership.

  **Acceptance Criteria**:
  - [ ] Active session list item has clear selected background/border.
  - [ ] `$@` or empty prompt does not appear as main title.
  - [ ] Model/session/time remain visible but visually secondary.

  **QA Scenarios**:
  ```
  Scenario: Active session visible in list
    Tool: Playwright
    Preconditions: App has one or more sessions loaded
    Steps:
      1. Open Project Memory → 对话搜索.
      2. Click session id `d907e492`.
      3. Assert clicked list item has selected styling class or `aria-selected=true`.
    Expected Result: Selected session is visually distinct and accessible.
    Evidence: .sisyphus/evidence/task-4-selected-session.png

  Scenario: Bad title fallback
    Tool: Bash
    Preconditions: Fixture or mock data includes title `$@`
    Steps:
      1. Run frontend/unit or component inspection command for title helper.
      2. Confirm displayed title is not `$@`.
    Expected Result: Title falls back to prompt or `无标题会话 d907e492`.
    Evidence: .sisyphus/evidence/task-4-title-fallback.txt
  ```

  **Evidence to Capture**:
  - [ ] Selected session screenshot
  - [ ] Title fallback output

  **Commit**: NO (group with UI tasks)

- [x] 5. **Replace bubble transcript view with text reader**

  **What to do**:
  - Refactor `TranscriptDetailView` away from `TurnBubble` chat bubbles.
  - Render transcript as a scrollable document with session summary header and sequential sections.
  - Use prose/text classes directly; do not import/use `MarkdownRenderer` in transcript turns.
  - Support readable code blocks and long lines.

  **Must NOT do**:
  - Do not keep `max-w-[85%]` message bubbles.
  - Do not render a TOC inside transcript details.

  **Recommended Agent Profile**:
  - **Category**: `visual-engineering`
  - **Skills**: []
  - **Skills Evaluated but Omitted**: None.

  **Parallelization**:
  - **Can Run In Parallel**: YES after Task 2/3
  - **Parallel Group**: Wave 2
  - **Blocks**: 8, 9
  - **Blocked By**: 2, 3

  **References**:
  - `src/components/TranscriptDetailView.tsx` — primary refactor target.
  - `src/components/MarkdownRenderer.tsx` — pattern to avoid for turns; can borrow typography ideas only.

  **Acceptance Criteria**:
  - [ ] Transcript details render as document sections with full available width.
  - [ ] No `MarkdownRenderer` import/use remains in `TranscriptDetailView`.
  - [ ] No chat bubble `justify-end`/`max-w-[85%]` layout remains for transcript content.

  **QA Scenarios**:
  ```
  Scenario: Transcript renders as document text
    Tool: Playwright
    Preconditions: App loaded with fpga_project_coarse_cfo session
    Steps:
      1. Open 对话搜索.
      2. Select session `d907e492`.
      3. Assert right pane contains headings `User` and `Assistant` in document flow.
      4. Assert no TOC placeholder text `当前文档没有可导航标题` appears inside transcript.
    Expected Result: Text transcript is readable and no large blank TOC area appears.
    Evidence: .sisyphus/evidence/task-5-text-transcript.png

  Scenario: Long code/text wraps or scrolls locally
    Tool: Playwright
    Preconditions: Transcript includes long command/output line
    Steps:
      1. Open the transcript.
      2. Locate a code/pre block.
      3. Assert the page body has no global horizontal scrollbar while block itself can scroll if needed.
    Expected Result: Long text does not break layout.
    Evidence: .sisyphus/evidence/task-5-long-line-layout.png
  ```

  **Evidence to Capture**:
  - [ ] Transcript screenshot
  - [ ] Layout overflow evidence

  **Commit**: YES (with Tasks 6-8)
  - Message: `refactor(memory): 改为文本式会话阅读器`
  - Files: Transcript UI files
  - Pre-commit: `npm run build`

- [x] 6. **L2 layout and selected/marked state wiring**

  **What to do**:
  - Adjust `ProjectMemoryPanel` L2 layout to left fixed/narrow and right flexible, e.g. `lg:grid-cols-[22rem_minmax(0,1fr)]` if implementation confirms needed.
  - Pass `selectedSessionId` into `SessionSearchView` for active item styling.
  - Replace `markedTurns={new Set()}` with real state keyed by `${sessionId}:${turnIndex}`.
  - Prevent duplicate candidate entries for same turn.

  **Must NOT do**:
  - Do not change candidate memory persistence format.
  - Do not remove mark action; adapt it to document section style.

  **Recommended Agent Profile**:
  - **Category**: `visual-engineering`
  - **Skills**: []
  - **Skills Evaluated but Omitted**: None.

  **Parallelization**:
  - **Can Run In Parallel**: YES after Task 2/4
  - **Parallel Group**: Wave 2
  - **Blocks**: 8, 9
  - **Blocked By**: 2, 4

  **References**:
  - `src/components/ProjectMemoryPanel.tsx:134-151` — current candidate creation.
  - `src/components/ProjectMemoryPanel.tsx:193-200` — `markedTurns={new Set()}` bug.
  - `src/components/SessionSearchView.tsx` — selected state target.

  **Acceptance Criteria**:
  - [ ] Marking a turn changes visible state to 已标记.
  - [ ] Marking the same turn twice does not duplicate candidate memory.
  - [ ] L2 right pane receives most horizontal space.

  **QA Scenarios**:
  ```
  Scenario: Marked state persists in current session
    Tool: Playwright
    Preconditions: A transcript is selected
    Steps:
      1. Click `标记` on first user section.
      2. Assert button text or state changes to `已标记`.
      3. Switch to 候选记忆 tab and assert one pending candidate exists.
      4. Return to 对话搜索 and assert same turn remains marked.
    Expected Result: Marked turn state persists within the ProjectMemoryPanel session.
    Evidence: .sisyphus/evidence/task-6-marked-state.png

  Scenario: Duplicate mark is prevented
    Tool: Playwright
    Preconditions: A transcript is selected
    Steps:
      1. Click mark on the same turn twice.
      2. Switch to 候选记忆.
      3. Count candidates from same session/turn.
    Expected Result: Exactly one candidate exists for that session/turn.
    Evidence: .sisyphus/evidence/task-6-no-duplicate-candidate.txt
  ```

  **Evidence to Capture**:
  - [ ] Marked state screenshot
  - [ ] Duplicate prevention output

  **Commit**: YES (with Task 5)

- [x] 7. **Tool call folding and status hint rendering**

  **What to do**:
  - Represent tools from `turn.tools` as collapsed sections or chips with expandable details where data is available.
  - Render `[Request interrupted by user]` as muted status hint.
  - Hide `thinking` / `redacted_thinking` by default.
  - If raw tool details are unavailable in current `SessionTurn`, avoid inventing content; show tool names only and plan parser extension separately if needed.

  **Must NOT do**:
  - Do not fabricate tool output when parser does not expose it.
  - Do not let tool/result content dominate transcript body by default.

  **Recommended Agent Profile**:
  - **Category**: `visual-engineering`
  - **Skills**: []
  - **Skills Evaluated but Omitted**: None.

  **Parallelization**:
  - **Can Run In Parallel**: YES after Task 2/3
  - **Parallel Group**: Wave 2
  - **Blocks**: 8, 9
  - **Blocked By**: 2, 3

  **References**:
  - `src-tauri/src/collectors/template/session_transcript.rs` — source of `turn.tools`.
  - `src/components/TranscriptDetailView.tsx` — current tool chip display.

  **Acceptance Criteria**:
  - [ ] Tool names appear as subdued collapsible/chip elements, not as primary body text.
  - [ ] Interrupted request text appears muted and labelled as status.
  - [ ] thinking content is not visible in default transcript.

  **QA Scenarios**:
  ```
  Scenario: Tool calls are collapsed by default
    Tool: Playwright
    Preconditions: Transcript contains Bash and Read tool metadata
    Steps:
      1. Open transcript.
      2. Locate tool labels `Bash` and `Read`.
      3. Assert detailed tool content is hidden until expanded.
    Expected Result: Tool labels are visible but detailed content does not dominate view.
    Evidence: .sisyphus/evidence/task-7-tools-collapsed.png

  Scenario: Interrupted request is status hint
    Tool: Playwright
    Preconditions: Transcript contains `[Request interrupted by user]`
    Steps:
      1. Open transcript.
      2. Locate interrupted marker.
      3. Assert it is rendered with muted/status styling and not as session title.
    Expected Result: Interruption appears as gray status hint.
    Evidence: .sisyphus/evidence/task-7-interrupted-status.png
  ```

  **Evidence to Capture**:
  - [ ] Tool folding screenshot
  - [ ] Status hint screenshot

  **Commit**: YES (with Task 5)

- [x] 8. **Frontend integration build fixes**

  **What to do**:
  - Resolve TypeScript type updates from changed props and transcript data shape.
  - Ensure imports are clean and no unused `MarkdownRenderer` remains in L2 components.
  - Run frontend build.

  **Must NOT do**:
  - Do not introduce `any` or suppressions like `@ts-ignore`.

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []
  - **Skills Evaluated but Omitted**: None.

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 2 integration
  - **Blocks**: 9, 10, 11
  - **Blocked By**: 5, 6, 7

  **References**:
  - `src/components/ProjectMemoryPanel.tsx`
  - `src/components/SessionSearchView.tsx`
  - `src/components/TranscriptDetailView.tsx`

  **Acceptance Criteria**:
  - [ ] `npm run build` passes.
  - [ ] No TypeScript suppressions added.
  - [ ] LSP/tsc errors are zero.

  **QA Scenarios**:
  ```
  Scenario: Frontend production build passes
    Tool: Bash
    Preconditions: UI refactor complete
    Steps:
      1. Run `npm run build`.
      2. Save command output.
    Expected Result: Build exits 0 with Vite bundle output.
    Evidence: .sisyphus/evidence/task-8-npm-build.txt

  Scenario: No TypeScript suppression introduced
    Tool: Bash
    Preconditions: UI refactor complete
    Steps:
      1. Search changed frontend files for `@ts-ignore`, `@ts-expect-error`, `as any`.
      2. Save result.
    Expected Result: No new suppressions found.
    Evidence: .sisyphus/evidence/task-8-no-ts-suppressions.txt
  ```

  **Evidence to Capture**:
  - [ ] Build output
  - [ ] Suppression search result

  **Commit**: YES (with UI refactor)

- [x] 9. **Linux test-machine real session verification**

  **What to do**:
  - Sync/build on `100.85.255.89`.
  - Use project `/home/yufei/Repo/fpga_project_coarse_cfo`.
  - Verify app shows the real Claude session from `/home/yufei/.claude/projects/-home-yufei-Repo-fpga-project-coarse-cfo/*.jsonl`.
  - Confirm title, content, status hints, tools, and marked state.

  **Must NOT do**:
  - Do not rely only on synthetic fixtures.
  - Do not require the human user to verify manually.

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: [`playwright`] if available in executor context.
  - **Skills Evaluated but Omitted**: None.

  **Parallelization**:
  - **Can Run In Parallel**: YES with Task 10 after Task 8
  - **Parallel Group**: Wave 3
  - **Blocks**: FINAL, 11
  - **Blocked By**: 1, 3, 5, 6, 7, 8

  **References**:
  - `AGENTS.md` — Linux test machine details.
  - `/home/yufei/Repo/ai_project_template_visualization` — test deployment path.
  - `/home/yufei/Repo/fpga_project_coarse_cfo` — monitored project.

  **Acceptance Criteria**:
  - [ ] Real session appears in UI.
  - [ ] Session title is meaningful and not `$@`.
  - [ ] Transcript is text/document format.
  - [ ] No large blank TOC area appears.

  **QA Scenarios**:
  ```
  Scenario: Real Linux session is visible in AppImage
    Tool: Playwright or equivalent GUI automation
    Preconditions: Release app is built on 100.85.255.89
    Steps:
      1. Launch ptv AppImage on Linux test environment.
      2. Open `/home/yufei/Repo/fpga_project_coarse_cfo` details.
      3. Click 项目记忆 → 对话搜索.
      4. Assert session `d907e492` or available session is listed.
      5. Assert transcript panel contains formatted document sections.
    Expected Result: Real session is loaded and readable.
    Evidence: .sisyphus/evidence/task-9-real-session-visible.png

  Scenario: Title and noise cleanup on real data
    Tool: Playwright or Bash+OCR/log extraction if GUI automation limited
    Preconditions: Real session loaded
    Steps:
      1. Inspect visible title and first transcript sections.
      2. Assert `$@` is absent as title.
      3. Assert `[Request interrupted by user]` is styled/labelled as status if present.
    Expected Result: No noisy placeholders as primary content.
    Evidence: .sisyphus/evidence/task-9-real-title-noise-cleanup.png
  ```

  **Evidence to Capture**:
  - [ ] Real session UI screenshot
  - [ ] Title/noise cleanup screenshot

  **Commit**: NO

- [x] 10. **Regression tests for static memory and candidate memory**

  **What to do**:
  - Verify L1 static memory still loads CLAUDE.md and rules.
  - Verify L3 candidate memory still receives marked entries and save flow still calls `save_candidate_memory` correctly.
  - Verify camelCase Tauri args fixes (`relativePath`, `sessionId`) remain intact.

  **Must NOT do**:
  - Do not change candidate memory file format.

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: [`playwright`] if available.
  - **Skills Evaluated but Omitted**: None.

  **Parallelization**:
  - **Can Run In Parallel**: YES with Task 9
  - **Parallel Group**: Wave 3
  - **Blocks**: FINAL
  - **Blocked By**: 8

  **References**:
  - `src/components/ProjectMemoryPanel.tsx` — L1/L2/L3 integration.
  - `src/components/CandidateMemoryBox.tsx` — candidate memory save flow.
  - `src-tauri/src/commands.rs:680-769` — `save_candidate_memory`.

  **Acceptance Criteria**:
  - [ ] L1 static memory content loads without `relativePath` error.
  - [ ] Marking transcript section creates one L3 pending candidate.
  - [ ] Saving candidate writes/updates `.sisyphus/notepads/project-memory/decisions.md`.

  **QA Scenarios**:
  ```
  Scenario: L1 static memory unaffected
    Tool: Playwright
    Preconditions: App running with fpga_project_coarse_cfo
    Steps:
      1. Open 项目记忆 → 静态记忆.
      2. Click `CLAUDE.md`.
      3. Assert content loads and no `missing required key relativePath` appears.
    Expected Result: Static memory works as before.
    Evidence: .sisyphus/evidence/task-10-l1-static-memory.png

  Scenario: Candidate memory save still works
    Tool: Playwright + Bash
    Preconditions: Transcript section can be marked
    Steps:
      1. Mark one transcript section.
      2. Open 候选记忆.
      3. Confirm/save candidate.
      4. Check `.sisyphus/notepads/project-memory/decisions.md` contains the saved category and source.
    Expected Result: Candidate memory persists.
    Evidence: .sisyphus/evidence/task-10-candidate-save.txt
  ```

  **Evidence to Capture**:
  - [ ] Static memory screenshot
  - [ ] Candidate memory saved file proof

  **Commit**: NO

- [x] 11. **Release AppImage rebuild and artifact verification**

  **What to do**:
  - Build release bundles on Linux test server after QA passes.
  - Verify AppImage timestamp and path.
  - Ensure no debug logs or temporary patches remain.

  **Must NOT do**:
  - Do not leave source files modified only on server; sync back if applicable.
  - Do not ship AppImage from stale build.

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []
  - **Skills Evaluated but Omitted**: None.

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 3 final packaging
  - **Blocks**: FINAL
  - **Blocked By**: 8, 9

  **References**:
  - `package.json` — `npm run build` / `npm run tauri build` scripts.
  - `src-tauri/tauri.conf.json` — bundle target config.
  - `/home/yufei/Repo/ai_project_template_visualization/src-tauri/target/release/bundle/appimage/ptv_0.1.0_amd64.AppImage` — expected artifact.

  **Acceptance Criteria**:
  - [ ] AppImage exists and has fresh timestamp.
  - [ ] `.deb` and `.rpm` bundles are also regenerated.
  - [ ] Source on server and local are synchronized for changed files.

  **QA Scenarios**:
  ```
  Scenario: Release bundle regenerated
    Tool: Bash
    Preconditions: All tests passed
    Steps:
      1. Run `npm run build && npx tauri build` on server.
      2. Run `ls -lh src-tauri/target/release/bundle/appimage/ptv_0.1.0_amd64.AppImage`.
      3. Save timestamp and size.
    Expected Result: Fresh AppImage exists and build exits 0.
    Evidence: .sisyphus/evidence/task-11-appimage-artifact.txt

  Scenario: No debug leftovers
    Tool: Bash
    Preconditions: Release build complete
    Steps:
      1. Search changed files for `[DEBUG`, `console.log`, `dbg!`.
      2. Save output.
    Expected Result: No debug leftovers in shipped files.
    Evidence: .sisyphus/evidence/task-11-no-debug-leftovers.txt
  ```

  **Evidence to Capture**:
  - [ ] Artifact details
  - [ ] Debug leftover search

  **Commit**: NO

---

## Final Verification Wave

> 4 review agents run in PARALLEL. ALL must APPROVE. Present consolidated results to user and get explicit okay before completing.

- [x] F1. **Plan Compliance Audit** — `oracle`
  Verify every Must Have and Must NOT Have. Confirm L2 is text-reader, not bubble UI. Confirm real Claude session loads. Output: `Must Have [N/N] | Must NOT Have [N/N] | VERDICT`.

- [x] F2. **Code Quality Review** — `unspecified-high`
  Run `npm run build`, Rust tests, inspect changed frontend/Rust files for `as any`, debug leftovers, over-abstraction, unused imports. Output: `Build [PASS/FAIL] | Tests [PASS/FAIL] | VERDICT`.

- [x] F3. **Real Manual QA** — `unspecified-high` (+ `playwright` if available)
  Execute every QA scenario on Linux test server with real `fpga_project_coarse_cfo` data. Capture screenshots/evidence. Output: `Scenarios [N/N pass] | VERDICT`.

- [x] F4. **Scope Fidelity Check** — `deep`
  Compare diff against this plan. Reject if L1/L3 persistence was unnecessarily refactored, AI summaries added, or chat bubbles remain. Output: `Scope [CLEAN/issues] | VERDICT`.

---

## Commit Strategy

- **Parser**: `fix(memory): 修复 Claude 会话解析` — `session_transcript.rs`, tests — pre-commit `cargo test encode_cwd_path session_transcript`
- **UI**: `refactor(memory): 改为文本式会话阅读器` — transcript/session components — pre-commit `npm run build`
- **Integration/QA fixes**: `fix(memory): 完善对话搜索集成验证` — any follow-up small fixes — pre-commit `npm run build && cargo test`

---

## Success Criteria

### Verification Commands
```bash
cd src-tauri && cargo test encode_cwd_path session_transcript
npm run build
ssh yufei@100.85.255.89 "ls /home/yufei/.claude/projects/-home-yufei-Repo-fpga-project-coarse-cfo/*.jsonl"
```

### Final Checklist
- [x] 对话搜索能读取真实 Claude Code 会话。
- [x] 会话标题不是 `$@`，空标题有 fallback。
- [x] 右侧详情是格式化文本/转录文档，不是气泡。
- [x] 工具默认折叠或弱化展示。
- [x] 中断消息以状态提示呈现。
- [x] 标记候选记忆可用且不重复。
- [x] L1 静态记忆与 L3 候选记忆未回归。
- [x] Release AppImage 重新构建。
