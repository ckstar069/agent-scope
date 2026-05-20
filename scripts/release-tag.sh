#!/usr/bin/env bash
#
# AgentScope 发布 tag 脚本
# 只创建 lightweight tag，禁止 annotated tag，避免 Windows Runner PowerShell ParserError
#
# 用法: scripts/release-tag.sh vX.Y.Z [commit]
#

set -euo pipefail

usage() {
    cat <<'EOF'
Usage: scripts/release-tag.sh vX.Y.Z [commit]

创建一个 lightweight tag 并推送到 origin。

参数:
  vX.Y.Z      语义化版本号，必须以 v 开头，格式为 v<major>.<minor>.<patch>
  commit      可选，要打上 tag 的 commit，默认为 HEAD

示例:
  scripts/release-tag.sh v0.2.15
  scripts/release-tag.sh v0.2.15 6bc23fa

限制:
  - 只创建 lightweight tag，禁止使用 annotated tag
  - 如果本地或远端 tag 已存在，立即失败
EOF
}

if [[ $# -eq 0 ]] || [[ "${1:-}" == "--help" ]] || [[ "${1:-}" == "-h" ]]; then
    usage
    exit 1
fi

TAG="$1"
COMMIT="${2:-HEAD}"

# 检查当前目录是 git repo
if ! git rev-parse --git-dir > /dev/null 2>&1; then
    echo "错误: 当前目录不是 git 仓库" >&2
    exit 1
fi

# 检查 origin remote 是否存在
if ! git remote get-url origin > /dev/null 2>&1; then
    echo "错误: origin remote 不存在" >&2
    exit 1
fi

# 验证 tag 格式: 必须是 semver vX.Y.Z 或 vX.Y.Z-prerelease
if [[ ! "$TAG" =~ ^v[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$ ]]; then
    echo "错误: tag 格式无效 '$TAG'，必须是 vX.Y.Z 或 vX.Y.Z-prerelease 格式（例如 v0.2.15, v0.2.2-rc.1）" >&2
    exit 1
fi

# 验证 commit 存在
if ! git rev-parse --verify --quiet "$COMMIT" > /dev/null; then
    echo "错误: commit '$COMMIT' 不存在" >&2
    exit 1
fi

COMMIT_SHA=$(git rev-parse "$COMMIT")
COMMIT_MSG=$(git log -1 --format=%s "$COMMIT_SHA")

# 同步远端 tag，确保检查最新状态
echo "==> 同步远端 tag..."
git fetch --tags origin

# 检查本地 tag 是否已存在
if git rev-parse --verify --quiet "$TAG" > /dev/null; then
    echo "错误: 本地 tag '$TAG' 已存在 ($(git rev-parse "$TAG"))" >&2
    exit 1
fi

# 检查远端 tag 是否已存在
if git ls-remote --tags origin "refs/tags/$TAG" | grep -q "refs/tags/$TAG"; then
    echo "错误: 远端 tag '$TAG' 已存在" >&2
    exit 1
fi

# 检查 staged changes
if ! git diff --cached --quiet; then
    echo "错误: working tree 存在 staged changes，请先提交或取消暂存" >&2
    git diff --cached --stat >&2
    exit 1
fi

# 提醒 untracked 文件（不阻塞）
UNTRACKED=$(git ls-files --others --exclude-standard)
if [[ -n "$UNTRACKED" ]]; then
    echo "==> 提醒: working tree 存在 untracked 文件（不阻塞发布）:"
    echo "$UNTRACKED" | sed 's/^/    /'
fi

echo ""
echo "==> 准备创建 lightweight tag:"
echo "    tag:    $TAG"
echo "    commit: $COMMIT_SHA"
echo "    msg:    $COMMIT_MSG"
echo ""

# 创建 lightweight tag
git tag "$TAG" "$COMMIT_SHA"
echo "==> 本地 tag '$TAG' 已创建"

# 推送
echo "==> 推送到 origin..."
git push origin "$TAG"
echo "==> 远端 tag '$TAG' 推送成功"
