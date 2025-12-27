#!/bin/bash

# 检查 Root 权限
if [ "$EUID" -ne 0 ]; then
    echo "请以 root 用户身份运行此脚本"
    exit 1
fi

# 定义变量
REPO="FTDRTD/Vps-auto-maintain"
FALLBACK_REPO="SKIPPINGpetticoatconvent/Vps-auto-maintain"
BOT_NAME="vps-tg-bot-rust"
BOT_BINARY="/usr/local/bin/$BOT_NAME"
BOT_CONFIG_DIR="/etc/$BOT_NAME"
BOT_SERVICE="/etc/systemd/system/$BOT_NAME.service"
BOT_LOG="/var/log/$BOT_NAME.log"
BOT_BACKUP_DIR="/etc/$BOT_NAME.bak"

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
            echo "  -u: 卸载 $BOT_NAME"
            echo "  -f: 强制卸载（跳过确认提示）"
            echo "  -uf: 强制卸载 $BOT_NAME"
            exit 1
            ;;
    esac
done

# 卸载函数
uninstall_vps_bot() {
    echo "==========================================="
    echo "         VPS Telegram Bot (Rust) 卸载程序"
    echo "==========================================="
    echo
    
    # 检查是否安装了 bot
    if [ ! -f "$BOT_BINARY" ] && [ ! -d "$BOT_CONFIG_DIR" ] && [ ! -f "$BOT_SERVICE" ]; then
        echo "⚠️  未检测到 $BOT_NAME 安装，跳过卸载。"
        exit 0
    fi
    
    # 显示将要删除的文件和目录
    echo "将要删除以下文件和目录："
    [ -f "$BOT_BINARY" ] && echo "  • 二进制文件: $BOT_BINARY"
    [ -d "$BOT_CONFIG_DIR" ] && echo "  • 配置目录: $BOT_CONFIG_DIR"
    [ -f "$BOT_SERVICE" ] && echo "  • Systemd 服务: $BOT_SERVICE"
    [ -d "$BOT_BACKUP_DIR" ] && echo "  • 备份目录: $BOT_BACKUP_DIR"
    [ -f "$BOT_LOG" ] && echo "  • 日志文件: $BOT_LOG"
    echo
    
    # 强制卸载模式检查
    if [ "$FORCE_UNINSTALL" != "true" ]; then
        read -p "⚠️  确定要卸载 $BOT_NAME 吗？这将删除所有配置和数据！(y/N): " -n 1 -r
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
    if systemctl is-active --quiet "$BOT_NAME" 2>/dev/null; then
        echo "  正在停止服务..."
        systemctl stop "$BOT_NAME" && echo "  ✓ 服务已停止" || echo "  ⚠️  停止服务失败"
    else
        echo "  ℹ️  服务未运行"
    fi
    
    if systemctl is-enabled --quiet "$BOT_NAME" 2>/dev/null; then
        echo "  正在禁用服务..."
        systemctl disable "$BOT_NAME" && echo "  ✓ 服务已禁用" || echo "  ⚠️  禁用服务失败"
    else
        echo "  ℹ️  服务未启用"
    fi
    
    # 2. 删除 Systemd 服务文件
    echo "[2/6] 删除 Systemd 服务文件..."
    if [ -f "$BOT_SERVICE" ]; then
        rm -f "$BOT_SERVICE"
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
    if [ -f "$BOT_BINARY" ]; then
        rm -f "$BOT_BINARY"
        echo "  ✓ 二进制文件已删除"
    else
        echo "  ℹ️  二进制文件不存在"
    fi
    
    # 4. 删除配置目录
    echo "[4/6] 删除配置目录..."
    if [ -d "$BOT_CONFIG_DIR" ]; then
        rm -rf "$BOT_CONFIG_DIR"
        echo "  ✓ 配置目录已删除"
    else
        echo "  ℹ️  配置目录不存在"
    fi
    
    # 5. 删除备份目录
    echo "[5/6] 删除备份目录..."
    if [ -d "$BOT_BACKUP_DIR" ]; then
        rm -rf "$BOT_BACKUP_DIR"
        echo "  ✓ 备份目录已删除"
    else
        echo "  ℹ️  备份目录不存在"
    fi
    
    # 6. 删除日志文件
    echo "[6/6] 删除日志文件..."
    if [ -f "$BOT_LOG" ]; then
        rm -f "$BOT_LOG"
        echo "  ✓ 日志文件已删除"
    else
        echo "  ℹ️  日志文件不存在"
    fi
    
    echo
    echo "==========================================="
    echo "         卸载完成！"
    echo "==========================================="
    echo "$BOT_NAME 已成功从系统中移除。"
    echo
    echo "感谢使用 VPS Telegram Bot (Rust)！"
    
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
    local repos=("$REPO" "$FALLBACK_REPO")
    
    for repo in "${repos[@]}"; do
        local api_url="https://api.github.com/repos/${repo}/releases/latest"
        
        if command -v wget &> /dev/null; then
            LATEST_RELEASE=$(wget -qO- --timeout=10 "$api_url" 2>/dev/null | grep '"tag_name"' | sed -E 's/.*"([^"]+)".*/\1/')
        else
            LATEST_RELEASE=$(curl -s --max-time 10 "$api_url" 2>/dev/null | grep '"tag_name"' | sed -E 's/.*"([^"]+)".*/\1/')
        fi
        
        if [ -n "$LATEST_RELEASE" ]; then
            echo "$LATEST_RELEASE"
            return 0
        fi
    done
    
    echo ""
}

VERSION=$(get_latest_release)
if [ -z "$VERSION" ]; then
    echo "无法获取最新 Release 版本号。"
    exit 1
fi

echo "最新版本：$VERSION"
echo

# 检查是否已安装
if [ -f "$BOT_BINARY" ]; then
    echo "检测到已安装的 $BOT_NAME。"
    read -p "是否要更新到最新版本？(y/n): " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        echo "正在更新 $BOT_NAME..."
        # 停止服务
        if systemctl is-active --quiet "$BOT_NAME"; then
            systemctl stop "$BOT_NAME"
        fi
        # 备份配置文件
        if [ -d "$BOT_CONFIG_DIR" ]; then
            cp -r "$BOT_CONFIG_DIR" "$BOT_BACKUP_DIR"
        fi
    else
        echo "取消更新。"
        exit 0
    fi
fi

# 下载二进制文件
echo "正在下载二进制文件..."

# 尝试多个仓库
DOWNLOAD_SUCCESS=false
REPOS=("$REPO" "$FALLBACK_REPO")

for repo in "${REPOS[@]}"; do
    BINARY_URL="github.com/${repo}/releases/download/${VERSION}/vps-tg-bot-rust-linux-amd64"
    echo "尝试从 $BINARY_URL 下载..."
    
    if command -v wget &> /dev/null; then
        if wget -O /tmp/$BOT_NAME --timeout=30 "$BINARY_URL" 2>/dev/null; then
            DOWNLOAD_SUCCESS=true
            break
        fi
    else
        if curl -L -o /tmp/$BOT_NAME --max-time 30 "$BINARY_URL" 2>/dev/null; then
            DOWNLOAD_SUCCESS=true
            break
        fi
    fi
done

if [ "$DOWNLOAD_SUCCESS" != "true" ]; then
    echo "❌ 无法从任何源下载二进制文件"
    echo "请检查网络连接或手动下载："
    echo "https://github.com/$REPO/releases"
    exit 1
fi

echo "✅ 下载成功"

# 安装二进制文件
echo "正在安装二进制文件..."
chmod +x /tmp/$BOT_NAME
mv /tmp/$BOT_NAME "$BOT_BINARY" || { echo "安装二进制文件失败"; exit 1; }

# 配置设置
echo "正在配置设置..."
mkdir -p "$BOT_CONFIG_DIR" || { echo "无法创建配置目录"; exit 1; }

# 如果是更新操作，恢复配置文件
if [ -d "$BOT_BACKUP_DIR" ]; then
    cp -r "$BOT_BACKUP_DIR"/* "$BOT_CONFIG_DIR"/
    rm -rf "$BOT_BACKUP_DIR"
else
    # 交互式询问用户输入 BOT_TOKEN 和 CHAT_ID
    read -p "请输入 BOT_TOKEN: " BOT_TOKEN
    read -p "请输入 CHAT_ID: " CHAT_ID

    # 生成配置文件
    cat > "$BOT_CONFIG_DIR/config.toml" <<EOL
[bot]
token = "$BOT_TOKEN"
chat_id = "$CHAT_ID"
EOL
fi

# Systemd 服务配置
cat > "$BOT_SERVICE" <<EOL
[Unit]
Description=VPS Telegram Bot (Rust)
After=network.target

[Service]
User=root
ExecStart=$BOT_BINARY run
Restart=always

[Install]
WantedBy=multi-user.target
EOL

# 启动服务
echo "正在启动服务..."
systemctl daemon-reload
systemctl enable "$BOT_NAME"
systemctl start "$BOT_NAME"

# 状态检查
echo "服务状态："
systemctl status "$BOT_NAME"

echo
echo "==========================================="
echo "         安装完成！"
echo "==========================================="
echo "$BOT_NAME 已成功安装并启动。"
echo
echo "管理命令："
echo "  查看状态: systemctl status $BOT_NAME"
echo "  查看日志: journalctl -u $BOT_NAME -f"
echo "  停止服务: systemctl stop $BOT_NAME"
echo "  启动服务: systemctl start $BOT_NAME"
echo "  重启服务: systemctl restart $BOT_NAME"
echo
echo "卸载命令: $0 -u"
echo "强制卸载: $0 -uf"
