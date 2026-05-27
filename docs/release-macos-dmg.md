# macOS DMG 手动构建与发布

> **此文档已废弃。** 请改用 [`docs/release-macos.md`](release-macos.md)，其中包含 DMG + Portable ZIP 两种产物的完整说明。
> 
> `scripts/release-macos-dmg.sh` 也已废弃，请改用 `scripts/release-macos.sh`。

本文档说明如何在本地 macOS 机器上构建 AgentScope DMG 安装包，并手动上传到 GitLab Package Registry 和 Release。

## 前提条件

- macOS 10.15+ （与 Tauri minimumSystemVersion 一致）
- Xcode Command Line Tools: `xcode-select --install`
- Node.js 20+
- Rust 工具链: `rustup`
- 项目依赖已安装: `npm install`

### Rust Target（可选）

构建 universal binary（同时支持 Apple Silicon 和 Intel）需要安装两个 target：

```bash
rustup target add aarch64-apple-darwin x86_64-apple-darwin
```

如果只安装一个 target，脚本会自动按实际架构构建单架构版本。

## 版本一致性检查

**`--tag` 为必填参数。** 脚本启动时会打印三项版本信息：

| 字段 | 来源 | 说明 |
|:---|:---|:---|
| Release tag | `--tag` 参数 | 上传目标 |
| Package version | tag 去掉 `v` 前缀 | 上传路径中的版本号 |
| tauri.conf.json | 自动解析 | 构建产物的内部版本号 |

**默认行为：如果 tauri.conf.json 版本与 `--tag` 不一致，脚本直接退出，不构建、不重命名、不上传。**

原因：构建产物内部版本号为 tauri.conf.json 中的值，如果与 Release tag 不一致，用户看到的版本号与实际运行版本不符，影响版本识别和安装判断。

### 给旧版 Release 补 DMG（推荐做法）

```bash
# 1. 先 checkout 对应 tag 的源码
git checkout v0.3.4

# 2. 安装依赖并构建
npm ci

# 3. 构建并上传（此时 tauri.conf.json 版本与 --tag 一致，脚本正常通过）
GITLAB_TOKEN=xxx GITLAB_PROJECT_ID=123 ./scripts/release-macos-dmg.sh --tag v0.3.4

# 4. 回到 main 分支
git checkout main
```

### 强行覆盖版本不一致（危险，不推荐）

```bash
# 仅在无法 checkout 对应源码且确认影响可控时使用
./scripts/release-macos-dmg.sh --tag v0.3.4 --allow-version-mismatch
```

`--allow-version-mismatch` 允许在版本不一致时继续，但脚本会打印强警告。用户看到的 DMG 文件名版本与实际运行版本不一致，可能导致混淆。

## 构建 DMG

### 完整构建+上传

```bash
GITLAB_TOKEN=xxx GITLAB_PROJECT_ID=123 ./scripts/release-macos-dmg.sh --tag v0.3.4
```

### 只构建，不上传

```bash
./scripts/release-macos-dmg.sh --skip-upload --tag v0.3.4
```

### 只上传已有的 DMG（给既有 Release 补 DMG）

```bash
GITLAB_TOKEN=xxx GITLAB_PROJECT_ID=123 ./scripts/release-macos-dmg.sh --skip-build --tag v0.3.4
```

### 预览操作（dry-run，不需要 GITLAB_TOKEN）

```bash
./scripts/release-macos-dmg.sh --dry-run --tag v0.3.4 --project-id 123
```

dry-run 不要求 `GITLAB_TOKEN`，输出不包含 token 内容。

## 产物路径

| 构建目标 | 产物路径 |
|:---|:---|
| universal | `src-tauri/target/universal-apple-darwin/release/bundle/dmg/` |
| Apple Silicon | `src-tauri/target/aarch64-apple-darwin/release/bundle/dmg/` |
| Intel | `src-tauri/target/x86_64-apple-darwin/release/bundle/dmg/` |
| 默认架构 | `src-tauri/target/release/bundle/dmg/` |

## 文件命名

| 架构 | 文件名 |
|:---|:---|
| Universal | `AgentScope_{version}_universal.dmg` |
| Apple Silicon | `AgentScope_{version}_aarch64.dmg` |
| Intel | `AgentScope_{version}_x64.dmg` |

脚本会自动检测构建产物的架构，并重命名为规范文件名。

## 上传到 GitLab Package Registry

脚本使用与 CI release job 一致的 generic package 路径：

```
/api/v4/projects/{project_id}/packages/generic/agent-scope/{version}/AgentScope_{version}_{arch}.dmg
```

上传需要 GitLab Personal Access Token，通过环境变量 `GITLAB_TOKEN` 传入。Token 需要 `api` 权限范围。

**Token 安全：** 脚本不会在输出中打印 token 值。dry-run 模式不需要 token，也不暴露 token。

### 获取 Project ID

在 GitLab 项目页面 → Settings → General 中可以看到 Project ID。

也可以通过 API 查询：

```bash
curl -k --header "PRIVATE-TOKEN: $GITLAB_TOKEN" \
  "${GITLAB_URL}/api/v4/projects/znxt_tools%2Fagent-scope" | jq .id
```

## 添加到 GitLab Release

脚本在上传成功后，会自动向对应 tag 的 Release 添加 asset link：

- **名称**: macOS DMG
- **类型**: package
- **URL**: Package Registry 中的文件 URL

前提是对应 tag 的 Release 已经存在（由 CI release job 创建）。

### 幂等性

脚本支持重复运行。如果 Release 中已存在名为 "macOS DMG" 的 asset link，脚本会先删除旧的再创建新的，不会因为重复运行而报错或产生重复 link。

### 手动添加（如脚本失败）

```bash
GITLAB_URL="https://your-gitlab-host"
PROJECT_ID="123"
TAG="v0.3.4"
VERSION="0.3.4"
ARCH="universal"

curl -k \
  --header "PRIVATE-TOKEN: $GITLAB_TOKEN" \
  --data-urlencode "name=macOS DMG" \
  --data-urlencode "url=${GITLAB_URL}/api/v4/projects/${PROJECT_ID}/packages/generic/agent-scope/${VERSION}/AgentScope_${VERSION}_${ARCH}.dmg" \
  --data-urlencode "link_type=package" \
  "${GITLAB_URL}/api/v4/projects/${PROJECT_ID}/releases/${TAG}/assets/links"
```

## 网络地址

`GITLAB_URL` 默认为 `https://192.168.3.100`（内网地址）。如果本机不在同一内网，需设置为可访问的地址，例如通过 Tailscale：

```bash
export GITLAB_URL="https://your-tailscale-host"  # 按实际可访问地址修改
```

## 环境变量

| 变量 | 必填条件 | 默认值 | 说明 |
|:---|:---|:---|:---|
| `GITLAB_TOKEN` | 非 dry-run 上传时必填 | — | GitLab Personal Access Token（`api` 权限） |
| `GITLAB_PROJECT_ID` | 上传时必填 | — | GitLab 项目 ID，也可用 `--project-id` 参数 |
| `GITLAB_URL` | 否 | `https://192.168.3.100` | GitLab 实例地址，按网络环境调整 |

## 脚本参数

| 参数 | 说明 |
|:---|:---|
| `--tag VERSION` | **必填**。指定 release tag（如 `v0.3.4`） |
| `--skip-build` | 跳过构建，只上传已有的 DMG |
| `--skip-upload` | 跳过上传，只构建 |
| `--project-id ID` | GitLab 项目 ID |
| `--dry-run` | 只打印脱敏操作说明，不实际执行，不要求 GITLAB_TOKEN |
| `--allow-version-mismatch` | 允许 tauri.conf.json 版本与 --tag 不一致时继续（危险，见文档） |

## 未签名/未公证限制

macOS DMG 中的 `.app` 未经过 Apple 代码签名和公证。用户安装时会遇到以下情况：

1. **Gatekeeper 阻止打开**: 双击 `.app` 会提示"无法验证开发者"
2. **解决方法**: 右键点击 `.app` → 选择"打开" → 在弹出的对话框中点击"打开"
3. **命令行绕过**: `xattr -cr /Applications/AgentScope.app`

### 未来如需签名和公证

1. 加入 Apple Developer Program（$99/年）
2. 获取 Developer ID Application 证书
3. 在构建时传入签名配置：

```bash
# 签名
npm run tauri build -- --target universal-apple-darwin

# tauri.conf.json 中配置:
# "macOS": {
#   "signingIdentity": "Developer ID Application: ...",
#   "entitlements": null
# }

# 公证
xcrun notarytool submit <dmg> --apple-id <id> --team-id <team> --password <app-specific-password>
xcrun stapler staple <dmg>
```

## 完整操作流程

```
1. 确认版本号已更新（src-tauri/tauri.conf.json / package.json）
2. 提交代码，打 tag：scripts/release-tag.sh v0.3.4
3. 等待 CI 完成 Linux/Windows 构建和 Release 创建
4. 在本地执行构建+上传：
   GITLAB_TOKEN=xxx GITLAB_PROJECT_ID=123 ./scripts/release-macos-dmg.sh --tag v0.3.4
5. 在 GitLab Release 页面确认 "macOS DMG" asset link 已添加
```

给既有 Release 补 DMG（推荐做法）：

```
1. checkout 对应 tag 源码：git checkout v0.3.4
2. 安装依赖：npm ci
3. 构建并上传：
   GITLAB_TOKEN=xxx GITLAB_PROJECT_ID=123 ./scripts/release-macos-dmg.sh --tag v0.3.4
4. 回到 main：git checkout main
```

## 故障排除

### 构建失败：缺少 Xcode Command Line Tools

```
error: xcrun: error: unable to lookup item ...
```

解决：`xcode-select --install`

### 构建失败：缺少 Rust target

```
error: can't find crate for `core`
```

解决：`rustup target add aarch64-apple-darwin`（或 `x86_64-apple-darwin`）

### 版本不一致被阻止

```
❌ 阻止: tauri.conf.json 版本 (0.3.1) 与 --tag 版本 (0.3.4) 不一致。
```

推荐做法：`git checkout v0.3.4` 后重新构建，确保 tauri.conf.json 版本与 --tag 一致。

不推荐做法：加 `--allow-version-mismatch` 强行继续，会导致 DMG 文件名版本与实际运行版本不一致。

### 上传失败：401 Unauthorized

检查 `GITLAB_TOKEN` 是否正确，是否具有 `api` 权限。

### 上传失败：404 Project Not Found

检查 `GITLAB_PROJECT_ID` 是否正确。也可以通过路径查找：

```bash
curl -k --header "PRIVATE-TOKEN: $GITLAB_TOKEN" \
  "${GITLAB_URL}/api/v4/projects/znxt_tools%2Fagent-scope" | jq .id
```

### Release asset link 添加失败

确认对应 tag 的 Release 已被 CI 创建。CI release job 在 tag push 后触发，可能需要等待几分钟。

脚本支持重复运行。如果已存在 "macOS DMG" link，脚本会先删除再创建。

### 构建产物中找不到 DMG

检查 `src-tauri/target/` 下的对应架构目录。脚本通过 `--bundles dmg` 参数指定 DMG 格式，不依赖 `tauri.conf.json` 中的 `bundle.targets` 配置。