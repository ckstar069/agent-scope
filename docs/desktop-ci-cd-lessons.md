# 桌面端应用 GitLab CI/CD 自动构建发布复盘

本文档总结 AgentScope（Tauri v2 桌面应用）在 GitLab CI/CD 环境下实现 Linux + Windows 自动构建与发布的完整过程，记录最终可用方案、踩坑经历与可复用经验，供其他桌面端项目参考。

---

## 1. 背景与目标

### 1.1 项目概况

AgentScope 是一个基于 Tauri v2 的跨平台桌面应用，技术栈为：

- **前端**：React 19 + TypeScript + Vite + Tailwind CSS v4
- **后端**：Rust（Tauri v2 框架）
- **测试**：Playwright E2E + Rust 单元测试
- **CI/CD**：自托管 GitLab + GitLab Runner

### 1.2 构建目标

| 平台 | 构建方式 | 产物类型 | 参与 CI |
|------|---------|---------|---------|
| Linux | 自动 | deb + AppImage（免安装） | ✅ |
| Windows | 自动 | NSIS exe + portable zip（免安装） | ✅ |
| macOS | 手动 | dmg | ❌（开发者本机构建） |

### 1.3 发布流程目标

- **strict semver tag**（如 `v0.2.14`）触发完整发布：build → release → Package Registry → GitLab Release
- **prerelease tag**（如 `v0.2.15-rc.1`）仅触发构建验证，不创建 Release
- **Release assets 永久有效**：不指向 `artifacts/raw/...`（随 job artifact 过期而失效），而是指向 GitLab Generic Package Registry
- **Windows CI 构建必须使用 lightweight tag**：annotated tag 会导致 PowerShell ParserError

---

## 2. 最终可用架构

### 2.1 基础设施拓扑

| 角色 | 地址 | 执行器 | 标签 | 职责 |
|------|------|--------|------|------|
| GitLab Server | `192.168.3.100` | — | — | 代码托管、流水线调度、Release 页面、Package Registry |
| Linux Runner | `192.168.3.42` | Docker | `linux` | Linux 构建、测试、检查 |
| Windows Runner | `192.168.3.10` | Shell / PowerShell | `windows` | Windows 桌面应用构建 |

### 2.2 Linux Runner 配置

- **执行器**：Docker
- **基础镜像**：`agent-scope-ci:node20-rust1.95`（自定义镜像，预装 Node.js 20、Rust 1.95.0、Tauri Linux 依赖、Playwright 系统依赖、cargo 工具）
- **Docker 配置**：`privileged = false`，`pull_policy = ["if-not-present"]`
- **缓存**：`.npm/`、`.cargo/registry/`、`.cargo/git/`、`.cache/ms-playwright/`

### 2.3 Windows Runner 配置

- **执行器**：Shell（`shell = "powershell"`）
- **运行账户**：LocalSystem（当前状态，存在已知限制，见 5.5 / 5.7）
- **预装组件**：
  - GitLab Runner（`C:\GitLab-Runner\gitlab-runner.exe`）
  - Visual Studio Build Tools 2022（MSVC C++ 工具链）
  - Windows 11 SDK (10.0.22621.0)
  - Git 2.53.0
  - Node.js v22.14.0
  - Rust 1.95.0
  - WebView2 Runtime

### 2.4 产物输出路径

| 平台 | 产物 | CI 中路径 |
|------|------|-----------|
| Linux | deb | `src-tauri/target/x86_64-unknown-linux-gnu/release/bundle/deb/*.deb` |
| Linux | AppImage | `src-tauri/target/x86_64-unknown-linux-gnu/release/bundle/appimage/*.AppImage` |
| Windows | NSIS Installer | `src-tauri/target/x86_64-pc-windows-msvc/release/bundle/nsis/*.exe` |
| Windows | Portable zip | `src-tauri/target/x86_64-pc-windows-msvc/release/bundle/portable/*.zip` |

> **注意**：使用 `--target <target-triple>` 时，产物输出到 `target/<target-triple>/release/bundle/`，而非 `target/release/bundle/`。`artifacts:paths` 必须与实际路径一致，否则产物上传会静默失败。

### 2.5 Release 与 Package Registry 机制

`release` job 使用 `CI_JOB_TOKEN` 完成两步操作：

1. **上传产物**到 GitLab Generic Package Registry：
   ```
   PUT /projects/:id/packages/generic/agent-scope/:version/:filename
   ```
2. **创建 GitLab Release**，assets 链接指向 Package Registry URL（永久有效）

项目设置中需确保 `CI_JOB_TOKEN` 授权 `write_package_registry` 权限。

---

## 3. 最终发布流程

### 3.1 发布命令

使用项目提供的脚本创建 lightweight tag 并推送：

```bash
# 默认 HEAD
scripts/release-tag.sh v0.2.15

# 指定 commit
scripts/release-tag.sh v0.2.15 <commit-sha>
```

脚本功能：
- 校验 tag 格式为 strict semver `vX.Y.Z`
- `git fetch --tags origin` 同步远端状态
- 检查本地/远端 tag 是否已存在，防止覆盖
- 检查 staged changes，避免误混合发布和提交
- **只创建 lightweight tag**（`git tag "$TAG" "$COMMIT"`）
- 自动推送到 origin

### 3.2 为什么禁止使用 annotated tag

**错误方式**（会导致 Windows 构建失败）：
```bash
git tag -a v0.2.15 -m "v0.2.15: 中文发布说明"
git push origin v0.2.15
```

**根因**：GitLab Runner 18.11.3 的 PowerShell executor 在处理 annotated tag 时，会将 tag message 注入到生成的 PowerShell 脚本中。若 tag message 包含中文字符或特殊符号，会在 `get_sources` 阶段触发 `ParserError`：

```
ParserError: UnexpectedToken
字符串缺少终止符
块语句中缺少右大括号 }
```

job 在 `before_script` 之前即失败，没有任何用户代码被执行。

**验证记录**：
- `v0.2.14-rc.1`（annotated + 中文 message）：Pipeline #221 build:windows #534/#535，get_sources 阶段 ParserError，失败
- `v0.2.14-rc.2`（lightweight）：Pipeline #222 build:windows #537，成功
- `v0.2.14`（lightweight，正式版）：Pipeline #223 build:windows #539，成功

---

## 4. 已验证成功的发布记录

### v0.2.14 正式发布

| 项目 | 值 |
|------|-----|
| 正式 tag | `v0.2.14` |
| tag 类型 | lightweight |
| 指向 commit | `6bc23faf`（`[CI] 稳定 Windows 发布构建流程`） |
| Pipeline | #223 |
| build:linux | Job #538，通过，产出 deb + AppImage |
| build:windows | Job #539，通过，产出 NSIS exe + portable zip |
| release | Job #540，通过，GitLab Release v0.2.14 创建成功 |
| Package Registry | `packages/generic/agent-scope/0.2.14/` |

**产物文件名**：
- `AgentScope_0.2.14_amd64.deb`
- `AgentScope_0.2.14_amd64.AppImage`
- `AgentScope_0.2.14_x64-setup.exe`
- `AgentScope_0.2.14_x64_portable.zip`

**Release assets 验证**：
- 四个链接均指向 `packages/generic/agent-scope/0.2.14/`
- 不指向 `artifacts/raw/...`
- 全部可下载（HTTP 200）

---

## 5. 踩坑清单

### 5.1 Tag push 未触发 pipeline

| 字段 | 内容 |
|------|------|
| 问题 | 推送 tag 后 GitLab 没有创建流水线 |
| 错误现象 | GitLab UI 中流水线列表为空，或 job 状态为 skipped / pending / stuck |
| 发生阶段 | tag push 后 |
| 根因 | 多种可能：`.gitlab-ci.yml` 中 `rules` 未匹配 tag 格式；runner tags 与 job tags 不匹配；runner 未注册到项目；protected tag 限制 |
| 错误尝试 | 反复修改 `.gitlab-ci.yml` 的 `only/except` 规则，未检查 runner 注册状态 |
| 最终解决 | 逐项排查：① `rules: - if: $CI_COMMIT_TAG =~ /^v\d+\.\d+\.\d+$/` 正则是否正确；② job `tags` 是否与 runner `tags` 匹配；③ runner 是否在项目/组的 CI/CD Settings 中显示为 online；④ tag 是否为 protected（非 protected tag 可能无法触发某些 job） |
| 经验教训 | 遇到 tag 未触发流水线时，不要只改 CI 配置，先确认 runner 状态（`gitlab-runner list`、`gitlab-runner verify`）和 tag 保护状态 |

### 5.2 Linux AppImage 构建依赖问题

| 字段 | 内容 |
|------|------|
| 问题 | Linux AppImage 在 Docker 容器内构建失败 |
| 错误现象 | `linuxdeploy` 报错，FUSE 挂载失败，或提示缺少 `file`、`xdg-utils`、`libfuse2` |
| 发生阶段 | build:linux job |
| 根因 | Docker 容器默认没有 FUSE 支持，`linuxdeploy` 尝试挂载 AppImage 时失败；缺少系统工具 |
| 错误尝试 | 尝试在容器内安装 FUSE 内核模块（不可能，容器共享宿主机内核） |
| 最终解决 | 三管齐下：① 环境变量 `APPIMAGE_EXTRACT_AND_RUN=1` 让 `linuxdeploy` 不挂载 FUSE 而是直接解压运行；② 环境变量 `NO_STRIP=true` 避免 strip 阶段失败；③ 自定义 CI 镜像预装 `file`、`xdg-utils`、`libfuse2`，不在 job 内重复 `apt-get install` |
| 经验教训 | Docker 内构建 AppImage 不需要 FUSE，关键是 `APPIMAGE_EXTRACT_AND_RUN=1`。系统依赖应预装在 CI 镜像中，不要在每个 job 内动态安装。 |

### 5.3 Release assets 指向 artifacts/raw 不持久

| 字段 | 内容 |
|------|------|
| 问题 | GitLab Release 的 assets 链接指向 job artifacts，artifact 过期后链接失效 |
| 错误现象 | Release 页面点击下载返回 404 |
| 发生阶段 | release job 创建 Release 后 |
| 根因 | GitLab job artifacts 有过期时间（默认 30 天），过期后删除 |
| 错误尝试 | 尝试增大 artifact 过期时间，或手动保留 artifact |
| 最终解决 | release job 改为两步：① 先用 `curl PUT` 上传产物到 GitLab Generic Package Registry（永久存储）；② 创建 Release 时，assets 的 `url` 和 `direct_asset_url` 均指向 Package Registry URL（`packages/generic/agent-scope/{VERSION}/{filename}`） |
| 经验教训 | 任何需要长期可用的产物，不要依赖 job artifacts。Generic Package Registry 是更持久的选择。 |

### 5.4 Windows Runner get_sources 证书问题

| 字段 | 内容 |
|------|------|
| 问题 | Windows Runner 在 get_sources 阶段 git clone 失败 |
| 错误现象 | `git clone` 返回 exit status 128，TLS 证书验证失败 |
| 发生阶段 | build:windows job 的 get_sources 阶段 |
| 根因 | GitLab 使用自签证书，Windows Runner 上 Git 不信任该证书 |
| 错误尝试 | 在 CI job 内设置 `GIT_SSL_NO_VERIFY=1`（只在 script 阶段生效，get_sources 阶段由 Runner 控制） |
| 最终解决 | 在 Windows Runner 主机上全局配置 Git 信任自签证书：`git config --global http.sslCAInfo "C:\path\to\gitlab.crt"`，或在 GitLab Runner 注册时指定 `tls-ca-file` |
| 经验教训 | get_sources 阶段的 git 操作由 GitLab Runner 控制，不受 job 内环境变量影响。证书问题必须在 Runner 主机层面解决。 |

### 5.5 Windows NSIS makensis error 0x2

| 字段 | 内容 |
|------|------|
| 问题 | Windows NSIS 安装器构建失败，makensis 返回 error 0x2 |
| 错误现象 | Tauri bundler 调用 makensis 时 `CreateProcessW` 失败，错误码 0x2（文件未找到），或提示无法读取 NSIS stub 文件 |
| 发生阶段 | build:windows job 的 `cargo tauri build` 阶段 |
| 根因 | GitLab Runner 在 Windows 上以 **LocalSystem** 账户运行。Tauri bundler 的 NSIS 安装器默认使用 `dirs::cache_dir()` 获取缓存目录，LocalSystem 的缓存路径指向 `C:\WINDOWS\system32\config\systemprofile\AppData\Local`。该目录受 Windows 安全策略限制，makensis 无法从 systemprofile 下读取 stub 文件或启动子进程。 |
| 错误尝试 | **patch tauri-bundler 源码**：修改 `tauri-bundler` crate 中的 `nsis/mod.rs`，强行将缓存路径改为其他目录，然后通过 `cargo install tauri-cli --force` 使用 patched 版本。这条路浪费了多个 pipeline 的验证时间，原因是：① patch 后需要确保 Cargo 重新编译（曾尝试修改 mtime 强制重新编译）；② `cargo install` 安装的 CLI 版本不受 `package-lock.json` 约束，可能与项目锁定的 Tauri 版本不一致；③ patch 方式不可维护，Tauri 升级后 patch 会失效。 |
| 最终解决 | 三层方案：① `src-tauri/tauri.conf.json` 设置 `"useLocalToolsDir": true`，使 Tauri bundler 将 NSIS 工具缓存到项目目录 `target/.tauri/NSIS/` 下，不再依赖 `dirs::cache_dir()`；② `.gitlab-ci.yml` 中保留 `LOCALAPPDATA: "C:\\Users\\yufei\\AppData\\Local"` 作为安全网（该路径已预装完整 NSIS toolset）；③ 改用 `npx tauri build` 而非 `cargo install tauri-cli --force`，使用 npm 安装的 CLI（由 `package-lock.json` 锁定版本），更稳定可复现。 |
| 经验教训 | **不要 patch 上游依赖**。遇到上游工具链问题时，先查配置项、环境变量或官方文档，patch 源码是最后手段且维护成本极高。`useLocalToolsDir` 是 Tauri 官方提供的配置项，应优先使用。 |

### 5.6 PowerShell 默认错误处理导致伪成功

| 字段 | 内容 |
|------|------|
| 问题 | Windows build job 在构建失败后仍被标记为成功 |
| 错误现象 | NSIS 构建失败，但后续 `Compress-Archive` 命令继续执行，生成了一个空的或不完整的 zip，job 整体显示绿色通过 |
| 发生阶段 | build:windows job 的 script 阶段 |
| 根因 | PowerShell 默认 `$ErrorActionPreference` 不是 `"Stop"`，native command（如 `cargo`、`npx`）失败后不会停止脚本执行；`$LASTEXITCODE` 未被显式检查 |
| 错误尝试 | 仅在关键命令后加 `if ($LASTEXITCODE -ne 0) { exit 1 }`，遗漏了部分命令 |
| 最终解决 | 脚本开头统一设置 `$ErrorActionPreference = "Stop"`；关键 native command 后显式检查 `$LASTEXITCODE`；构建完成后强制校验产物文件是否存在且非空：`Test-Path`、`Get-Item` 检查 exe 和 zip 文件。 |
| 经验教训 | Windows CI 脚本必须显式处理错误，不能依赖默认行为。每个构建产物都必须有存在性+非空性校验。 |

### 5.7 annotated tag 中文 message 导致 Windows ParserError

| 字段 | 内容 |
|------|------|
| 问题 | 使用 annotated tag 后 Windows build 在 get_sources 阶段失败 |
| 错误现象 | `ParserError: UnexpectedToken`、`字符串缺少终止符`、`块语句中缺少右大括号 }` |
| 发生阶段 | build:windows job 的 get_sources 阶段（GitLab Runner 内部，before before_script） |
| 根因 | GitLab Runner 18.11.3 的 PowerShell executor 在处理 annotated tag 时，将 tag message 注入到生成的 PowerShell 脚本中。中文字符或特殊符号的转义处理存在缺陷，导致脚本解析失败。 |
| 错误尝试 | 尝试修改 `.gitlab-ci.yml` 的 before_script 或 script，但 get_sources 阶段完全由 Runner 控制，用户配置无法干预。 |
| 最终解决 | **使用 lightweight tag**（`git tag v0.2.14` 不带 `-a` / `-m`）。创建 `scripts/release-tag.sh` 脚本强制只创建 lightweight tag，并记录规则到项目文档。 |
| 经验教训 | **Windows CI 发布必须使用 lightweight tag**。这是一个 Runner 层面的兼容性问题，不是 CI 配置问题，无法通过 `.gitlab-ci.yml` 规避。 |

### 5.8 Linux npm ci 偶发 ECONNRESET

| 字段 | 内容 |
|------|------|
| 问题 | Linux build 中 `npm ci` 偶发失败 |
| 错误现象 | `npm ERR! code ECONNRESET`、`npm ERR! network Socket timeout` |
| 发生阶段 | build:linux job 的 `npm ci` 阶段 |
| 根因 | Runner 网络环境波动，偶发连接重置 |
| 错误尝试 | 无 |
| 最终解决 | 当前为偶发问题，重跑一次通常通过。长期可考虑配置 npm 镜像或内网缓存。 |
| 经验教训 | 偶发网络问题不应阻塞主线，但应记录并规划内网缓存治理。 |

### 5.9 Windows cache 保存耗时过长

| 字段 | 内容 |
|------|------|
| 问题 | Windows build job 的 cache archive 阶段耗时 5-8 分钟 |
| 错误现象 | Job 日志中 `cache-archive` 阶段耗时异常长 |
| 发生阶段 | build:windows job 结束后 |
| 根因 | `.cargo/registry/` 下文件数量极多，压缩/上传耗时 |
| 错误尝试 | 无 |
| 最终解决 | 当前可接受，未做优化。后续可缩小 cache 范围，仅保留 `.cargo/registry/cache/` 和 `.cargo/registry/src/` 的必要子集。 |
| 经验教训 | cache 不是越快越好，需要评估压缩/解压/上传总耗时与重新下载的对比。 |

### 5.10 不要重建已触发过 pipeline 的 release tag

| 字段 | 内容 |
|------|------|
| 问题 | 重建已存在的 tag 后 Windows build 在 get_sources 阶段 ParserError |
| 错误现象 | `ParserError: UnexpectedToken`、`${CI_SHARED_ENVIRONMENT}="true"` 等 Bash 语法出现在 PowerShell 错误中 |
| 发生阶段 | build:windows job 的 get_sources 阶段 |
| 根因 | GitLab Runner 18.11.3 PowerShell executor 在 tag 首次创建和重建时走不同的脚本生成路径。首次创建时成功，但删除后重建同一名称的 tag 时，Runner 生成的 `get_sources` 脚本中会混入 `${CI_SHARED_ENVIRONMENT}="true"` 这类 Bash 语法，在 PowerShell 中非法。 |
| 错误尝试 | 最初以为是 annotated vs lightweight tag 类型问题；尝试清理 Windows Runner 工作目录残留，均不解决 |
| 最终解决 | 废弃已有 tag，使用全新 tag 名（如从 `v0.2.1` 改为 `v0.2.2`）。首次创建的新 tag 不会触发此问题。 |
| 经验教训 | **发布 tag 一旦推送并触发了 pipeline，即使 pipeline 失败，也不要删除重建同名 tag。** 如需重新发布，必须使用新的版本号（如 `v0.2.2` 替代 `v0.2.1`）。`scripts/release-tag.sh` 已内置本地/远端 tag 存在性检查，防止误覆盖。 |

### 5.11 CI job artifact 无限堆积影响外部系统

| 字段 | 内容 |
|------|------|
| 问题 | 外部软件发布页项目通过 GitLab Jobs API 查询 artifact，获取到了 71 个 job 的数百个文件 |
| 错误现象 | 发布页展示了大量历史构建版本（包括测试版本 `99.99.99`），与 GitLab Releases 页面显示的 5 个 release 严重不一致 |
| 发生阶段 | 外部系统调用 `GET /projects/:id/jobs?scope=success&per_page=100` 时 |
| 根因 | 1. `build:linux` / `build:windows` job 的 `artifacts.expire_in` 设为 `1 month`，历史构建产物长期保留；2. 外部系统误用 Jobs API 查询"软件发布"，而 CI job artifact 不等于 release |
| 错误尝试 | 外部系统尝试按版本号过滤、按文件名去重，均无法根本解决 |
| 最终解决 | **两层修复**：① `.gitlab-ci.yml` 中 build job 的 `expire_in` 从 `1 month` 缩短为 `1 day`（release job 保持较长或不设置，因为它负责向 Package Registry 上传永久产物）；② 文档明确规范：外部系统应查询 **GitLab Releases API** 或 **Package Registry API** 获取软件发布，不应直接查询 Jobs API |
| 经验教训 | **CI job artifact 是临时构建产物，不是发布产物。** 长期存储应走 Package Registry，查询应走 Releases API。Build job 的 artifact 保留期应尽可能短（仅够 release job 下载即可），避免服务器存储无限增长和外部系统误用。 |

### 5.12 Linux npm ci "Exit handler never called!" crash（v0.3.0 发现）

| 字段 | 内容 |
|------|------|
| 问题 | Linux `verify` 和 `build:linux` job 中 `npm ci` / `npm install` 在 tag pipeline Docker executor 下稳定失败 |
| 错误现象 | `npm error Exit handler never called!` → node_modules 安装不完整 → 后续 `tsc: not found` |
| 发生阶段 | verify / build:linux job 的 `npm ci` 步骤 |
| 根因 | CI 镜像 `agent-scope-ci:node20-rust1.95` 中 Node 20 (NodeSource) 捆绑 npm 10.8.2；该 npm 在 tag pipeline Docker executor 环境下无法稳定完成安装，与 §5.8 记录的 ECONNRESET 不是同一问题 |
| 错误尝试 | 1. 直接重试；2. `npm ci` / `npm install` / `npm install --prefer-offline`；3. 串联重试、脚本块重试、独立重试脚本；4. 运行时 `npx npm@latest`。共 8 次独立重现，均无法产出可用 `node_modules` |
| 最终解决 | **镜像层修复**：`ci/Dockerfile` 增加 `NPM_VERSION=10.9.8`、`NPM_REGISTRY=https://registry.npmmirror.com`、`CARGO_REGISTRY_INDEX=sparse+https://rsproxy.cn/index/`；镜像构建期固定 npm，仓库级 `.cargo/config.toml` 将 crates.io 替换为 rsproxy sparse registry。Linux job 恢复单次 `npm ci`。修复后需在 Linux Runner（当前从本机经 Tailscale `100.70.62.93` 访问）重建同名镜像，并用新的 prerelease tag 验证 |
| 经验教训 | 1. Docker 镜像中的 Node.js 预装 npm 版本不一定适合当前 Docker executor，应在 Dockerfile 构建时固定 npm 版本；2. 运行时升级或用 `npx npm@latest` 仍依赖问题环境，不适合作为 release 修复；3. tag pipeline 多次失败后不要继续在 rc tag 上堆 workaround，应转向 Runner 镜像修复；4. npm 和 Cargo registry 访问路径都要固定，避免镜像问题和公网 TLS 问题混在一起；5. 已触发过 pipeline 的 tag 不删除不复用，使用新的 rc tag 验证 |

---


## 6. 推荐的标准实施步骤

以下 checklist 可供其他桌面端项目直接参考：

1. **明确平台与产物**：先确定需要哪些平台、哪些产物格式，macOS 是否进 CI
2. **Linux Runner 用 Docker executor**：制作自定义 CI 镜像，预装所有系统依赖，不要在 job 内动态安装
3. **Windows Runner 用 Shell executor**：直接在主机上构建，预装 MSVC、Rust、Node.js、WebView2
4. **先跑 prerelease tag 验证**：使用 `vX.Y.Z-rc.N` 格式，只触发 build 不触发 release，确认产物正确
5. **build job 只负责构建和上传 artifacts**：产物路径加入 `artifacts:paths`
6. **release job 统一上传 Generic Package Registry**：不依赖 artifacts 作为长期存储
7. **release 前强制校验所有产物**：检查 deb、AppImage、exe、zip 是否全部存在且非空
8. **使用 lightweight tag 发布**：annotated tag 会导致 Windows Runner PowerShell ParserError
9. **文档记录每个 runner 的环境和 tag**：包括主机地址、预装组件版本、runner tags
10. **不要在 CI 里依赖隐式状态**：所有工具版本应在 CI 镜像或 `.gitlab-ci.yml` 中显式声明

---

## 7. 推荐后续优化

以下任务已识别但尚未实施，后续 agent 可按优先级处理：

- **Windows Runner 账户切换**：将 GitLab Runner 服务从 LocalSystem 切换为专用普通用户（如 `yufei` 或新建 CI 用户），消除 systemprofile 目录限制。切换前需验证：Git 证书访问、git clone、Node/npm、Rust/cargo、MSVC、NSIS、WebView2 Runtime、gitlab-runner verify 全部正常。切换后必须跑 prerelease tag 验证。
- **release job 增加 assets HTTP 200 校验**：创建 Release 后，用 curl 逐个验证 assets 链接可下载，避免 Package Registry 上传成功但 Release 链接错误的情况。
- **Linux Runner 配 npm/cargo 内网缓存**：配置 npm registry mirror 或 Nexus，消除 `npm ci` 偶发 ECONNRESET。
- **Windows cache 缩小范围**：评估仅缓存 `.cargo/registry/cache/` 和必要子目录，减少 archive 耗时。
- **抽象 release-tag.sh**：将 `scripts/release-tag.sh` 中的通用逻辑（format 校验、tag 重复检查、lightweight 强制）提取为其他项目可复用的模板。
- **GitLab Runner 版本评估**：跟踪 GitLab Runner PowerShell executor 对 annotated tag 的兼容性修复，评估未来升级 Runner 版本的可行性。

---

## 8. 关键配置速查

### 8.1 src-tauri/tauri.conf.json（Windows NSIS 关键配置）

```json
{
  "bundle": {
    "active": true,
    "useLocalToolsDir": true
  }
}
```

### 8.2 .gitlab-ci.yml（Windows build 关键片段）

```yaml
build:windows:
  stage: build
  tags:
    - windows
  variables:
    LOCALAPPDATA: "C:\\Users\\yufei\\AppData\\Local"
  script:
    - $ErrorActionPreference = "Stop"
    - npm ci
    - npm run build
    - npx tauri build --target x86_64-pc-windows-msvc
    # 校验产物
    - $exe = Get-ChildItem src-tauri/target/x86_64-pc-windows-msvc/release/bundle/nsis/*.exe | Select-Object -First 1
    - if (-not $exe) { throw "NSIS exe not found" }
    - $zip = Get-ChildItem src-tauri/target/x86_64-pc-windows-msvc/release/bundle/portable/*.zip | Select-Object -First 1
    - if (-not $zip) { throw "Portable zip not found" }
```

### 8.3 scripts/release-tag.sh 用法

```bash
# 查看帮助
scripts/release-tag.sh --help

# 发布（默认 HEAD）
scripts/release-tag.sh v0.2.15

# 发布指定 commit
scripts/release-tag.sh v0.2.15 <commit-sha>
```

---

## 9. 脱敏说明

本文档已脱敏处理：

- Runner token 写作 `<RUNNER_TOKEN>`
- 密码写作 `<PASSWORD>`
- 个人访问令牌写作 `<PRIVATE_TOKEN>`
- 未记录任何真实凭据

---

*文档创建时间：2026-05-19*
*对应版本：v0.2.14*
*对应 commit：6bc23faf*
