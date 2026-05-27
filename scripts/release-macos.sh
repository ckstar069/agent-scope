#!/usr/bin/env bash
# macOS 构建 + 上传到 GitLab Package Registry + 添加 Release asset link
# 支持 DMG 安装包 和 Portable ZIP 免安装版 两种产物
#
# 用法:
#   ./scripts/release-macos.sh [选项]
#
# 选项:
#   --tag VERSION            指定 release tag（如 v0.3.6），必填
#   --project-id ID          GitLab 项目 ID（默认从环境变量 GITLAB_PROJECT_ID 读取）
#   --skip-build             跳过构建，只上传已有的 DMG 和 Portable ZIP
#   --skip-upload            跳过上传，只构建
#   --dry-run                只打印脱敏操作说明，不实际执行
#   --allow-version-mismatch 允许版本文件与 --tag 不一致（危险，见文档）
#
# 环境变量:
#   GITLAB_TOKEN       GitLab Personal Access Token（非 dry-run 上传时必填，需 api 权限）
#   GITLAB_PROJECT_ID  GitLab 项目 ID（上传时必填，或用 --project-id 指定）
#   GITLAB_URL         GitLab 实例地址（默认 https://192.168.3.100）

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
      sed -n '2,/^$/p' "$0" | sed -E 's/^# ?//'
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

# 读取 package.json 中的 version
read_package_version() {
  local pkg="$1"
  if [[ -f "$pkg" ]]; then
    python3 -c "
import json
with open('$pkg') as f:
    d = json.load(f)
print(d.get('version', ''))
" 2>/dev/null || jq -r '.version // ""' "$pkg" 2>/dev/null || echo ""
  else
    echo ""
  fi
}

# 读取 tauri.conf.json 中的 version（支持路径引用如 ../package.json）
read_tauri_version() {
  local conf="$1"
  local conf_dir="$2"
  if [[ ! -f "$conf" ]]; then
    echo ""
    return
  fi
  local raw
  raw=$(python3 -c "
import json
with open('$conf') as f:
    d = json.load(f)
print(d.get('version', ''))
" 2>/dev/null || jq -r '.version // ""' "$conf" 2>/dev/null || echo "")

  # 如果是路径引用，读取对应文件
  if [[ "$raw" == ../* || "$raw" == ./* ]]; then
    local ref_path="$conf_dir/$raw"
    if [[ -f "$ref_path" ]]; then
      read_package_version "$ref_path"
      return
    fi
  fi
  echo "$raw"
}

# 读取 Cargo.toml 中的 version
read_cargo_version() {
  local cargo="$1"
  if [[ -f "$cargo" ]]; then
    grep '^version = ' "$cargo" 2>/dev/null | head -1 | sed -E 's/version = "([^"]+)"/\1/' || echo ""
  else
    echo ""
  fi
}

# 从文件名中提取 AgentScope_X.Y.Z 中的版本号
extract_file_version() {
  local filename="$1"
  echo "$filename" | grep -oE 'AgentScope_[0-9]+\.[0-9]+\.[0-9]+' | head -1 | sed 's/AgentScope_//' || echo ""
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

# 按精确文件名查找产物（用于 skip-build 严格匹配）
find_exact_file() {
  local dir="$1"
  local name="$2"
  if [[ -f "$dir/$name" ]]; then
    echo "$dir/$name"
    return 0
  fi
  echo ""
}

# 按 glob 查找产物（过滤掉 Tauri 临时文件），返回第一个匹配
find_glob_artifact() {
  local pattern="$1"
  local -a files
  files=( $pattern )
  for f in "${files[@]}"; do
    if [[ -e "$f" && ! "$(basename "$f")" =~ ^rw\.[0-9]+\. ]]; then
      echo "$f"
      return 0
    fi
  done
  echo ""
}

# ── 路径与版本处理 ──
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
PKG_JSON="$PROJECT_ROOT/package.json"
TAURI_CONF="$PROJECT_ROOT/src-tauri/tauri.conf.json"
CARGO_TOML="$PROJECT_ROOT/src-tauri/Cargo.toml"

# 读取各文件版本
PKG_VERSION=$(read_package_version "$PKG_JSON")
TAURI_VERSION=$(read_tauri_version "$TAURI_CONF" "$PROJECT_ROOT/src-tauri")
CARGO_VERSION=$(read_cargo_version "$CARGO_TOML")

# --tag 必填
if [[ -z "$TAG" ]]; then
  echo "错误: 必须指定 --tag（如 --tag v0.3.6）" >&2
  echo "" >&2
  echo "原因: 版本文件中的版本号可能与目标 Release tag 不一致。" >&2
  echo "给既有 Release 补产物时，必须显式传 --tag 以确保上传到正确的 Release。" >&2
  exit 1
fi

# 验证 tag 格式
if [[ ! "$TAG" =~ ^v[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$ ]]; then
  die "tag 格式无效 '$TAG'，必须是 vX.Y.Z 格式"
fi

VERSION="${TAG#v}"

# 确定架构后缀
MACHINE=$(uname -m)
if [[ "$MACHINE" == "arm64" ]]; then
  ARCH="aarch64"
elif [[ "$MACHINE" == "x86_64" ]]; then
  ARCH="x64"
else
  ARCH="$MACHINE"
fi

# 提前计算预期文件名（后续搜索和上传均依赖此名称）
EXPECTED_DMG="AgentScope_${VERSION}_${ARCH}.dmg"
EXPECTED_ZIP="AgentScope_${VERSION}_${ARCH}_portable.zip"

# 打印版本信息
echo "═══ 版本信息 ═══"
echo "Release tag:     $TAG"
echo "Package version: $VERSION"
echo "目标架构:        $ARCH ($MACHINE)"
echo "预期 DMG:        $EXPECTED_DMG"
echo "预期 ZIP:        $EXPECTED_ZIP"
[[ -n "$PKG_VERSION" ]]    && echo "package.json:    $PKG_VERSION"
[[ -n "$TAURI_VERSION" ]]  && echo "tauri.conf.json: $TAURI_VERSION"
[[ -n "$CARGO_VERSION" ]]  && echo "Cargo.toml:      $CARGO_VERSION"

# 版本一致性检查
VERSION_MISMATCH=false
MISMATCH_DETAILS=""

if [[ -n "$PKG_VERSION" && "$PKG_VERSION" != "$VERSION" ]]; then
  VERSION_MISMATCH=true
  MISMATCH_DETAILS="$MISMATCH_DETAILS\n  - package.json ($PKG_VERSION)"
fi
if [[ -n "$TAURI_VERSION" && "$TAURI_VERSION" != "$VERSION" ]]; then
  VERSION_MISMATCH=true
  MISMATCH_DETAILS="$MISMATCH_DETAILS\n  - tauri.conf.json ($TAURI_VERSION)"
fi
if [[ -n "$CARGO_VERSION" && "$CARGO_VERSION" != "$VERSION" ]]; then
  VERSION_MISMATCH=true
  MISMATCH_DETAILS="$MISMATCH_DETAILS\n  - Cargo.toml ($CARGO_VERSION)"
fi

if $VERSION_MISMATCH; then
  echo ""
  if ! $ALLOW_MISMATCH; then
    echo -e "❌ 阻止: 以下版本文件与 --tag 版本 ($VERSION) 不一致:$MISMATCH_DETAILS" >&2
    echo "" >&2
    echo "构建产物的内部版本号与 Release tag 不符，会影响用户安装和版本识别。" >&2
    echo "" >&2
    echo "推荐做法: 先 checkout 对应 tag 的源码，再构建：" >&2
    echo "  git checkout v${VERSION}" >&2
    echo "  npm ci && npm run tauri build -- --bundles app && npm run tauri build -- --bundles dmg" >&2
    echo "" >&2
    echo "如确认要强行继续（危险），请传 --allow-version-mismatch。" >&2
    exit 1
  fi

  echo -e "🚨  强警告: 以下版本文件与 --tag 版本 ($VERSION) 不一致:$MISMATCH_DETAILS"
  echo "   构建产物内部版本号与 Release tag 不符，可能导致用户混淆。"
  echo "   推荐做法: 先 checkout v${VERSION} 源码，再构建。"
  echo ""
fi

# ── 构建 ──
BUNDLE_DIR=""
DMG_FILE=""
APP_DIR=""
ZIP_FILE=""

if ! $SKIP_BUILD; then
  echo ""
  echo "═══ 构建 macOS 产物 ═══"

  if ! command -v rustup &>/dev/null; then
    die "未安装 rustup，请先安装: https://rustup.rs"
  fi

  cd "$PROJECT_ROOT"

  # 分别构建 App bundle 和 DMG，避免产物残留互相干扰
  echo "构建 App bundle..."
  run_cmd npm run tauri build -- --bundles app

  echo ""
  echo "构建 DMG..."
  run_cmd npm run tauri build -- --bundles dmg

  echo ""
  echo "构建完成"
fi

# ── 定位产物 ──
TARGET_DIR="$PROJECT_ROOT/src-tauri/target"

# 各架构可能的产物目录（用于按预期文件名精确查找和通配搜索）
TARGET_SUBDIRS=(
  "universal-apple-darwin/release/bundle"
  "aarch64-apple-darwin/release/bundle"
  "x86_64-apple-darwin/release/bundle"
  "release/bundle"
)

DMG_FILE=""
ZIP_FILE=""
APP_DIR=""

# ── DMG 定位 ──
if $SKIP_BUILD; then
  # skip-build 模式：严格按预期文件名精确查找，不重命名旧版本产物
  for subdir in "${TARGET_SUBDIRS[@]}"; do
    found=$(find_exact_file "$TARGET_DIR/$subdir/dmg" "$EXPECTED_DMG")
    if [[ -n "$found" ]]; then
      DMG_FILE="$found"
      break
    fi
    # Tauri v2 某些配置下 DMG 也可能输出到 macos/ 目录
    found=$(find_exact_file "$TARGET_DIR/$subdir/macos" "$EXPECTED_DMG")
    if [[ -n "$found" ]]; then
      DMG_FILE="$found"
      break
    fi
  done
else
  # 非 skip-build 模式：先尝试按预期文件名查找（Tauri 可能已经输出正确名称）
  for subdir in "${TARGET_SUBDIRS[@]}"; do
    found=$(find_exact_file "$TARGET_DIR/$subdir/dmg" "$EXPECTED_DMG")
    if [[ -n "$found" ]]; then
      DMG_FILE="$found"
      break
    fi
    found=$(find_exact_file "$TARGET_DIR/$subdir/macos" "$EXPECTED_DMG")
    if [[ -n "$found" ]]; then
      DMG_FILE="$found"
      break
    fi
  done

  # 如果没找到精确匹配，再用通配搜索（兼容旧产物命名）
  if [[ -z "$DMG_FILE" ]]; then
    DMG_SEARCH_PATHS=()
    for subdir in "${TARGET_SUBDIRS[@]}"; do
      DMG_SEARCH_PATHS+=("$TARGET_DIR/$subdir/dmg/*.dmg")
      DMG_SEARCH_PATHS+=("$TARGET_DIR/$subdir/macos/*.dmg")
    done

    for path in "${DMG_SEARCH_PATHS[@]}"; do
      found=$(find_glob_artifact "$path")
      if [[ -n "$found" ]]; then
        # 安全检查：如果文件名中包含另一个 semver 版本号且不等于当前 VERSION，阻止重命名
        file_ver=$(extract_file_version "$(basename "$found")")
        if [[ -n "$file_ver" && "$file_ver" != "$VERSION" ]]; then
          die "发现 DMG 文件版本 ($file_ver) 与目标版本 ($VERSION) 不一致，拒绝重命名: $found\n请清理旧产物后重试，或使用 --skip-build 指定精确文件。"
        fi
        DMG_FILE="$found"
        break
      fi
    done
  fi
fi

# ── ZIP 定位（skip-build 优先查找已有 ZIP） ──
if $SKIP_BUILD; then
  for subdir in "${TARGET_SUBDIRS[@]}"; do
    found=$(find_exact_file "$TARGET_DIR/$subdir/macos" "$EXPECTED_ZIP")
    if [[ -n "$found" ]]; then
      ZIP_FILE="$found"
      break
    fi
    found=$(find_exact_file "$TARGET_DIR/$subdir/app" "$EXPECTED_ZIP")
    if [[ -n "$found" ]]; then
      ZIP_FILE="$found"
      break
    fi
    found=$(find_exact_file "$TARGET_DIR/$subdir/dmg" "$EXPECTED_ZIP")
    if [[ -n "$found" ]]; then
      ZIP_FILE="$found"
      break
    fi
  done
fi

# ── App bundle 定位（仅在需要打包 ZIP 时才查找） ──
APP_NEEDED=false
if ! $SKIP_BUILD; then
  # 非 skip-build：必须重新打包 ZIP
  APP_NEEDED=true
elif [[ -z "$ZIP_FILE" ]]; then
  # skip-build 且没找到已有 ZIP：需要从 .app 打包
  APP_NEEDED=true
fi

if $APP_NEEDED; then
  APP_SEARCH_PATHS=()
  for subdir in "${TARGET_SUBDIRS[@]}"; do
    APP_SEARCH_PATHS+=("$TARGET_DIR/$subdir/macos/*.app")
    APP_SEARCH_PATHS+=("$TARGET_DIR/$subdir/app/*.app")
  done

  for path in "${APP_SEARCH_PATHS[@]}"; do
    found=$(find_glob_artifact "$path")
    if [[ -n "$found" ]]; then
      APP_DIR="$found"
      break
    fi
  done
fi

# ── 产物校验 ──
if [[ -z "$DMG_FILE" ]]; then
  echo "" >&2
  echo "未找到 DMG 文件。" >&2
  if $SKIP_BUILD; then
    echo "--skip-build 模式下只接受精确匹配的文件名: $EXPECTED_DMG" >&2
    echo "已搜索的目录:" >&2
    for subdir in "${TARGET_SUBDIRS[@]}"; do
      echo "  $TARGET_DIR/$subdir/dmg/" >&2
      echo "  $TARGET_DIR/$subdir/macos/" >&2
    done
  else
    echo "已搜索以下路径:" >&2
    for subdir in "${TARGET_SUBDIRS[@]}"; do
      echo "  $TARGET_DIR/$subdir/dmg/*.dmg" >&2
      echo "  $TARGET_DIR/$subdir/macos/*.dmg" >&2
    done
  fi
  die "请确认构建是否成功，或使用 --skip-build 指定已有产物"
fi

if $APP_NEEDED && [[ -z "$APP_DIR" ]]; then
  echo "" >&2
  echo "未找到 App bundle (.app)。" >&2
  if $SKIP_BUILD; then
    echo "--skip-build 模式下未找到已有 ZIP ($EXPECTED_ZIP)，需要从 .app 重新打包，" >&2
    echo "但也没有找到可用的 App bundle。" >&2
  fi
  echo "已搜索以下路径:" >&2
  for subdir in "${TARGET_SUBDIRS[@]}"; do
    echo "  $TARGET_DIR/$subdir/macos/*.app" >&2
    echo "  $TARGET_DIR/$subdir/app/*.app" >&2
  done
  die "请确认构建是否成功"
fi

echo ""
echo "═══ 产物 ═══"
echo "DMG:   $DMG_FILE"
DMG_SIZE=$(du -h "$DMG_FILE" | cut -f1)
echo "大小:  $DMG_SIZE"

# DMG 重命名（仅在非 skip-build 且名称不匹配时）
DMG_BASENAME=$(basename "$DMG_FILE")
if [[ "$DMG_BASENAME" != "$EXPECTED_DMG" ]]; then
  if $SKIP_BUILD; then
    # skip-build 不应该走到这里，因为前面已经严格按预期文件名查找
    die "内部错误: skip-build 模式下 DMG 文件名不匹配 ($DMG_BASENAME != $EXPECTED_DMG)"
  fi
  DMG_RENAMED="$(dirname "$DMG_FILE")/$EXPECTED_DMG"
  echo ""
  echo "重命名 DMG: $DMG_BASENAME → $EXPECTED_DMG"
  run_cmd mv "$DMG_FILE" "$DMG_RENAMED"
  DMG_FILE="$DMG_RENAMED"
fi

# ── ZIP 处理 ──
if [[ -n "$ZIP_FILE" && "$SKIP_BUILD" == true ]]; then
  # skip-build 模式下找到已有 ZIP，直接使用
  echo ""
  echo "Portable ZIP:  $ZIP_FILE"
  ZIP_SIZE=$(du -h "$ZIP_FILE" | cut -f1)
  echo "大小:          $ZIP_SIZE"
else
  # 需要从 App bundle 打包 ZIP
  if [[ -z "$APP_DIR" ]]; then
    die "内部错误: 需要打包 ZIP 但未找到 App bundle"
  fi

  echo ""
  echo "App:   $APP_DIR"
  APP_SIZE=$(du -sh "$APP_DIR" | cut -f1)
  echo "大小:  $APP_SIZE"

  ZIP_FILE="$(dirname "$APP_DIR")/$EXPECTED_ZIP"
  echo ""
  echo "打包 Portable ZIP..."
  echo "源:   $APP_DIR"
  echo "目标: $ZIP_FILE"
  if $DRY_RUN; then
    echo "[DRY RUN] ditto -c -k --sequesterRsrc --keepParent '$APP_DIR' '$ZIP_FILE'"
  else
    # 进入 app 所在目录，确保 ZIP 顶层直接是 AgentScope.app
    APP_PARENT=$(dirname "$APP_DIR")
    APP_NAME=$(basename "$APP_DIR")
    (cd "$APP_PARENT" && ditto -c -k --sequesterRsrc --keepParent "$APP_NAME" "$EXPECTED_ZIP")
    echo "打包完成: $ZIP_FILE"
  fi

  ZIP_SIZE=""
  if [[ -f "$ZIP_FILE" && ! $DRY_RUN ]]; then
    ZIP_SIZE=$(du -h "$ZIP_FILE" | cut -f1)
    echo "ZIP 大小: $ZIP_SIZE"
  fi
fi

# ── 上传到 Package Registry ──
if ! $SKIP_UPLOAD; then
  echo ""
  echo "═══ 上传到 GitLab Package Registry ═══"

  if ! $DRY_RUN; then
    [[ -n "${GITLAB_TOKEN:-}" ]] || die "GITLAB_TOKEN 环境变量未设置"
    [[ -n "$PROJECT_ID" ]] || die "GITLAB_PROJECT_ID 环境变量未设置（或用 --project-id 指定）"
  else
    echo "[DRY RUN] 不验证 GITLAB_TOKEN 和 PROJECT_ID"
  fi

  DMG_URL="${GITLAB_URL}/api/v4/projects/${PROJECT_ID}/packages/generic/agent-scope/${VERSION}/${EXPECTED_DMG}"
  ZIP_URL="${GITLAB_URL}/api/v4/projects/${PROJECT_ID}/packages/generic/agent-scope/${VERSION}/${EXPECTED_ZIP}"

  echo ""
  echo "DMG 目标 URL: $DMG_URL"
  upload_file "$DMG_FILE" "$DMG_URL" || die "DMG 上传到 Package Registry 失败"

  echo ""
  echo "ZIP 目标 URL: $ZIP_URL"
  upload_file "$ZIP_FILE" "$ZIP_URL" || die "Portable ZIP 上传到 Package Registry 失败"

  # ── 添加/更新 Release asset links（幂等） ──
  echo ""
  echo "═══ 添加/更新 Release asset links ═══"

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

  # 查询现有 asset links，收集需要删除的 link IDs
  if $DRY_RUN; then
    echo "[DRY RUN] 查询现有 asset links: GET $ASSET_API"
    echo "[DRY RUN] 如已存在 'macOS DMG' 或 'macOS Portable' link，将先删除再创建"
  else
    EXISTING_LINKS=$(api_get "$ASSET_API" 2>/dev/null || echo "[]")

    # 查找 macOS DMG link ID
    DMG_LINK_ID=$(echo "$EXISTING_LINKS" | python3 -c "
import json, sys
links = json.loads(sys.stdin.read())
for l in links:
    if l.get('name') == 'macOS DMG':
        print(l['id'])
        break
" 2>/dev/null || echo "")

    # 查找 macOS Portable link ID
    PORTABLE_LINK_ID=$(echo "$EXISTING_LINKS" | python3 -c "
import json, sys
links = json.loads(sys.stdin.read())
for l in links:
    if l.get('name') == 'macOS Portable':
        print(l['id'])
        break
" 2>/dev/null || echo "")

    if [[ -n "$DMG_LINK_ID" ]]; then
      echo "已有 'macOS DMG' link (id=$DMG_LINK_ID)，删除后重新创建"
      api_delete "${ASSET_API}/${DMG_LINK_ID}" || die "删除已有 DMG link 失败"
    fi

    if [[ -n "$PORTABLE_LINK_ID" ]]; then
      echo "已有 'macOS Portable' link (id=$PORTABLE_LINK_ID)，删除后重新创建"
      api_delete "${ASSET_API}/${PORTABLE_LINK_ID}" || die "删除已有 Portable link 失败"
    fi
  fi

  # 创建新的 asset links
  echo ""
  if api_post "$ASSET_API" "name=macOS DMG" "url=${DMG_URL}" "link_type=package"; then
    echo "Release asset link 添加成功: macOS DMG → $DMG_URL"
  else
    die "添加 macOS DMG Release asset link 失败"
  fi

  echo ""
  if api_post "$ASSET_API" "name=macOS Portable" "url=${ZIP_URL}" "link_type=package"; then
    echo "Release asset link 添加成功: macOS Portable → $ZIP_URL"
  else
    die "添加 macOS Portable Release asset link 失败"
  fi
fi

# ── 完成 ──
echo ""
echo "═══ 完成 ═══"
echo "Release tag:   $TAG"
echo "架构:          $ARCH"
echo "DMG:           $DMG_FILE"
echo "Portable ZIP:  $ZIP_FILE"
if ! $SKIP_UPLOAD; then
  echo ""
  echo "Package Registry 下载地址:"
  echo "  DMG:  ${GITLAB_URL}/api/v4/projects/${PROJECT_ID}/packages/generic/agent-scope/${VERSION}/${EXPECTED_DMG}"
  echo "  ZIP:  ${GITLAB_URL}/api/v4/projects/${PROJECT_ID}/packages/generic/agent-scope/${VERSION}/${EXPECTED_ZIP}"
fi
echo ""
echo "⚠️  注意: 此 macOS 产物未签名、未公证，用户首次打开需右键 → 打开"
