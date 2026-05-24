#!/bin/bash
# AgentScope CI - npm install with retry workaround
#
# agent-scope-ci:node20-rust1.95 Docker 镜像中 npm 存在信号处理竞争条件，
# 在 tag pipeline 的 Docker executor 环境下 "Exit handler never called!" 稳定重现。
# Branch pipeline (verify) 环境下同样镜像不受影响，根因尚未完全定位。
#
# 关键发现：npm crash 时可能仍返回 exit code 0（rc.5 证据），
# 因此本脚本同时检查：exit code、stderr 中的 crash 消息、安装后 node_modules 完整性。
# 任何一项不通过都触发重试。
#
# 用法: bash scripts/ci-npm-install.sh [npm-install-args]
# 见 docs/desktop-ci-cd-lessons.md §5.12

set -uo pipefail

MAX_RETRIES=3
RETRY_DELAY=3
ATTEMPT=1

echo "==> CI npm install (max $MAX_RETRIES retries)"

while [ $ATTEMPT -le $MAX_RETRIES ]; do
  echo "==> Attempt $ATTEMPT/$MAX_RETRIES: npm install $*"

  # 捕获 stderr 用于检测 npm crash 消息
  NPM_ERR=$(mktemp)
  set +e
  npm install "$@" 2>"$NPM_ERR"
  NPM_EXIT=$?
  set -e

  CRASHED=0
  if grep -q "Exit handler never called" "$NPM_ERR" 2>/dev/null; then
    echo "==> Detected npm crash (Exit handler never called!)"
    CRASHED=1
  fi
  rm -f "$NPM_ERR"

  # 验证安装完整性：检查 tsc 是否存在
  TSC_OK=0
  if [ -x "node_modules/.bin/tsc" ]; then
    TSC_OK=1
  else
    echo "==> tsc not found in node_modules/.bin/tsc"
  fi

  if [ $NPM_EXIT -eq 0 ] && [ $CRASHED -eq 0 ] && [ $TSC_OK -eq 1 ]; then
    echo "==> npm install succeeded on attempt $ATTEMPT"
    exit 0
  fi

  echo "==> npm install attempt $ATTEMPT failed (exit=$NPM_EXIT crashed=$CRASHED tsc_ok=$TSC_OK)"

  if [ $ATTEMPT -lt $MAX_RETRIES ]; then
    echo "==> Waiting ${RETRY_DELAY}s before retry..."
    sleep $RETRY_DELAY
    RETRY_DELAY=$((RETRY_DELAY * 2))
  fi
  ATTEMPT=$((ATTEMPT + 1))
done

echo "==> npm install failed after $MAX_RETRIES attempts"
exit 1
