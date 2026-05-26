#!/usr/bin/env bash
# macOS DMG 手动构建 + 上传到 GitLab Package Registry + 添加 Release asset link
#
# 用法:
#   ./scripts/release-macos-dmg.sh [选项]
#
# 选项:
#   --tag VERSION            指定 release tag（如 v0.3.4），必填
#   --project-id ID          GitLab 项目 ID（默认从环境变量 GITLAB_PROJECT_ID 读取）
#   --skip-build             跳过构建，只上传已有的 DMG
#   --skip-upload            跳过上传，只构建
#   --dry-run                只打印脱敏操作说明，不实际执行
#   --allow-version-mismatch 允许 tauri.conf.json 版本与 --tag 不一致（危险，见文档）
#
# 环境变量:
#   GITLAB_TOKEN       GitLab Personal Access Token（非 dry-run 上传时必填，需 api 权限）
#   GITLAB_PROJECT_ID  GitLab 项目 ID（上传时必填，或用 --project-id 指定）
#   GITLAB_URL         GitLab 实例地址（默认 https://192.168.3.100）
#
# 示例:
#   # 完整构建+上传到 v0.3.4 Release
#   GITLAB_TOKEN=xxx GITLAB_PROJECT_ID=123 ./scripts/release-macos-dmg.sh --tag v0.3.4
#
#   # 只构建不上传
#   ./scripts/release-macos-dmg.sh --skip-upload --tag v0.3.4
#
#   # 给既有 Release 补 DMG（推荐先 checkout 对应 tag 源码）
#   git checkout v0.3.4
#   GITLAB_TOKEN=xxx GITLAB_PROJECT_ID=123 ./scripts/release-macos-dmg.sh --tag v0.3.4
#
#   # dry-run 预览流程（不需要 GITLAB_TOKEN）
#   ./scripts/release-macos-dmg.sh --dry-run --tag v0.3.4 --project-id 123
#
# 重要:
#   --tag 必填。如果 tauri.conf.json 版本与 --tag 不一致，脚本默认退出。
#   给旧版 Release 补 DMG 时，推荐 checkout 对应 tag 源码后再构建，
#   而不是使用 --allow-version-mismatch 强行覆盖。

set -euo pipefail

# ── 默认值 ──
GITLAB_URL="${GITLAB_URL:-https://192.168.3.100}"
SKIP_BUILD=false
SKIP_UPLOAD=false
TAG=""
PROJECT_ID="${GITLAB_PROJECT_ID:-}"
DRY_RUN=false
ALLOW_MISMATCH=false

# ── 参数解析 ──
while [[ $# -gt 0 ]]; do
  case $1 in
    --skip-build)             SKIP_BUILD=true; shift ;;
    --skip-upload)            SKIP_UPLOAD=true; shift ;;
    --tag)                    TAG="$2"; shift 2 ;;
    --project-id)             PROJECT_ID="$2"; shift 2 ;;
    --dry-run)                DRY_RUN=true; shift ;;
    --allow-version-mismatch) ALLOW_MISMATCH=true; shift ;;
    -h|--help)
      sed -n '2,/^$/p' "$0" | sed 's/^# \?//'
      exit 0
      ;;
    *)
      echo "未知参数: $1" >&2
      exit 1
      ;;
  esac
done

# ── 辅助函数 ──
die() {
  echo "错误: $*" >&2
  exit 1
}

run_cmd() {
  if $DRY_RUN; then
    echo "[DRY RUN] $*"
  else
    echo "$ $*"
    "$@"
  fi
}

# 读取 tauri.conf.json 中的 version 字段
read_conf_version() {
  local conf="$1"
  local raw
  raw=$(python3 -c "
import json
with open('$conf') as f:
    d = json.load(f)
print(d.get('version', ''))
" 2>/dev/null || jq -r '.version // ""' "$conf" 2>/dev/null || echo "")
  echo "$raw"
}

# 从 package.json 读取实际版本号（处理 "../package.json" 引用）
resolve_package_version() {
  local conf_dir="$1"
  local raw="$2"

  if [[ "$raw" == ../* ]]; then
    local pkg="$conf_dir/$raw"
    if [[ -f "$pkg" ]]; then
      jq -r '.version' "$pkg" 2>/dev/null || python3 -c "
import json
with open('$pkg') as f:
    d = json.load(f)
print(d.get('version', ''))
" 2>/dev/null || echo ""
      return
    fi
  fi

  echo "$raw"
}

# 上传文件到 GitLab Generic Package Registry
# 只输出 URL、HTTP code、文件名，不泄露 token
upload_file() {
  local file="$1"
  local url="$2"

  if $DRY_RUN; then
    echo "[DRY RUN] 上传 $(basename "$file") → $url"
    return 0
  fi

  local http_code
  http_code=$(curl -k \
    --fail-with-body \
    -o /dev/null \
    -w "%{http_code}" \
    --header "PRIVATE-TOKEN: $GITLAB_TOKEN" \
    --upload-file "$file" \
    "$url" 2>/dev/null) || true

  if [[ "$http_code" == "201" || "$http_code" == "200" ]]; then
    echo "上传成功 (HTTP $http_code): $(basename "$file")"
    return 0
  else
    echo "上传失败 (HTTP ${http_code:-???}): $(basename "$file")" >&2
    return 1
  fi
}

# GitLab API: GET
api_get() {
  local url="$1"
  curl -k -s --fail-with-body \
    --header "PRIVATE-TOKEN: $GITLAB_TOKEN" \
    "$url" 2>/dev/null
}

# GitLab API: DELETE
api_delete() {
  local url="$1"

  if $DRY_RUN; then
    echo "[DRY RUN] DELETE $url"
    return 0
  fi

  local http_code
  http_code=$(curl -k -s -o /dev/null -w "%{http_code}" \
    --request DELETE \
    --header "PRIVATE-TOKEN: $GITLAB_TOKEN" \
    "$url" 2>/dev/null) || true

  if [[ "$http_code" == "204" || "$http_code" == "200" ]]; then
    return 0
  else
    echo "DELETE 失败 (HTTP ${http_code:-???}): $url" >&2
    return 1
  fi
}

# GitLab API: POST (form-urlencoded)
api_post() {
  local url="$1"
  shift

  if $DRY_RUN; then
    local desc="POST $url"
    for pair in "$@"; do
      desc="$desc  $pair"
    done
    echo "[DRY RUN] $desc"
    return 0
  fi

  local -a data_args=()
  for pair in "$@"; do
    data_args+=("--data-urlencode" "$pair")
  done

  local http_code
  http_code=$(curl -k -s -o /dev/null -w "%{http_code}" \
    --request POST \
    --header "PRIVATE-TOKEN: $GITLAB_TOKEN" \
    "${data_args[@]}" \
    "$url" 2>/dev/null) || true

  if [[ "$http_code" == "201" || "$http_code" == "200" ]]; then
    return 0
  else
    echo "POST 失败 (HTTP ${http_code:-???}): $url" >&2
    return 1
  fi
}

# ── 版本处理 ──
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
CONF="$PROJECT_ROOT/src-tauri/tauri.conf.json"

CONF_VERSION_RESOLVED=""
if [[ -f "$CONF" ]]; then
  CONF_VERSION_RAW=$(read_conf_version "$CONF")
  if [[ -n "$CONF_VERSION_RAW" ]]; then
    CONF_VERSION_RESOLVED=$(resolve_package_version "$PROJECT_ROOT/src-tauri" "$CONF_VERSION_RAW")
  fi
fi

# --tag 必填
if [[ -z "$TAG" ]]; then
  echo "错误: 必须指定 --tag（如 --tag v0.3.4）" >&2
  echo "" >&2
  echo "原因: tauri.conf.json 中的版本号可能与目标 Release tag 不一致。" >&2
  echo "给既有 Release 补 DMG 时，必须显式传 --tag 以确保上传到正确的 Release。" >&2
  exit 1
fi

# 验证 tag 格式
if [[ ! "$TAG" =~ ^v[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$ ]]; then
  die "tag 格式无效 '$TAG'，必须是 vX.Y.Z 格式"
fi

VERSION="${TAG#v}"

# 打印版本信息
echo "═══ 版本信息 ═══"
echo "Release tag:          $TAG"
echo "Package version:      $VERSION"
if [[ -n "$CONF_VERSION_RESOLVED" ]]; then
  echo "tauri.conf.json:      $CONF_VERSION_RESOLVED"
fi

# 版本不一致检查：默认阻止，--allow-version-mismatch 才放行
if [[ -n "$CONF_VERSION_RESOLVED" && "$CONF_VERSION_RESOLVED" != "$VERSION" ]]; then
  echo ""
  if ! $ALLOW_MISMATCH; then
    echo "❌ 阻止: tauri.conf.json 版本 ($CONF_VERSION_RESOLVED) 与 --tag 版本 ($VERSION) 不一致。" >&2
    echo "" >&2
    echo "构建产物的内部版本号为 ${CONF_VERSION_RESOLVED}，但上传路径和 Release tag 为 ${TAG}。" >&2
    echo "这会导致 DMG 文件名与实际内容版本不符，影响用户安装和版本识别。" >&2
    echo "" >&2
    echo "推荐做法: 先 checkout 对应 tag 的源码，再构建：" >&2
    echo "  git checkout v${VERSION}" >&2
    echo "  npm ci && npm run tauri build -- --bundles dmg" >&2
    echo "" >&2
    echo "如确认要强行继续（危险），请传 --allow-version-mismatch。" >&2
    exit 1
  fi

  echo "🚨  强警告: tauri.conf.json 版本 ($CONF_VERSION_RESOLVED) 与 --tag 版本 ($VERSION) 不一致！"
  echo "   构建产物内部版本号为 ${CONF_VERSION_RESOLVED}，但 DMG 文件名将为 AgentScope_${VERSION}_<arch>.dmg"
  echo "   用户看到的版本号与实际运行版本不一致，可能导致混淆。"
  echo "   推荐做法: 先 checkout v${VERSION} 源码，再构建。"
  echo ""
fi

# ── 构建 DMG ──
if ! $SKIP_BUILD; then
  echo ""
  echo "═══ 构建 macOS DMG ═══"

  if ! command -v rustup &>/dev/null; then
    die "未安装 rustup，请先安装: https://rustup.rs"
  fi

  HAS_AARCH64=$(rustup target list --installed | grep -c 'aarch64-apple-darwin' || true)
  HAS_X64=$(rustup target list --installed | grep -c 'x86_64-apple-darwin' || true)

  BUILD_TARGET=""
  if [[ "$HAS_AARCH64" -gt 0 && "$HAS_X64" -gt 0 ]]; then
    echo "检测到 aarch64 + x86_64 双目标，构建 universal binary"
    BUILD_TARGET="universal-apple-darwin"
  elif [[ "$HAS_AARCH64" -gt 0 ]]; then
    echo "仅检测到 aarch64 目标，构建 Apple Silicon 版本"
    BUILD_TARGET="aarch64-apple-darwin"
  elif [[ "$HAS_X64" -gt 0 ]]; then
    echo "仅检测到 x86_64 目标，构建 Intel 版本"
    BUILD_TARGET="x86_64-apple-darwin"
  else
    echo "未检测到额外 Rust target，使用当前架构构建"
    BUILD_TARGET=""
  fi

  cd "$PROJECT_ROOT"

  if [[ -n "$BUILD_TARGET" && "$BUILD_TARGET" == "universal-apple-darwin" ]]; then
    run_cmd npm run tauri build -- --target universal-apple-darwin --bundles dmg
  elif [[ -n "$BUILD_TARGET" ]]; then
    run_cmd npm run tauri build -- --target "$BUILD_TARGET" --bundles dmg
  else
    run_cmd npm run tauri build -- --bundles dmg
  fi

  echo ""
  echo "构建完成"
fi

# ── 定位产物 ──
DMG_DIR="$PROJECT_ROOT/src-tauri/target"

SEARCH_PATHS=(
  "$DMG_DIR/universal-apple-darwin/release/bundle/dmg/*.dmg"
  "$DMG_DIR/aarch64-apple-darwin/release/bundle/dmg/*.dmg"
  "$DMG_DIR/x86_64-apple-darwin/release/bundle/dmg/*.dmg"
  "$DMG_DIR/release/bundle/dmg/*.dmg"
)

DMG_FILE=""
for path in "${SEARCH_PATHS[@]}"; do
  files=( $path )
  if [[ ${#files[@]} -gt 0 && -f "${files[0]}" ]]; then
    DMG_FILE="${files[0]}"
    break
  fi
done

if [[ -z "$DMG_FILE" ]]; then
  echo "" >&2
  echo "未找到 DMG 文件，已搜索以下路径:" >&2
  for path in "${SEARCH_PATHS[@]}"; do
    echo "  $path" >&2
  done
  die "请确认构建是否成功"
fi

echo ""
echo "═══ 产物 ═══"
echo "文件: $DMG_FILE"
DMG_SIZE=$(du -h "$DMG_FILE" | cut -f1)
echo "大小: $DMG_SIZE"

# 确定架构后缀
BASENAME="$(basename "$DMG_FILE")"
if echo "$BASENAME" | grep -q '_universal'; then
  ARCH="universal"
elif echo "$BASENAME" | grep -q '_aarch64'; then
  ARCH="aarch64"
elif echo "$BASENAME" | grep -q '_x64'; then
  ARCH="x64"
elif [[ "$(uname -m)" == "arm64" ]]; then
  ARCH="aarch64"
else
  ARCH="x64"
fi

# 规范化文件名：AgentScope_{version}_{arch}.dmg
EXPECTED_NAME="AgentScope_${VERSION}_${ARCH}.dmg"
if [[ "$BASENAME" != "$EXPECTED_NAME" ]]; then
  RENAMED_PATH="$(dirname "$DMG_FILE")/$EXPECTED_NAME"
  echo "重命名: $BASENAME → $EXPECTED_NAME"
  run_cmd mv "$DMG_FILE" "$RENAMED_PATH"
  DMG_FILE="$RENAMED_PATH"
fi

# ── 上传到 Package Registry ──
if ! $SKIP_UPLOAD; then
  echo ""
  echo "═══ 上传到 GitLab Package Registry ═══"

  # dry-run 不要求 token；非 dry-run 才检查
  if ! $DRY_RUN; then
    [[ -n "${GITLAB_TOKEN:-}" ]] || die "GITLAB_TOKEN 环境变量未设置"
    [[ -n "$PROJECT_ID" ]] || die "GITLAB_PROJECT_ID 环境变量未设置（或用 --project-id 指定）"
  else
    echo "[DRY RUN] 不验证 GITLAB_TOKEN 和 PROJECT_ID"
  fi

  PACKAGE_URL="${GITLAB_URL}/api/v4/projects/${PROJECT_ID}/packages/generic/agent-scope/${VERSION}/${EXPECTED_NAME}"

  echo "目标 URL: $PACKAGE_URL"
  upload_file "$DMG_FILE" "$PACKAGE_URL" || die "上传到 Package Registry 失败"

  # ── 添加 Release asset link（幂等） ──
  echo ""
  echo "═══ 添加 Release asset link ═══"

  RELEASE_API="${GITLAB_URL}/api/v4/projects/${PROJECT_ID}/releases/${TAG}"

  if $DRY_RUN; then
    echo "[DRY RUN] 检查 Release ${TAG} 是否存在: GET $RELEASE_API"
  else
    if ! api_get "$RELEASE_API" > /dev/null 2>&1; then
      die "Release ${TAG} 不存在，请先通过 CI 创建 Release"
    fi
    echo "Release ${TAG} 已确认存在"
  fi

  ASSET_API="${GITLAB_URL}/api/v4/projects/${PROJECT_ID}/releases/${TAG}/assets/links"

  # 查询现有 asset links（幂等：先删除同名 link 再创建）
  if $DRY_RUN; then
    echo "[DRY RUN] 查询现有 asset links: GET $ASSET_API"
    echo "[DRY RUN] 如已存在 'macOS DMG' link，将先删除再创建"
  else
    EXISTING_LINKS=$(api_get "$ASSET_API" 2>/dev/null || echo "[]")

    EXISTING_LINK_ID=$(echo "$EXISTING_LINKS" | python3 -c "
import json, sys
links = json.loads(sys.stdin.read())
for l in links:
    if l.get('name') == 'macOS DMG':
        print(l['id'])
        break
" 2>/dev/null || echo "")

    if [[ -n "$EXISTING_LINK_ID" ]]; then
      echo "已有 'macOS DMG' link (id=$EXISTING_LINK_ID)，删除后重新创建"
      api_delete "${ASSET_API}/${EXISTING_LINK_ID}" || die "删除已有 link 失败"
    fi
  fi

  # 创建新的 asset link
  if api_post "$ASSET_API" "name=macOS DMG" "url=${PACKAGE_URL}" "link_type=package"; then
    echo "Release asset link 添加成功: macOS DMG → $PACKAGE_URL"
  else
    die "添加 Release asset link 失败"
  fi
fi

# ── 完成 ──
echo ""
echo "═══ 完成 ═══"
echo "Release tag:  $TAG"
echo "架构:        $ARCH"
echo "产物:        $DMG_FILE"
if ! $SKIP_UPLOAD; then
  echo "下载 URL:    ${GITLAB_URL}/api/v4/projects/${PROJECT_ID}/packages/generic/agent-scope/${VERSION}/${EXPECTED_NAME}"
fi
echo ""
echo "⚠️  注意: 此 DMG 未签名、未公证，用户首次打开需右键 → 打开"