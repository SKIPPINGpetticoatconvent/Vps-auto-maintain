#!/bin/bash
# 快速启动脚本 - 忽略所有权限检查
set -e

echo "🤖 Telegram端口监控机器人 - 快速启动"
echo "======================================"

# 创建虚拟环境
if [ ! -d "venv" ]; then
    echo "🔧 创建虚拟环境..."
    python3 -m venv venv
fi

# 激活虚拟环境
echo "🔧 激活虚拟环境..."
if [ -f "venv/bin/activate" ]; then
    source venv/bin/activate
    echo "✅ 虚拟环境激活成功"
elif [ -f "venv/Scripts/activate" ]; then
    source venv/Scripts/activate
    echo "✅ 虚拟环境激活成功 (Windows路径)"
else
    echo "❌ 虚拟环境激活脚本未找到"
    exit 1
fi

# 安装依赖
echo "📦 安装依赖..."
pip install -r requirements.txt

# 创建日志目录
if [ ! -d "logs" ]; then
    mkdir -p logs
    echo "📁 日志目录创建完成"
fi

# 检查配置文件
if [ ! -f "config.json" ]; then
    echo "❌ 未找到配置文件，请先配置config.json"
    echo "请编辑config.json文件，设置Telegram机器人令牌和聊天ID"
    exit 1
fi

# 检查配置
if grep -q "YOUR_BOT_TOKEN_HERE\|YOUR_CHAT_ID_HERE" config.json; then
    echo "❌ 配置文件中包含默认占位符，请先配置正确的Telegram令牌和聊天ID"
    exit 1
fi

# 启动机器人
echo "🚀 启动机器人..."
python start_bot.py