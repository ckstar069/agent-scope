#!/usr/bin/env bash
# ⚠️  已废弃 — 请使用 scripts/release-macos.sh
#
# 此脚本仅为兼容保留，实际调用 scripts/release-macos.sh。
# release-macos.sh 同时支持 DMG 安装包 和 Portable ZIP 免安装版。
#
# 用法与原脚本完全一致：
#   ./scripts/release-macos.sh --tag v0.3.6

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

echo "══════════════════════════════════════════════════════════════" >&2
echo "  警告: release-macos-dmg.sh 已废弃" >&2
echo "  请改用: scripts/release-macos.sh" >&2
echo "  release-macos.sh 支持 DMG + Portable ZIP 两种产物" >&2
echo "══════════════════════════════════════════════════════════════" >&2
echo "" >&2

exec "$SCRIPT_DIR/release-macos.sh" "$@"
