# Claude Code 记忆管理 v0.2 手动验证日志

> 用途：记录 AgentScope v0.2 加载链模拟与 /memory 命令的对照验证结果
> 模板来源：`docs/design-claude-memory-management-v0.2.md` §9.1

---

## 验证记录模板

```markdown
## 验证记录 YYYY-MM-DD

### 环境
- Claude Code 版本: x.x.x
- AgentScope 版本: v0.2.x
- 操作系统: macOS / Linux / Windows

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

---

## 待验证假设清单

### A. 启动链顺序

| # | 验证项 | 来源 | 验证方法 | 状态 |
|---|--------|------|---------|------|
| A0 | managed CLAUDE.md 是否在启动链最前（若存在） | ✅ 官方确认 | 在系统级放置 managed CLAUDE.md，/memory 观察 | ⬜ |
| A1 | 用户全局 ~/.claude/CLAUDE.md 始终最先加载（或紧随 managed 之后） | ✅ 官方确认 | /memory 观察 | ⬜ |
| A2 | 从根目录到 cwd 的上级 CLAUDE.md 逐级加载 | 官方文档 | 深层目录启动，/memory 观察 | ⬜ |
| A3 | 上级目录加载顺序：根 → cwd（从上到下） | 推断 | /memory 观察 | ⬜ |
| A4 | cwd/CLAUDE.md 在上级目录之后加载 | 官方文档 | /memory 观察 | ⬜ |
| A5 | CLAUDE.local.md 在 CLAUDE.md 之后加载 | 官方文档 | /memory 观察 | ⬜ |
| A6 | 无 paths 的 rule 无条件加载 | 官方文档 | /memory 观察 | ⬜ |
| A7 | 同 scope rule 按文件名排序 | 推断 | 多 rule 测试 | ⬜ |
| A8 | Auto Memory (MEMORY.md) 在 rules 之后加载 | 推断 | /memory 观察 | ⬜ |
| A9 | 祖先目录的 `.claude/CLAUDE.md` 是否实际被 Claude Code 启动加载 | ⚠️ 推断 | 上层目录放 .claude/CLAUDE.md，/memory 观察 | ⬜ |
| A10 | 当前目录 `.claude/CLAUDE.md` 是否被加载 | ✅ 官方确认 | 项目根放 .claude/CLAUDE.md，/memory 观察 | ⬜ |
| A11 | 祖先目录的 `CLAUDE.local.md` 是否被加载 | ✅ 官方确认 | 上层目录放 CLAUDE.local.md，/memory 观察 | ⬜ |

### B. Path-scoped Rules

| # | 验证项 | 来源 | 验证方法 | 状态 |
|---|--------|------|---------|------|
| B1 | path-scoped rules 不在启动链中加载 | 推断 | /memory 观察（确认不显示在启动列表） | ⬜ |
| B2 | paths 匹配对象是"会话中读取的文件路径" | 官方文档 | 创建 paths rule，读取匹配/不匹配文件，观察行为 | ⬜ |
| B3 | paths 使用 glob 模式（minimatch 风格） | 推断 | 测试 `**/*.rs` 等模式 | ⬜ |
| B4 | 多个 paths 模式是 OR 关系 | 推断 | 一个匹配即触发 | ⬜ |

### C. claudeMdExcludes

| # | 验证项 | 来源 | 验证方法 | 状态 |
|---|--------|------|---------|------|
| C1 | 排除模式匹配绝对路径 | 官方文档 | 配置 excludes 后 /memory 观察 | ⬜ |
| C2 | 排除模式是 glob 语法 | 推断 | 测试通配符匹配 | ⬜ |
| C3 | user/project/local 多层设置合并 | 推断 | 多层配置测试 | ⬜ |
| C4 | managed policy 不可被用户覆盖 | 推断 | 若可访问 managed settings 测试 | ⬜ |
| C5 | claudeMdExcludes 是数组合并（concat），不是覆盖 | 推断 | 配置不同层的 excludes，/memory 观察合并结果 | ⬜ |
| C6 | file-based managed settings 的载体是 `managed-settings.json` + `managed-settings.d/*.json`，与 managed CLAUDE.md 是不同载体 | 推断 | 分别检查系统目录下是否存在这两种文件，验证其独立作用 | ⬜ |

### D. @import

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

### E. Auto Memory

| # | 验证项 | 来源 | 验证方法 | 状态 |
|---|--------|------|---------|------|
| E1 | MEMORY.md 启动加载上限：200 行或 25KB | 官方文档 | 创建超大 MEMORY.md，/memory 观察截断位置 | ⬜ |
| E2 | topic 文件（非 MEMORY.md）不自动加载 | 官方文档 | /memory 观察是否有 topic 文件 | ⬜ |
| E3 | Auto Memory 是否与 AgentScope 当前 encode_cwd_path(cwd) 精确匹配结果一致 | 推断 | 在同一 cwd 下对比 /memory 与 AgentScope 模拟结果 | ⬜ |
| E4 | 非 git 目录启动时 Auto Memory 是否尝试路径匹配（或标记为无项目） | 设计文档 §2.3 | 在纯目录（无 .git）启动，/memory 观察 | ⬜ |

---

## 验证记录

### P1 实现完成记录 2026-05-21

#### 实现范围
- **Backend (Rust)**
  - `load_chain.rs`：`simulate_load_chain(cwd)` 核心函数，完整实现 A 区域（启动链）和 B 区域（路径作用域规则）模拟
  - `settings_reader.rs`：多层 `claudeMdExcludes` 读取（managed file-based + user + project + local），glob 匹配
  - `models.rs`：P1 数据类型扩展（`SerLoadChain`, `SerLoadChainStep`, `SerPathScopedRule`, `SerExcludedAsset`, `SerLoadChainWarning`）
  - Service/Routes/Lib：注册 `simulate_claude_memory_load_chain` Tauri 命令

- **Frontend (React/TS)**
  - `LoadChainSimulator.tsx`：加载链模拟器页面，含路径输入、A/B 区域分区展示、被排除资产折叠面板、warnings 提示
  - `useLoadChain.ts`：调用 Rust 命令的 hook
  - 类型扩展：`LoadChainResult`, `LoadChainStep`, `PathScopedRule`, `ExcludedAsset`, `LoadChainWarning`
  - 导航：`ClaudeMemoryPage` 新增 `"load-chain"`，Sidebar 增加"加载链模拟器"子导航

- **Tests**
  - Rust 单元测试：11 个 load_chain 测试（祖先顺序、当前目录三层、rules 递归、paths 分区、excludes 排除、managed warning、Auto Memory 截断行优先、Auto Memory 截断字节优先、不可读容错、managed CLAUDE.md 不被排除、路径不存在容错）
  - E2E 测试：新增"Claude 记忆域可切换到加载链模拟器"导航测试（7 个测试用例）

#### 已覆盖验证项

| # | 验证项 | 覆盖方式 | 状态 |
|---|--------|---------|------|
| A2 | 祖先 CLAUDE.md 逐级加载 | `test_ancestor_claude_md_order` | ✅ 单元测试 |
| A3 | 根 → cwd 加载顺序 | `test_ancestor_claude_md_order` | ✅ 单元测试 |
| A4 | cwd/CLAUDE.md 在上级之后 | `test_ancestor_claude_md_order` + `test_current_dir_three_files` | ✅ 单元测试 |
| A5 | CLAUDE.local.md 加载 | `test_ancestor_local_md_order` + `test_current_dir_three_files` | ✅ 单元测试 |
| A6 | 无 paths 的 rule 无条件加载 | `test_paths_vs_unconditional_rules` | ✅ 单元测试 |
| A7 | 同 scope rule 按文件名排序 | `scan_rules_dir` 实现中显式 `sort_by` | ✅ 代码审查 |
| A10 | 当前目录 `.claude/CLAUDE.md` | `test_current_dir_three_files` | ✅ 单元测试 |
| A11 | 祖先目录 CLAUDE.local.md | `test_ancestor_local_md_order` | ✅ 单元测试 |
| B1 | path-scoped rules 不在启动链 | `test_paths_vs_unconditional_rules` | ✅ 单元测试 |
| C1 | 排除模式匹配绝对路径 | `test_claude_md_excludes` | ✅ 单元测试 |
| C2 | glob 语法支持 | `test_glob_match_star` + `test_glob_match_double_star` | ✅ 单元测试 |
| C3 | 多层设置合并 | `test_concat_merge` | ✅ 单元测试 |
| E1 | MEMORY.md 截断（200行/25KB） | `test_auto_memory_truncation_lines_first` + `test_auto_memory_truncation_bytes_first` | ✅ 单元测试 |

#### 待手动验证项（需要 /memory 命令对照）
- A0, A1, A8, A9：managed / 用户全局 / Auto Memory 顺序、祖先 `.claude/CLAUDE.md`
- B2-B4：paths 匹配行为（需实际 Claude Code 运行时验证）
- C4-C6：managed policy 不可覆盖、file-based 载体验证
- D1-D8：@import 行为（P2/P3/P4 范围）
- E2-E4：Auto Memory 匹配语义

#### 质量检查
- `cargo fmt --check`：✅ 通过
- `cargo clippy -- -D warnings`：✅ 通过
- `cargo test`：175 passed / 0 failed
- `npm run build`：✅ 通过
- `npm test`（E2E）：71 passed / 0 failed

---

## 安全原则（验证阶段）

> P1 语义验证为纯观察行为，严禁修改任何真实 Claude 资产。

### 验证阶段 DO NOT
- **禁止**在真实 `~/.claude/` 目录下创建、修改、删除任何文件
- **禁止**在真实项目目录下创建测试用的 `CLAUDE.md`、`.claude/CLAUDE.md`、`CLAUDE.local.md`
- **禁止**修改真实项目的 `.claude/rules/` 或 `.claude/settings.json`
- **禁止**以任何方式触发 Claude Code 的写入操作（如让 Claude 生成测试文件）

### 验证阶段 DO
- 使用**已有的**真实项目目录进行观察（如本仓库 `/Users/ckstar/Repo/agent-scope`）
- 使用**临时目录**进行边界测试（如 `/tmp/test-no-git/`），验证后清理
- 使用**专用测试项目**（若需要创建文件）
- 记录 `/memory` 输出时只复制文本，不触发任何命令执行

### 违规后果
- 若验证过程中意外修改了真实 `~/.claude/` 目录：
  - 立即停止验证操作
  - 保留被修改文件当前状态作为证据，不做自行回滚
  - 向用户报告：哪个文件、什么操作、预期变化
  - 等待用户指示，不自行恢复
- 验证脚本必须显式检查 `CLAUDE_CONFIG_DIR` 环境变量，确保不操作真实目录

---

## P1 第一批真实对照验证指令

> 目标：使用真实 Claude Code `/memory` 输出与 AgentScope 加载链模拟器对照，验证 P1 核心假设。
> 执行前提：AgentScope 已编译并可运行（`npm run tauri dev` 或已安装版本）。
> 范围：仅验证 P1（加载链模拟），不验证 P2 `@import`。

### 一、选择至少 2 个真实 cwd 的原则

| # | cwd 类型 | 选择理由 | 必须包含的文件 |
|---|---------|---------|--------------|
| 1 | **已注册的 git 项目**（如本仓库 `/Users/ckstar/Repo/agent-scope`） | 覆盖 Auto Memory 匹配、project rules、祖先目录链 | `CLAUDE.md`、`.claude/rules/*.md`、`.claude/CLAUDE.md`（若有） |
| 2 | **深层子目录**（如项目内的 `src-tauri/src/collectors/claude_memory/`） | 覆盖 A2-A4 祖先目录加载顺序 | 祖先目录（项目根）有 `CLAUDE.md`，当前目录无 `CLAUDE.md` |
| 3 | **非 git 纯目录**（如临时创建的 `/tmp/test-no-git/`） | 覆盖 E4（非 git 目录 Auto Memory 行为） | 可放入 `CLAUDE.md` 和 `.claude/rules/*.md` 作为测试数据 |
| 4 | **用户 home 目录或有用户级 rules 的目录** | 覆盖 A1（用户全局 `~/.claude/CLAUDE.md`）、A6-A7（rules 排序） | 确保 `~/.claude/CLAUDE.md` 和 `~/.claude/rules/*.md` 存在 |

**最低要求**：至少选择 **#1（已注册 git 项目）** 和 **#2（深层子目录）** 两个 cwd 进行对照。

### 二、每个 cwd 在 Claude Code 中的记录步骤

在 Claude Code CLI 中执行：

```bash
# 1. 进入目标目录
cd <目标 cwd>

# 2. 启动 Claude Code（如果未在会话中）
claude

# 3. 执行 /memory 命令
/memory
```

**记录内容**：
1. `/memory` 命令的完整输出（可复制文本或截图）
2. 输出中显示的加载顺序列表（从上到下）
3. 每个加载项的 scope 标注（如 `user`、`project`、`local`、`auto`）
4. path-scoped rules 的展示方式（是否在启动链中？单独列出？）
5. Auto Memory 的展示（如果有）：行数、大小、是否截断
6. 被排除的 assets（如果有 `claudeMdExcludes` 配置）
7. 任何 warnings 或 errors

### 三、在 AgentScope 加载模拟器中的记录步骤

1. 打开 AgentScope 桌面应用
2. 切换到 **Claude 记忆** 域
3. 点击侧边栏 **加载链模拟器**
4. 输入与 Claude Code 中相同的 `cwd` 路径
5. 点击 **模拟加载**
6. 记录：
   - A 区域（启动链）的完整列表和顺序
   - B 区域（路径作用域规则）的列表
   - 被排除资产（如有）
   - warnings（如有）
   - host_profile 信息（OS、主机名、Claude 配置目录）

### 四、优先核对的假设清单

按优先级排序，逐项核对：

#### 高优先级（启动链核心顺序）

| 假设 | 核对方法 | AgentScope 预期 | 若不符则归类为 |
|------|---------|----------------|--------------|
| **A0** managed `CLAUDE.md` 在最前 | 检查 `/memory` 输出第一项 | 若 `resolve_managed_dir()` 下 `CLAUDE.md` 存在，则 `order=1` | **代码 bug**（若存在但未放最前）或 **假设待定**（若 `/memory` 未显示 managed） |
| **A1** 用户全局 `~/.claude/CLAUDE.md` 紧随 managed 之后 | 检查第二项（或第一项，若无 managed） | `scope=user, asset_type=user_claude_md` | **代码 bug**（顺序错误）或 **假设待定**（若 `/memory` 不加载用户全局） |
| **A2-A3** 祖先目录 `CLAUDE.md` 从根到 cwd 逐级加载 | 深层子目录测试，检查祖先文件顺序 | 根目录 `CLAUDE.md` 在上，cwd 父目录在下 | **代码 bug**（遍历方向错误） |
| **A4** cwd `CLAUDE.md` 在祖先目录之后 | 检查 cwd 的 `CLAUDE.md` 是否在祖先列表之后 | `asset_type=project_claude_md` 在 `ancestor_claude_md` 之后 | **代码 bug** |
| **A5** `CLAUDE.local.md` 在同级 `CLAUDE.md` 之后 | 检查同一目录下两者顺序 | `local_md` 在 `project_claude_md` 之后 | **代码 bug** |
| **A6** 无 paths 的 rule 无条件加载到 A 区域 | 检查 `/memory` 中 rules 是否显示在启动链 | `global_rule` / `project_rule` 在 startup_chain 中 | **代码 bug**（被错误分到 B 区域）或 **假设待定**（Claude Code 实际不加载） |
| **A10** 当前目录 `.claude/CLAUDE.md` 被加载 | **⚠️ 第一批只读验证不覆盖**：需在项目根创建真实 `.claude/CLAUDE.md`。若后续验证，必须在隔离环境或经用户确认后操作 | `asset_type=project_dot_claude_md` 存在 | 单元测试已覆盖（`test_current_dir_three_files`），语义验证为 P2/P3 范围 |

#### 中优先级（边界行为）

| 假设 | 核对方法 | AgentScope 预期 | 若不符则归类为 |
|------|---------|----------------|--------------|
| **A9** 祖先目录 `.claude/CLAUDE.md` 是否被加载 | **⚠️ 第一批只读验证不覆盖**：需在祖先目录创建 `.claude/CLAUDE.md`。当前实现不扫描祖先 `.claude/CLAUDE.md` | 若 `/memory` 显示加载，则标记为 **假设待定**（需补充实现） | P2/P3 范围 |
| **A11** 祖先目录 `CLAUDE.local.md` 被加载 | **⚠️ 第一批只读验证不覆盖**：需在祖先目录创建 `CLAUDE.local.md`。单元测试已覆盖（`test_ancestor_local_md_order`） | `asset_type=ancestor_local_md` 存在 | 语义验证为 P2/P3 范围 |
| **E4** 非 git 目录 Auto Memory 行为 | 在临时目录（如 `/tmp/test-no-git/`）启动，观察 `/memory` | 当前实现用 `encode_cwd_path` 匹配；若 `/memory` 不显示 Auto Memory，则归类为 **假设待定** | 隔离环境可安全验证 |

### 五、差异写入验证日志的格式

对每个测试的 cwd，在本文档的 **验证记录** 章节追加：

```markdown
## 验证记录 YYYY-MM-DD

### 环境
- Claude Code 版本: x.x.x（执行 `claude --version` 获取）
- AgentScope 版本: v0.2.x
- 操作系统: macOS / Linux / Windows
- 测试 cwd: `/absolute/path/to/cwd`

### 用例: {简要描述，如"已注册 git 项目根目录"}

#### 预期行为（来自官方文档 / 推断）
- 启动链应包含：managed CLAUDE.md → 用户全局 CLAUDE.md → 祖先 CLAUDE.md（根→cwd）→ cwd CLAUDE.md → cwd .claude/CLAUDE.md → cwd CLAUDE.local.md → 无条件 rules → Auto Memory
- path-scoped rules 不在启动链中

#### /memory 命令输出
```
{粘贴 /memory 完整输出}
```

#### AgentScope 模拟输出
```
{粘贴加载链模拟器输出（或截图）}
```

#### 对照结果
- [ ] 完全一致
- [ ] 有差异（差异说明: ...）
- [ ] 无法验证（原因: ...）

#### 差异分类
- **代码 bug**：{具体描述，如"AgentScope 将祖先目录遍历方向弄反了"}
- **假设待定**：{具体描述，如"Claude Code 似乎不加载祖先 CLAUDE.local.md，需进一步验证"}

#### 假设更新
- A2：验证通过 / 待更新 / 需补充实现
- A3：...
```

### 六、差异分类规则

| 分类 | 判定标准 | 后续动作 |
|------|---------|---------|
| **代码 bug** | AgentScope 输出与 `/memory` 明显矛盾，且 `/memory` 行为符合官方文档描述 | 修复代码，重新测试 |
| **假设待定** | `/memory` 行为与设计文档的假设不一致，但官方文档未明确说明；或 `/memory` 输出本身模糊/难以解读 | 记录差异，标记假设状态为"待进一步验证"，不立即修改代码 |
| **环境差异** | 差异由环境特有因素导致（如 managed 目录权限、特定操作系统路径差异） | 在验证记录中注明环境，判断是否需增加环境适配代码 |
| **已知限制** | 差异来自当前已实现范围的边界（如 A9 祖先 `.claude/CLAUDE.md` 当前未实现） | 在验证记录中标记为"当前实现未覆盖"，作为 P2/P3 参考 |

### 七、验证执行检查清单

#### 必做项（只用现有真实资产即可观察）

- [ ] 已选择至少 2 个真实 cwd（建议 #1 git 项目 + #2 深层子目录）
- [ ] 已在每个 cwd 执行 Claude Code `/memory` 并记录输出
- [ ] 已在 AgentScope 加载模拟器中输入相同 cwd 并记录输出
- [ ] 已核对 A 区域启动链顺序（无需创建任何新文件即可观察的项）
- [ ] 已核对 B 区域 path-scoped rules 展示
- [ ] 已记录 warnings / excluded assets / Auto Memory 观察结果（若环境自然存在）
- [ ] 已将差异按"代码 bug" / "假设待定" / "环境差异" / "已知限制"分类
- [ ] 已在本文档追加验证记录（使用上方模板）
- [ ] 已更新假设清单中的状态列（⬜ → ✅ 或 ⚠️）

#### 本批不强制覆盖（需创建真实资产才能验证）

以下项若所选真实 cwd 已天然存在对应文件，可观察记录；若不存在则视为本批未覆盖，不创建文件来满足验证：

- **A10**（当前目录 `.claude/CLAUDE.md`）：需项目天然存在该文件。单元测试已覆盖（`test_current_dir_three_files`），语义验证为后续批次范围
- **A9**（祖先 `.claude/CLAUDE.md`）：当前实现不扫描，需验证 Claude Code 是否实际加载
- **A11**（祖先 `CLAUDE.local.md`）：需祖先目录天然存在该文件。单元测试已覆盖（`test_ancestor_local_md_order`），语义验证为后续批次范围
- **E4**（非 git 目录 Auto Memory）：若本批进行，必须使用临时隔离目录（如 `/tmp/test-no-git/`）；若不做，则标记"本批未覆盖"
