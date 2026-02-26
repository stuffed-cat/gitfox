#!/bin/bash

# GitFox Workhorse 启动脚本

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "==================================="
echo "GitFox Workhorse 启动脚本"
echo "==================================="

# 检查前端构建
if [ ! -d "$PROJECT_ROOT/frontend/dist" ]; then
    echo "❌ 前端未构建，正在构建..."
    cd "$PROJECT_ROOT/frontend"
    npm run build
    cd "$PROJECT_ROOT"
else
    echo "✅ 前端构建已存在"
fi

# 检查 WebIDE 构建
if [ ! -d "$PROJECT_ROOT/webide/dist" ]; then
    echo "❌ WebIDE 未构建，正在构建..."
    cd "$PROJECT_ROOT/webide"
    npm run build
    cd "$PROJECT_ROOT"
else
    echo "✅ WebIDE 构建已存在"
fi

# 设置环境变量
export WORKHORSE_LISTEN_ADDR="${WORKHORSE_LISTEN_ADDR:-0.0.0.0}"
export WORKHORSE_LISTEN_PORT="${WORKHORSE_LISTEN_PORT:-8080}"
export WORKHORSE_BACKEND_URL="${WORKHORSE_BACKEND_URL:-http://127.0.0.1:8081}"
export WORKHORSE_FRONTEND_DIST="$PROJECT_ROOT/frontend/dist"
export WORKHORSE_WEBIDE_DIST="$PROJECT_ROOT/webide/dist"
export WORKHORSE_ASSETS_PATH="$PROJECT_ROOT/assets"
export WORKHORSE_GIT_REPOS_PATH="$PROJECT_ROOT/repos"
export RUST_LOG="${RUST_LOG:-gitfox_workhorse=info,actix_web=info}"

echo ""
echo "配置信息:"
echo "  监听地址: $WORKHORSE_LISTEN_ADDR:$WORKHORSE_LISTEN_PORT"
echo "  后端地址: $WORKHORSE_BACKEND_URL"
echo "  前端目录: $WORKHORSE_FRONTEND_DIST"
echo "  WebIDE目录: $WORKHORSE_WEBIDE_DIST"
echo "  Assets目录: $WORKHORSE_ASSETS_PATH"
echo "  日志级别: $RUST_LOG"
echo ""

# 检查后端是否运行
echo "检查后端服务..."
if curl -s -f "$WORKHORSE_BACKEND_URL/-/health" > /dev/null 2>&1; then
    echo "✅ 后端服务正常运行"
else
    echo "⚠️  警告: 无法连接到后端服务 $WORKHORSE_BACKEND_URL"
    echo "   请确保后端服务已启动"
fi

echo ""
echo "==================================="
echo "启动 GitFox Workhorse..."
echo "==================================="
echo ""

cd "$SCRIPT_DIR"

# 判断是否使用 release 模式
if [ "$1" = "release" ] || [ "$1" = "--release" ]; then
    echo "🚀 使用 release 模式启动"
    cargo run --release
else
    echo "🔧 使用 debug 模式启动"
    cargo run
fi
