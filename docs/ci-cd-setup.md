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
- **项目路径**: `znxt_tools/agent-scope`
- **访问方式**: Web 界面 + API（`PRIVATE-TOKEN` 认证）

### 2.2 GitLab Runner

- **服务器**: `192.168.3.144`（Ubuntu 24.04 VM，用户 `yufei/yufei`）
- **Runner 名称**: `agent-scope-runner`
- **执行器类型**: Docker
- **基础镜像**: `ubuntu:22.04`
- **SSH 访问**: `sshpass -p yufei ssh yufei@192.168.3.144`

### 2.3 流水线配置概览

`.gitlab-ci.yml` 的 `verify` 阶段包含以下步骤：

1. 安装系统依赖（`libwebkit2gtk-4.1-dev` 等）
2. 安装 Node.js 20
3. 安装 Rust（stable toolchain）
4. 安装 `cargo-binstall`
5. 安装 `cargo-audit`
6. `npm ci`（前端依赖）
7. `npx playwright install chromium`
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

## 3. 遇到的所有问题

### 问题 1: cargo-binstall 安装路径与 `CARGO_HOME` 不一致

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

### 问题 2: Clippy 版本差异导致 CI 失败

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

- `scanner.rs:113`：降序时间排序改为 `sort_by_key(|entry| Reverse(entry.timestamp))`
- `session_transcript.rs:478`：移除 `days as i64` 冗余转换
- `session_transcript.rs:574`：合并 `custom-title` 分支中的嵌套判断
- `session_transcript.rs:661`：降序 mtime 排序改为 `sort_by_key(|(_, mtime)| Reverse(*mtime))`

**状态**: ✅ 已在本地验证通过。2026-05-14 执行 `cargo fmt --check`、`cargo clippy -- -D warnings`、`cargo test` 均通过；仍需重新触发 GitLab Pipeline 验证 CI 环境。

---

### 问题 3: cargo-binstall URL 使用了 `latest/download/`

**现象**: 存在潜在的版本不稳定风险。

**根因**: 原始配置使用 `.../latest/download/...` 指向最新版本，可能导致未来版本不兼容。

**修复**: 锁定到固定版本 `v1.19.1`。

```diff
- https://github.com/cargo-bins/cargo-binstall/releases/latest/download/...
+ https://github.com/cargo-bins/cargo-binstall/releases/download/v1.19.1/...
```

**状态**: ✅ 已修复。

---

## 4. 当前现状

### 4.1 流水线状态

| 流水线 | 状态 | 说明 |
|-------|------|------|
| Pipeline #48 | ❌ 失败 | `cargo-binstall` 路径错误（已修复） |
| Pipeline #49 | ❌ 失败 | Clippy 4 个 lint 错误（本地已修复，待重新触发验证） |

**当前阻塞点**: 4 个 Clippy 警告已在本地修复，下一步需要重新触发 GitLab Pipeline，确认 CI 环境通过。

### 4.2 环境版本差异

| 环境 | Rust 版本 | Clippy 行为 |
|-----|----------|------------|
| 本地开发环境 | 1.93.0 | 修复后 `cargo clippy -- -D warnings` 通过 |
| CI (Docker) | 1.95.0 (2026-04-14) | 触发 4 个新 lint 规则 |

**建议**: 定期更新本地 Rust 工具链，或在 CI 中锁定特定 Rust 版本以避免版本漂移。

### 4.3 待办事项

1. [x] 修复 4 个 Clippy 警告（`scanner.rs:113`、`session_transcript.rs:478/574/661`）
2. [ ] 重新触发 Pipeline 验证
3. [ ] 考虑在 CI 中锁定 Rust 版本（如 `rustup default 1.95.0`）
4. [ ] 考虑统一本地和 CI 的 Rust 版本

---

## 附录：相关文件

- `.gitlab-ci.yml` — GitLab CI 配置
- `src/collectors/claude_history/scanner.rs` — Clippy 警告 #1
- `src/collectors/template/session_transcript.rs` — Clippy 警告 #2~4
