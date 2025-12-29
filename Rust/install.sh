#!/bin/bash
set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# 颜色输出函数
print_color() {
    local color=$1
    shift
    echo -e "${color}$@${NC}"
}

print_header() {
    echo
    print_color "$BLUE" "==========================================="
    print_color "$BLUE" "         $1"
    print_color "$BLUE" "==========================================="
    echo
}

print_success() {
    print_color "$GREEN" "✅ $1"
}

print_warning() {
    print_color "$YELLOW" "⚠️  $1"
}

print_error() {
    print_color "$RED" "❌ $1"
}

print_info() {
    print_color "$CYAN" "ℹ️  $1"
}

# 检查 Root 权限
if [ "$EUID" -ne 0 ]; then
    print_error "请以 root 用户身份运行此脚本"
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
SCHEDULER_STATE="/etc/$BOT_NAME/scheduler_state.json"

# 定义操作类型
ACTION="install"
FORCE_UNINSTALL="false"

# 解析命令行参数
while getopts "u:-:" opt; do
    case $opt in
        u)
            ACTION="uninstall"
            ;;
        -)
            case "${OPTARG}" in
                uninstall)
                    ACTION="uninstall"
                    ;;
                force-uninstall)
                    FORCE_UNINSTALL="true"
                    ;;
                *)
                    echo "用法: $0 [--uninstall] [--force-uninstall]"
                    echo "  --uninstall: 卸载 $BOT_NAME"
                    echo "  --force-uninstall: 强制卸载（跳过确认提示）"
                    exit 1
                    ;;
            esac
            ;;
        f)
            FORCE_UNINSTALL="true"
            ;;
        *)
            echo "用法: $0 [--uninstall] [--force-uninstall] [-u] [-f]"
            echo "  -u, --uninstall: 卸载 $BOT_NAME"
            echo "  -f, --force-uninstall: 强制卸载（跳过确认提示）"
            exit 1
            ;;
    esac
done

# 检测旧版本安装
detect_existing_installation() {
    local has_binary=false
    local has_config=false
    local has_service=false
    local has_scheduler=false
    
    [ -f "$BOT_BINARY" ] && has_binary=true
    [ -d "$BOT_CONFIG_DIR" ] && has_config=true
    [ -f "$BOT_SERVICE" ] && has_service=true
    [ -f "$SCHEDULER_STATE" ] && has_scheduler=true
    
    if [ "$has_binary" = true ] || [ "$has_config" = true ] || [ "$has_service" = true ]; then
        echo "true"
    else
        echo "false"
    fi
}

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

# 卸载函数
uninstall_vps_bot() {
    print_header "VPS Telegram Bot (Rust) 卸载程序"

    # 检查是否安装了 bot
    local existing_installation=$(detect_existing_installation)
    if [ "$existing_installation" = "false" ]; then
        print_warning "未检测到 $BOT_NAME 安装，跳过卸载。"
        exit 0
    fi

    # 显示将要删除的文件和目录
    print_info "将要删除以下文件和目录："
    [ -f "$BOT_BINARY" ] && print_info "  • 二进制文件: $BOT_BINARY"
    [ -d "$BOT_CONFIG_DIR" ] && print_info "  • 配置目录: $BOT_CONFIG_DIR"
    [ -f "$BOT_SERVICE" ] && print_info "  • Systemd 服务: $BOT_SERVICE"
    [ -d "$BOT_BACKUP_DIR" ] && print_info "  • 备份目录: $BOT_BACKUP_DIR"
    [ -f "$BOT_LOG" ] && print_info "  • 日志文件: $BOT_LOG"
    [ -f "$SCHEDULER_STATE" ] && print_info "  • 调度器状态: $SCHEDULER_STATE"
    echo

    # 询问是否保留配置
    local preserve_config=false
    if [ "$FORCE_UNINSTALL" != "true" ]; then
        read -p "是否要保留配置目录？(推荐保留，以便将来重新安装) (y/N): " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            preserve_config=true
        fi
    fi

    # 强制卸载模式检查
    if [ "$FORCE_UNINSTALL" != "true" ]; then
        read -p "⚠️  确定要卸载 $BOT_NAME 吗？这将删除所有数据！(y/N): " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            print_info "取消卸载。"
            exit 0
        fi
    fi

    print_info "开始卸载过程..."
    echo

    # 1. 停止并禁用 Systemd 服务
    print_info "[1/7] 停止并禁用 Systemd 服务..."
    if systemctl is-active --quiet "$BOT_NAME" 2>/dev/null; then
        print_info "  正在停止服务..."
        if systemctl stop "$BOT_NAME"; then
            print_success "服务已停止"
        else
            print_warning "停止服务失败"
        fi
    else
        print_info "  ℹ️  服务未运行"
    fi

    if systemctl is-enabled --quiet "$BOT_NAME" 2>/dev/null; then
        print_info "  正在禁用服务..."
        if systemctl disable "$BOT_NAME"; then
            print_success "服务已禁用"
        else
            print_warning "禁用服务失败"
        fi
    else
        print_info "  ℹ️  服务未启用"
    fi

    # 2. 删除 Systemd 服务文件
    print_info "[2/7] 删除 Systemd 服务文件..."
    if [ -f "$BOT_SERVICE" ]; then
        rm -f "$BOT_SERVICE"
        print_success "服务文件已删除"
        # 重载 Systemd daemon
        print_info "  正在重载 Systemd daemon..."
        systemctl daemon-reload
        print_success "Systemd daemon 已重载"
    else
        print_info "  ℹ️  服务文件不存在"
    fi

    # 3. 删除二进制文件
    print_info "[3/7] 删除二进制文件..."
    if [ -f "$BOT_BINARY" ]; then
        rm -f "$BOT_BINARY"
        print_success "二进制文件已删除"
    else
        print_info "  ℹ️  二进制文件不存在"
    fi

    # 4. 删除配置目录（除非用户选择保留）
    print_info "[4/7] 处理配置目录..."
    if [ -d "$BOT_CONFIG_DIR" ]; then
        if [ "$preserve_config" = "true" ]; then
            print_info "  ℹ️  保留配置目录: $BOT_CONFIG_DIR"
        else
            rm -rf "$BOT_CONFIG_DIR"
            print_success "配置目录已删除"
        fi
    else
        print_info "  ℹ️  配置目录不存在"
    fi

    # 5. 删除备份目录
    print_info "[5/7] 删除备份目录..."
    if [ -d "$BOT_BACKUP_DIR" ]; then
        rm -rf "$BOT_BACKUP_DIR"
        print_success "备份目录已删除"
    else
        print_info "  ℹ️  备份目录不存在"
    fi

    # 6. 删除日志文件
    print_info "[6/7] 删除日志文件..."
    if [ -f "$BOT_LOG" ]; then
        rm -f "$BOT_LOG"
        print_success "日志文件已删除"
    else
        print_info "  ℹ️  日志文件不存在"
    fi

    # 7. 删除调度器状态文件
    print_info "[7/7] 删除调度器状态文件..."
    if [ -f "$SCHEDULER_STATE" ]; then
        rm -f "$SCHEDULER_STATE"
        print_success "调度器状态文件已删除"
    else
        print_info "  ℹ️  调度器状态文件不存在"
    fi

    echo
    print_header "卸载完成！"
    print_success "$BOT_NAME 已成功从系统中移除。"
    echo
    print_info "感谢使用 VPS Telegram Bot (Rust)！"
    
    if [ "$preserve_config" = "true" ]; then
        print_info "配置已保留在: $BOT_CONFIG_DIR"
        print_info "如需完全删除，请运行: $0 --uninstall --force-uninstall"
    fi

    exit 0
}

# 检查 wget 或 curl 是否存在
if ! command -v wget &> /dev/null && ! command -v curl &> /dev/null; then
    print_error "未找到 wget 或 curl。请先安装其中一个。"
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
    print_error "无法获取最新 Release 版本号。"
    exit 1
fi

print_success "最新版本：$VERSION"
echo

# 检测现有配置
EXISTING_CONFIG=$(detect_existing_config)

# 检测现有安装
EXISTING_INSTALLATION=$(detect_existing_installation)

# 显示当前安装状态
if [ "$EXISTING_INSTALLATION" = "true" ]; then
    print_info "检测到已安装的 $BOT_NAME"
    print_info "配置类型: $EXISTING_CONFIG"
    echo
    
    # 进入更新模式
    print_header "更新模式"
    print_info "正在更新到版本 $VERSION..."
    echo
    
    # 停止服务
    if systemctl is-active --quiet "$BOT_NAME"; then
        print_info "正在停止服务..."
        if systemctl stop "$BOT_NAME"; then
            print_success "服务已停止"
        else
            print_warning "停止服务失败，继续更新"
        fi
    fi

    # 检查现有配置是否可用
    if [ "$EXISTING_CONFIG" = "encrypted" ] || [ "$EXISTING_CONFIG" = "legacy" ]; then
        print_success "检测到现有配置，将保留现有设置"
        print_info "配置文件: $BOT_CONFIG_DIR/$([ "$EXISTING_CONFIG" = "encrypted" ] && echo "config.enc" || echo "config.toml")"
        
        # 尝试验证配置
        if [ "$EXISTING_CONFIG" = "encrypted" ]; then
            print_info "正在验证加密配置..."
            if ! "$BOT_BINARY" verify-config --config "$ENCRYPTED_CONFIG" &>/dev/null; then
                print_warning "加密配置验证失败，请重新配置"
                EXISTING_CONFIG="none"
            else
                print_success "加密配置验证成功"
            fi
        fi
    else
        print_warning "未检测到有效配置，将在更新后要求重新配置"
    fi
    
    UPDATE_MODE=true
else
    print_info "新安装模式"
    UPDATE_MODE=false
fi

# 下载二进制文件
print_info "正在下载二进制文件..."

# 尝试多个仓库
DOWNLOAD_SUCCESS=false
REPOS=("$REPO" "$FALLBACK_REPO")

for repo in "${REPOS[@]}"; do
    BINARY_URL="github.com/${repo}/releases/download/${VERSION}/vps-tg-bot-rust-linux-amd64"
    print_info "尝试从 $BINARY_URL 下载..."

    if command -v wget &> /dev/null; then
        if wget -O /tmp/$BOT_NAME --timeout=30 "$BINARY_URL" 2>/dev/null; then
            DOWNLOAD_SUCCESS=true
            break
        fi
    else
        if curl -L -o /tmp/$BOT_NAME --max-time=30 "$BINARY_URL" 2>/dev/null; then
            DOWNLOAD_SUCCESS=true
            break
        fi
    fi
done

if [ "$DOWNLOAD_SUCCESS" != "true" ]; then
    print_error "无法从任何源下载二进制文件"
    print_info "请检查网络连接或手动下载："
    print_info "https://github.com/$REPO/releases"
    exit 1
fi

print_success "下载成功"

# 安装二进制文件
print_info "正在安装二进制文件..."
chmod +x /tmp/$BOT_NAME
if ! mv /tmp/$BOT_NAME "$BOT_BINARY"; then
    print_error "安装二进制文件失败"
    exit 1
fi

# 创建配置目录
mkdir -p "$BOT_CONFIG_DIR" || { print_error "无法创建配置目录"; exit 1; }

# 处理配置
if [ "$UPDATE_MODE" = "true" ] && [ "$EXISTING_CONFIG" != "none" ]; then
    print_success "保留现有配置，跳过配置输入"
    print_info "将在更新后验证配置完整性"
else
    # 新安装或更新但无有效配置，需要输入配置
    print_header "配置设置"
    
    # 默认使用加密文件配置
    print_info "使用加密文件存储配置（AES-256-GCM）"
    
    # 收集敏感配置
    collect_credentials() {
        if [ -z "$BOT_TOKEN" ]; then
            read -p "请输入 BOT_TOKEN: " BOT_TOKEN
        fi
        if [ -z "$CHAT_ID" ]; then
            read -p "请输入 CHAT_ID: " CHAT_ID
        fi
    }

    # 创建加密文件配置
    setup_encrypted_config() {
        print_info "正在配置加密文件..."

        # 收集凭据
        collect_credentials

        # 使用 init-config 命令创建加密配置
        print_info "正在生成加密配置文件..."
        if ! "$BOT_BINARY" init-config --token "$BOT_TOKEN" --chat-id "$CHAT_ID" --output "$ENCRYPTED_CONFIG" 2>/dev/null; then
            print_warning "加密配置生成失败，尝试使用明文配置作为后备..."
            # 回退到明文配置（仅作为最后的手段）
            cat > "$LEGACY_CONFIG" <<EOF
[bot]
token = "$BOT_TOKEN"
chat_id = "$CHAT_ID"
EOF
            print_warning "已创建明文配置文件（不推荐用于生产环境）"
        else
            # 设置文件权限
            chmod 600 "$ENCRYPTED_CONFIG"
            print_success "加密配置文件已创建"

            # 删除明文配置（如果存在）
            if [ -f "$LEGACY_CONFIG" ]; then
                rm -f "$LEGACY_CONFIG"
                print_success "已删除旧版明文配置文件"
            fi
        fi

        # 清除脚本变量中的敏感信息
        unset BOT_TOKEN
        unset CHAT_ID

        # Systemd 服务配置（无环境变量，直接读取配置文件）
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

    # 执行配置
    setup_encrypted_config
fi

# 启动服务
print_info "正在启动服务..."
systemctl daemon-reload
systemctl enable "$BOT_NAME"
systemctl start "$BOT_NAME"

# 状态检查
print_info "服务状态："
systemctl status "$BOT_NAME" --no-pager || true

echo
print_header "安装完成！"
if [ "$UPDATE_MODE" = "true" ]; then
    print_success "$BOT_NAME 已成功更新到版本 $VERSION 并启动。"
else
    print_success "$BOT_NAME 已成功安装并启动。"
fi
echo

print_info "管理命令："
echo "  查看状态: systemctl status $BOT_NAME"
echo "  查看日志: journalctl -u $BOT_NAME -f"
echo "  停止服务: systemctl stop $BOT_NAME"
echo "  启动服务: systemctl start $BOT_NAME"
echo "  重启服务: systemctl restart $BOT_NAME"
echo

print_info "卸载命令: $0 --uninstall"
print_info "强制卸载: $0 --uninstall --force-uninstall"
echo

print_info "可用命令："
echo "  init-config   - 初始化加密配置"
echo "  migrate-config - 迁移明文配置到加密"
echo "  verify-config  - 验证配置完整性"
echo "  check-config   - 检查配置状态"