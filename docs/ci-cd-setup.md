# AgentScope CI/CD 配置与问题记录

本文档记录 AgentScope 项目的持续集成/持续部署（CI/CD）环境搭建过程、遇到的问题及当前状态。

---

## 1. 背景

AgentScope 使用自托管 GitLab 作为 CI/CD 平台，通过 GitLab Runner 执行流水线任务，覆盖代码检查、构建、测试等验证环节。

| 平台 | 用途 | 配置文件 |
|-----|------|---------|
| **GitLab CI** | 代码检查、构建、测试 | `.gitlab-ci.yml` |

---

## 2. 搭建方式

### 2.1 GitLab 服务器

- **地址**: `http://192.168.3.100`
- **标准拓扑**: 新项目也按 `192.168.3.100` 作为 GitLab 服务器记录，Runner 统一使用 `192.168.3.144`。
- **项目路径**: `znxt_tools/agent-scope`
- **访问方式**: Web 界面 + API（`PRIVATE-TOKEN` 认证）

### 2.2 GitLab Runner

- **服务器**: `192.168.3.144`（Ubuntu 24.04 VM）
- **Runner 名称**: `agent-scope-runner`
- **执行器类型**: Docker
- **基础镜像**: `ubuntu:22.04`
- **SSH 访问**: `sshpass` 可连接，凭据由项目负责人单独提供，避免在文档中继续扩散明文密码。

### 2.3 流水线配置概览

`.gitlab-ci.yml` 的 `verify` 阶段包含以下步骤：

1. 安装系统依赖（`libwebkit2gtk-4.1-dev` 等）
2. 安装 Node.js 20
3. 安装 Rust（stable toolchain）
4. 安装 `cargo-binstall`
5. 安装 `cargo-audit`
6. `npm ci`（前端依赖）
7. `npx playwright install --with-deps chromium`
8. `npm run build`（前端构建）
9. `cargo fmt --check`（格式化检查）
10. `cargo clippy -- -D warnings`（静态分析）
11. `cargo check`（编译检查）
12. `cargo test`（Rust 单元测试）
13. `cargo audit`（安全审计，非阻塞）
14. `npm audit --audit-level=moderate`（前端安全审计，非阻塞）
15. `npm test`（E2E 测试，Playwright）

缓存目录：`.npm/`、`.cargo/registry/`、`.cargo/git/`、`.cache/ms-playwright/`

---

## 3. 遇到的问题与修复

### 问题 1: Git 相关单元测试缺少 `git`

**现象**: Pipeline #36 的 `test:rust` job 失败，5 个 Git collector 单元测试失败。

**错误日志**:
```
git init should work: Os { code: 2, kind: NotFound, message: "No such file or directory" }
test result: FAILED. 115 passed; 5 failed
```

**根因**: 测试用例会在临时目录中调用 `git init`，但 CI 镜像内未安装 `git`。

**修复**: 系统依赖安装阶段加入 `git`。

**状态**: ✅ 已修复，后续 `cargo test` 为 `120 passed; 0 failed`。

---

### 问题 2: NodeSource / rustup 下载受网络波动影响

**现象**: Pipeline #37 / #43 在安装 Rust 时失败，Pipeline #41 在安装 NodeSource 时超时。

**错误日志**:
```
curl: (92) HTTP/2 stream 0 was not closed cleanly: PROTOCOL_ERROR
error: command failed: downloader https://static.rust-lang.org/rustup/dist/...

curl: (28) Failed to connect to deb.nodesource.com port 443 after 132511 ms
```

**根因**: `ubuntu:22.04` 裸镜像每次执行都从公网安装 Node.js、Rust 和工具链，CI 对外网链路波动敏感。

**修复现状**: 目前通过重试和缓存降低影响，但仍不是根治。

**建议**: 制作内部 CI 基础镜像，预置 Node.js 20、Rust stable、Tauri Linux 系统依赖、Playwright 系统依赖、`cargo-binstall`、`cargo-audit`。这是缩短流水线时间和降低网络失败率的最大收益点。

**状态**: ⚠️ 已识别，待工程化优化。

---

### 问题 3: apt 安装阶段被系统终止

**现象**: Pipeline #38 在 `apt-get install` 阶段以 exit code 137 退出。

**错误日志**:
```
ERROR: Job failed: exit code 137
```

**根因**: 137 通常表示进程收到 `SIGKILL`，结合日志停在大量系统包安装阶段，优先判断为 Runner 资源压力或容器被外部终止。

**修复现状**: 后续流水线已跨过该阶段；属于基础设施稳定性问题，不是代码问题。

**建议**: 若再次出现，优先查看 Runner 主机同一时间段的内存、Docker 和 GitLab Runner journal。

**状态**: ⚠️ 偶发基础设施问题，暂不改代码。

---

### 问题 4: CI 脚本工作目录错误

**现象**: Pipeline #39 在 Rust 检查阶段失败。

**错误日志**:
```
/usr/bin/bash: line 187: cd: src-tauri: No such file or directory
```

**根因**: CI 脚本在前序命令改变工作目录后继续使用相对路径 `src-tauri`，导致路径解析错误。

**修复**: 后续配置统一使用 `cd "$CI_PROJECT_DIR/src-tauri"`。

**状态**: ✅ 已修复。

---

### 问题 5: Runner system failure / job 被终止

**现象**: Pipeline #44 / #45 / MR Pipeline #35 出现 `runner_system_failure`。

**错误日志**:
```
ERROR: Job failed (system failure): aborted: terminated
```

**根因**: GitLab Runner 进程或容器执行环境层面的中断。Pipeline #44 在 E2E 执行中被终止，Pipeline #45 在 cache restore 阶段被终止，MR Pipeline #35 在 prepare executor 阶段被终止。

进一步查看 Runner 主机 `192.168.3.144` 的 journal 后，三次 `runner_system_failure` 都与 Runner 服务重启、Runner 重新注册或 `config.toml` 配置异常高度相关：

- MR Pipeline #35：Runner 曾出现自签证书校验失败、旧 Runner token `403 Forbidden`，随后服务在已有 build 运行时收到 stop signal。
- Pipeline #44：Runner 配置文件曾出现 TOML 解析错误，随后服务重启，正在执行的 job 收到 `context canceled`。
- Pipeline #45：job 刚开始后 Runner 服务再次停止，日志显示 active build 被终止。

**补充证据**: Pipeline #54 的 Runner journal 中还出现过 Docker API 超时：

```
Failed to exec create to container: ... context deadline exceeded
```

该日志发生在测试已失败后的收尾阶段，不是 #54 的主失败因，但说明 Docker/Runner 通道存在偶发迟滞。

**修复现状**: 后续同一 Runner 可成功执行 #46/#47/#52/#53/#55/#56，说明不是稳定复现的项目配置错误，也没有发现当前主机存在持续 OOM 或磁盘耗尽。当前 Runner 主机为 4 vCPU / 16GiB 内存，Docker root 目录剩余约 33GiB；资源紧张仍需观察，但不是这几次 `runner_system_failure` 的主要证据链。

**建议**:

- Runner 配置或重新注册前，先确认没有运行中的 job；必要时先暂停 Runner 或等待 active build 为 0。
- 修改 `/etc/gitlab-runner/config.toml` 后执行 `gitlab-runner verify` / `gitlab-runner list`，避免 malformed TOML 进入服务运行状态。
- 清理旧的无效 Runner 注册信息，避免旧 token、自签证书、当前 Runner 混杂导致误判。
- 已将 `concurrent` 从 3 降为 **1**，减少多 job 并发对 Docker/磁盘/内存的压力。

**状态**: ⚠️ 已完成日志归因；后续按 Runner 运维规范观察，不需要改项目代码。

---

### 问题 6: cargo-binstall 安装路径与 `CARGO_HOME` 不一致

**现象**: Pipeline #48（Job 217）在 `cargo-binstall` 安装步骤失败。

**错误日志**:
```
tar: /root/.cargo/bin: Cannot open: No such file or directory
tar: Error is not recoverable: exiting now
```

**根因**: `.gitlab-ci.yml` 中设置了 `CARGO_HOME: "$CI_PROJECT_DIR/.cargo"`，但 `cargo-binstall` 的安装命令使用了 `~/.cargo/bin/`，而 Docker 容器中 `~` 解析为 `/root`，导致路径不匹配。

**修复**: 将安装路径从 `~/.cargo/bin/` 改为 `"$CARGO_HOME/bin/"`。

```diff
- curl ... | tar -xz -C ~/.cargo/bin/
+ curl ... | tar -xz -C "$CARGO_HOME/bin/"
```

**状态**: ✅ 已修复，Pipeline #49 验证通过此步骤。

---

### 问题 7: Clippy 版本差异导致 CI 失败

**现象**: Pipeline #49（Job 218）在 `cargo clippy -- -D warnings` 步骤失败，但本地通过。

**错误日志**（Rust 1.95.0 触发）：
```
error: consider using `sort_by_key`
   --> src/collectors/claude_history/scanner.rs:113
error: casting to the same type is unnecessary (`i64` -> `i64`)
   --> src/collectors/template/session_transcript.rs:478
error: this `if` can be collapsed into the outer `match`
   --> src/collectors/template/session_transcript.rs:574
error: consider using `sort_by_key`
   --> src/collectors/template/session_transcript.rs:661
```

**根因**: CI 环境安装的 Rust 版本（1.95.0，2026-04-14）比本地版本更新，包含了更多 lint 规则。本地环境未触发这些警告。

**涉及的 lint 规则**:
- `clippy::unnecessary_sort_by`（2 处）
- `clippy::unnecessary_cast`（1 处）
- `clippy::collapsible_match`（1 处）

**修复**:

- `src-tauri/src/collectors/claude_history/scanner.rs`：降序时间排序改为 `sort_by_key(|entry| Reverse(entry.timestamp))`
- `src-tauri/src/collectors/template/session_transcript.rs`：移除 `days as i64` 冗余转换
- `src-tauri/src/collectors/template/session_transcript.rs`：合并 `custom-title` 分支中的嵌套判断
- `src-tauri/src/collectors/template/session_transcript.rs`：降序 mtime 排序改为 `sort_by_key(|(_, mtime)| Reverse(*mtime))`

**状态**: ✅ 已修复。Pipeline #51 已验证 Clippy 通过。

---

### 问题 8: cargo-binstall URL 使用了 `latest/download/`

**现象**: 存在潜在的版本不稳定风险。

**根因**: 原始配置使用 `.../latest/download/...` 指向最新版本，可能导致未来版本不兼容。

**修复**: 锁定到固定版本 `v1.19.1`。

```diff
- https://github.com/cargo-bins/cargo-binstall/releases/latest/download/...
+ https://github.com/cargo-bins/cargo-binstall/releases/download/v1.19.1/...
```

**状态**: ✅ 已修复。

---

### 问题 9: Playwright Chromium 缺少系统运行库

**现象**: Pipeline #51（Job 220）已通过 `cargo clippy -- -D warnings`、`cargo check`、`cargo test`，但在 `npm test` 阶段大量 E2E 失败。

**错误日志**:
```
Error: browserType.launch: Target page, context or browser has been closed
chrome-headless-shell: error while loading shared libraries: libnspr4.so: cannot open shared object file: No such file or directory
```

**根因**: `.gitlab-ci.yml` 使用 `npx playwright install chromium`，只安装 Chromium 浏览器文件，不安装浏览器运行所需的 Linux 系统依赖。Ubuntu 22.04 基础镜像中缺少 `libnspr4` 等动态库，导致 Chromium 启动即退出。

**修复**: 使用 Playwright 官方依赖安装模式：

```diff
- npx playwright install chromium
+ npx playwright install --with-deps chromium
```

**状态**: ✅ 已修复。Pipeline #52 已验证通过。

---

### 问题 10: E2E 测试在成功流水线中仍有 flaky

**现象**: Pipeline #52 成功，但 `npm test` 输出 `1 flaky`，首个 `AgentMonitor` 用例第一次 `page.goto("/")` 超时，retry 后通过。

**错误日志**:
```
Test timeout of 30000ms exceeded while running "beforeEach" hook.
Error: page.goto: Test timeout of 30000ms exceeded.
navigating to "http://localhost:1420/", waiting until "load"
1 flaky
42 passed (1.9m)
```

**根因**: CI 中 Playwright 使用 `npm run dev` 启动 Vite dev server。dev server 在冷启动、依赖扫描或资源抖动时可能导致首个页面加载超过 30 秒。

**修复**: CI 已先执行 `npm run build`，因此 Playwright 在 CI 中改用 `vite preview` 服务已构建产物；本地仍使用 `npm run dev`。

**状态**: ✅ 已调整。本地执行 `npm run build && CI=1 npm test` 验证为 `43 passed (13.3s)`，Pipeline #53 验证为 `43 passed` 且无 flaky。

---

### 问题 11: watcher 单元测试存在 mtime 精度 flaky

**现象**: Pipeline #54（Job 223）在 `cargo test` 阶段失败。

**错误日志**:
```
test watcher::tests::test_deeply_nested_file_change ... FAILED
thread 'watcher::tests::test_deeply_nested_file_change' panicked at src/watcher.rs:975:9:
deeply nested file change should be detected
test result: FAILED. 119 passed; 1 failed
```

**根因**: `FileWatcher` 使用 `std::fs::metadata().modified()` 的 mtime 轮询来检测变化。失败用例在短时间内把文件内容从 `"initial"` 写成 `"changed"`，两者都是 7 字节。如果 CI 容器/overlayfs 的 mtime 精度没有跨过一个可见 tick，且文件大小也未变化，快照对比可能认为文件未变化。

**影响范围**: 这是测试稳定性问题，不是 #54 中发现的业务功能必现失败。相同代码在 Pipeline #53 和本地可通过，说明该问题具备时序/文件系统相关的偶发性。

**建议修复**:

- 测试层：将该用例的二次写入改成不同长度内容，或显式等待 mtime 可观察变化后再写入。
- 测试隔离：临时目录名加入进程 ID/时间戳，避免不同测试进程或残留目录碰撞。
- 设计层：如后续需要更强可靠性，可考虑快照中记录 `(mtime, len)`，但这会扩大实现行为面，应单独评估。

**修复**: 已在测试层处理：

- 临时目录名加入进程 ID 和时间戳，避免测试残留或并发进程碰撞。
- `test_deeply_nested_file_change` 的二次写入改为不同长度内容，避免 CI/overlayfs 上短时间同长度写入导致变化不可观察。

**状态**: ✅ 已修复并由 Pipeline #56 验证。`cargo fmt --check`、`cargo test watcher::tests::test_deeply_nested_file_change -- --nocapture`、`cargo test` 本地均通过；Pipeline #56 全流程通过，GitLab API 记录 job duration 为 `685.881862s`。

---

### 问题 12: 单 job 串行 verify 放大失败率

**现象**: 当前主线只有一个 `verify` job，负责系统依赖、Node、Rust、前端构建、Rust 检查、Rust 测试、安全审计、E2E。历史上任一环节失败都会让整条流水线失败。

**根因**: 验证面过宽，且大量步骤依赖外部网络或容器内冷安装。单 job 方便串行定位，但会放大单点波动对整条流水线成功率的影响。

**影响范围**: 失败统计会被配置错误、网络波动、Runner 问题、测试 flaky 混在一起；从“流水线失败率”看会显得整体不可靠，但实际需要按失败源分层治理。

**建议修复**:

- 短期：保留单 job，但先修复已确认的 flaky 和脚本配置缺口。
- 中期：拆分 `build:frontend`、`check:rust`、`test:rust`、`test:e2e`，让失败归因更清楚。
- 长期：配合内部基础镜像，减少每个 job 的安装成本。

**状态**: ⚠️ 已识别，待 CI 结构优化。

---

### 问题 13: 历史遗留 GitLab 模板 job 拉低统计口径

**现象**: Pipeline #27/#28 包含 `secret_detection`、`semgrep-sast`、`nodejs-scan-sast`、`container_scanning`、`code_quality` 等旧 job，其中 #28 出现 `stuck_or_timeout_failure`。

**根因**: 这些 job 属于早期/模板化 CI 配置，与当前 `.gitlab-ci.yml` 的 `verify` 主线不同。把它们和当前主线合并统计，会放大历史失败率。

**影响范围**: 用于复盘时应保留记录；用于判断当前主线稳定性时，应单独剔除或分组。

**状态**: ℹ️ 历史记录，非当前主线阻塞项。

---

### 问题 14: build 产物路径未包含 target triple，导致 artifact 上传静默失败

**现象**: Pipeline #107 中 `build:linux` 和 `build:windows` 均显示「Job succeeded」，但产物未上传，后续 `release` job 无法找到构建产物。

**错误日志**（build:linux）：
```
WARNING: src-tauri/target/release/bundle/deb/*.deb: no matching files
```

**错误日志**（build:windows）：
```
WARNING: src-tauri/target/release/bundle/nsis/*.exe: no matching files
WARNING: src-tauri/target/release/bundle/msi/*.msi: no matching files
```

**根因**: `.gitlab-ci.yml` 中 build job 的 `artifacts:paths` 使用了 `src-tauri/target/release/bundle/...`，但 `cargo tauri build --target <triple>` 时，产物实际输出到 `src-tauri/target/<target-triple>/release/bundle/...`。路径不匹配导致 GitLab artifact 上传阶段找不到文件，但 job 本身仍然标记为成功（因为编译确实通过了）。

**涉及的目标三元组**：
- Linux: `x86_64-unknown-linux-gnu`
- Windows: `x86_64-pc-windows-msvc`

**修复**: 更新 `.gitlab-ci.yml` 中 build job 的 artifact 路径，加入 target triple 目录：

```diff
   artifacts:
     paths:
-      - src-tauri/target/release/bundle/deb/*.deb
+      - src-tauri/target/x86_64-unknown-linux-gnu/release/bundle/deb/*.deb
```

```diff
   artifacts:
     paths:
-      - src-tauri/target/release/bundle/nsis/*.exe
-      - src-tauri/target/release/bundle/msi/*.msi
+      - src-tauri/target/x86_64-pc-windows-msvc/release/bundle/nsis/*.exe
+      - src-tauri/target/x86_64-pc-windows-msvc/release/bundle/msi/*.msi
```

**release job 的 artifact 查找**使用 `**/*.deb`、`**/*.exe`、`**/*.msi` 通配，只要 build job 正确上传，release job 即可正常收集。

**状态**: ✅ 已修复，Pipeline #108 验证通过。build:linux ✅ → build:windows ✅ → release ✅，GitLab Release v0.2.10 成功创建并包含所有产物。

---

### 问题 15: AppImage 在 Docker 容器内构建失败

**现象**: Pipeline #102 / #105 的 `build:linux` job 在构建 AppImage 时失败。

**错误日志**:
```
failed to run linuxdeploy
```

**根因**: AppImage 构建依赖 `linuxdeploy` 和 FUSE，在 Docker 容器环境中不可靠。Tauri 官方文档也指出 AppImage bundling 在容器内存在已知限制。

**修复**: 从构建目标中移除 AppImage：

1. `src-tauri/tauri.conf.json` 中 `bundle.targets` 从 `"all"` 改为 `["deb", "rpm", "nsis", "msi"]`
2. `.gitlab-ci.yml` 中 `build:linux` 不再收集 AppImage 产物
3. CI 中不再尝试构建 AppImage

**状态**: ✅ 已修复。Linux 产物仅保留 deb，已在 Pipeline #108 验证。

---

### 问题 16: Release 描述中换行符显示为字面量 `\n`

**现象**: GitLab Release 页面中，Release 描述显示为 `AgentScope v0.2.10\n\n### Linux\n- deb: ...`，换行符以字面量 `\n` 出现，排版混乱。

**根因**: `.gitlab-ci.yml` release job 中构建描述时使用了 shell 双引号字符串：

```bash
DESC="## AgentScope ${CI_COMMIT_TAG}\n\n### Linux\n"
```

在 bash 的双引号中，`\n` **不会**被解释为换行符，而是两个普通字符 `\` 和 `n`。随后通过 `jq -sR .` JSON 编码时，反斜杠被二次转义，最终 GitLab API 收到的是字面量 `\n`，导致页面上直接显示为 `\n`。

**修复**: 使用 `printf` 构建描述，确保 `\n` 被解析为真正的换行符（ASCII 0x0a）：

```bash
DESC=$(printf '## AgentScope %s\n\n### Linux\n' "${CI_COMMIT_TAG}")
[ -n "$DEB" ] && DESC=$(printf '%s- deb: `%s`\n' "$DESC" "$(basename "$DEB")")
DESC=$(printf '%s\n### Windows\n' "$DESC")
[ -n "$EXE" ] && DESC=$(printf '%s- Installer: `%s`\n' "$DESC" "$(basename "$EXE")")
[ -n "$MSI" ] && DESC=$(printf '%s- MSI: `%s`\n' "$DESC" "$(basename "$MSI")")
DESC=$(printf '%s\n---\n点击下方 Artifacts 区域下载各平台安装包。' "$DESC")
```

**状态**: ✅ 已修复，将在下一次 tag 发布时验证。

---

## 4. Pipeline #52 耗时分析

Pipeline #52（Job 221）成功，GitLab API 记录 job duration 为 `779.982949s`，约 13 分钟。

### 4.1 阶段耗时

| 阶段 | 开始 | 结束 | 耗时 | 说明 |
|------|------|------|------|------|
| prepare / get sources | 07:50:43 | 07:50:52 | ~9s | 正常 |
| restore cache | 07:50:52 | 07:52:52 | ~120s | 明显偏长 |
| before_script + script | 07:52:52 | 08:02:56 | ~604s | 主要执行时间 |
| archive cache | 08:02:56 | 08:03:38 | ~42s | 缓存打包偏长 |
| upload artifacts / cleanup | 08:03:38 | 08:03:43 | ~5s | 正常 |

### 4.2 关键命令耗时

| 命令 | 耗时 | 说明 |
|------|------|------|
| `apt-get update` + Tauri 系统依赖 | ~61s | 每次容器冷安装 |
| NodeSource setup + `apt-get install nodejs` | ~31s | 每次从公网配置源并安装 |
| `rustup` 安装 stable Rust | ~140s | 最大单项耗时之一 |
| `cargo-binstall` + `cargo-audit` 安装 | ~11s | 可通过预置工具减少 |
| `npm ci` | ~10s | npm cache 生效，耗时可接受 |
| `npx playwright install --with-deps chromium` | ~52s | 安装/校验浏览器和系统依赖 |
| `npm run build` | ~111s | 前端构建耗时明显 |
| `cargo clippy` | ~9s | 已受前序依赖缓存影响 |
| `cargo check` | ~5s | 与 clippy 有一定重复，可考虑移除 |
| `cargo test` | ~43s | 单元测试本身约 3s，主要是测试编译 |
| `cargo audit` + `npm audit` | ~13s | 非阻塞 |
| `npm test` | ~113s | 43 个 E2E，含一次 flaky retry |

### 4.3 结论

耗时长的核心原因不是某个测试特别慢，而是 `ubuntu:22.04` 裸镜像每次冷启动后动态安装完整工具链和桌面/WebView/浏览器依赖，同时 GitLab cache restore/archive 也较重。短期可减少 flaky 和少量重复检查；中期应使用内部基础镜像解决大头。

---

## 5. 失败超半数归因

截至 Pipeline #56，当前可见流水线状态为：`8 success`、`15 failed`、`1 canceled`。截至 Pipeline #54 时，失败 job 的 failure reason 分布为：`11 script_failure`、`3 runner_system_failure`、`3 stuck_or_timeout_failure`。

失败超半数不是单一根因，而是以下几类问题叠加：

| 类别 | 对应流水线 | 性质 | 当前处理 |
|------|------------|------|----------|
| CI 配置缺口 | #36、#39、#48、#49/#50、#51 | 项目配置问题 | 已逐项修复 |
| 公网依赖波动 | #37、#41、#43 | 环境/网络问题 | 待基础镜像治理 |
| Runner/Docker 基础设施波动 | #35、#44、#45，#54 有 Docker 超时信号 | 基础设施问题 | 已完成日志归因，按 Runner 运维规范观察 |
| 测试 flaky | #52、#54 | 测试稳定性问题 | #52 已修复；#54 watcher 已由 #56 验证 |
| 历史旧 job | #27/#28 | 统计口径问题 | 归档为历史，不作为当前主线阻塞 |

优先级建议：

1. #54 的 watcher flaky 已修复，并由 #56 验证通过。
2. Runner/Docker system failure 已完成日志归因，后续重点是避免运行中重启 Runner 和配置异常。
3. 下一优先级是内部 CI 基础镜像，消除 NodeSource/rustup/Playwright 依赖公网下载，并显著缩短流水线。
4. 最后考虑拆分 `verify` job，让失败归因更清楚。

---

## 6. 当前现状

### 6.1 流水线状态

| 流水线 | 状态 | 说明 |
|-------|------|------|
| Pipeline #36 | ❌ 失败 | Rust Git 测试缺少 `git`（已修复） |
| Pipeline #37 | ❌ 失败 | rustup 下载 HTTP/2 错误（外网波动） |
| Pipeline #38 | ❌ 失败 | apt 阶段 exit 137（Runner/容器被杀） |
| Pipeline #39 | ❌ 失败 | `cd src-tauri` 路径错误（已修复） |
| Pipeline #41 | ❌ 失败 | NodeSource 连接超时（外网波动） |
| Pipeline #43 | ❌ 失败 | rustup 下载 HTTP/2 错误（外网波动） |
| Pipeline #44/#45 | ❌ 失败 | Runner system failure（基础设施中断） |
| Pipeline #48 | ❌ 失败 | `cargo-binstall` 路径错误（已修复） |
| Pipeline #49/#50 | ❌ 失败 | Clippy 4 个 lint 错误（已修复） |
| Pipeline #51 | ❌ 失败 | Chromium 缺少 `libnspr4.so`（已修复） |
| Pipeline #52 | ✅ 成功 | 全流程通过；存在一次 E2E flaky retry，已调整 Playwright CI server，并已本地验证 CI 模式 E2E 无 flaky |
| Pipeline #53 | ✅ 成功 | 全流程通过，43 passed (12.9s)，无 flaky |
| Pipeline #54 | ❌ 失败 | `watcher::tests::test_deeply_nested_file_change` mtime 精度 flaky（已修复） |
| Pipeline #55 | ✅ 成功 | 文档变更流水线通过 |
| Pipeline #56 | ✅ 成功 | watcher flaky 修复后全流程通过，Rust tests `120 passed; 0 failed`，E2E `43 passed` |
| Pipeline #57 | ✅ 成功 | 内部 CI 基础镜像验证通过 |
| Pipeline #102~#105 | ❌ 失败 | build/release 阶段调试：AppImage 容器内构建失败、产物路径错误、curl SSL 证书问题（已逐项修复） |
| Pipeline #107 | ❌ 失败 | build:linux / build:windows 编译成功但 artifact 路径未包含 target triple，产物上传静默失败（已修复，详见问题 14） |
| Pipeline #108 | ✅ 成功 | 修复产物路径后首次完整 build + release 成功，GitLab Release v0.2.10 包含 deb、exe、msi 三个产物 |

**当前阻塞点**: 当前没有已知代码或测试阻塞点；自动构建与发布流水线已通过 Pipeline #108 验证。

### 6.2 环境版本差异

| 环境 | Rust 版本 | Clippy 行为 |
|-----|----------|------------|
| 本地开发环境 | 1.93.0 | 修复后 `cargo clippy -- -D warnings` 通过 |
| CI (Docker) | 1.95.0 (2026-04-14) | 已锁定版本，避免漂移 |

**版本锁定**: CI 中已固定 Rust 版本为 `1.95.0`，本地建议同步升级以避免未来版本差异。如需升级，需同时更新 `.gitlab-ci.yml` 中的版本号并验证所有检查通过。

### 6.3 后续优化建议

1. [x] 修复 4 个 Clippy 警告（`scanner.rs`、`session_transcript.rs`）
2. [x] 修复 Playwright Chromium 系统依赖安装方式（`--with-deps`）
3. [x] Pipeline #52 / #53 验证全流程通过
4. [x] CI 下 Playwright 改用 `vite preview`，减少 dev server 冷启动 flaky
5. [x] CI 中锁定 Rust 版本为 1.95.0
6. [x] 修复 watcher mtime 精度 flaky，并由 Pipeline #56 验证
7. [x] 检查 Runner/Docker 健康度和资源限制，确认 system failure 主要来自 Runner 服务重启/配置变更
8. [x] 制作内部 CI 基础镜像，预置 Node.js、Rust、Tauri Linux 依赖、Playwright 依赖和常用 cargo 工具
9. [x] 将 Runner `concurrent` 从 3 降为 1，减少多 job 并发资源竞争
9. [x] 评估 cache 策略：保留全部 4 个缓存目录；ms-playwright 缓存浏览器文件价值最高，移除后重新下载 Chromium 成本约 +120s
10. [x] ~~考虑移除 `cargo check`~~ — 不实施。`cargo clippy` 虽覆盖编译检查，但 `cargo check` 更快（~5s），作为 fallback 保留价值大于节省的时间。
11. [x] 配置 Windows Shell Runner（`192.168.3.10`），安装 MSVC、Rust、Node.js、Tauri CLI
12. [x] 配置 GitLab CI build + release 阶段：Linux deb/AppImage、Windows exe/zip 自动构建
13. [x] 修复 AppImage Docker 容器内构建失败，从构建目标中排除 AppImage
14. [x] 修复 build job artifact 路径，加入 target triple 目录（`x86_64-unknown-linux-gnu`、`x86_64-pc-windows-msvc`）
15. [x] 修复 release job curl SSL 自签证书问题（`-k` 参数）
16. [x] Pipeline #108 验证完整 build + release 流程，GitLab Release v0.2.10 成功创建
17. [x] 增加 Windows Portable zip 免安装版（`Compress-Archive` 打包 `agent-scope.exe`）
18. [x] 增加 Linux AppImage 免安装版（`APPIMAGE_EXTRACT_AND_RUN=1` 解决 Docker FUSE 限制）
19. [x] 修复 Release 描述换行符显示为字面量 `\n` 的问题

**内部 CI 基础镜像**

| 项目 | 内容 |
|------|------|
| Dockerfile | `ci/Dockerfile` |
| 镜像标签 | `agent-scope-ci:node20-rust1.95` |
| 构建位置 | Runner 主机 `192.168.3.144` |
| 预置内容 | Ubuntu 22.04 + Node.js 20 + Rust 1.95.0 + Tauri 依赖 + cargo-binstall + cargo-audit + Playwright 系统依赖 |
| 效果 | 流水线耗时从 ~779s 降到 ~517s（节省约 34%） |
| 验证 | Pipeline #57（Job 235）成功通过 |

**镜像更新流程**

当需要升级 Node.js、Rust 或工具版本时：
1. 修改 `ci/Dockerfile` 中的版本号
2. 在 Runner 主机执行 `docker build` 重新构建
3. 更新 `.gitlab-ci.yml` 中的 `image:` 标签（如需要）
4. 触发流水线验证

---

## 7. 后续排查与修复交接建议

本节面向后续接手的 agent。当前代码和测试主线已经通过 Pipeline #56 验证，自动构建与发布已经通过 Pipeline #108 验证，后续重点不是继续修业务代码，而是把 CI 环境从”能跑通”提升到”稳定、快、可复用”。

### 7.1 先确认基础事实

接手后先做一次只读确认，避免基于过期信息继续排查：

```bash
git remote -v
git status --short
curl -k -fsSL "https://<gitlab-host>/api/v4/projects/<project-id>/pipelines?per_page=10" | jq -r '.[] | [.id,.status,.ref,.sha,.created_at] | @tsv'
sshpass -p '<password>' ssh yufei@192.168.3.144 'hostname; gitlab-runner --version; gitlab-runner status; docker info --format "{{.ServerVersion}} {{.CgroupVersion}}"; free -h; df -h / /var/lib/docker'
```

需要特别注意 GitLab 地址：本仓库 remote/API 当前实测为 `192.168.3.100`，后续新项目也按 `192.168.3.100` 作为 GitLab 服务器；Runner 统一使用 `192.168.3.144`。

### 7.2 Runner 稳定性专项

目标：避免再次出现 #35/#44/#45 这类 `runner_system_failure`。

建议动作：

1. 在 Runner 主机建立运维约束：有 job running 时不要重启 `gitlab-runner`、不要重新注册 Runner、不要直接编辑 `config.toml`。
2. 修改 Runner 配置前，先在 GitLab 页面暂停 Runner 或确认 active build 为 0。
3. 每次修改 `/etc/gitlab-runner/config.toml` 后执行：

```bash
sudo gitlab-runner verify
sudo gitlab-runner list
sudo systemctl status gitlab-runner --no-pager
sudo journalctl -u gitlab-runner --since "30 min ago" --no-pager
```

4. 若再次出现 `runner_system_failure`，按时间线抓证据：

```bash
sudo journalctl -u gitlab-runner --since "YYYY-MM-DD HH:MM:SS" --until "YYYY-MM-DD HH:MM:SS" --no-pager
sudo journalctl -u docker --since "YYYY-MM-DD HH:MM:SS" --until "YYYY-MM-DD HH:MM:SS" --no-pager
dmesg -T | tail -200
docker ps -a --no-trunc | head -50
```

5. 稳定期建议将 Runner `concurrent` 临时降到 `1`，等基础镜像和 job 拆分完成后再评估是否恢复到 `2` 或 `3`。

验收标准：

- 连续 5 次主线流水线无 `runner_system_failure`。
- Runner journal 中没有运行中 job 被 stop signal 中断的记录。
- `config.toml` 没有 TOML 解析错误、旧 token `403 Forbidden`、证书校验失败等噪音。

### 7.3 内部 CI 基础镜像

这是当前收益最高的优化。Pipeline #52/#56 仍需 11 到 13 分钟，主要耗时来自每次在 `ubuntu:22.04` 裸镜像中安装系统依赖、Node.js、Rust、Playwright 和 cargo 工具。

建议制作一个内部基础镜像，例如：

```text
registry.<gitlab-host>/ci-images/agent-scope-tauri:node20-rust1.95-pw
```

镜像应预置：

- Ubuntu 22.04 基础环境
- Git、curl、ca-certificates、build-essential、pkg-config
- Tauri Linux 依赖：`libwebkit2gtk-4.1-dev`、`libgtk-3-dev`、`libayatana-appindicator3-dev`、`librsvg2-dev` 等
- Node.js 20
- Rust 1.95.0，包含 `rustfmt`、`clippy`
- `cargo-binstall` 固定版本
- `cargo-audit`
- Playwright Chromium 及其 Linux 运行库

实施步骤：

1. 新建 CI 镜像仓库或在当前项目下新增 `ci/Dockerfile`。
2. 在 Runner 主机或专用构建机上构建镜像，并推送到 GitLab Container Registry 或内网 registry。
3. 将 `.gitlab-ci.yml` 的 `image: ubuntu:22.04` 切换到内部镜像。
4. 删除 CI 中重复的 NodeSource setup、rustup 安装、Playwright 系统依赖安装，仅保留版本检查和项目依赖安装。
5. 连续跑 3 次流水线，对比耗时和失败率。

验收标准：

- `before_script + script` 主要耗时从约 600s 降到 300s 以内。
- 不再出现 NodeSource/rustup 下载失败。
- Playwright Chromium 不再因系统库缺失失败。

### 7.4 Cache 策略评估

当前 cache restore 约 120s，archive cache 约 42s，已经不是小开销。后续 agent 不应默认认为“缓存一定更快”，需要实测。

建议做两组对比：

1. 保留现有 cache，记录 3 次流水线耗时。
2. 临时禁用或缩小 cache，只保留 `.npm/`、`.cargo/registry/`，记录 3 次流水线耗时。

重点观察：

- restore cache 是否稳定超过 60s。
- archive cache 是否超过从公网重新下载的收益。
- `.cache/ms-playwright/` 在使用内部基础镜像后是否还需要缓存。
- `.cargo/git/` 和 `.cargo/registry/` 是否命中有效，还是压缩/解压成本更高。

建议结论方向：

- 如果使用内部基础镜像，优先删除 Playwright 浏览器缓存。
- 如果 Rust 依赖变化不频繁，保留 cargo registry/git cache。
- npm cache 可以保留，但需避免缓存 `node_modules`。

### 7.5 拆分 verify job

当前单个 `verify` job 适合早期排错，但长期会放大失败率，也让失败归因不清楚。基础镜像完成后再拆分更合适。

建议拆分为：

```text
build:frontend
check:rust
test:rust
test:e2e
audit
```

拆分原则：

- `check:rust` 执行 `cargo fmt --check`、`cargo clippy -- -D warnings`。
- `test:rust` 只执行 `cargo test`。
- `build:frontend` 执行 `npm ci`、`npm run build`，产物作为 artifact 给 `test:e2e`。
- `test:e2e` 使用 `vite preview` 测试已构建产物，不使用 Vite dev server。
- `audit` 可以 `allow_failure: true`，避免安全数据库临时波动阻塞主线。

验收标准：

- 任一失败能直接归因到前端构建、Rust 检查、Rust 测试、E2E 或审计。
- E2E 不再重复构建前端产物。
- 单个 job 日志长度下降，排查时间缩短。

### 7.6 网络与 GitLab 地址治理

当前已确认 Runner `192.168.3.144` 能访问 NodeSource、Rust、GitHub、npm registry。但长期仍建议减少 CI 对公网实时下载的依赖。

后续需要确认：

- GitLab 服务器是否统一使用 `192.168.3.100`。
- 当前项目 remote 是否已指向 `192.168.3.100`。
- Runner 注册 URL 是否与最终 GitLab 地址一致。
- GitLab 自签证书是否已被 Runner 主机信任，避免再次出现 x509 失败。

只读检查命令：

```bash
git remote -v
sshpass -p '<password>' ssh yufei@192.168.3.144 'sudo gitlab-runner list'
sshpass -p '<password>' ssh yufei@192.168.3.144 'curl -k -I https://192.168.3.100 || true'
```

### 7.7 推荐执行顺序

1. 确认 GitLab 地址统一为 `192.168.3.100`。
2. 固化 Runner 运维规范，避免运行中重启和配置异常。
3. 制作内部 CI 基础镜像。
4. 基于基础镜像重新评估 cache。
5. 拆分 `verify` job。
6. 连续运行 5 次主线流水线，统计成功率和耗时。

---

## 8. 本次会话总结

本次会话聚焦 CI/CD 稳定性完善，从问题排查、修复到基础设施优化，最终达成流水线稳定通过的目标。

### 8.1 流水线配置现状

| 配置项 | 当前值 |
|-------|--------|
| CI 平台 | GitLab CI（`192.168.3.100`） |
| Runner | `agent-scope-runner` @ `192.168.3.144`（Docker executor） |
| 基础镜像 | `agent-scope-ci:node20-rust1.95`（内部镜像，Runner 本地） |
| Rust 版本 | 1.95.0（已锁定） |
| Runner concurrent | 1 |
| 缓存目录 | `.npm/`、`.cargo/registry/`、`.cargo/git/`、`.cache/ms-playwright/` |
| 流水线耗时 | ~517s（约 8.6 分钟） |

流水线步骤：
1. `npm ci` + `npx playwright install chromium`
2. `npm run build`
3. `cargo fmt --check`
4. `cargo clippy -- -D warnings`
5. `cargo check`
6. `cargo test`
7. `cargo audit`（非阻塞）
8. `npm audit`（非阻塞）
9. `npm test`（E2E，Playwright）

### 8.2 本次会话过程

**Phase 1：问题排查与修复**
- 分析 Pipeline #48~#56 失败日志，定位根因
- 修复 cargo-binstall 路径错误（`~/.cargo/bin/` → `$CARGO_HOME/bin/`）
- 修复 Clippy 4 个 lint 错误（Rust 1.95.0 新规则）
- 修复 Playwright Chromium 缺少系统库（`--with-deps`）
- 修复 E2E flaky（CI 改用 `vite preview`，`127.0.0.1` 替代 `localhost`）
- 修复 watcher mtime 精度 flaky（进程 ID 隔离 + 不同长度写入内容）
- 清理未使用的 GitHub Actions 配置

**Phase 2：基础设施优化**
- 制作内部 CI 基础镜像 `agent-scope-ci:node20-rust1.95`
  - Dockerfile: `ci/Dockerfile`
  - 预置：Node.js 20、Rust 1.95.0、Tauri 依赖、Playwright 依赖、cargo 工具
  - 效果：耗时从 ~779s → ~517s（-34%）
- Runner `concurrent` 从 3 降为 1，减少并发资源竞争
- Cache 策略评估：实验对比后确认保留全部 4 个缓存目录最优
- Rust 版本锁定：`.gitlab-ci.yml` 中固定 `1.95.0`

**Phase 3：文档治理**
- 创建/更新 `docs/ci-cd-setup.md`，记录全部 10+ 个问题和修复过程
- 补充 Pipeline 耗时分析、失败归因、后续接手指南

### 8.3 验证结果

| 流水线 | 状态 | 说明 |
|-------|------|------|
| Pipeline #53 | ✅ | 修复 Clippy 后首次全流程通过 |
| Pipeline #56 | ✅ | watcher flaky 修复后验证 |
| Pipeline #57 | ✅ | 内部镜像验证，耗时 ~517s |
| Pipeline #58 | ✅ | cache 评估实验（缩小缓存） |
| Pipeline #59 | ✅ | cache 评估 round-2 |

当前阻塞点：**无**。流水线已稳定。

### 8.4 自动构建与发布（已完成）

自动构建与发布已配置并验证通过（Pipeline #108）。详见 Section 9。

**当前能力**：

| 能力 | 状态 | 说明 |
|------|------|------|
| 自动验证 | ✅ | Push/MR 自动触发 verify |
| 自动构建桌面应用 | ✅ | tag push 触发 `build:linux` + `build:windows` |
| 多平台构建 | ✅ | Linux (Docker) + Windows (Shell) |
| 自动发布 | ✅ | `release` job 自动创建 GitLab Release |
| 免安装版 | ✅ | Linux AppImage + Windows Portable zip |

---

## 9. 自动构建与发布

本节记录 AgentScope 桌面应用的自动构建与发布配置，覆盖 Linux 和 Windows 平台。

### 9.1 方案概述

| 平台 | 构建方式 | 产物 | Runner |
|------|---------|------|--------|
| Linux | GitLab CI 自动 | deb + AppImage（免安装） | `192.168.3.144` Docker executor |
| Windows | GitLab CI 自动 | exe (NSIS) + zip（免安装） | `192.168.3.10` Shell executor |
| macOS | 本机手动 | dmg | 开发者本机（不参与 CI） |

### 9.2 触发方式

- **自动构建**：推送符合语义化版本的 tag（如 `v0.2.1`）时自动触发
- **验证阶段**：Push 到 `main` 分支或 MR 时运行 verify（原有行为不变）
- **构建阶段**：Tag push 时运行 `build:linux` + `build:windows`，然后执行 `release`

Tag 命名规则：`v<major>.<minor>.<patch>`，例如 `v0.2.1`

### 9.3 版本号同步

CI 自动将 Git tag 中的版本号同步到以下文件：

| 文件 | 同步方式 |
|------|---------|
| `package.json` | CI 脚本直接修改 `version` 字段 |
| `src-tauri/Cargo.toml` | CI 脚本直接修改 `version` 字段 |
| `src-tauri/tauri.conf.json` | 已配置 `"version": "../package.json"`，自动读取 package.json |

同步逻辑：
1. 读取 `CI_COMMIT_TAG`（如 `v0.2.1`）
2. 去掉 `v` 前缀得到 `0.2.1`
3. 写入 `package.json` 和 `Cargo.toml`

### 9.4 Runner 配置

#### Linux Runner（已有）

- **主机**：`192.168.3.144`
- **执行器**：Docker
- **镜像**：`agent-scope-ci:node20-rust1.95`
- **Tags**：`linux`

#### Windows Runner（新增）

- **主机**：`192.168.3.10`（Windows 11 ESXi VM）
- **执行器**：Shell
- **Tags**：`windows`
- **必需组件**：
  - GitLab Runner（`C:\GitLab-Runner\gitlab-runner.exe`）
  - Visual Studio Build Tools 2022（MSVC C++ 工具链）
  - Windows 11 SDK (10.0.22621.0)
  - Git 2.53.0
  - Node.js v22.14.0
  - Rust 1.95.0
  - Tauri CLI 2.11.1
  - WebView2 Runtime

**Windows Runner 注册命令**（在 `192.168.3.10` 上执行）：

```powershell
cd C:\GitLab-Runner
.\gitlab-runner.exe register `
  --non-interactive `
  --url "https://192.168.3.100" `
  --registration-token "<从 GitLab 项目设置获取>" `
  --executor "shell" `
  --tag-list "windows" `
  --name "agent-scope-windows-runner" `
  --locked="false"

# 启动服务
.\gitlab-runner.exe start
```

获取 registration token：
1. 登录 GitLab（`https://192.168.3.100`）
2. 进入项目 `znxt_tools/agent-scope`
3. 设置 → CI/CD → Runners → 项目 runners → 注册令牌

### 9.5 流水线结构

```
stages:
  - verify    # 代码检查、测试（main/MR 触发）
  - build     # 桌面应用构建（tag 触发）
  - release   # 创建 GitLab Release（tag 触发）
```

**build:linux** job：
- 使用 Docker Runner + 内部镜像
- 同步版本号 → `npm ci` → `npm run build` → `cargo tauri build --target x86_64-unknown-linux-gnu`
- 环境变量：`APPIMAGE_EXTRACT_AND_RUN=1`（解决 Docker 内 FUSE 限制）
- 产物：`*.deb`、`*.AppImage`

**build:windows** job：
- 使用 Windows Shell Runner
- 同步版本号 → `npm ci` → `npm run build` → `cargo tauri build --target x86_64-pc-windows-msvc`
- 额外步骤：`Compress-Archive` 打包 `agent-scope.exe` 为 zip 便携版
- 产物：`*.exe`（NSIS）、`*.zip`（免安装）

**release** job：
- 依赖 build:linux 和 build:windows 的 artifacts
- 调用 GitLab API 创建 Release
- Release 描述包含各平台产物列表
- 产物链接指向 job artifacts 下载地址

### 9.6 产物路径

| 平台 | 产物类型 | 路径（CI 中） |
|------|---------|-------------|
| Linux | deb | `src-tauri/target/x86_64-unknown-linux-gnu/release/bundle/deb/*.deb` |
| Linux | AppImage（免安装） | `src-tauri/target/x86_64-unknown-linux-gnu/release/bundle/appimage/*.AppImage` |
| Windows | NSIS Installer | `src-tauri/target/x86_64-pc-windows-msvc/release/bundle/nsis/*.exe` |
| Windows | Portable zip（免安装） | `src-tauri/target/x86_64-pc-windows-msvc/release/bundle/portable/*.zip` |

> **重要**：当使用 `cargo tauri build --target <target-triple>` 时，产物输出到 `target/<target-triple>/release/bundle/`，而非 `target/release/bundle/`。CI 中 `artifacts:paths` 必须与实际输出路径一致，否则产物上传会静默失败。详见「问题 14」。

**AppImage Docker 构建**：之前排除 AppImage 是因为 Docker 容器内 FUSE 权限不足导致 `linuxdeploy` 失败。修复方式是在 `build:linux` job 中设置环境变量 `APPIMAGE_EXTRACT_AND_RUN=1`，让 linuxdeploy 不挂载 FUSE 而是直接解压运行。已在 Pipeline #108 之后验证可行。

### 9.7 GitLab Release 权限

`release` job 使用 `CI_JOB_TOKEN` 调用 GitLab Release API。需要在项目设置中授权：

1. 项目设置 → CI/CD → Token Access
2. 确保 `CI_JOB_TOKEN` 允许访问 Release API
3. 或创建 Project Access Token（`api` + `write_repository` 权限）并设为 `GITLAB_TOKEN` 变量

### 9.8 首次发布测试步骤

1. 确认 Windows Runner 已注册并 online
2. 确认 MSVC 安装完成（`cl.exe` 可用）
3. 本地测试版本号同步逻辑：
   ```bash
   # 测试 Linux 同步脚本
   export CI_COMMIT_TAG=v0.2.1
   VERSION=${CI_COMMIT_TAG#v}
   node -e "const fs=require('fs'); const pkg=JSON.parse(fs.readFileSync('package.json')); pkg.version='$VERSION'; fs.writeFileSync('package.json', JSON.stringify(pkg,null,2)+'\n');"
   sed -i "s/^version = \".*\"/version = \"$VERSION\"/" src-tauri/Cargo.toml
   ```
4. 推送测试 tag：
   ```bash
   git tag v0.2.1-test
   git push origin v0.2.1-test
   ```
5. 在 GitLab 查看流水线状态
6. 确认 Release 页面已创建且产物可下载
7. 删除测试 tag：
   ```bash
   git push --delete origin v0.2.1-test
   git tag -d v0.2.1-test
   ```

### 9.9 macOS 手动构建说明

macOS 产物（`.dmg`）不参与 CI，需要在本机手动构建：

```bash
# 在本机执行
npm ci
npm run build
cd src-tauri
cargo tauri build --target aarch64-apple-darwin  # Apple Silicon
cargo tauri build --target x86_64-apple-darwin   # Intel（如需要）
```

产物位置（Apple Silicon）：`src-tauri/target/aarch64-apple-darwin/release/bundle/dmg/*.dmg`
产物位置（Intel）：`src-tauri/target/x86_64-apple-darwin/release/bundle/dmg/*.dmg`

如需将 macOS 产物加入 GitLab Release，可手动上传到同一 Release 页面。

---

## 附录：相关文件

- `.gitlab-ci.yml` — GitLab CI 配置
- `ci/Dockerfile` — 内部 CI 基础镜像
- `src/collectors/claude_history/scanner.rs` — Clippy 警告 #1
- `src/collectors/template/session_transcript.rs` — Clippy 警告 #2~4
