#!/bin/bash
# -------------------------------------------------------------------
# 快速下载测试脚本
# 验证修复后的下载链接
# -------------------------------------------------------------------

echo "============================================================"
echo "快速下载链接验证"
echo "============================================================"

echo ""
echo "测试 1: Go 版本下载链接"
echo "----------------------------"

# 获取 Go 版本的下载链接
API_RESPONSE=$(curl -s --max-time 10 "https://api.github.com/repos/FTDRTD/Vps-auto-maintain/releases/latest")

if echo "$API_RESPONSE" | grep -q "vps-tg-bot-go-linux-amd64"; then
    echo "✅ 找到 Go 版本下载链接"
    GO_DOWNLOAD_URL=$(echo "$API_RESPONSE" | grep -o "https://[^\"]*vps-tg-bot-go-linux-amd64[^\"]*" | head -n1)
    echo "下载链接: $GO_DOWNLOAD_URL"
else
    echo "❌ 未找到 Go 版本下载链接"
fi

echo ""
echo "测试 2: Rust 版本下载链接"
echo "----------------------------"

if echo "$API_RESPONSE" | grep -q "vps-tg-bot-rust-linux-amd64"; then
    echo "✅ 找到 Rust 版本下载链接"
    RUST_DOWNLOAD_URL=$(echo "$API_RESPONSE" | grep -o "https://[^\"]*vps-tg-bot-rust-linux-amd64[^\"]*" | head -n1)
    echo "下载链接: $RUST_DOWNLOAD_URL"
else
    echo "❌ 未找到 Rust 版本下载链接"
fi

echo ""
echo "测试 3: 测试下载"
echo "----------------------------"

# 测试下载一个小文件
TEST_URL="https://api.github.com/repos/FTDRTD/Vps-auto-maintain/releases/latest"
if curl -L --max-time 30 -o /tmp/test_release.json "$TEST_URL" 2>/dev/null; then
    echo "✅ GitHub API 访问正常"
    rm -f /tmp/test_release.json
else
    echo "❌ GitHub API 访问失败"
fi

echo ""
echo "============================================================"
echo "测试完成"
echo "============================================================"

if echo "$API_RESPONSE" | grep -q "vps-tg-bot-go-linux-amd64"; then
    echo "🎉 修复成功！现在可以运行以下命令部署："
    echo ""
    echo "Go 版本："
    echo "cd Go/ && bash deploy.sh"
    echo ""
    echo "Rust 版本："
    echo "cd Rust/ && bash install.sh"
else
    echo "⚠️  仍需检查下载链接配置"
fi