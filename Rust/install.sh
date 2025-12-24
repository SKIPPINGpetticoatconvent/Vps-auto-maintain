#!/bin/bash

# 检查 Root 权限
if [ "$EUID" -ne 0 ]; then
    echo "请以 root 用户身份运行此脚本"
    exit 1
fi

# 定义 REPO 变量
REPO="ANGOM/Vps-auto-maintain"

# 检查 wget 或 curl 是否存在
if ! command -v wget &> /dev/null && ! command -v curl &> /dev/null; then
    echo "未找到 wget 或 curl。请先安装其中一个。"
    exit 1
fi

# 获取最新 Release 版本号
get_latest_release() {
    if command -v wget &> /dev/null; then
        LATEST_RELEASE=$(wget -qO- "https://api.github.com/repos/$REPO/releases/latest" | grep '"tag_name"' | sed -E 's/.*"([^"]+)".*/\1/')
    else
        LATEST_RELEASE=$(curl -s "https://api.github.com/repos/$REPO/releases/latest" | grep '"tag_name"' | sed -E 's/.*"([^"]+)".*/\1/')
    fi
    echo "$LATEST_RELEASE"
}

VERSION=$(get_latest_release)
if [ -z "$VERSION" ]; then
    echo "无法获取最新 Release 版本号。"
    exit 1
fi

echo "最新版本：$VERSION"

# 下载二进制文件
BINARY_URL="https://github.com/$REPO/releases/download/$VERSION/vps-tg-bot-linux-amd64"
echo "正在下载二进制文件..."

if command -v wget &> /dev/null; then
    wget -O /tmp/vps-tg-bot "$BINARY_URL" || { echo "下载失败"; exit 1; }
else
    curl -L -o /tmp/vps-tg-bot "$BINARY_URL" || { echo "下载失败"; exit 1; }
fi

# 安装二进制文件
echo "正在安装二进制文件..."
chmod +x /tmp/vps-tg-bot
mv /tmp/vps-tg-bot /usr/local/bin/vps-tg-bot || { echo "安装二进制文件失败"; exit 1; }

# 配置设置
echo "正在配置设置..."
mkdir -p /etc/vps-tg-bot/ || { echo "无法创建配置目录"; exit 1; }

# 交互式询问用户输入 BOT_TOKEN 和 CHAT_ID
read -p "请输入 BOT_TOKEN: " BOT_TOKEN
read -p "请输入 CHAT_ID: " CHAT_ID

# 生成配置文件
cat > /etc/vps-tg-bot/config.toml <<EOL
[bot]
token = "$BOT_TOKEN"
chat_id = "$CHAT_ID"
EOL

# Systemd 服务配置
cat > /etc/systemd/system/vps-tg-bot.service <<EOL
[Unit]
Description=VPS Telegram Bot
After=network.target

[Service]
User=root
ExecStart=/usr/local/bin/vps-tg-bot
Restart=always

[Install]
WantedBy=multi-user.target
EOL

# 启动服务
echo "正在启动服务..."
systemctl daemon-reload
systemctl enable vps-tg-bot
systemctl start vps-tg-bot

# 状态检查
echo "服务状态："
systemctl status vps-tg-bot