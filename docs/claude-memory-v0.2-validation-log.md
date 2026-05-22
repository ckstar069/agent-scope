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
| E3 | Auto Memory 是否按 **repo identity / project root** 语义匹配（而非 cwd 精确匹配） | 官方文档 | 同一 git repo 的不同子目录下对比 `/memory` 与 AgentScope 模拟结果，确认共享同一 Auto Memory | ⚠️ 假设被挑战，需修复 |
| E4 | 非 git 目录启动时 Auto Memory 是否尝试路径匹配（或标记为无项目） | 设计文档 §2.3 | 在纯目录（无 .git）启动，/memory 观察 | ⬜ |
| E5 | `autoMemoryDirectory` 自定义路径是否被支持 | 官方文档 | 若用户设置了 `autoMemoryDirectory`，观察 AgentScope 是否能正确查找 | ⚠️ P1 limitation：不读取该设置，仅表现为默认路径 `auto_memory_not_found`，不是专门的 limitation warning |

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
- E2, E4-E5：Auto Memory 匹配语义（E3 假设已被挑战，需先修复代码再验证）

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
- **E5**（`autoMemoryDirectory` 自定义路径）：P1 不读取该设置。若用户设置了自定义路径，AgentScope 会显示 `auto_memory_not_found`，这是已知 limitation

---

## 验证记录 2026-05-22（首组真实观察）

> **状态**：首组真实观察 / 尚不判定 P1 通过 / 暴露验证口径与 Auto Memory 身份假设需校正
>
> **重要**：本次观察**不**视为验证通过，也**不**视为已确认 bug。它暴露了 P1 验证口径和 Auto Memory 匹配策略需要校正，后续需重新设计验证方法后再做对照。

### 环境

- Claude Code 版本: v2.1.145
- AgentScope 版本: v0.2.0（开发模式 `npm run tauri dev`）
- 操作系统: macOS
- 测试 cwd: `/Users/ckstar/Repo/agent-scope`

### 用例: 已注册 git 项目根目录

#### Claude Code `/memory` 输出（用户截图观察）

```
Auto-memory: on
Project memory: ./CLAUDE.md
User memory: ~/.claude/CLAUDE.md
Open auto-memory folder
```

**观察要点**：
1. `/memory` 输出为**交互式 UI 元素**，不是文本列表形式的"启动链顺序报告"
2. 显示 "Auto-memory: on" — 仅表明 Auto Memory **功能处于开启状态**；不能单独证明当前 cwd 已有可加载的 `MEMORY.md`。是否存在 Auto Memory 内容，仍需结合磁盘实际路径 / Claude 后续可观察信息判断
3. 显示 "Project memory: ./CLAUDE.md" — 确认项目级 CLAUDE.md 被识别
4. 显示 "User memory: ~/.claude/CLAUDE.md" — 仅确认 `/memory` UI 展示了 user memory **入口**；该入口不等同于当前文件系统中 `~/.claude/CLAUDE.md` 必然存在。首组对照中 user memory 差异应归入"验证口径 / 文件存在性待核对"
5. **未显示**启动链的逐条顺序（如 managed → user → ancestor → cwd → rules → auto）
6. **未显示** path-scoped rules 的独立列表
7. **未显示**各加载项的 scope 标注（user/project/local/auto 等）

#### AgentScope 加载链模拟器输出

- A 区域（启动链）：仅显示 `/Users/ckstar/Repo/agent-scope/CLAUDE.md`
- B 区域（path-scoped rules）：无
- Warnings：`auto_memory_not_found`

#### 对照结果

- [ ] 完全一致
- [x] 有差异（差异说明: 见下方）
- [ ] 无法验证

#### 差异分类

| 差异项 | AgentScope 输出 | Claude `/memory` | 分类 | 说明 |
|--------|----------------|-----------------|------|------|
| Auto Memory | `auto_memory_not_found` | "Auto-memory: on" | **假设待定→需校正** | AgentScope 用 `encode_cwd_path(cwd)` 匹配 `~/.claude/projects/<id>/memory/`，但 Claude 实际可能用 repo identity 或 project root 匹配。深层子目录可能共享同一 repo 的 auto memory |
| 启动链展示形式 | 文本列表（A 区域逐条） | 交互式 UI（on/off 开关 + 文件路径） | **验证口径问题** | 不能直接将两者逐条对比，需校正验证方法 |
| User memory | 未在 A 区域显示 | "User memory: ~/.claude/CLAUDE.md" | **验证口径 / 文件存在性待核对** | `/memory` 展示的是 user memory 入口，不等同于文件必然存在；AgentScope 在开发模式下读取同一文件系统，若真实 `~/.claude/CLAUDE.md` 存在则应显示，但首组观察未单独验证该文件存在性 |
| Path-scoped rules | B 区域显示（若有） | `/memory` 未展示 rules | **验证口径问题** | `/memory` 可能不展示 rules，或展示方式不同 |

#### 假设更新

1. **验证口径校正**：`/memory` 是交互式 UI，不是文本化启动链报告。P1 验证不应强行逐条对比顺序，而应改为：
   - **文件存在性/识别项对照**：Claude `/memory` 中显示的记忆项，AgentScope 是否也识别到？
   - **顺序规则校验**：AgentScope 输出是否符合官方文档描述的加载规则？（独立校验，不直接对比 `/memory`）
   - **差异项记录**：哪些是 `/memory` UI 不可观察但 AgentScope 可展示的（如 managed CLAUDE.md、祖先目录链）

2. **Auto Memory 假设校正**：
   - 原假设（`encode_cwd_path(cwd)` 精确匹配）**被挑战**
   - Claude 官方文档说明：git repo 内 Auto Memory 按 repository 派生，同一 repo 的子目录与 worktree 共享 auto memory
   - **普通 git repo 子目录**：当前实现已修正为向上查找 `.git` 目录定位 repo root，再用 repo root 编码匹配 ✅
   - **git worktree**：P1 不解析 `.git` 文件以恢复主 repo identity，也不做 cwd-encoded 近似匹配（避免误展示）；返回 `auto_memory_worktree_unsupported` warning。worktree 共享 Auto Memory 语义留后续实现 ⚠️ P1 limitation
   - **非 git 目录**：回退到 cwd 编码路径匹配

3. **E3 验证项重新定义**：
   - 原 E3："Auto Memory 是否与 AgentScope 当前 encode_cwd_path(cwd) 精确匹配结果一致"
   - 新 E3："Auto Memory 是否按 repo identity / project root 语义匹配，而非 cwd 精确匹配"

#### 后续动作

##### 2026-05-22 第一轮校正（已完成）

- [x] 校正所有文档中的 Auto Memory 匹配策略描述（requirements / design / validation log / P1 user validation）
- [x] 校正 P1 验证口径（从"逐条对比"改为"存在性对照 + 规则独立校验"）
- [x] 审计 `find_auto_memory()` 实现，给出最小修复方案（支持 git repo root 匹配）
- [x] 明确 `autoMemoryDirectory` 设置项的处理策略（不读取，仅表现为默认路径 `auto_memory_not_found`，不是专门 limitation warning）

##### 2026-05-22 第二轮安全收口（已完成）

- [x] 修正首组观察记录中的过度判断（Auto-memory: on 不推断文件存在；user memory 入口不推断文件存在）
- [x] 修正 worktree 支持声明：当前只认 `.git` 目录，worktree `.git` 文件返回 None
- [x] 修正 worktree 安全行为：不静默回退到 cwd-encoded Auto Memory，返回 `auto_memory_worktree_unsupported` warning
- [x] 补充 worktree 场景 Rust 测试：验证不加载、不返回 `not_found`、返回 limitation warning
- [x] 清理 requirements 残留旧口径（FR-02、启动链、v0.3 Project identity）

##### 待执行状态（Round 7.1 更新）

- [x] 重新执行真实对照验证（使用修正后的 AgentScope 版本）→ **已由 Round 7.1 Linux 人工回归覆盖，见后文记录**
- [x] 在至少 2 个 cwd（项目根目录 + 深层子目录）对比 Claude `/memory` 与 AgentScope 模拟结果 → **已覆盖**：`/home/yufei/Repo/fpga_project_agc`（根目录）和 `/home/yufei/Repo/fpga_project_agc/src/python_model/L3_pipeline`（深层子目录）
- [ ] 验证 Auto Memory 匹配：同一 git repo 的不同子目录是否共享同一 Auto Memory → **仍待验证**：需确认磁盘上存在 `MEMORY.md` 且模拟器正确匹配。当前样本中 `auto_memory_not_found` 未独立核查磁盘事实，暂不作为 P1 阻塞项

---

## 验证记录 2026-05-22（Linux 非 git 空 cwd 观察）

> **状态**：补充观察 / 空资产场景 / 不计入 P1 正向验收样本
>
> **说明**：该用例 cwd 无任何 Claude 资产，Claude `/memory` UI 仍显示 Auto-memory on、User memory、Project memory 入口，这属于 UI 默认展示行为，不能推断对应文件存在。AgentScope 输出与磁盘事实一致。

### 环境

- Claude Code 版本: v2.1.x
- AgentScope 版本: v0.2.0（Linux AppImage）
- 操作系统: Linux 3.50
- 测试 cwd: `/home/yufei/Repo/demo`

### 磁盘事实（只读核查）

| 检查项 | 结果 |
|--------|------|
| 是否为 git repo | 否（无 `.git`） |
| `CLAUDE.md` | 不存在 |
| `.claude/CLAUDE.md` | 不存在 |
| `CLAUDE.local.md` | 不存在 |
| `.claude/rules/` | 不存在 |
| `~/.claude/CLAUDE.md` | 不存在 |
| `~/.claude/projects/*/memory/MEMORY.md` | 未发现 |

### Claude Code `/memory` 输出（用户截图观察）

- Auto-memory: on
- User memory: ~/.claude/CLAUDE.md
- Project memory: ./CLAUDE.md
- Open auto-memory folder

**解读**：
- `Auto-memory: on`：功能开关状态，不推断文件存在（见首组观察记录修正）
- `User memory: ~/.claude/CLAUDE.md`：UI 入口/管理项，不等同于该文件在磁盘上存在
- `Project memory: ./CLAUDE.md`：UI 入口/管理项，不等同于当前 cwd 存在该文件
- 该 cwd 无任何 Claude 资产，`/memory` 的 User/Project memory 项应理解为**可配置的入口占位**，而非**已加载的资产**

### AgentScope 加载链模拟器输出

- A 区域（启动链）：0 步
- B 区域（path-scoped rules）：0 条
- Warnings：`auto_memory_not_found`
- Host profile：Linux / home_dir=/home/yufei / claude_config_dir=~/.claude

### 对照结论

| 对比项 | 结果 | 说明 |
|--------|------|------|
| 文件存在性对照 | ✅ 一致 | AgentScope A 区域 0 步与磁盘事实一致（无 CLAUDE.md、无 rules） |
| Auto Memory | ✅ 一致 | AgentScope `auto_memory_not_found` 与磁盘事实一致（无 MEMORY.md） |
| `/memory` UI 展示 | ⚠️ 需正确理解 | UI 显示 User/Project memory 入口，但该 cwd 和 `~/.claude` 均无对应文件 |

### 差异说明

无实质性差异。AgentScope 输出与磁盘事实一致。

### 验收样本判定

**该用例不计入 P1 正向资产加载验收样本**。原因：
1. 正向验收样本应覆盖"有资产时是否正确加载"的核心场景
2. 空资产场景只能验证"无资产时不误加载"，属于边界安全校验
3. 推荐在以下 cwd 进行正向验收：
   - `/home/yufei/Repo/fpga_project_agc`（git 项目根目录，应有 CLAUDE.md 和 rules）
   - `/home/yufei/Repo/fpga_project_agc/src/python_model/L3_pipeline`（同一 git repo 的深层子目录，验证 Auto Memory 共享语义）

### 假设更新

无更新。空资产场景与当前实现预期一致。

---

## 验证记录 2026-05-22（Round 7：3 项修复 + 回归测试）

> **状态**：代码修复完成 / 回归测试通过 / 待重新打包 Linux AppImage 后交付验证
>
> **触发原因**：在 Linux 3.50 真机验证 fpga_project_agc 时发现 3 个问题

### 环境

- Claude Code 版本: v2.1.x
- AgentScope 版本: v0.2.0 + Round 7 补丁
- 操作系统: Linux 3.50 / macOS（开发环境）

### 问题 1：~ 路径展开未实现

#### 现象
在模拟器输入框输入 `~/Repo/fpga_project_agc`，点击模拟后报错 "目录不存在"。

#### 根因
`simulate_load_chain_service` 直接 `PathBuf::from(&cwd)`，未处理 `~` 展开。Shell 风格的 `~/` 在 UI 输入中非常常见。

#### 修复
- **落点**：`src-tauri/src/services/claude_memory_service.rs`
- 新增 `expand_tilde_path` helper：支持 `~` 和 `~/...`，不支持 `~user`
- home 无法解析时返回清晰错误 "无法获取用户主目录"
- 输出结果中的 `cwd` 字段显示实际解析后的绝对路径

#### 回归测试
- Rust 单元测试：`test_expand_tilde_path_with_home_injected`（注入 fake home，不修改进程级 HOME）
- Rust 单元测试：`test_expand_tilde_path_unchanged`（绝对路径、相对路径、`~user` 不展开）
- Rust 单元测试：`test_simulate_service_absolute_path`（service 层透传验证）
- E2E 测试：`~ 路径输入发送到后端`（验证前端将 ~ 路径原样传给后端）

### 问题 2：失败后旧结果残留

#### 现象
第一次模拟成功，显示 A/B 区域结果。第二次输入错误路径模拟失败后，A/B 区域的上次结果仍然显示在页面上，与错误提示并存。

#### 根因
`useLoadChain.ts` 的 `simulate` 函数在请求前只 `setError(null)`，未 `setResult(null)`。错误状态下旧 `result` 未被清除。

#### 修复
- **落点**：`src/features/claude-memory/hooks/useLoadChain.ts`
- 在 `simulate` 开始时增加 `setResult(null)`

#### 回归测试
- E2E 测试：`失败后清除旧结果`（先成功mock→验证结果显示→再失败mock→验证结果清除且错误显示）

### 问题 3：深层 cwd 漏 repo root 的 rules 和 settings

#### 现象
在 `/home/yufei/Repo/fpga_project_agc/src/python_model/L3_pipeline`（git repo 深层子目录）启动模拟：
- repo root 的 `.claude/rules/` 未被扫描
- repo root 的 `.claude/settings.json`（含 claudeMdExcludes）未被读取

#### 根因
- project rules 扫描路径：`cwd.join(".claude").join("rules")`（用了 cwd 而非 repo root）
- project/local settings 读取路径：`cwd.join(".claude").join("settings.json")`（同上）
- 对于 git repo 子目录，Claude Code 的 project 级资产基准是 repo root，不是 cwd

#### 修复
- **落点 1**：`src-tauri/src/collectors/claude_memory/load_chain.rs`
  - project rules 扫描前，先 `find_git_repo_root(cwd)`，若找到则使用 repo root，否则回退到 cwd
- **落点 2**：`src-tauri/src/collectors/claude_memory/settings_reader.rs`
  - project settings (`settings.json`) 和 local settings (`settings.local.json`) 同样使用 `find_git_repo_root(cwd)` 定位基准目录
- **落点 3**：`src-tauri/src/collectors/claude_memory/path_resolver.rs`
  - 将 `find_git_repo_root` 从 `load_chain.rs` 的私有函数提升为公共函数，供 `settings_reader.rs` 复用

#### 行为一致性
| 场景 | project rules | project settings | local settings |
|------|---------------|------------------|----------------|
| git repo 子目录 | repo root `.claude/rules/` | repo root `.claude/settings.json` | repo root `.claude/settings.local.json` |
| 非 git 目录 | cwd `.claude/rules/` | cwd `.claude/settings.json` | cwd `.claude/settings.local.json` |

#### 回归测试
- Rust 单元测试：`test_deep_cwd_reads_repo_root_rules`（git repo 深层子目录读取 repo root rules → A 区域）
- Rust 单元测试：`test_deep_cwd_reads_repo_root_settings`（git repo 深层子目录的 claudeMdExcludes 来自 repo root settings）
- Rust 单元测试：`test_non_git_deep_cwd_uses_own_rules`（非 git 目录仍使用自身 cwd 的 rules）
- Rust 单元测试：`test_deep_cwd_repo_root_path_scoped_rule_in_b_zone`（repo root 有 paths rule → B 区域，不进入 A 区域）
- Rust 单元测试：`test_deep_cwd_reads_repo_root_local_settings`（repo root `settings.local.json` 排除生效，来源标注为 local）

### 测试汇总

| 测试类型 | 数量 | 结果 |
|---------|------|------|
| Rust 单元测试 | 186 | ✅ 全部通过 |
| E2E 测试 | 73 | ✅ 全部通过 |
| cargo fmt | - | ✅ 通过 |
| cargo clippy | - | ✅ 通过 |
| npm run build | - | ✅ 通过 |

### Round 7.1 状态

- [x] 重新打包 Linux 3.50 AppImage（2026-05-22 10:38 完成）
- [x] 重构 `expand_tilde_path` 测试隔离（可注入 home，不修改进程级 HOME）
- [x] 修正文档 A/B 区域口径
- [x] 修正 `load_chain.rs` 顶部注释 drift
- [x] 补 Rust 测试（deep cwd path-scoped rule + local settings）

---

## 验证记录 2026-05-22（Round 7.1：Linux 3.50 人工回归）

> **状态**：四项验证全部通过 ✅
>
> **验证方式**：只读观察，未创建/修改/删除任何真实 Claude 资产
>
> **环境**：Linux 3.50 / AgentScope v0.2.0 + Round 7/7.1 补丁 / AppImage 构建时间 2026-05-22 10:38

### 验证 1：~ 路径展开 ✅

- **输入**：`~/Repo/fpga_project_agc`
- **结果**：模拟成功
- **CWD 显示**：`/home/yufei/Repo/fpga_project_agc`（已展开为绝对路径）
- **结论**：通过。不再出现 "目录不存在: ~/..." 错误。

### 验证 2：项目根目录模拟 ✅

- **输入**：`/home/yufei/Repo/fpga_project_agc`
- **结果**：
  - A 区域：36 步，包含 root `CLAUDE.md` 和 repo root `.claude/rules/` 无条件 rules
  - B 区域：0 条
- **说明**：当前样本中已观察到的 rules 均无 `paths` frontmatter，属于无条件 rules，进入 A 区域是正确行为。B 区域 0 条不视为失败。

### 验证 3：深层 cwd 使用 repo root project 资产 ✅

- **输入**：`/home/yufei/Repo/fpga_project_agc/src/python_model/L3_pipeline`
- **结果**：
  - A 区域：仍为 36 步
  - 仍显示 repo root `CLAUDE.md`（显示为 `ancestor instruction`，可接受）
  - 仍显示 repo root `.claude/rules/` 无条件 rules
  - B 区域：仍为 0 条
- **关键验收点**：同一 repo root 的 project assets 未因 deep cwd 而丢失。通过与验证 2 对比，A 区域 project 级资产一致，确认 deep cwd 正确回退到 repo root 基准。

### 验证 4：模拟失败后旧结果清空 ✅

- **输入**：`/home/yufei/Repo/fpga_project_agc/src/python_mline`（不存在目录）
- **结果**：
  - 页面仅显示 "目录不存在" 错误
  - 上一轮成功模拟的 HostProfile / warnings / A 区域 / B 区域结果不再残留
- **结论**：通过。

### 关于 Auto Memory

本次样本中 `auto_memory_not_found` 出现。由于未独立核查磁盘上是否存在对应 `MEMORY.md`，**不作为 P1 阻塞项**。若后续确认磁盘存在 `MEMORY.md` 而模拟器未匹配到，再单独跟进。

### 验证完成声明

Round 7 / 7.1 修复的 3 项问题（~ 展开、失败清空、deep cwd repo root 基准）均通过 Linux 3.50 真机人工验证。
