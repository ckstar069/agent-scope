# ptv v0.4 — 模板与项目记忆来源区分

## TL;DR

> **快速摘要**：为"静态记忆"面板增加文件来源区分能力。在 Settings 中配置全局模板路径，扫描时对比模板文件集合，自动标注每个记忆文件是"来自模板"还是"项目特有"。前端增加三级筛选（全部/模板记忆/项目记忆）和来源视觉标记。
>
> **交付物**：
> - Rust 后端：`TemplateFingerprint` 模块 + `AppState` 扩展 + `set_template_path`/`get_template_path` 命令 + `get_project_files` 增强
> - 前端：Settings 模板路径配置 + `MemoryFileTree` 筛选控件 + 来源视觉标记
> - E2E 测试：Playwright 覆盖三级筛选、空状态、路径错误降级
>
> **预估工作量**：小型（Rust 4 任务 + 前端 4 任务 + E2E 1 任务 = 9 任务）
> **并行执行**：YES — 4 个 Wave
> **关键路径**：Task 1 → Task 3 → Task 7 → Task 9 → F1-F4

---

## 上下文

### 原始需求
用户在使用 ptv 查看业务项目的"静态记忆"面板时，所有记忆文件（CLAUDE.md、.claude/rules/*.md、.sisyphus/notepads/*.md 等）混合展示，无法区分哪些是从 `ai_project_template` 模板继承的，哪些是业务项目自己创建的。需要在展示时区分来源，并提供按来源筛选的能力。

### 访谈总结
**关键讨论**：
- 技术路线：路径指纹对比——配置全局模板路径，扫描时按文件路径集合对比
- 筛选粒度：三级——全部 / 模板记忆 / 项目记忆
- 模板路径配置：全局设置（Settings 页面），所有监控项目共享
- 本期不做"已修改检测"（模板文件被业务修改过的情况）
- 模板路径为空或无效时：降级处理，所有文件标注为"unknown"
- 跨机器：每台机器独立配置模板路径
- 缓存策略：Rust 端 `AppState` 缓存模板文件路径集合，路径变化时自动重建

**研究结论**：
- 模板项目路径：`/Users/ckstar/Repo/ai_project_template`
- 模板有 `.template_root` 标记文件（不会被复制到业务项目）
- 模板白名单目录中约 53 个 .md 文件（CLAUDE.md + .claude/rules/* + docs/design/* + .sisyphus/notepads/*）
- 模板本身没有 AGENTS.md、没有 .opencode/ 目录、没有 skill 文件
- 业务项目从模板克隆后会创建自己的 AGENTS.md、自定义 rules、额外 notepads
- 当前 `ProjectFilesCollector` 仅按目录类型分类（source_group），不区分来源

### Metis 审查要点
**已识别并解决的缺口**：
- `TemplateDataPayload` 不含 `project_files`，origin 计算只需在 `get_project_files` 中处理，不影响 watched 热路径
- Rust↔TypeScript 双端 `SerProjectFile` 定义需同步更新 `origin` 字段
- 模板路径存储在独立 `settings.json` 而非 `projects.json`，职责分离
- 模板路径通过 `AppState` 管理（`set_template_path`/`get_template_path` 命令），`get_project_files` 从 AppState 读取
- `origin` 默认值 `"unknown"`（未配置模板路径时），语义准确
- 筛选逻辑：先按 origin 过滤，再按 source_group 分组

**Metis 识别并锁定的防护栏**：
- ❌ 不做修改检测（mtime 对比）——即使文件内容被修改，只要路径在模板中存在，就标记为 `"template"`
- ❌ 不触碰 `TemplateDataPayload`、`WatchedCollector`、`template-update` 事件
- ❌ 不改变 `MemoryFileTree` 现有的 `source_group` 分组层级结构
- ❌ 不跨项目聚合统计数据
- ❌ 不支持多模板路径（仅一个全局模板）

---

## 工作目标

### 核心目标
在 ptv"静态记忆"面板中增加文件来源标注和筛选能力，使用户能一目了然哪些记忆文件是模板继承的，哪些是项目特有的。

### 具体交付物
- `src-tauri/src/collectors/template/template_fingerprint.rs` — 模板文件路径指纹模块
- `src-tauri/src/commands.rs` — 新增 `set_template_path`、`get_template_path` 命令；扩展 `get_project_files` 计算 origin
- `src-tauri/src/lib.rs` — 注册新命令
- `src/pages/Settings.tsx` — 新增模板路径配置区域
- `src/components/MemoryFileTree.tsx` — 新增来源筛选控件和视觉标记
- `src/components/ProjectMemoryPanel.tsx` — 筛选逻辑集成
- `e2e/template-origin.spec.ts` — E2E 测试

### 完成定义
- [ ] `cargo test -p ptv` 全部通过（含新 TemplateFingerprint 单测）
- [ ] `npm run build` 通过
- [ ] Playwright E2E 测试全部通过
- [ ] 配置有效模板路径后，CLAUDE.md 标注为"模板"，AGENTS.md 标注为"项目"
- [ ] 模板路径为空时筛选控件降级正常

### 必须包含
- Settings 页面模板路径配置（文本输入 + 文件夹选择按钮）
- `set_template_path` / `get_template_path` Tauri 命令
- `AppState` 中模板路径 + fingerprint 缓存
- `get_project_files` 自动计算 `origin` 字段
- `MemoryFileTree` 来源筛选下拉（全部 / 模板记忆 / 项目记忆）
- 文件项来源视觉标记（圆点 + 文字标签）
- 筛选切换时空状态差异化文案

### 必须不包含（护栏）
- ❌ 模板文件修改检测（内容/mtime 对比）
- ❌ 批量操作（按 origin 删除/导出）
- ❌ Dashboard 统计（各项目模板/项目文件比例）
- ❌ 多模板路径
- ❌ `project_files` 加入 `template-update` 事件
- ❌ 触碰 `TemplateDataPayload`、`WatchedCollector`
- ❌ 修改 `MemoryFileTree` 现有 `source_group` 分组结构

---

## 验证策略

> **零人工干预** — 所有验证均由 agent 执行。

### 测试决策
- **基础设施存在**：YES（Rust 端 `#[cfg(test)]` 单元测试 + Playwright E2E）
- **自动化测试**：Rust 端新增单元测试 + Playwright E2E
- **框架**：Rust: `cargo test`；前端: Playwright

### QA 策略
每个任务包含 Agent-Executed QA Scenarios：
- **Rust 任务**：`cargo test` 验证新模块和命令
- **前端任务**：Playwright 操作浏览器，验证 UI 行为
- **E2E 任务**：Playwright 覆盖完整用户流程

---

## 执行策略

### 并行执行 Waves

```
Wave 1（立即开始 — Rust 后端基础）：
├── Task 1: TemplateFingerprint 模块 + settings.json 持久化 [quick]
└── Task 2: AppState 扩展 + set/get_template_path 命令 [quick]

Wave 2（依赖 Wave 1 — Rust 集成）：
├── Task 3: get_project_files 增强 origin 计算 [deep]
└── Task 4: Rust 单元测试 [quick]

Wave 3（依赖 Wave 1-2 — 前端设置）：
└── Task 5: Settings 模板路径配置 UI [quick]

Wave 4（依赖 Wave 1-2 — 前端核心，MAX PARALLEL）：
├── Task 6: TypeScript SerProjectFile 接口更新 [quick]
├── Task 7: MemoryFileTree 筛选控件 + 视觉标记 [visual-engineering]
└── Task 8: ProjectMemoryPanel 筛选逻辑集成 [quick]

Wave 5（依赖 Wave 3-4 — E2E 测试）：
└── Task 9: Playwright E2E 测试 [visual-engineering]

Wave FINAL（所有任务完成后 — 4 个并行审查，等待用户确认）：
├── Task F1: 计划合规审计 (oracle)
├── Task F2: 代码质量审查 (unspecified-high)
├── Task F3: 手动 QA (unspecified-high + playwright)
└── Task F4: 范围保真度检查 (deep)
→ 展示结果 → 获取用户明确"okay"

关键路径：Task 1 → Task 3 → Task 7 → Task 9 → F1-F4 → 用户确认
最大并行数：3（Wave 4）
```

### 依赖矩阵

- **1**: - - 3, 4, 2
- **2**: - - 3, 5, 2
- **3**: 1, 2 - 6, 7, 8, 3
- **4**: 1 - 9, 2
- **5**: 2 - 9, 2
- **6**: 3 - 7, 3
- **7**: 3, 6 - 9, 3
- **8**: 3 - 9, 3
- **9**: 4, 5, 7, 8 - F1-F4, 4

### Agent 调度概要

- **1**: **2** - T1 → quick, T2 → quick
- **2**: **2** - T3 → deep, T4 → quick
- **3**: **1** - T5 → quick
- **4**: **3** - T6 → quick, T7 → visual-engineering, T8 → quick
- **5**: **1** - T9 → visual-engineering
- **FINAL**: **4** - F1 → oracle, F2 → unspecified-high, F3 → unspecified-high, F4 → deep

---

## TODOs

> 实现 + 测试 = 一个任务。绝不拆分。
> 每个任务必须有：推荐 Agent Profile + 并行化信息 + QA Scenarios。
> **缺少 QA Scenarios 的任务不完整。没有例外。**

- [ ] 1. TemplateFingerprint 模块 + settings.json 持久化

  **要做什么**：
  - 创建 `src-tauri/src/collectors/template/template_fingerprint.rs`
  - 定义 `TemplateFingerprint` struct：包含 `paths: HashSet<String>`（模板文件相对路径集合）
  - 实现 `TemplateFingerprint::build(template_path: &Path) -> Result<Self>`：扫描模板项目白名单目录，构建相对路径集合。复用 `ProjectFilesCollector` 的扫描逻辑（只取 `relative_path`，不读文件内容）
  - 实现 `settings.json` 读写：路径 `{data_local_dir}/ptv/settings.json`，JSON 格式 `{ "template_path": "/path/to/template" }`
  - 实现 `load_template_path(data_dir: &Path) -> Option<PathBuf>` 和 `save_template_path(data_dir: &Path, path: &Path) -> Result<()>`
  - settings.json 读写出错时降级（返回 None / 打印警告），不崩溃
  - 在 `collectors/template/mod.rs` 中注册新模块

  **必须不做**：
  - 不修改 `ProjectFilesCollector` 的扫描逻辑
  - 不在 fingerprint 中存储文件内容（仅路径）
  - 不创建额外的依赖（只用 std）

  **推荐 Agent Profile**：
  - **Category**：`quick` — 单一模块，纯 Rust，无跨语言交互
  - **Skills**：不需要
  - **理由**：逻辑集中在单个新文件，参照现有 `project_files.rs` 和 `registry.rs` 的模式即可

  **并行化**：
  - **可并行**：YES（与 Task 2 不冲突）
  - **并行组**：Wave 1
  - **阻塞**：Task 3, Task 4
  - **被阻塞**：无（可立即开始）

  **参考资料**：
  - `src-tauri/src/collectors/template/project_files.rs:33-85` — 白名单条目定义（whitelist_entries 函数），fingerprint 扫描需复用相同目录列表
  - `src-tauri/src/registry.rs:22-50` — ProjectRegistry 的 JSON 读写模式（serde + 文件读写），settings.json 同理
  - `src-tauri/src/collectors/template/mod.rs:1-17` — 模块导出模式，新模块需在此注册

  **验收标准**：

  **QA Scenarios**：
  ```
  Scenario: 从模板项目构建 fingerprint 成功
    Tool: Bash (cargo test)
    Preconditions: 存在 /Users/ckstar/Repo/ai_project_template 模板项目
    Steps:
      1. 在单元测试中创建临时目录，模拟模板项目结构（含 CLAUDE.md、.claude/rules/00-core.md、docs/design/arch.md）
      2. 调用 TemplateFingerprint::build(&temp_dir)
      3. 断言 paths 集合包含 "CLAUDE.md"、".claude/rules/00-core.md"、"docs/design/arch.md"
      4. 断言临时目录根目录的 README.md 不在集合中（非白名单文件）
    Expected Result: paths.len() >= 3，包含白名单文件，不包含非白名单文件
    Evidence: .sisyphus/evidence/task-1-fingerprint-build.txt (cargo test 输出)

  Scenario: 模板路径不存在时 settings 读取出错降级
    Tool: Bash (cargo test)
    Preconditions: data_dir 指向不存在的路径
    Steps:
      1. 调用 load_template_path(data_dir)
      2. 断言返回 None（文件不存在时降级）
    Expected Result: 返回 None，不 panic，不创建空文件
    Evidence: .sisyphus/evidence/task-1-settings-degrade.txt
  ```

  **Evidence 捕获**：
  - [ ] `task-1-fingerprint-build.txt` — 构建测试输出
  - [ ] `task-1-settings-degrade.txt` — 降级测试输出

  **Commit**：YES（组 1）
  - Message：`feat(backend): 新增 TemplateFingerprint 模块与 settings.json 持久化`
  - Files：`src-tauri/src/collectors/template/template_fingerprint.rs`, `src-tauri/src/collectors/template/mod.rs`

---

- [ ] 2. AppState 扩展 + set/get_template_path 命令

  **要做什么**：
  - 在 `AppState` (commands.rs:451) 中新增两个字段：
    - `template_path: Mutex<Option<PathBuf>>` — 当前模板路径
    - `template_fingerprint: Mutex<Option<TemplateFingerprintCache>>` — 缓存的 fingerprint
  - 定义 `TemplateFingerprintCache` struct：`paths: HashSet<String>` + `generated_at: Instant`（用于未来缓存刷新）
  - 实现 `#[tauri::command] fn set_template_path(path: String, app_handle: AppHandle, state: State<'_, AppState>) -> Result<(), String>`：
    1. 验证路径存在且是目录
    2. 调用 `TemplateFingerprint::build(&path)` 构建 fingerprint
    3. 更新 `state.template_path`
    4. 更新 `state.template_fingerprint`
    5. 调用 `save_template_path` 持久化到 settings.json
  - 实现 `#[tauri::command] fn get_template_path(state: State<'_, AppState>) -> Result<Option<String>, String>`：
    1. 从 AppState 返回当前模板路径
  - 在 `init_app_state` (commands.rs:771) 中：启动时从 settings.json 加载模板路径并构建 fingerprint
  - 在 `lib.rs` 中注册 `set_template_path` 和 `get_template_path` 命令

  **必须不做**：
  - 不在 `set_template_path` 中做文件内容对比
  - AppState 初始化失败时不阻塞应用启动（打印警告，继续运行）

  **推荐 Agent Profile**：
  - **Category**：`quick` — 在现有 commands.rs 基础上追加，模式明确
  - **Skills**：不需要
  - **理由**：参照现有 `add_project`/`list_projects` 等命令的模式，在 AppState 追加字段和 Mutex

  **并行化**：
  - **可并行**：YES（与 Task 1 不冲突，但共享 AppState 类型）
  - **并行组**：Wave 1
  - **阻塞**：Task 3, Task 5
  - **被阻塞**：无（可立即开始）

  **参考资料**：
  - `src-tauri/src/commands.rs:451-465` — AppState 现有定义，在此追加字段
  - `src-tauri/src/commands.rs:467-482` — `add_project` 命令模式（State 使用 + 路径验证 + 返回 Result）
  - `src-tauri/src/commands.rs:771-778` — `init_app_state` 函数，在此添加启动加载逻辑
  - `src-tauri/src/lib.rs:39-54` — 命令注册位置

  **验收标准**：
  - [ ] `cargo check` 无错误
  - [ ] `set_template_path("/Users/ckstar/Repo/ai_project_template")` 返回 Ok
  - [ ] `get_template_path()` 返回 Some 且路径正确
  - [ ] `set_template_path("/nonexistent/path")` 返回 Err

  **QA Scenarios**：
  ```
  Scenario: 设置有效模板路径并读取
    Tool: Bash (cargo test)
    Preconditions: /Users/ckstar/Repo/ai_project_template 存在
    Steps:
      1. 在测试中创建 AppState，调用 set_template_path("/Users/ckstar/Repo/ai_project_template")
      2. 断言返回 Ok(())
      3. 调用 get_template_path()
      4. 断言返回 Some，路径以 "ai_project_template" 结尾
      5. 检查 settings.json 文件存在且包含正确路径
    Expected Result: 模板路径正确保存到 AppState 和 settings.json
    Failure Indicators: 返回 Err，settings.json 未创建或内容错误
    Evidence: .sisyphus/evidence/task-2-set-get-path.txt

  Scenario: 设置无效模板路径返回错误
    Tool: Bash (cargo test)
    Preconditions: /nonexistent/path 不存在
    Steps:
      1. 调用 set_template_path("/nonexistent/path")
      2. 断言返回 Err，错误信息包含"路径"
    Expected Result: 返回错误信息，不更新 AppState，不写入 settings.json
    Evidence: .sisyphus/evidence/task-2-invalid-path.txt
  ```

  **Evidence 捕获**：
  - [ ] `task-2-set-get-path.txt`
  - [ ] `task-2-invalid-path.txt`

  **Commit**：YES（组 1）
  - Message：`feat(backend): AppState 扩展模板路径与 set/get_template_path 命令`
  - Files：`src-tauri/src/commands.rs`, `src-tauri/src/lib.rs`

---

- [ ] 3. get_project_files 增强 origin 计算

  **要做什么**：
  - 修改 `get_project_files` 命令签名：从 `fn get_project_files(path: String)` 改为 `fn get_project_files(path: String, state: State<'_, AppState>)`
  - 在命令函数中：
    1. 从 `state.template_fingerprint` 读取缓存的 fingerprint（`Mutex<Option<TemplateFingerprintCache>>`）
    2. 调用 `ProjectFilesCollector::collect()` 获取文件列表（已有逻辑）
    3. 遍历每个文件：检查 `file.relative_path` 是否在 fingerprint.paths 中 → `origin = "template"`；否则 → `origin = "project"`；如果 fingerprint 为 None → `origin = "unknown"`
  - 修改 `SerProjectFile` (commands.rs:217)：新增 `origin: String` 字段
  - 修改 `ProjectFile` (project_files.rs:93)：新增 `origin: String` 字段，默认值 `"unknown"`
  - 修改 `From<ProjectFile> for SerProjectFile`：传递 `origin` 字段
  - 确保向后兼容：`origin` 字段在 JSON 序列化中始终存在

  **必须不做**：
  - 不读取文件内容做对比（仅路径匹配）
  - 不修改 `ProjectFilesCollector::collect()` 的逻辑
  - 不在 `get_project_files` 中触发 fingerprint 重建（fingerprint 由 `set_template_path` 管理）

  **推荐 Agent Profile**：
  - **Category**：`deep` — 涉及修改命令签名（会影响前端调用）、数据模型扩展、fingerprint 查询，需要仔细确保不破坏现有逻辑
  - **Skills**：不需要
  - **理由**：改动分散在 commands.rs + project_files.rs 两处，需要确保数据流传递正确

  **并行化**：
  - **可并行**：NO（依赖 Task 1 和 Task 2 完成）
  - **阻塞**：Task 4, Task 6, Task 7, Task 8
  - **被阻塞**：Task 1, Task 2

  **参考资料**：
  - `src-tauri/src/commands.rs:350-361` — 当前 `get_project_files` 实现
  - `src-tauri/src/commands.rs:216-243` — `SerProjectFile` 定义和 `From<ProjectFile>` 实现
  - `src-tauri/src/collectors/template/project_files.rs:92-103` — `ProjectFile` struct 定义
  - `src-tauri/src/commands.rs:467-482` — `add_project` 的 `State<'_, AppState>` 参数模式

  **验收标准**：
  - [ ] `cargo check` 无错误
  - [ ] 配置模板路径后，`get_project_files` 返回的 `SerProjectFile` 包含正确的 `origin` 值
  - [ ] 模板文件（CLAUDE.md）的 origin 为 `"template"`
  - [ ] 业务特有文件（AGENTS.md）的 origin 为 `"project"`
  - [ ] 未配置模板路径时，所有文件 origin 为 `"unknown"`

  **QA Scenarios**：
  ```
  Scenario: 配置模板路径后正确标注文件来源
    Tool: Bash (cargo test)
    Preconditions: 测试中创建模拟模板项目（含 CLAUDE.md）和模拟业务项目（含 CLAUDE.md + AGENTS.md）
    Steps:
      1. 调用 set_template_path 设置模板路径
      2. 调用 get_project_files 获取业务项目文件
      3. 找到 CLAUDE.md → 断言 origin == "template"
      4. 找到 AGENTS.md → 断言 origin == "project"
    Expected Result: 模板文件标注为 "template"，业务特有文件标注为 "project"
    Evidence: .sisyphus/evidence/task-3-origin-labeling.txt

  Scenario: 未配置模板路径时 origin 为 unknown
    Tool: Bash (cargo test)
    Preconditions: AppState 中 template_fingerprint 为 None
    Steps:
      1. 不设置模板路径
      2. 调用 get_project_files
      3. 断言所有文件的 origin 均为 "unknown"
    Expected Result: 所有文件 origin == "unknown"
    Evidence: .sisyphus/evidence/task-3-unknown-origin.txt
  ```

  **Evidence 捕获**：
  - [ ] `task-3-origin-labeling.txt`
  - [ ] `task-3-unknown-origin.txt`

  **Commit**：YES（组 2）
  - Message：`feat(backend): get_project_files 增强 origin 计算`
  - Files：`src-tauri/src/commands.rs`, `src-tauri/src/collectors/template/project_files.rs`

- [ ] 4. Rust 单元测试

  **要做什么**：
  - 在 `template_fingerprint.rs` 中追加 `#[cfg(test)]` 测试模块
  - 测试 `TemplateFingerprint::build()`：含白名单文件的目录、空目录、部分白名单目录缺失
  - 测试 `settings.json` 序列化/反序列化：往返一致性
  - 测试 `settings.json` 文件损坏时降级：返回 None，不 panic
  - 运行 `cargo test -p ptv` 确保全部通过

  **必须不做**：
  - 不在测试中硬编码绝对路径（使用 tempfile crate）
  - 不测试 `get_project_files` 的网络/SSH 路径（仅本地路径）

  **推荐 Agent Profile**：
  - **Category**：`quick` — 纯测试，参照现有 test 模式
  - **Skills**：不需要

  **并行化**：
  - **可并行**：NO（依赖 Task 1 和 Task 3 完成）
  - **并行组**：Wave 2
  - **阻塞**：Task 9
  - **被阻塞**：Task 1, Task 3

  **参考资料**：
  - `src-tauri/src/collectors/template/project_files.rs:395-621` — 现有测试模式

  **验收标准**：
  - [ ] `cargo test -p ptv` 全部通过

  **QA Scenarios**：
  ```
  Scenario: cargo test 全部通过
    Tool: Bash
    Steps:
      1. 运行 cargo test -p ptv -- --nocapture
      2. 断言 test result: ok. N passed; 0 failed
    Expected Result: 所有测试通过
    Evidence: .sisyphus/evidence/task-4-cargo-test.txt
  ```

  **Evidence 捕获**：
  - [ ] `task-4-cargo-test.txt`

  **Commit**：YES（组 2）
  - Message：`test(backend): TemplateFingerprint 与 origin 计算单元测试`
  - Files：`src-tauri/src/collectors/template/template_fingerprint.rs`

---

- [ ] 5. Settings 模板路径配置 UI

  **要做什么**：
  - 在 `Settings.tsx` 中新增"模板路径"配置卡片区域
  - UI 结构：Card 容器内包含标题行 + 输入行（文本输入框 + "选择文件夹"按钮）+ 保存按钮
  - 使用 `invoke<string>("get_template_path")` 加载当前路径
  - 使用 `invoke<void>("set_template_path", { path })` 保存新路径
  - 路径为空时显示"未配置模板路径"占位提示
  - 保存成功/失败时显示状态提示

  **必须不做**：
  - 不修改现有 Settings 的项目管理、主题、字号区域逻辑

  **推荐 Agent Profile**：
  - **Category**：`quick` — 在现有 Settings 页面追加一个 Card 区域
  - **Skills**：不需要

  **并行化**：
  - **可并行**：NO（依赖 Task 2）
  - **并行组**：Wave 3
  - **阻塞**：Task 9
  - **被阻塞**：Task 2

  **参考资料**：
  - `src/pages/Settings.tsx` — 现有 Settings 页面结构
  - `src/hooks/useTauri.ts` — invoke 和 listen 的使用方式

  **验收标准**：
  - [ ] Settings 页面显示"模板路径"配置卡片
  - [ ] 可以查看和修改模板路径

  **QA Scenarios**：
  ```
  Scenario: Settings 模板路径配置流程
    Tool: Playwright
    Preconditions: 应用已打开，导航到 Settings
    Steps:
      1. 定位"模板项目路径"相关文本
      2. 输入无效路径，点击保存 → 断言出现错误提示
      3. 输入 /Users/ckstar/Repo/ai_project_template，点击保存 → 断言出现成功提示
    Expected Result: 无效路径错误，有效路径成功
    Evidence: .sisyphus/evidence/task-5-settings-flow.png
  ```

  **Evidence 捕获**：
  - [ ] `task-5-settings-flow.png`

  **Commit**：YES（组 3）
  - Message：`feat(frontend): Settings 模板路径配置 UI`
  - Files：`src/pages/Settings.tsx`

---

- [ ] 6. MemoryFileTree 筛选控件 + 来源视觉标记

  **要做什么**：
  - 在 `MemoryFileTree.tsx` 的 `SerProjectFile` 接口中新增 `origin?: string` 字段
  - 在组件顶部添加来源筛选下拉框，选项：`全部 | 模板记忆 | 项目记忆`
  - 筛选逻辑：先按 `origin` 过滤，再按 `source_group` 分组。筛选后某组为空时折叠或显示"(空)"提示
  - 在每个文件项的名前添加来源标记：灰色圆点 ● + "模板"文字（template），蓝色圆点 ● + "项目"文字（project），灰色圆点 + "未知"（unknown）
  - `origin` 为 undefined 时视为 `"unknown"` 处理
  - 类型更新后 `npm run build` 通过

  **必须不做**：
  - 不改变现有 `source_group` 分组结构和排序逻辑
  - 不改变现有"变更红点"功能

  **推荐 Agent Profile**：
  - **Category**：`visual-engineering` — 涉及 UI 交互（筛选下拉 + 视觉标记），需要用户体验感知
  - **Skills**：不需要

  **并行化**：
  - **可并行**：YES（与 Task 7 并行）
  - **并行组**：Wave 4
  - **阻塞**：Task 8, Task 9
  - **被阻塞**：Task 3

  **参考资料**：
  - `src/components/MemoryFileTree.tsx:7` — `SOURCE_GROUP_ORDER` 常量，筛选重置后保持此顺序
  - `src/components/MemoryFileTree.tsx:11-17` — `SerProjectFile` 接口
  - `src/components/MemoryFileTree.tsx:118-136` — `MemoryFileTreeItem` 现有结构

  **验收标准**：
  - [ ] `npm run build` 通过
  - [ ] 筛选下拉显示三个选项，默认选中"全部"
  - [ ] 选择"模板记忆"时仅显示 origin === "template" 的文件
  - [ ] 选择"项目记忆"时仅显示 origin === "project" 的文件
  - [ ] 文件项显示来源标签

  **QA Scenarios**：
  ```
  Scenario: 三级筛选正常工作
    Tool: Playwright
    Preconditions: 已配置有效模板路径，打开某项目的静态记忆面板
    Steps:
      1. 检查筛选下拉默认值为"全部"
      2. 断言 CLAUDE.md 和 AGENTS.md 都可见
      3. 选择"模板记忆"
      4. 断言 CLAUDE.md 可见，AGENTS.md 不可见
      5. 选择"项目记忆"
      6. 断言 CLAUDE.md 不可见，AGENTS.md 可见
    Expected Result: 筛选后仅显示对应来源的文件
    Evidence: .sisyphus/evidence/task-6-filter-all.png, .sisyphus/evidence/task-6-filter-template.png, .sisyphus/evidence/task-6-filter-project.png
  ```

  **Evidence 捕获**：
  - [ ] `task-6-filter-all.png` / `task-6-filter-template.png` / `task-6-filter-project.png`

  **Commit**：YES（组 4）
  - Message：`feat(frontend): MemoryFileTree 来源筛选与视觉标记`
  - Files：`src/components/MemoryFileTree.tsx`

---

- [ ] 7. ProjectMemoryPanel 筛选逻辑集成

  **要做什么**：
  - 在 `ProjectMemoryPanel.tsx` 中确保文件列表状态 (`files`) 与 `MemoryFileTree` 的筛选参数正确通信
  - 当筛选改变时，如果当前选中的文件不在筛选结果中，自动选择第一个可见文件或清空选择
  - `origin` 为 undefined 时做降级处理
  - 确保 `changedPaths` 相关的变更红点逻辑在筛选后仍正常工作

  **必须不做**：
  - 不修改 L2（对话搜索）、L3（记忆标记）面板逻辑

  **推荐 Agent Profile**：
  - **Category**：`quick` — 集成层，对接已完成的 MemoryFileTree
  - **Skills**：不需要

  **并行化**：
  - **可并行**：YES（与 Task 6 并行）
  - **并行组**：Wave 4
  - **阻塞**：Task 9
  - **被阻塞**：Task 3

  **参考资料**：
  - `src/components/ProjectMemoryPanel.tsx:18-27` — 组件状态管理
  - `src/components/ProjectMemoryPanel.tsx:133-155` — `handleMarkMemory` 逻辑

  **验收标准**：
  - [ ] 筛选后选中文件同步正确
  - [ ] `npm run build` 通过

  **QA Scenarios**：
  ```
  Scenario: 筛选后选中文件状态正确
    Tool: Playwright
    Steps:
      1. 打开项目静态记忆面板，选中 CLAUDE.md（模板文件）
      2. 切换到"项目记忆"筛选
      3. 断言 CLAUDE.md 不再被选中，内容区域更新为第一个项目文件或显示空状态
    Expected Result: 筛选切换时选中状态正确更新
    Evidence: .sisyphus/evidence/task-7-filter-selection.png
  ```

  **Evidence 捕获**：
  - [ ] `task-7-filter-selection.png`

  **Commit**：YES（组 4）
  - Message：`feat(frontend): ProjectMemoryPanel 筛选逻辑集成`
  - Files：`src/components/ProjectMemoryPanel.tsx`

---

- [ ] 8. Playwright E2E 测试

  **要做什么**：
  - 创建 `e2e/template-origin.spec.ts`
  - 测试场景 1（未配置模板路径）：打开静态记忆 → 筛选下拉显示"未配置模板"或不可用 → 所有文件可见
  - 测试场景 2（有效模板路径）：配置路径后 → 筛选生效 → CLAUDE.md 标注"模板"，AGENTS.md 标注"项目"
  - 测试场景 3（切换项目）：在不同项目间切换 → 筛选状态正确保持或重置
  - 测试场景 4（边缘情况）：空项目（无记忆文件）→ 空状态文案正确

  **必须不做**：
  - 不测试 L2/L3 面板功能

  **推荐 Agent Profile**：
  - **Category**：`visual-engineering` — Playwright 浏览器自动化
  - **Skills**：`playwright`

  **并行化**：
  - **可并行**：NO（依赖 Task 5, Task 6, Task 7 全部完成）
  - **并行组**：Wave 5
  - **阻塞**：无
  - **被阻塞**：Task 4, Task 5, Task 6, Task 7

  **参考资料**：
  - `e2e/` 目录下现有测试文件 — 测试模式参考
  - `playwright.config.ts` — Playwright 配置

  **验收标准**：
  - [ ] `npx playwright test e2e/template-origin.spec.ts` 全部通过

  **QA Scenarios**：
  ```
  Scenario: 完整 E2E 流程
    Tool: Playwright
    Steps:
      1. 测试未配置模板路径时的降级行为
      2. 测试配置有效路径后的筛选功能
      3. 测试切换项目后筛选状态
    Expected Result: 所有场景通过
    Evidence: .sisyphus/evidence/task-8-e2e-output.txt
  ```

  **Evidence 捕获**：
  - [ ] `task-8-e2e-output.txt`

  **Commit**：YES（组 5）
  - Message：`test(e2e): 模板来源区分 E2E 测试`
  - Files：`e2e/template-origin.spec.ts`

---

## 最终验证 Wave

> 4 个审查 agent 并行运行。全部必须 APPROVE。向用户展示合并结果并获取明确"okay"后才能标记完成。
> **F1-F4 完成前不主动标记。被拒绝或用户反馈 → 修复 → 重新运行 → 再次展示 → 等待确认。**

- [ ] F1. **计划合规审计** — `oracle`

  通读计划。对每个"必须包含"：验证实现存在（读文件、curl 端点、运行命令）。对每个"必须不包含"：搜索代码库中的禁止模式 — 发现即拒绝并指明 file:line。检查 `.sisyphus/evidence/` 中的 evidence 文件是否存在。对比交付物与计划。
  输出：`必须包含 [N/N] | 必须不包含 [N/N] | 任务 [N/N] | 判定: APPROVE/REJECT`

- [ ] F2. **代码质量审查** — `unspecified-high`

  运行 `cargo check` + `cargo test` + `npm run build`。审查所有变更文件：`as any`/`@ts-ignore`、空 catch、console.log、注释掉的代码、未使用的导入。检查 AI slop：过度注释、过度抽象、通用命名（data/result/item/temp）。
  输出：`构建 [PASS/FAIL] | 测试 [N pass/N fail] | 文件 [N clean/N issues] | 判定`

- [ ] F3. **手动 QA** — `unspecified-high`（+ `playwright` skill）

  从干净状态开始。执行每个任务的每个 QA 场景 — 严格按步骤执行，捕获 evidence。测试跨任务集成（各功能协同工作，而非孤立）。测试边缘情况：空状态、无效输入、快速操作。保存到 `.sisyphus/evidence/final-qa/`。
  输出：`场景 [N/N pass] | 集成 [N/N] | 边缘情况 [N tested] | 判定`

- [ ] F4. **范围保真度检查** — `deep`

  对每个任务：阅读"要做什么"，阅读实际 diff（git log/diff）。验证 1:1 — 规范中的所有内容都已实现（无缺失），没有超出规范的内容（无蔓延）。检查"必须不包含"合规性。检测跨任务污染：任务 N 触及任务 M 的文件。标记未计入的变更。
  输出：`任务 [N/N compliant] | 污染 [CLEAN/N issues] | 未计入 [CLEAN/N files] | 判定`

---

## 提交策略

- **1**: `feat(backend): 新增 TemplateFingerprint 模块与 settings.json 持久化` — `src-tauri/src/collectors/template/template_fingerprint.rs`
- **2**: `feat(backend): AppState 扩展模板路径与 set/get_template_path 命令` — `src-tauri/src/commands.rs, src-tauri/src/lib.rs`
- **3**: `feat(backend): get_project_files 增强 origin 计算` — `src-tauri/src/commands.rs`
- **4**: `test(backend): TemplateFingerprint 与 origin 计算单元测试` — `src-tauri/src/collectors/template/template_fingerprint.rs`
- **5**: `feat(frontend): Settings 模板路径配置 UI` — `src/pages/Settings.tsx`
- **6**: `feat(frontend): TypeScript SerProjectFile 接口新增 origin 字段` — `src/components/MemoryFileTree.tsx`
- **7**: `feat(frontend): MemoryFileTree 来源筛选与视觉标记` — `src/components/MemoryFileTree.tsx`
- **8**: `feat(frontend): ProjectMemoryPanel 筛选逻辑集成` — `src/components/ProjectMemoryPanel.tsx`
- **9**: `test(e2e): 模板来源区分 E2E 测试` — `e2e/template-origin.spec.ts`

---

## 成功标准

### 验证命令
```bash
cargo test -p ptv                           # Rust 单测全部通过
npm run build                               # 前端构建成功
npx playwright test e2e/template-origin.spec.ts  # E2E 全部通过
```

### 最终检查清单
- [ ] 所有"必须包含"已实现
- [ ] 所有"必须不包含"已排除
- [ ] Rust 单测通过
- [ ] 前端构建通过
- [ ] Playwright E2E 通过
- [ ] 用户明确确认
