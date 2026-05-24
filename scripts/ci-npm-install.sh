#!/bin/bash
# AgentScope CI - npm install with npx fallback
#
# agent-scope-ci:node20-rust1.95 Docker 镜像中 npm 在 tag pipeline 的
# Docker executor 环境下完全无法完成安装（8 次重现，exit code 0/1/217，
# 均无法产出 node_modules）。
#
# 本脚本使用 npx（随 Node 预装）下载并运行最新 npm，绕过镜像中的损坏 npm。
# npx 的下载管道独立于 npm，不受同一种竞争条件影响。
#
# 用法: bash scripts/ci-npm-install.sh [npm-install-args]
# 见 docs/desktop-ci-cd-lessons.md §5.12

set -uo pipefail

echo "==> CI npm install (npx bootstrap)"

# 用 npx 运行最新 npm，隔离镜像中的损坏版本
if npx --yes npm@latest install --no-audit --no-fund "$@"; then
  echo "==> npx npm install succeeded"
  exit 0
fi

echo "==> npx npm install failed (exit code $?), falling back to built-in npm..."
npm install --no-audit --no-fund "$@" || {
  echo "==> Fallback also failed"
  exit 1
}
echo "==> Fallback npm install succeeded"
