#!/bin/bash
# AgentScope CI - npm install with retry workaround
#
# agent-scope-ci:node20-rust1.95 Docker 镜像中 npm 存在信号处理竞争条件，
# 在 tag pipeline 的 Docker executor 环境下 "Exit handler never called!" 稳定重现。
# Branch pipeline (verify) 环境下同样镜像不受影响，根因尚未完全定位。
#
# 本脚本在 npm install 失败时自动重试，最多 3 次。
# 每次重试间隔 3 秒，给 Docker 环境充分恢复时间。
# 可在同一 CI job 中反复调用，后续调用检测到已安装的 node_modules 后为快速增量运行。
#
# 用法: bash scripts/ci-npm-install.sh [npm-install-args]
# 示例: bash scripts/ci-npm-install.sh --no-audit --no-fund
#
# 见 docs/desktop-ci-cd-lessons.md §5.12

set -euo pipefail

MAX_RETRIES=3
RETRY_DELAY=3
ATTEMPT=1

echo "==> CI npm install (max $MAX_RETRIES retries)"

while [ $ATTEMPT -le $MAX_RETRIES ]; do
  echo "==> Attempt $ATTEMPT/$MAX_RETRIES: npm install $*"
  if npm install "$@"; then
    echo "==> npm install succeeded on attempt $ATTEMPT"
    exit 0
  fi
  EXIT_CODE=$?
  echo "==> npm install attempt $ATTEMPT failed (exit code $EXIT_CODE)"

  if [ $ATTEMPT -lt $MAX_RETRIES ]; then
    echo "==> Waiting ${RETRY_DELAY}s before retry..."
    sleep $RETRY_DELAY
    # Increase delay for subsequent retries
    RETRY_DELAY=$((RETRY_DELAY * 2))
  fi
  ATTEMPT=$((ATTEMPT + 1))
done

echo "==> npm install failed after $MAX_RETRIES attempts"
exit $EXIT_CODE
