# GitLab + Docker Runner CI 模式说明

本文总结一套可复用的内网 GitLab CI/CD 模式，供所有需要接入 CI/CD 的项目使用。目标是把 GitLab 服务、Runner 执行环境、项目 `.gitlab-ci.yml` 的职责边界说清楚，避免每个项目重新摸索。

---

## 1. 标准拓扑

| 角色 | 地址 | 用户名 | 密码 | 职责 |
|------|------|--------|------|------|
| GitLab 服务器 | `192.168.3.100` | `yufei` | `yufei` | 托管代码、项目、CI 配置、流水线状态、Job 日志、Container Registry |
| GitLab Runner 服务器（Linux） | `192.168.3.42` | `znxt` | `znxt` | 拉取 GitLab job，使用 Docker executor 在容器中执行 CI |
| GitLab Runner 服务器（Windows） | `192.168.3.10` | `yufei` | `yufei` | 拉取 GitLab job，使用 Shell executor 执行 Windows 桌面应用构建 |
| 项目仓库 | GitLab 项目路径 | 按项目权限配置 | 按项目权限配置 | 提供 `.gitlab-ci.yml`、源码、测试和构建脚本 |

常用访问方式：

```bash
# 登录 GitLab 服务器
sshpass -p yufei ssh yufei@192.168.3.100

# 登录 Linux Runner 服务器
sshpass -p znxt ssh znxt@192.168.3.42

# 只读检查 Runner 服务和 Docker 状态
sshpass -p znxt ssh znxt@192.168.3.42 'hostname; gitlab-runner --version; sudo gitlab-runner status; sudo docker info --format "{{.ServerVersion}} {{.CgroupVersion}}"; free -h; df -h / /var/lib/docker'
```

---

## 2. 职责边界

### 2.1 GitLab 服务器

GitLab 负责：

- 保存 Git 仓库。
- 保存 `.gitlab-ci.yml` 并生成流水线。
- 调度 job 给可用 Runner。
- 展示流水线、job、artifact、cache 状态。
- 可选：提供 GitLab Container Registry 存放内部 CI 基础镜像。

GitLab 不负责：

- 直接执行构建命令。
- 安装项目依赖。
- 管理 Runner 主机资源。

### 2.2 Runner 服务器

Runner 负责：

- 通过 GitLab Runner 服务向 GitLab 轮询 job。
- 使用 Docker executor 创建临时容器。
- 在容器中执行 `.gitlab-ci.yml` 里的命令。
- 管理 job cache、artifact 上传、容器清理。

Runner 主机需要稳定运行：

- `gitlab-runner`
- `docker`
- 到 GitLab 服务器的网络访问
- 到必要外部依赖源或内部镜像源的网络访问

Runner 在 `192.168.3.42` 上应作为 systemd 常驻服务运行。它不是每个项目手动启动一次的临时命令，而是后台持续轮询 GitLab；只要 GitLab 上有匹配 tag 的 job，Runner 就会自动拉取并创建 Docker 容器执行。

常用运维命令：

```bash
# 查看 Runner 服务状态
sshpass -p znxt ssh znxt@192.168.3.42 'sudo systemctl status gitlab-runner --no-pager'

# 启动 Runner
sshpass -p znxt ssh znxt@192.168.3.42 'sudo systemctl start gitlab-runner'

# 设置开机自启
sshpass -p znxt ssh znxt@192.168.3.42 'sudo systemctl enable gitlab-runner'

# 重启 Runner，仅在确认没有 job 正在运行时执行
sshpass -p znxt ssh znxt@192.168.3.42 'sudo systemctl restart gitlab-runner'

# 查看 Runner 注册与连通性
sshpass -p znxt ssh znxt@192.168.3.42 'sudo gitlab-runner list; sudo gitlab-runner verify'

# 查看 Runner 最近日志
sshpass -p znxt ssh znxt@192.168.3.42 'sudo journalctl -u gitlab-runner --since "1 hour ago" --no-pager'
```

保持运行的检查点：

- `systemctl status gitlab-runner` 应为 `active (running)`。
- `systemctl status docker` 应为 `active (running)`。
- `gitlab-runner verify` 应能连通 GitLab，不能出现 `403 Forbidden`、证书校验失败或 runner unhealthy。
- GitLab 项目或 group 的 Runner 页面应显示 Runner online。
- Runner tag 必须与 `.gitlab-ci.yml` 中 job 的 `tags` 匹配；否则 job 会 pending。
- 有流水线运行时，不要重启 `gitlab-runner`、重启 Docker、重新注册 Runner 或编辑 `/etc/gitlab-runner/config.toml`。这些操作会直接中断正在执行的 job，表现为 `runner_system_failure`。

### 2.3 项目仓库

项目负责：

- 提供 `.gitlab-ci.yml`。
- 固定语言版本、工具版本和检查命令。
- 避免依赖本机路径或开发者个人环境。
- 将测试和构建命令做成可在干净容器中重复执行。

---

## 3. Runner 推荐配置

Runner 建议使用 Docker executor。典型配置形态：

```toml
concurrent = 1
check_interval = 0
shutdown_timeout = 0

[[runners]]
  name = "project-or-shared-runner"
  url = "https://192.168.3.100"
  executor = "docker"

  [runners.cache]
    Type = "cache"
    MaxUploadedArchiveSize = 0
    [runners.cache.s3]
      AssumeRoleMaxConcurrency = 0
    [runners.cache.gcs]
    [runners.cache.azure]

  [runners.docker]
    image = "ubuntu:22.04"
    privileged = false
    oom_kill_disable = false
    volumes = ["/cache"]
    shm_size = 268435456
    pull_policy = ["if-not-present"]
```

#### Windows Shell executor

用于构建 Windows 桌面应用（如 Tauri），典型配置：

```toml
concurrent = 1
check_interval = 0
shutdown_timeout = 0

[[runners]]
  name = "agent-scope-windows-runner"
  url = "https://192.168.3.100"
  executor = "shell"
  shell = "powershell"

  [runners.custom_build_dir]
  [runners.cache]
    [runners.cache.s3]
    [runners.cache.gcs]
    [runners.cache.azure]
```

Windows Shell executor 特点：
- 直接在主机上执行命令（非容器化），适合需要完整 Windows 环境的构建任务
- 默认使用 `cmd`，可通过 `shell = "powershell"` 切换
- 缓存路径使用 `\\` 分隔符
- 需要预装构建工具链（MSVC、Node.js、Rust 等）

建议：

- 稳定期先用 `concurrent = 1`，避免多个重型 job 同时争抢 CPU、内存、磁盘和 Docker。
- 稳定后再根据主机规格提升到 `2` 或 `3`。
- 不要在 job 运行中重启 Runner 或修改 `config.toml`。
- 修改 Runner 配置后必须执行 `gitlab-runner verify` 和 `gitlab-runner list`。

---

## 4. 项目接入流程

### 4.1 GitLab 项目准备

1. 在 GitLab 上创建项目。
2. 推送代码。
3. 确认项目默认分支。
4. 确认 Git remote 指向标准 GitLab 地址：

```bash
git remote -v
git remote set-url origin https://192.168.3.100/<group>/<project>.git
```

### 4.2 Runner 注册

在 Runner 主机 `192.168.3.42` 上注册 Runner。注册 token 从 GitLab 项目或 group 的 CI/CD Runner 页面获取。

```bash
sudo gitlab-runner register
```

关键选项：

- GitLab URL：`https://192.168.3.100`
- Executor：`docker`
- Default image：优先使用内部 CI 基础镜像；没有时临时使用 `ubuntu:22.04`
- Tags：按项目或技术栈设置，例如 `linux`、`docker`、`tauri`、`node-rust`

注册后检查：

```bash
sudo gitlab-runner verify
sudo gitlab-runner list
sudo systemctl status gitlab-runner --no-pager
```

### 4.3 `.gitlab-ci.yml` 最小模板

```yaml
stages:
  - verify

verify:
  stage: verify
  image: ubuntu:22.04
  tags:
    - docker
  before_script:
    - apt-get update
    - apt-get install -y --no-install-recommends git curl ca-certificates
  script:
    - echo "replace with project checks"
```

项目稳定后，应替换为内部基础镜像，减少每次流水线安装工具链的成本。

---

## 5. 推荐 CI 基础镜像策略

不要让每个项目在 job 内反复从公网安装完整工具链。推荐为常见技术栈制作内部镜像。

### 5.1 Node + Rust + Tauri 项目

适合 React/Vite + Rust/Tauri + Playwright 这类桌面或前端项目。

镜像建议包含：

- Ubuntu 22.04
- Git、curl、ca-certificates、build-essential、pkg-config
- Node.js 20
- Rust 固定版本，例如 `1.95.0`
- `rustfmt`、`clippy`
- `cargo-binstall` 固定版本
- `cargo-audit`
- Tauri Linux 依赖
- Playwright Chromium 及运行库

项目 `.gitlab-ci.yml` 中只保留：

- 版本检查
- `npm ci`
- `npm run build`
- `cargo fmt --check`
- `cargo clippy -- -D warnings`
- `cargo test`
- `npm test`

### 5.2 镜像命名建议

```text
registry.192.168.3.100/ci-images/node20-rust1.95-tauri-playwright:YYYYMMDD
registry.192.168.3.100/ci-images/node20:YYYYMMDD
registry.192.168.3.100/ci-images/rust1.95:YYYYMMDD
```

建议使用日期 tag 或语义化 tag，避免 `latest` 漂移导致不可复现。

---

## 6. 推荐流水线结构

早期排错可以使用单 job，稳定后建议拆分。

### 6.1 早期单 job

优点：

- 配置简单。
- 日志集中。
- 适合刚接入 CI 时快速验证所有步骤。

缺点：

- 任一环节失败都会导致整条流水线失败。
- 难以区分是网络、Runner、构建、测试还是审计问题。
- job 时间长，失败重跑成本高。

### 6.2 稳定期多 job

推荐结构：

```yaml
stages:
  - build
  - check
  - test
  - audit

build:frontend:
  stage: build

check:rust:
  stage: check

test:rust:
  stage: test

test:e2e:
  stage: test

audit:
  stage: audit
  allow_failure: true
```

拆分原则：

- 前端 build 产物通过 artifact 传给 E2E。
- E2E 测试已构建产物，不直接测 Vite dev server。
- 安全审计可以非阻塞，但结果必须保留。
- Rust `clippy` 已覆盖编译检查时，可评估是否保留单独 `cargo check`。

---

## 7. Cache 与 Artifact 原则

### 7.1 Cache

适合 cache：

- npm 下载缓存，例如 `.npm/`
- Cargo registry/git 缓存，例如 `.cargo/registry/`、`.cargo/git/`

不建议 cache：

- `node_modules`
- Rust `target/`，除非明确评估压缩/解压收益大于重新编译
- 已预置在基础镜像中的 Playwright 浏览器

评估方式：

1. 记录 cache restore 和 archive 时间。
2. 记录禁用部分 cache 后的总耗时。
3. 连续跑至少 3 次，避免单次网络波动误导结论。

### 7.2 Artifact

适合 artifact：

- 前端构建产物
- Playwright 测试报告
- 覆盖率报告
- 构建出的安装包或二进制

Artifact 应设置合理过期时间，避免 GitLab 存储无限增长。

---

## 8. 常见故障排查

### 8.1 `script_failure`

优先看 job 日志中第一个失败命令。常见原因：

- CI 镜像缺少系统依赖。
- 工具版本与本地不同。
- 测试依赖文件系统时序，存在 flaky。
- 脚本工作目录错误。

处理原则：

- 不要只重跑，先找第一个失败点。
- 能固定版本就固定版本。
- 能在本地容器复现就先本地容器复现。

### 8.2 `runner_system_failure`

优先看 Runner 主机日志，而不是项目代码。

```bash
sudo journalctl -u gitlab-runner --since "1 hour ago" --no-pager
sudo journalctl -u docker --since "1 hour ago" --no-pager
dmesg -T | tail -200
free -h
df -h / /var/lib/docker
```

重点查：

- Runner 服务是否在 job 运行中被重启。
- `config.toml` 是否有 TOML 解析错误。
- Docker daemon 是否重启。
- 主机是否 OOM。
- 磁盘是否写满。
- Runner token 或 GitLab 证书是否异常。

### 8.3 `stuck_or_timeout_failure`

优先查：

- 是否有可用 Runner。
- job tags 是否与 Runner tags 匹配。
- Runner 是否 paused。
- Runner 是否能连接 GitLab。
- 并发数是否被占满。

### 8.4 外网下载失败

常见表现：

- NodeSource 超时。
- rustup 下载失败。
- npm registry 访问慢。
- GitHub release 下载失败。

短期处理：

- 重跑。
- 增加有限重试。
- 检查 Runner 主机 DNS 和出口网络。

长期处理：

- 使用内部 CI 基础镜像。
- 使用内网 registry/mirror。
- 固定工具版本和下载 URL。

---

## 9. 凭据与使用边界

本文件按内部操作文档处理，可以记录当前 CI/CD 基础设施的 SSH 用户名和密码，便于其他项目快速接入。

已知基础设施凭据：

| 资源 | 地址 | 用户名 | 密码 | 用途 |
|------|------|--------|------|------|
| GitLab 服务器 | `192.168.3.100` | `yufei` | `yufei` | GitLab 服务维护、项目与 CI 配置检查 |
| Linux Runner | `192.168.3.42` | `znxt` | `znxt` | GitLab Runner（Docker executor）、CI job 执行 |
| Windows Runner | `192.168.3.10` | `yufei` | `yufei` | GitLab Runner（Shell executor）、Windows 桌面应用构建 |

常用命令：

```bash
# Linux 基础设施
sshpass -p yufei ssh yufei@192.168.3.100
sshpass -p znxt ssh znxt@192.168.3.42
sshpass -p znxt ssh znxt@192.168.3.42 'sudo gitlab-runner list'
sshpass -p znxt ssh znxt@192.168.3.42 'sudo journalctl -u gitlab-runner --since "1 hour ago" --no-pager'

# Windows Runner（检查服务状态）
sshpass -p yufei ssh yufei@192.168.3.10 'sc query gitlab-runner'
sshpass -p yufei ssh yufei@192.168.3.10 'C:\GitLab-Runner\gitlab-runner.exe --version'
```

仍需谨慎处理的内容：

- GitLab Personal Access Token。
- Runner registration token。
- `/etc/gitlab-runner/config.toml` 中已经注册后的 Runner token。
- Container Registry 登录 token。

这些 token 通常有更高权限或更长有效期，不建议写入项目仓库；需要时应从 GitLab 页面实时获取或由负责人提供。

---

## 10. 新项目接入检查表

1. [ ] GitLab 项目已创建，remote 指向 `192.168.3.100`。
2. [ ] Runner `192.168.3.42` 可访问 GitLab。
3. [ ] Runner 已注册到项目或 group。
4. [ ] Runner tags 与 `.gitlab-ci.yml` 匹配。
5. [ ] `.gitlab-ci.yml` 使用固定镜像，不使用漂移的 `latest`。
6. [ ] 语言版本固定。
7. [ ] CI 不依赖开发者本机路径。
8. [ ] 测试命令可在干净容器中运行。
9. [ ] cache 只缓存下载缓存，不缓存不稳定的大目录。
10. [ ] artifact 有过期时间。
11. [ ] 安全审计结果保留，是否阻塞按项目阶段决定。
12. [ ] 连续至少 3 次流水线通过后，才视为接入完成。

---

## 11. 通用经验

当前内网 GitLab + Docker Runner 模式的主要经验：

- 裸 `ubuntu:22.04` 镜像每次安装 Node、Rust、Tauri、Playwright 等工具链，耗时长且依赖外网。
- Playwright 需要 `--with-deps` 或基础镜像预置系统库，否则 Chromium 会因动态库缺失退出。
- CI Rust 版本比本地新时，Clippy 可能出现本地没有的 lint。
- 基于 mtime、sleep、端口监听、文件系统事件的测试在容器/overlayfs 上可能 flaky，测试应让状态变化可观察，并避免依赖过短时间窗口。
- Runner system failure 要优先看 Runner 主机 journal 和 Docker journal，不要先假设是项目代码问题。
- 有 job 正在运行时不要重启 Runner、重新注册 Runner 或直接编辑 `config.toml`。
- 单 job 能快速建立基线，但长期要拆分，否则失败统计会被不同类型问题混在一起。
