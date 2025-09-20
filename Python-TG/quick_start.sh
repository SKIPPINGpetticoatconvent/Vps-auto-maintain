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
source venv/bin/activate

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
    echo "⚠️ 未找到配置文件，请先配置config.json"
    echo "请编辑config.json文件，设置Telegram机器人令牌和聊天ID"
    exit 1
fi

# 启动机器人
echo "🚀 启动机器人..."
python start_bot.py