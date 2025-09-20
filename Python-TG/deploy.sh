#!/bin/bash
# ---------------------------------------------------------------------------------
# Telegram端口监控机器人部署脚本
# 基于detect_ports_ultimate.sh的Python版本
#
# 功能：
# - 自动创建Python虚拟环境
# - 安装依赖项
# - 配置日志目录
# - 启动机器人服务
# ---------------------------------------------------------------------------------

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 日志函数
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# 检查是否为root用户
check_root() {
    if [[ $EUID -eq 0 ]]; then
        log_error "请勿使用root用户运行此脚本"
        exit 1
    fi
}

# 检测操作系统
detect_os() {
    if [[ -f /etc/os-release ]]; then
        . /etc/os-release
        OS=$ID
    else
        log_error "无法检测操作系统"
        exit 1
    fi
    log_info "检测到操作系统: $OS"
}

# 安装Python依赖
install_python_deps() {
    log_info "安装Python依赖..."

    if command -v pip3 &> /dev/null; then
        PIP_CMD="pip3"
    elif command -v pip &> /dev/null; then
        PIP_CMD="pip"
    else
        log_error "未找到pip命令，请先安装Python和pip"
        exit 1
    fi

    # 升级pip
    $PIP_CMD install --upgrade pip

    # 安装依赖
    if [[ -f "requirements.txt" ]]; then
        $PIP_CMD install -r requirements.txt
        log_success "Python依赖安装完成"
    else
        log_error "未找到requirements.txt文件"
        exit 1
    fi
}

# 创建虚拟环境
create_venv() {
    log_info "创建Python虚拟环境..."

    if [[ -d "venv" ]]; then
        log_warning "虚拟环境已存在，跳过创建"
    else
        python3 -m venv venv
        log_success "虚拟环境创建完成"
    fi
}

# 配置日志目录
setup_logging() {
    log_info "配置日志目录..."

    if [[ ! -d "logs" ]]; then
        mkdir -p logs
        log_success "日志目录创建完成"
    else
        log_warning "日志目录已存在"
    fi
}

# 创建配置文件
create_config() {
    log_info "检查配置文件..."

    if [[ ! -f "config.json" ]]; then
        log_warning "未找到配置文件，创建默认配置"
        cat > config.json << 'EOF'
{
  "telegram": {
    "token": "YOUR_BOT_TOKEN_HERE",
    "allowed_chat_ids": [YOUR_CHAT_ID_HERE],
    "admin_chat_ids": [YOUR_CHAT_ID_HERE],
    "notification_enabled": true
  },
  "monitoring": {
    "check_interval": 300,
    "alert_on_changes": true,
    "services": {
      "xray": {
        "enabled": true,
        "process_name": "xray",
        "config_paths": ["/etc/xray/config.json", "/usr/local/etc/xray/config.json"]
      },
      "sing_box": {
        "enabled": true,
        "process_name": "sing-box",
        "config_paths": [
          "/etc/sing-box/config.json",
          "/usr/local/etc/sing-box/config.json",
          "/etc/sing-box/conf/*.json"
        ]
      },
      "ssh": {
        "enabled": true,
        "port": 22
      }
    }
  },
  "firewall": {
    "auto_configure": true,
    "allowed_ports": [22, 80, 443],
    "auto_clean_unknown": true,
    "supported_types": ["ufw", "firewalld"]
  },
  "security": {
    "secure_lock_enabled": true,
    "auto_secure_on_startup": false,
    "whitelist_ports": [22],
    "max_unknown_ports": 10
  },
  "logging": {
    "level": "INFO",
    "file": "logs/bot.log",
    "max_size": 10485760,
    "backup_count": 5,
    "log_to_console": true
  },
  "system": {
    "timezone": "auto",
    "hostname": "auto",
    "os_detection": true
  }
}
EOF
        log_success "默认配置文件创建完成"
        log_warning "请编辑config.json文件，设置正确的Telegram机器人令牌和聊天ID"
        return 1
    else
        log_success "配置文件存在"
        return 0
    fi
}

# 检查Telegram配置
check_telegram_config() {
    if [[ -f "config.json" ]]; then
        if grep -q "YOUR_BOT_TOKEN_HERE\|YOUR_CHAT_ID_HERE" config.json; then
            log_error "配置文件中包含默认占位符，请先配置正确的Telegram令牌和聊天ID"
            return 1
        fi
    fi
    return 0
}

# 创建系统服务
create_service() {
    log_info "创建系统服务..."

    SERVICE_FILE="/etc/systemd/system/tg-port-monitor.service"

    if [[ ! -f "$SERVICE_FILE" ]]; then
        cat > "$SERVICE_FILE" << EOF
[Unit]
Description=Telegram Port Monitor Bot
After=network.target

[Service]
Type=simple
User=$USER
WorkingDirectory=$PWD
ExecStart=$PWD/venv/bin/python start_bot.py
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
EOF

        # 重新加载systemd配置
        sudo systemctl daemon-reload
        log_success "系统服务创建完成"
    else
        log_warning "系统服务已存在"
    fi
}

# 启动服务
start_service() {
    log_info "启动机器人服务..."

    if systemctl is-active --quiet tg-port-monitor; then
        log_warning "服务已在运行，先停止现有服务"
        sudo systemctl stop tg-port-monitor
    fi

    sudo systemctl start tg-port-monitor
    sudo systemctl enable tg-port-monitor

    # 检查服务状态
    if systemctl is-active --quiet tg-port-monitor; then
        log_success "机器人服务启动成功"
        log_info "服务状态: $(sudo systemctl status tg-port-monitor --no-pager -l)"
    else
        log_error "服务启动失败"
        log_info "查看日志: sudo journalctl -u tg-port-monitor -f"
        exit 1
    fi
}

# 主函数
main() {
    log_info "开始部署Telegram端口监控机器人..."

    check_root
    detect_os
    create_venv
    setup_logging

    # 激活虚拟环境
    source venv/bin/activate

    install_python_deps

    if create_config; then
        check_telegram_config
    else
        log_error "请先配置Telegram令牌和聊天ID"
        exit 1
    fi

    create_service
    start_service

    log_success "部署完成！"
    log_info "使用方法:"
    log_info "1. 查看状态: sudo systemctl status tg-port-monitor"
    log_info "2. 查看日志: sudo journalctl -u tg-port-monitor -f"
    log_info "3. 重启服务: sudo systemctl restart tg-port-monitor"
    log_info "4. 停止服务: sudo systemctl stop tg-port-monitor"
}

# 参数处理
case "$1" in
    --help|-h)
        echo "用法: $0 [选项]"
        echo "选项:"
        echo "  --help, -h    显示帮助信息"
        echo "  --start       启动服务"
        echo "  --stop        停止服务"
        echo "  --restart     重启服务"
        echo "  --status      查看服务状态"
        echo "  --log         查看服务日志"
        echo "  --no-service  不创建系统服务"
        exit 0
        ;;
    --start)
        sudo systemctl start tg-port-monitor
        echo "服务已启动"
        ;;
    --stop)
        sudo systemctl stop tg-port-monitor
        echo "服务已停止"
        ;;
    --restart)
        sudo systemctl restart tg-port-monitor
        echo "服务已重启"
        ;;
    --status)
        sudo systemctl status tg-port-monitor --no-pager
        ;;
    --log)
        sudo journalctl -u tg-port-monitor -f
        ;;
    *)
        main
        ;;
esac