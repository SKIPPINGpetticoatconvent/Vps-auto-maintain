#!/bin/bash
set -e

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
LEGACY_CONFIG="$BOT_CONFIG_DIR/config.toml"
ENCRYPTED_CONFIG="$BOT_CONFIG_DIR/config.enc"

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

# 检测现有配置类型
detect_existing_config() {
    if [ -f "$ENCRYPTED_CONFIG" ]; then
        echo "encrypted"
    elif [ -f "$LEGACY_CONFIG" ]; then
        echo "legacy"
    else
        echo "none"
    fi
}

EXISTING_CONFIG=$(detect_existing_config)

# 检查是否已安装
if [ -f "$BOT_BINARY" ]; then
    echo "检测到已安装的 $BOT_NAME。"
    read -p "是否要更新到最新版本？(y/n): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo "取消更新。"
        exit 0
    fi

    echo "正在更新 $BOT_NAME..."
    # 停止服务
    if systemctl is-active --quiet "$BOT_NAME"; then
        systemctl stop "$BOT_NAME"
    fi

    # 如果存在旧版配置，备份
    if [ "$EXISTING_CONFIG" = "legacy" ]; then
        echo "检测到旧版明文配置，正在备份..."
        mkdir -p "$BOT_BACKUP_DIR"
        cp -r "$BOT_CONFIG_DIR"/* "$BOT_BACKUP_DIR/" 2>/dev/null || true
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

# 创建配置目录
mkdir -p "$BOT_CONFIG_DIR" || { echo "无法创建配置目录"; exit 1; }

# 配置设置
echo "正在配置设置..."

# 如果是更新操作且有备份配置
if [ -d "$BOT_BACKUP_DIR" ]; then
    # 恢复配置
    if [ -f "$BOT_BACKUP_DIR/config.enc" ]; then
        cp "$BOT_BACKUP_DIR/config.enc" "$BOT_CONFIG_DIR/"
    elif [ -f "$BOT_BACKUP_DIR/config.toml" ]; then
        cp "$BOT_BACKUP_DIR/config.toml" "$BOT_CONFIG_DIR/"
    fi
    rm -rf "$BOT_BACKUP_DIR"
    EXISTING_CONFIG=$(detect_existing_config)
fi

# 处理现有配置
if [ "$EXISTING_CONFIG" = "legacy" ]; then
    echo
    echo "⚠️  检测到旧版明文配置文件！"
    echo "为了安全起见，建议迁移到加密存储或使用环境变量。"
    echo
    echo "请选择配置存储方式："
    echo "  1) 环境变量 (推荐 - 适用于 systemd/容器部署)"
    echo "  2) 加密文件 (更安全 - 使用 AES-256-GCM 加密)"
    echo "  3) 保留现有明文配置（不推荐）"
    read -p "请输入选项 [1/2/3]: " -n 1 -r
    echo
else
    # 新安装或无现有配置
    echo "请选择配置存储方式："
    echo "  1) 环境变量 (推荐 - 适用于 systemd/容器部署)"
    echo "  2) 加密文件 (更安全 - 使用 AES-256-GCM 加密)"
    read -p "请输入选项 [1/2]: " -n 1 -r
    echo
fi

CONFIG_METHOD="${REPLY:-1}"

# 收集敏感配置
collect_credentials() {
    if [ -z "$BOT_TOKEN" ]; then
        read -p "请输入 BOT_TOKEN: " BOT_TOKEN
    fi
    if [ -z "$CHAT_ID" ]; then
        read -p "请输入 CHAT_ID: " CHAT_ID
    fi
}

# 创建环境变量配置
setup_env_config() {
    echo "正在配置环境变量..."

    # 收集凭据
    collect_credentials

    # 交互式输入 token，不在脚本中保留
    local ENV_TOKEN="$BOT_TOKEN"
    local ENV_CHAT_ID="$CHAT_ID"

    # 清除脚本变量中的敏感信息
    unset BOT_TOKEN
    unset CHAT_ID

    # Systemd 服务配置（使用环境变量）
    cat > "$BOT_SERVICE" <<EOF
[Unit]
Description=VPS Telegram Bot (Rust)
After=network.target

[Service]
User=root
WorkingDirectory=/etc/vps-tg-bot-rust
ExecStart=$BOT_BINARY run
Restart=always
Environment="BOT_TOKEN=$ENV_TOKEN"
Environment="CHAT_ID=$ENV_CHAT_ID"

[Install]
WantedBy=multi-user.target
EOF

    # 清除环境变量
    unset ENV_TOKEN
    unset ENV_CHAT_ID

    echo "✅ 环境变量配置完成"
    echo "  配置位置: $BOT_SERVICE"
    echo "  敏感信息将通过 systemd Environment= 注入"
}

# 创建加密文件配置
setup_encrypted_config() {
    echo "正在配置加密文件..."

    # 收集凭据
    collect_credentials

    # 使用 init-config 命令创建加密配置
    echo "正在生成加密配置文件..."
    if ! "$BOT_BINARY" init-config --token "$BOT_TOKEN" --chat-id "$CHAT_ID" --output "$ENCRYPTED_CONFIG" 2>/dev/null; then
        echo "❌ 加密配置生成失败，尝试明文配置..."
        # 回退到明文配置
        cat > "$LEGACY_CONFIG" <<EOF
[bot]
token = "$BOT_TOKEN"
chat_id = "$CHAT_ID"
EOF
        echo "⚠️  已创建明文配置文件（不推荐用于生产环境）"
    else
        # 设置文件权限
        chmod 600 "$ENCRYPTED_CONFIG"
        echo "✅ 加密配置文件已创建"

        # 删除明文配置（如果存在）
        if [ -f "$LEGACY_CONFIG" ]; then
            rm -f "$LEGACY_CONFIG"
            echo "✅ 已删除旧版明文配置文件"
        fi
    fi

    # 清除脚本变量中的敏感信息
    unset BOT_TOKEN
    unset CHAT_ID

    # Systemd 服务配置（无环境变量）
    cat > "$BOT_SERVICE" <<EOF
[Unit]
Description=VPS Telegram Bot (Rust)
After=network.target

[Service]
User=root
WorkingDirectory=/etc/vps-tg-bot-rust
ExecStart=$BOT_BINARY run
Restart=always

[Install]
WantedBy=multi-user.target
EOF
}

# 根据选择执行配置
case "$CONFIG_METHOD" in
    1)
        setup_env_config
        ;;
    2)
        setup_encrypted_config
        ;;
    3)
        echo "保留现有明文配置..."
        if [ -f "$LEGACY_CONFIG" ]; then
            # 读取现有配置
            BOT_TOKEN=$(grep -oP 'token\s*=\s*"\K[^"]+' "$LEGACY_CONFIG" 2>/dev/null || echo "")
            CHAT_ID=$(grep -oP 'chat_id\s*=\s*"\K[^"]+' "$LEGACY_CONFIG" 2>/dev/null || echo "")

            # Systemd 服务配置
            cat > "$BOT_SERVICE" <<EOF
[Unit]
Description=VPS Telegram Bot (Rust)
After=network.target

[Service]
User=root
WorkingDirectory=/etc/vps-tg-bot-rust
ExecStart=$BOT_BINARY run
Restart=always

[Install]
WantedBy=multi-user.target
EOF
            echo "✅ 保留现有配置"
        else
            echo "❌ 无现有配置可保留"
            exit 1
        fi
        ;;
    *)
        echo "❌ 无效选项，使用默认配置（环境变量）"
        setup_env_config
        ;;
esac

# 启动服务
echo "正在启动服务..."
systemctl daemon-reload
systemctl enable "$BOT_NAME"
systemctl start "$BOT_NAME"

# 状态检查
echo "服务状态："
systemctl status "$BOT_NAME" --no-pager || true

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
echo
echo "可用命令："
echo "  init-config   - 初始化加密配置"
echo "  migrate-config - 迁移明文配置到加密"
echo "  verify-config  - 验证配置完整性"
echo "  check-config   - 检查配置状态"
