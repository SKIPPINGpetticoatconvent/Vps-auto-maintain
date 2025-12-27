#!/bin/bash
# E2E 测试运行脚本
# 模拟真实用户与 Telegram Bot 按钮的交互

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "=========================================="
echo "  VPS Telegram Bot E2E 测试"
echo "=========================================="
echo ""

cd "$PROJECT_DIR"

# 确保依赖已安装
echo "📦 检查依赖..."
go mod tidy

# 运行 E2E 测试
echo ""
echo "🧪 运行端到端测试..."
echo ""

go test -v -run TestE2E_ ./test/e2e/... -count=1 -timeout=5m

if [ $? -eq 0 ]; then
    echo ""
    echo "=========================================="
    echo "✅ 所有 E2E 测试通过！"
    echo "=========================================="
else
    echo ""
    echo "=========================================="
    echo "❌ 部分测试失败，请检查日志"
    echo "=========================================="
    exit 1
fi
