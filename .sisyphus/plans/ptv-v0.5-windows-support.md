# ptv v0.5 — Windows 平台支持

## TL;DR

> **快速摘要**：为 PTV 增加 Windows 10+ 平台支持。主要改动：Python3 命令自动探测（适配 Windows 的 `python`/`py`）、`encode_cwd_path` 条件编译门控（Windows 上 Claude Code 不可用）、约 29 处 Rust 测试 Unix 路径替换为跨平台 `temp_dir()`、NSIS 打包配置。核心依赖 `abtop-collector` 已全面兼容 Windows，无需修改。
>
> **交付物**：
> - Rust 后端：`find_python()` 自动探测函数 + `encode_cwd_path` `#[cfg]` 门控 + 测试路径修复 + 跨平台单元测试
> - 构建配置：NSIS installer 最小化配置
> - 前端：确认无平台问题
> - 文档：README.md 更新平台支持说明
> - E2E 测试：新增 Windows 构建验证脚本
>
> **预估工作量**：小型（Rust 5 任务 + 构建 1 任务 + 文档 1 任务 = 7 任务）
> **并行执行**：YES — 3 个 Wave
> **关键路径**：Task 1 → Task 2 → Task 3 → Task 5 → F1-F4

---

## 上下文

### 原始需求
用户希望 PTV 桌面应用支持 Windows 平台。当前仅支持 macOS 和 Linux（AGENTS.md 明确标注 "Windows 不做"）。

### 调研总结
**关键发现**：
- **`abtop-collector` 已全面兼容 Windows**：所有平台相关函数（进程发现、端口发现、会话映射）均有 `#[cfg(target_os = "windows")]` 实现，使用 `sysinfo::System` 和 `netstat`。无需任何修改。
- **Tauri v2 官方支持 Windows**：NSIS installer 可从 macOS 交叉编译（使用 `cargo check --target x86_64-pc-windows-msvc` 验证）；WebView2 Runtime 在 Windows 11/较新 Win10 上预装。
- **ptv 自身仅 2 个实际问题**：`python3` 命令硬编码（config.rs）+ `encode_cwd_path` Unix 路径处理（session_transcript.rs）+ 29 处测试硬编码 Unix 路径。
- **前端代码**：无平台特定代码，Web 技术天然跨平台。

**用户决策**：
- Python：自动探测（`python3` → `python` → `py`），运行时发现
- Installer：仅 NSIS（`.exe`），不做 MSI
- encode_cwd_path：`#[cfg(not(windows))]` 条件编译门控
- 测试路径：本次一并修复，使用 `temp_dir()` 或 `tempdir()`
- 测试策略：包含自动化测试（Rust 单元测试 + 跨平台编译验证）
- 版本：0.5.0（v0.3 和 v0.4 已实现）

### Metis 审查要点
**已识别的缺口**：
- 跨平台编译可行性：需验证 `cargo check --target x86_64-pc-windows-msvc` 在 macOS 上是否能通过
- `sessions_dir()` 依赖门控后的 `encode_cwd_path`，需提供 Windows 回退
- Python 检测需平台感知（Unix: `python3`→`python`，Windows: `python`→`py`）
- 29 处测试路径（不是最初估计的 25 处），分布在 6 个文件中
- `ParameterError::PythonNotFound` 错误消息写死 "Python3"，应改为通用 "Python"
- 测试路径语义正确性问题：`Path::new("/tmp/x")` 在 Windows 上变成相对路径

**Metis 锁定的防护栏**：
- ❌ 不做 MSI installer（`wix` 配置）
- ❌ 不做 WebView2 捆绑（假设系统已安装）
- ❌ 不做 CI/CD pipeline 改造（本任务仅验证交叉编译，CI 另开任务）
- ❌ 不做 Windows 特有功能（通知、托盘、自启动）
- ❌ 不做 Python venv 探测（仅系统 Python）
- ❌ 不修改 `abtop-collector/` 任何文件
- ❌ 不重构非 Windows 相关代码
- ❌ 不升级任何依赖版本

---

## 工作目标

### 核心目标
使 PTV 的代码和构建配置兼容 Windows 10+ 平台，能够通过交叉编译检查，为后续 Windows 上的实际构建和运行奠定基础。

### 具体交付物
- `src-tauri/src/collectors/template/config.rs` — `find_python()` 辅助函数 + 错误消息修正
- `src-tauri/src/collectors/template/session_transcript.rs` — `encode_cwd_path` + `sessions_dir` `#[cfg]` 门控
- `src-tauri/src/collectors/template/config.rs` — 测试路径修复
- `src-tauri/src/collectors/template/session_transcript.rs` — 测试路径修复
- `src-tauri/src/collectors/template/template_fingerprint.rs` — 测试路径修复
- `src-tauri/src/registry.rs` — 测试路径修复
- `src-tauri/src/watcher.rs` — 测试路径修复
- `src-tauri/src/collectors/agent/mod.rs` — 测试路径修复
- `src-tauri/tauri.conf.json` — NSIS 构建配置
- `README.md` — 平台支持更新
- `AGENTS.md` — Windows 支持声明更新

### 完成定义
- [ ] `cargo test -p ptv` 全部通过（macOS，无回归）
- [ ] `cargo check --target x86_64-pc-windows-msvc` 通过（交叉编译检查）
- [ ] `npm run build` 通过
- [ ] Playwright E2E 全部通过（`npm run test`）
- [ ] Python 自动探测在 macOS/Linux 上行为不变
- [ ] `encode_cwd_path` 在 Windows target 上不编译
- [ ] 所有测试路径使用 `temp_dir()` 或 `tempdir()`

### 必须包含
- `find_python()` 平台感知辅助函数（Unix: python3→python，Windows: python→py）
- `encode_cwd_path` + `sessions_dir` `#[cfg(not(windows))]` 门控
- `sessions_dir()` Windows 回退逻辑（返回 `dirs::home_dir()/.claude/projects/`，即使目录不存在）
- NSIS 最小化配置（`installMode` + `startMenu` shortcut + `displayLanguageSelector`）
- 29 处测试 Unix 路径替换为 `std::env::temp_dir()` 或 `tempfile::tempdir()`
- `ParameterError::PythonNotFound` 错误消息 "Python3" → "Python"
- `find_python()` 单元测试（成功场景 + 失败降级场景）
- `encode_cwd_path` doctest `#[cfg(not(windows))]` 门控
- README.md 平台支持从"Windows（不支持）"改为"Windows（测试中）"
- AGENTS.md 平台声明更新

### 必须不包含（护栏）
- ❌ MSI installer（`wix`）配置
- ❌ WebView2 捆绑配置
- ❌ CI/CD pipeline 改造
- ❌ Windows 特有功能（通知、托盘、自启动）
- ❌ Python venv 探测
- ❌ `abtop-collector/` 修改
- ❌ 非 Windows 相关代码重构
- ❌ 依赖版本升级
- ❌ 全局路径规范化层
- ❌ Tauri plugin 版本更新

---

## 验证策略

> **零人工干预** — 所有验证均由 agent 执行。

### 测试决策
- **基础设施存在**：YES（Rust 端 `#[cfg(test)]` 单元测试 + Playwright E2E）
- **自动化测试**：Rust 端新增单元测试 + 交叉编译验证
- **框架**：Rust: `cargo test`；前端: Playwright；交叉编译: `cargo check --target`

### QA 策略
每个任务包含 Agent-Executed QA Scenarios：
- **Rust 任务**：`cargo test` 验证 + `cargo check --target` 验证
- **构建任务**：`cargo check --target x86_64-pc-windows-msvc` 验证 + Tauri 配置文件 JSON 验证
- **文档任务**：文件内容验证
- **E2E 任务**：Playwright 运行现有测试套件（无回归验证）

---

## 执行策略

### 并行执行 Waves

```
Wave 1（立即开始 — Rust 核心修复）：
├── Task 1: find_python() 自动探测 + 错误消息修正 [quick]
└── Task 2: encode_cwd_path + sessions_dir #[cfg] 门控 [quick]

Wave 2（依赖 Wave 1 — 批量测试路径修复，MAX PARALLEL）：
├── Task 3: 批量测试路径修复 [quick]
└── Task 4: Rust 单元测试（find_python + 跨平台验证）[quick]

Wave 3（依赖 Wave 1-2 — 构建配置 + 文档 + 验证）：
├── Task 5: NSIS 构建配置 + 平台声明更新 [quick]
├── Task 6: 跨平台编译验证 + 回归测试 [quick]
└── Task 7: README.md + AGENTS.md 文档更新 [quick]

Wave FINAL（所有任务完成后 — 4 个并行审查，等待用户确认）：
├── Task F1: 计划合规审计 (oracle)
├── Task F2: 代码质量审查 (unspecified-high)
├── Task F3: 手动 QA (unspecified-high + playwright)
└── Task F4: 范围保真度检查 (deep)
→ 展示结果 → 获取用户明确"okay"

关键路径：Task 1 → Task 3 → Task 5 → Task 6 → F1-F4 → 用户确认
并行加速：~50% 快于顺序执行
最大并行数：3（Wave 3）
```

### 依赖矩阵

- **1**: - - 3, 4, 2
- **2**: - - 3, 4, 2
- **3**: 1, 2 - 5, 6, 7, 3
- **4**: 1, 2 - 6, 3
- **5**: 3 - 6, 3
- **6**: 3, 4, 5 - F1-F4, 4
- **7**: 3 - F1-F4, 4

### Agent 调度概要

- **1**: **2** - T1 → quick, T2 → quick
- **2**: **2** - T3 → quick, T4 → quick
- **3**: **3** - T5 → quick, T6 → quick, T7 → quick
- **FINAL**: **4** - F1 → oracle, F2 → unspecified-high, F3 → unspecified-high, F4 → deep

---

## TODOs

> 实现 + 测试 = 一个任务。绝不拆分。
> 每个任务必须有：推荐 Agent Profile + 并行化信息 + QA Scenarios。
> **缺少 QA Scenarios 的任务不完整。没有例外。**

- [x] 1. find_python() 自动探测 + 错误消息修正

  **要做什么**：
  - 在 `config.rs` 中新增 `find_python()` 辅助函数，平台感知探测：
    - Unix: 依次尝试 `python3 --version` → `python --version`
    - Windows: 依次尝试 `python --version` → `py --version`
    - 任一成功返回其命令名字符串，全部失败返回 `None`
  - 修改 `parse_parameters_py()`：先调用 `find_python()` 获取解释器名，替换两处 `Command::new("python3")`
  - 修改 `ParameterError::PythonNotFound` 的 `Display` 实现：`"未安装 Python3"` → `"未安装 Python"`
  - 修改 config.rs:270,292,313 测试中的 `#!/usr/bin/env python3` shebang
  - 确保探测逻辑可被单元测试注入（使用内部可配置的探测顺序参数）

  **必须不做**：不探测 venv、不修改参数传递逻辑、不修改其他 ParameterError 变体

  **推荐 Agent Profile**：
  - **Category**：`quick` — 单一文件，新增辅助函数 + 替换 2 处调用
  - **Skills**：不需要
  - **理由**：逻辑简单直白，纯 Rust

  **并行化**：
  - **可并行**：YES（与 Task 2 不冲突）
  - **并行组**：Wave 1
  - **阻塞**：Task 3, Task 4
  - **被阻塞**：无

  **参考资料**：
  - `src-tauri/src/collectors/template/config.rs:150-181` — 当前 `parse_parameters_py()` 实现，两处 `Command::new("python3")` 需替换
  - `src-tauri/src/collectors/template/config.rs:60-114` — `ParameterError` 定义和 `Display` 实现
  - `src-tauri/src/collectors/template/config.rs:259-330` — 测试代码，shebang 需替换
  - `abtop-collector/src/collector/process.rs:381-419` — `cmd_has_binary()` Windows 探测模式参考

  **验收标准**：
  - [ ] macOS/Linux 上 `find_python()` 返回 `Some("python3")` 或 `Some("python")`
  - [ ] `parse_parameters_py()` 使用探测结果而非硬编码
  - [ ] 错误消息不再包含 "Python3"
  - [ ] `cargo test -p ptv` 全部通过

  **QA Scenarios**：
  ```
  Scenario: macOS/Linux 上 Python 自动探测成功
    Tool: Bash (cargo test)
    Steps:
      1. 运行 find_python 相关的单元测试
      2. 断言返回有效的 Python 解释器名
      3. 断言该解释器可成功执行 --version
    Expected Result: Some("python3") 或 Some("python")
    Evidence: .sisyphus/evidence/task-1-find-python.txt

  Scenario: 探测失败时优雅降级
    Tool: Bash (cargo test)
    Steps:
      1. 使用模拟的探测逻辑（空搜索路径）
      2. 断言返回 None → parse_parameters_py() 返回 PythonNotFound
    Expected Result: PythonNotFound 错误，消息不含 "Python3"
    Evidence: .sisyphus/evidence/task-1-find-python-none.txt
  ```

  **Evidence**：`task-1-find-python.txt`, `task-1-find-python-none.txt`

  **Commit**：YES（组 1）
  - Message：`feat(backend): 跨平台 Python 自动探测与错误消息修正`
  - Files：`src-tauri/src/collectors/template/config.rs`

---

- [x] 2. encode_cwd_path + sessions_dir #[cfg] 门控

  **要做什么**：
  - 在 `encode_cwd_path` 函数签名前添加 `#[cfg(not(windows))]`（`session_transcript.rs:26`）
  - 给所有 `encode_cwd_path` doctest 添加 `#[cfg(not(windows))]` 门控
  - 修改 `sessions_dir`：`#[cfg(not(windows))]` 分支使用 `encode_cwd_path`，`#[cfg(windows)]` 分支返回 `{home}/.claude/projects/`
  - 确保所有调用 `encode_cwd_path` / `sessions_dir` 的路径兼容双平台编译

  **必须不做**：不改 `encode_cwd_path` 函数体、不实现 Windows 路径编码、不修改核心采集逻辑

  **推荐 Agent Profile**：
  - **Category**：`quick` — 纯条件编译门控
  - **Skills**：不需要

  **并行化**：YES（与 Task 1 不冲突）；Wave 1；阻塞 Task 3, Task 4；被阻塞：无

  **参考资料**：
  - `src-tauri/src/collectors/template/session_transcript.rs:26-34` — 需门控的函数
  - `src-tauri/src/collectors/template/session_transcript.rs:18-25` — 需门控的 doctest
  - `src-tauri/src/collectors/template/session_transcript.rs:682-686` — `sessions_dir` 需 Windows 回退
  - `abtop-collector/src/collector/claude.rs:1720-1741` — `#[cfg(target_os)]` 参考

  **验收标准**：
  - [ ] `cargo check` 通过（macOS）
  - [ ] `encode_cwd_path` 在 Windows target 上不编译
  - [ ] `sessions_dir` Windows 上返回 `{home}/.claude/projects/`
  - [ ] `cargo test -p ptv` 全部通过

  **QA Scenarios**：
  ```
  Scenario: encode_cwd_path Unix 正常 + Windows 编译检查
    Tool: Bash
    Steps:
      1. cargo test -p ptv -- session_transcript
      2. cargo check --target x86_64-pc-windows-msvc 2>&1
      3. 断言两步骤均通过
    Expected Result: Unix 测试通过，Windows target 编译无错误
    Evidence: .sisyphus/evidence/task-2-unix-ok.txt, .sisyphus/evidence/task-2-win-check.txt
  ```

  **Evidence**：`task-2-unix-ok.txt`, `task-2-win-check.txt`

  **Commit**：YES（组 1）
  - Message：`feat(backend): encode_cwd_path 与 sessions_dir Windows 条件编译门控`
  - Files：`src-tauri/src/collectors/template/session_transcript.rs`

---
- [x] 3. 批量测试路径修复

  **要做什么**：
  - 扫描以下文件中所有硬编码的 Unix 路径模式（`/tmp/`、`/Users/`、`/home/`），共约 29 处：
    - `src-tauri/src/collectors/template/config.rs:259` — `Path::new("/tmp/nonexistent_parameters_file.py")`
    - `src-tauri/src/collectors/template/config.rs:247-253` — `temp_dir()` 已正确使用，无需修复
    - `src-tauri/src/collectors/template/config.rs:267,289,310` — 测试中 `dir.join("bad_parameters.py")` 正确
    - `src-tauri/src/collectors/template/session_transcript.rs:885-904` — doctest 中的 `/Users/ckstar/Repo/` 和 `/root`（`#[cfg(not(windows))]` 门控）
    - `src-tauri/src/collectors/template/template_fingerprint.rs:363,377` — `/tmp/nonexistent-ptv-test-path`、`/tmp/my-template`
    - `src-tauri/src/registry.rs:380` — `PathBuf::from("/tmp/__ptv_nonexistent_test_xyz__")`
    - `src-tauri/src/watcher.rs:826-828` — `/tmp/test1`、`/tmp/test2`（仅在 add/remove 测试中，不访问文件系统）
    - `src-tauri/src/collectors/agent/mod.rs:578-587` — `/home/user/project-a`、`/home/user/project-b`
  - 修复策略：
    - 访问文件系统的路径 → 替换为 `std::env::temp_dir()` + `join()`
    - 纯字符串路径标识（如 reader/watcher 测试中的路径）→ 替换为 `temp_dir().join("test1")`
    - `agent/mod.rs` 中的模拟路径数据 → 无需改（测试用 mock 数据，语义无关）
  - 确保所有修复后 `cargo test -p ptv` 通过
  - 确保修复后的测试在语义上跨平台正确

  **必须不做**：
  - 不改 `#[cfg(not(windows))]` 门控的 doctest（已由 Task 2 处理）
  - 不改 mock 数据中的虚构路径（agent/mod.rs 测试数据）
  - 不修改非测试代码（仅 `#[cfg(test)]` 和 doctest 范围）

  **推荐 Agent Profile**：
  - **Category**：`quick` — 机械替换，但需仔细确保每个替换语义正确
  - **Skills**：不需要

  **并行化**：
  - **可并行**：NO（依赖 Task 1 和 2 完成，确保函数签名稳定）
  - **并行组**：Wave 2
  - **阻塞**：Task 5, Task 6
  - **被阻塞**：Task 1, Task 2

  **参考资料**：
  - `src-tauri/src/collectors/template/project_files.rs:395-621` — 已正确使用 `tempfile::tempdir()` 的测试模式
  - `src-tauri/src/collectors/template/git.rs:303-360` — 已正确使用 `tempfile::tempdir()` 的测试模式
  - `src-tauri/src/registry.rs:308-314` — 已正确使用 `std::env::temp_dir()` 的 test_env() 模式（可作为替换目标模式）

  **验收标准**：
  - [ ] `cargo test -p ptv` 全部通过（macOS）
  - [ ] 所有替换后的测试路径不包含 `/tmp/`、`/Users/`、`/home/` 字面量（除已门控的 doctest）
  - [ ] 语义正确：测试行为与修复前一致

  **QA Scenarios**：
  ```
  Scenario: 全部 Rust 测试通过
    Tool: Bash
    Steps:
      1. cargo test -p ptv -- --nocapture 2>&1
      2. 断言 test result: ok. N passed; 0 failed
    Expected Result: 所有测试通过
    Evidence: .sisyphus/evidence/task-3-cargo-test.txt

  Scenario: 无残留硬编码路径
    Tool: Bash
    Steps:
      1. grep -rn '"/tmp/' src-tauri/src/ --include="*.rs" | grep -v 'cfg.*windows'
      2. 断言无匹配（或仅剩已门控的 doctest）
    Expected Result: 无残留硬编码 /tmp/ 路径
    Evidence: .sisyphus/evidence/task-3-no-hardcoded-paths.txt
  ```

  **Evidence**：`task-3-cargo-test.txt`, `task-3-no-hardcoded-paths.txt`

  **Commit**：YES（组 2）
  - Message：`fix(test): 替换硬编码 Unix 路径为跨平台 temp_dir()`
  - Files：上述 6 个文件的测试代码部分

---

- [x] 4. Rust 单元测试（find_python + 跨平台验证）

  **要做什么**：
  - 在 `config.rs` 测试模块中新增 `find_python()` 的单元测试：成功探测、失败降级
  - 测试 `parse_parameters_py()` 在 `find_python()` 返回 `None` 时正确传播 `PythonNotFound` 错误
  - 验证 `sessions_dir` 双平台逻辑（`#[cfg(not(windows))]` 和 `#[cfg(windows)]` 各写测试）
  - 运行 `cargo test -p ptv` 确保全部通过

  **必须不做**：不引入外部依赖、不在 CI 中配置 Windows runner

  **推荐 Agent Profile**：
  - **Category**：`quick` — 在现有测试模块中追加测试
  - **Skills**：不需要

  **并行化**：YES（与 Task 3 不冲突）；Wave 2；阻塞 Task 6；被阻塞：Task 1, Task 2

  **参考资料**：
  - `src-tauri/src/collectors/template/config.rs:259-330` — 现有测试模式

  **验收标准**：
  - [ ] `cargo test -p ptv` 全部通过
  - [ ] 新增 `find_python` 测试 ≥ 2 个
  - [ ] 新增 `sessions_dir` 双平台测试 ≥ 1 个

  **QA Scenarios**：
  ```
  Scenario: 全部测试通过
    Tool: Bash
    Steps:
      1. cargo test -p ptv -- --nocapture 2>&1
      2. 断言 test result: ok. N passed; 0 failed
    Expected Result: 所有测试通过（含新增测试）
    Evidence: .sisyphus/evidence/task-4-test-results.txt
  ```

  **Evidence**：`task-4-test-results.txt`

  **Commit**：YES（组 2）
  - Message：`test(backend): find_python 探测与 sessions_dir 双平台单元测试`
  - Files：`src-tauri/src/collectors/template/config.rs`, `src-tauri/src/collectors/template/session_transcript.rs`

---
- [x] 5. NSIS 构建配置

  **要做什么**：
  - 在 `tauri.conf.json` 的 `bundle` 节中新增 `windows` 配置块：
    ```json
    "windows": {
      "nsis": {
        "installMode": "perMachine",
        "displayLanguageSelector": true
      }
    }
    ```
  - 保留现有 `targets: "all"`（已包含 Windows）
  - 确认 `icon.ico` 文件存在且有效
  - 更新 `AGENTS.md` 中 "平台: macOS + Linux（Windows 不做）" 改为 "平台: macOS + Linux + Windows（测试中）"
  - 在 `src-tauri/Cargo.toml` 中确认无需新增 Windows 特定依赖

  **必须不做**：不添加 MSI/wix 配置、不添加 custom NSIS hooks、不修改 macOS/Linux bundle 配置

  **推荐 Agent Profile**：
  - **Category**：`quick` — JSON 配置 + 文档更新
  - **Skills**：不需要

  **并行化**：
  - **可并行**：NO（依赖 Task 3 确保测试通过后再发布构建配置）
  - **并行组**：Wave 3
  - **阻塞**：Task 6
  - **被阻塞**：Task 3

  **参考资料**：
  - `src-tauri/tauri.conf.json:25-36` — 当前 bundle 配置
  - `AGENTS.md:10-11` — 当前平台声明
  - `src-tauri/icons/icon.ico` — 需确认存在

  **验收标准**：
  - [ ] `tauri.conf.json` 包含 `bundle.windows.nsis` 配置
  - [ ] JSON 格式合法（`python3 -m json.tool tauri.conf.json` 通过）
  - [ ] `AGENTS.md` 平台声明已更新

  **QA Scenarios**：
  ```
  Scenario: tauri.conf.json 格式合法
    Tool: Bash
    Steps:
      1. python3 -m json.tool src-tauri/tauri.conf.json > /dev/null
      2. 断言 exit code = 0
    Expected Result: JSON 格式有效
    Evidence: .sisyphus/evidence/task-5-config-valid.txt
  ```

  **Evidence**：`task-5-config-valid.txt`

  **Commit**：YES（组 3）
  - Message：`build(windows): NSIS 安装包配置与平台声明更新`
  - Files：`src-tauri/tauri.conf.json`, `AGENTS.md`

---

- [x] 6. 跨平台编译验证 + 回归测试

  **要做什么**：
  - 运行 `cargo test -p ptv` 确保全部通过（macOS 回归检查）
  - 运行 `cargo check --target x86_64-pc-windows-msvc` 验证 Windows target 编译
  - 运行 `npm run build` 确保前端构建通过
  - 运行 `npm run test`（Playwright E2E）确保无回归
  - 如果 `x86_64-pc-windows-msvc` target 在 macOS 上不可用，使用 `cargo check --target x86_64-pc-windows-gnu` 作为替代
  - 报告所有结果

  **必须不做**：不在 CI 中配置、不在真实 Windows 上运行（本任务仅验证交叉编译）

  **推荐 Agent Profile**：
  - **Category**：`quick` — 运行验证命令，收集结果
  - **Skills**：不需要

  **并行化**：
  - **可并行**：NO（依赖 Task 3, 4, 5 全部完成）
  - **并行组**：Wave 3（与 Task 5, 7 可并行，但逻辑上在它们之后最佳）
  - **阻塞**：F1-F4
  - **被阻塞**：Task 3, Task 4, Task 5

  **参考资料**：
  - `package.json:11-13` — 测试命令
  - `src-tauri/Cargo.toml` — Rust 依赖和目标配置

  **验收标准**：
  - [ ] `cargo test -p ptv` → PASS
  - [ ] `cargo check --target x86_64-pc-windows-msvc` → PASS（或最小警告）
  - [ ] `npm run build` → PASS
  - [ ] `npm run test` → PASS

  **QA Scenarios**：
  ```
  Scenario: 全栈验证通过
    Tool: Bash
    Steps:
      1. cargo test -p ptv
      2. cargo check --target x86_64-pc-windows-msvc || cargo check --target x86_64-pc-windows-gnu
      3. npm run build
      4. npm run test
      5. 断言所有命令 exit code = 0
    Expected Result: 全部通过
    Evidence: .sisyphus/evidence/task-6-full-verify.txt
  ```

  **Evidence**：`task-6-full-verify.txt`

  **Commit**：YES（组 3）
  - Message：`test(cross): 跨平台编译验证与全栈回归测试通过`
  - Files：无代码变更（仅 evidence）

---

- [x] 7. README.md + AGENTS.md 文档更新

  **要做什么**：
  - 更新 `README.md`：
    - "平台支持"章节：添加 "Windows（测试中）"
    - "开发环境"章节："系统依赖（Linux）" 改为 "系统依赖（Linux/Windows）"，添加 Windows 依赖说明（WebView2 Runtime、VS Build Tools）
    - "快速开始"章节：添加 Windows 构建命令（`cargo tauri build --target x86_64-pc-windows-msvc`）
    - "开发规范"章节：保持不变
  - 更新 `AGENTS.md`：
    - 平台声明：`macOS + Linux（Windows 不做）` → `macOS + Linux + Windows（测试中）`
    - 关键决策表：添加 "Windows 支持" 行，方案 "NSIS installer, WebView2 runtime"

  **必须不做**：不重写 README 其他章节、不修改不相关的内容

  **推荐 Agent Profile**：
  - **Category**：`quick` — 文档更新
  - **Skills**：不需要

  **并行化**：
  - **可并行**：YES（与 Task 5, 6 可并行）
  - **并行组**：Wave 3
  - **阻塞**：F1-F4
  - **被阻塞**：Task 3

  **参考资料**：
  - `README.md:121-133` — 当前"平台支持"和"开发规范"章节
  - `AGENTS.md:10-11` — 当前平台声明
  - `AGENTS.md:84-94` — 关键决策表

  **验收标准**：
  - [ ] README.md "平台支持"包含 Windows
  - [ ] README.md "开发环境"包含 Windows 依赖说明
  - [ ] AGENTS.md 平台声明已更新

  **QA Scenarios**：
  ```
  Scenario: 文档验证
    Tool: Bash (grep)
    Steps:
      1. grep "Windows" README.md
      2. 断言匹配数 ≥ 3（平台支持 + 依赖 + 命令）
      3. grep "Windows（测试中）" AGENTS.md
      4. 断言匹配
    Expected Result: 文档包含完整的 Windows 支持说明
    Evidence: .sisyphus/evidence/task-7-docs.txt
  ```

  **Evidence**：`task-7-docs.txt`

  **Commit**：YES（组 3）
  - Message：`docs: 更新 README 与 AGENTS 添加 Windows 平台支持说明`
  - Files：`README.md`, `AGENTS.md`

---

## 最终验证 Wave（MANDATORY — 所有实现任务完成后）

> 4 个审查 agent 并行运行。全部必须 APPROVE。向用户展示合并结果并获取明确"okay"后才能标记完成。
> **F1-F4 完成前不主动标记。被拒绝或用户反馈 → 修复 → 重新运行 → 再次展示 → 等待确认。**

- [x] F1. **计划合规审计** — `oracle`
  通读计划。对每个"必须包含"：验证实现存在（读文件、运行命令）。对每个"必须不包含"：搜索代码库中的禁止模式 — 发现即拒绝并指明 file:line。检查 `.sisyphus/evidence/` 中的 evidence 文件是否存在。对比交付物与计划。
  输出：`必须包含 [N/N] | 必须不包含 [N/N] | 任务 [N/N] | 判定: APPROVE/REJECT`

- [x] F2. **代码质量审查** — `unspecified-high`
  运行 `cargo test` + `cargo check` + `npm run build`。审查所有变更文件：`as any`/`@ts-ignore`、空 catch、console.log、注释掉的代码、未使用的导入。检查 AI slop：过度注释、过度抽象、通用命名（data/result/item/temp）。
  输出：`构建 [PASS/FAIL] | 测试 [N pass/N fail] | 文件 [N clean/N issues] | 判定`

- [x] F3. **手动 QA** — `unspecified-high`（+ `playwright` skill）
  从干净状态开始。执行每个任务的每个 QA 场景 — 严格按步骤执行，捕获 evidence。测试跨任务集成（各功能协同工作，而非孤立）。测试边缘情况：空状态、无效输入、跨平台路径。
  输出：`场景 [N/N pass] | 集成 [N/N] | 边缘情况 [N tested] | 判定`

- [x] F4. **范围保真度检查** — `deep`
  对每个任务：阅读"要做什么"，阅读实际 diff（git log/diff）。验证 1:1 — 规范中的所有内容都已实现（无缺失），没有超出规范的内容（无蔓延）。检查"必须不包含"合规性。检测跨任务污染：任务 N 触及任务 M 的文件。标记未计入的变更。
  输出：`任务 [N/N compliant] | 污染 [CLEAN/N issues] | 未计入 [CLEAN/N files] | 判定`

---

## 提交策略

- **1**: `feat(backend): 跨平台 Python 自动探测与错误消息修正` — `src-tauri/src/collectors/template/config.rs`
- **2**: `feat(backend): encode_cwd_path 与 sessions_dir Windows 条件编译门控` — `src-tauri/src/collectors/template/session_transcript.rs`
- **3**: `fix(test): 替换硬编码 Unix 路径为跨平台 temp_dir()` — 6 个文件的测试代码
- **4**: `test(backend): find_python 探测与 sessions_dir 双平台单元测试` — `config.rs`, `session_transcript.rs`
- **5**: `build(windows): NSIS 安装包配置与平台声明更新` — `tauri.conf.json`, `AGENTS.md`
- **6**: `test(cross): 跨平台编译验证与全栈回归测试通过` — 无代码变更
- **7**: `docs: 更新 README 与 AGENTS 添加 Windows 平台支持说明` — `README.md`, `AGENTS.md`

---

## 成功标准

### 验证命令
```bash
cargo test -p ptv                                          # Rust 单测全部通过
cargo check --target x86_64-pc-windows-msvc                # Windows target 编译通过
npm run build                                              # 前端构建成功
npm run test                                               # Playwright E2E 全部通过
```

### 最终检查清单
- [ ] 所有"必须包含"已实现
- [ ] 所有"必须不包含"已排除
- [ ] Rust 单测通过（macOS，无回归）
- [ ] 交叉编译检查通过
- [ ] 前端构建通过
- [ ] Playwright E2E 通过
- [ ] 用户明确确认

