#!/bin/bash

# 检查 Root 权限
if [ "$EUID" -ne 0 ]; then
    echo "请以 root 用户身份运行此脚本"
    exit 1
fi

# 定义 REPO 变量
REPO="SKIPPINGpetticoatconvent/Vps-auto-maintain"

# 定义操作类型
ACTION="install"
FORCE_UNINSTALL="false"

# 解析命令行参数
while getopts "u:f" opt; do
    case $opt in
        u)
            ACTION="uninstall"
            ;;
        f)
            FORCE_UNINSTALL="true"
            ;;
        *)
            echo "用法: $0 [ -u ] [ -f ]"
            echo "  -u: 卸载 vps-tg-bot"
            echo "  -f: 强制卸载（跳过确认提示）"
            echo "  -uf: 强制卸载 vps-tg-bot"
            exit 1
            ;;
    esac
done

# 卸载函数
uninstall_vps_bot() {
    echo "==========================================="
    echo "         VPS Telegram Bot 卸载程序"
    echo "==========================================="
    echo
    
    # 检查是否安装了 vps-tg-bot
    if [ ! -f /usr/local/bin/vps-tg-bot ] && [ ! -d /etc/vps-tg-bot ] && [ ! -f /etc/systemd/system/vps-tg-bot.service ]; then
        echo "⚠️  未检测到 vps-tg-bot 安装，跳过卸载。"
        exit 0
    fi
    
    # 显示将要删除的文件和目录
    echo "将要删除以下文件和目录："
    [ -f /usr/local/bin/vps-tg-bot ] && echo "  • 二进制文件: /usr/local/bin/vps-tg-bot"
    [ -d /etc/vps-tg-bot ] && echo "  • 配置目录: /etc/vps-tg-bot"
    [ -f /etc/systemd/system/vps-tg-bot.service ] && echo "  • Systemd 服务: /etc/systemd/system/vps-tg-bot.service"
    [ -d /etc/vps-tg-bot.bak ] && echo "  • 备份目录: /etc/vps-tg-bot.bak"
    [ -f /var/log/vps-tg-bot.log ] && echo "  • 日志文件: /var/log/vps-tg-bot.log"
    echo
    
    # 强制卸载模式检查
    if [ "$FORCE_UNINSTALL" != "true" ]; then
        read -p "⚠️  确定要卸载 vps-tg-bot 吗？这将删除所有配置和数据！(y/N): " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            echo "取消卸载。"
            exit 0
        fi
    fi
    
    echo "开始卸载过程..."
    echo
    
    # 1. 停止并禁用 Systemd 服务
    echo "[1/6] 停止并禁用 Systemd 服务..."
    if systemctl is-active --quiet vps-tg-bot 2>/dev/null; then
        echo "  正在停止服务..."
        systemctl stop vps-tg-bot && echo "  ✓ 服务已停止" || echo "  ⚠️  停止服务失败"
    else
        echo "  ℹ️  服务未运行"
    fi
    
    if systemctl is-enabled --quiet vps-tg-bot 2>/dev/null; then
        echo "  正在禁用服务..."
        systemctl disable vps-tg-bot && echo "  ✓ 服务已禁用" || echo "  ⚠️  禁用服务失败"
    else
        echo "  ℹ️  服务未启用"
    fi
    
    # 2. 删除 Systemd 服务文件
    echo "[2/6] 删除 Systemd 服务文件..."
    if [ -f /etc/systemd/system/vps-tg-bot.service ]; then
        rm -f /etc/systemd/system/vps-tg-bot.service
        echo "  ✓ 服务文件已删除"
        # 重载 Systemd daemon
        echo "  正在重载 Systemd daemon..."
        systemctl daemon-reload
        echo "  ✓ Systemd daemon 已重载"
    else
        echo "  ℹ️  服务文件不存在"
    fi
    
    # 3. 删除二进制文件
    echo "[3/6] 删除二进制文件..."
    if [ -f /usr/local/bin/vps-tg-bot ]; then
        rm -f /usr/local/bin/vps-tg-bot
        echo "  ✓ 二进制文件已删除"
    else
        echo "  ℹ️  二进制文件不存在"
    fi
    
    # 4. 删除配置目录
    echo "[4/6] 删除配置目录..."
    if [ -d /etc/vps-tg-bot ]; then
        rm -rf /etc/vps-tg-bot
        echo "  ✓ 配置目录已删除"
    else
        echo "  ℹ️  配置目录不存在"
    fi
    
    # 5. 删除备份目录
    echo "[5/6] 删除备份目录..."
    if [ -d /etc/vps-tg-bot.bak ]; then
        rm -rf /etc/vps-tg-bot.bak
        echo "  ✓ 备份目录已删除"
    else
        echo "  ℹ️  备份目录不存在"
    fi
    
    # 6. 删除日志文件
    echo "[6/6] 删除日志文件..."
    if [ -f /var/log/vps-tg-bot.log ]; then
        rm -f /var/log/vps-tg-bot.log
        echo "  ✓ 日志文件已删除"
    else
        echo "  ℹ️  日志文件不存在"
    fi
    
    echo
    echo "==========================================="
    echo "         卸载完成！"
    echo "==========================================="
    echo "vps-tg-bot 已成功从系统中移除。"
    echo
    echo "感谢使用 VPS Telegram Bot！"
    
    exit 0
}

# 检查 wget 或 curl 是否存在
if ! command -v wget &> /dev/null && ! command -v curl &> /dev/null; then
    echo "未找到 wget 或 curl。请先安装其中一个。"
    exit 1
fi

# 如果是卸载操作，直接执行卸载
if [ "$ACTION" = "uninstall" ]; then
    uninstall_vps_bot
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
echo

# 检查是否已安装
if [ -f /usr/local/bin/vps-tg-bot ]; then
    echo "检测到已安装的 vps-tg-bot。"
    read -p "是否要更新到最新版本？(y/n): " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        echo "正在更新 vps-tg-bot..."
        # 停止服务
        if systemctl is-active --quiet vps-tg-bot; then
            systemctl stop vps-tg-bot
        fi
        # 备份配置文件
        if [ -d /etc/vps-tg-bot ]; then
            cp -r /etc/vps-tg-bot /etc/vps-tg-bot.bak
        fi
    else
        echo "取消更新。"
        exit 0
    fi
fi

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

# 如果是更新操作，恢复配置文件
if [ -d /etc/vps-tg-bot.bak ]; then
    cp -r /etc/vps-tg-bot.bak/* /etc/vps-tg-bot/
    rm -rf /etc/vps-tg-bot.bak
else
    # 交互式询问用户输入 BOT_TOKEN 和 CHAT_ID
    read -p "请输入 BOT_TOKEN: " BOT_TOKEN
    read -p "请输入 CHAT_ID: " CHAT_ID

    # 生成配置文件
    cat > /etc/vps-tg-bot/config.toml <<EOL
[bot]
token = "$BOT_TOKEN"
chat_id = "$CHAT_ID"
EOL
fi

# Systemd 服务配置
cat > /etc/systemd/system/vps-tg-bot.service <<EOL
[Unit]
Description=VPS Telegram Bot
After=network.target

[Service]
User=root
ExecStart=/usr/local/bin/vps-tg-bot run
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

echo
echo "==========================================="
echo "         安装完成！"
echo "==========================================="
echo "vps-tg-bot 已成功安装并启动。"
echo
echo "管理命令："
echo "  查看状态: systemctl status vps-tg-bot"
echo "  查看日志: journalctl -u vps-tg-bot -f"
echo "  停止服务: systemctl stop vps-tg-bot"
echo "  启动服务: systemctl start vps-tg-bot"
echo "  重启服务: systemctl restart vps-tg-bot"
echo
echo "卸载命令: $0 -u"
echo "强制卸载: $0 -uf"