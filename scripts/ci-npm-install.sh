#!/bin/bash
# AgentScope CI - npm install entrypoint
#
# agent-scope-ci:node20-rust1.95 旧镜像中 NodeSource 捆绑 npm 10.8.2
# 在 tag pipeline Docker executor 环境下无法稳定完成安装。
#
# 长期修复在 ci/Dockerfile 中固定 npm 版本；本脚本只保留统一入口，
# 避免 CI 配置到处散落 npm 参数。
#
# 用法: bash scripts/ci-npm-install.sh [npm-install-args]
# 见 docs/desktop-ci-cd-lessons.md §5.12

set -euo pipefail

echo "==> CI npm install via pinned npm $(npm --version)"
npm ci "$@"
echo "==> npm ci succeeded"
