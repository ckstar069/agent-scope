#!/usr/bin/env bash
set -euo pipefail

VERSION="${1:-}"

if [[ -z "$VERSION" ]]; then
    echo "用法: $0 <版本号>"
    echo "示例: $0 0.3.0"
    exit 1
fi

# 去掉前缀 v（如果用户输入了 v0.3.0）
VERSION="${VERSION#v}"

echo "更新版本号到 $VERSION ..."

# 更新 package.json
sed -i.bak -E 's/"version": "[^"]+"/"version": "'"$VERSION"'"/' package.json
rm -f package.json.bak

# 更新 src-tauri/Cargo.toml
sed -i.bak -E 's/^version = "[^"]+"/version = "'"$VERSION"'"/' src-tauri/Cargo.toml
rm -f src-tauri/Cargo.toml.bak

# 更新 src-tauri/tauri.conf.json
sed -i.bak -E 's/"version": "[^"]+"/"version": "'"$VERSION"'"/' src-tauri/tauri.conf.json
rm -f src-tauri/tauri.conf.json.bak

# 更新 Cargo.lock
cd src-tauri && cargo update --workspace && cd ..

echo "版本号已更新到 $VERSION"
echo ""
echo "请检查变更，然后执行以下命令提交并打标签："
echo "  git add -A"
echo "  git commit -m \"[release] 发布 v$VERSION\""
echo "  git tag v$VERSION"
echo "  git push origin main --tags"
