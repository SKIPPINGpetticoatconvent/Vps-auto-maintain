#!/bin/bash
# VPS Telegram Bot 容器化 E2E 测试脚本
# 使用 Podman 运行真实 Linux 环境测试

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
IMAGE_NAME="vps-tg-bot-e2e-test"
CONTAINER_NAME="vps-bot-e2e-test"

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_info() { echo -e "${GREEN}[INFO]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

# 检查 Podman 是否安装
check_podman() {
    if ! command -v podman &> /dev/null; then
        log_error "Podman 未安装，请先安装 Podman"
        exit 1
    fi
    log_info "Podman 版本: $(podman --version)"
}

# 构建测试镜像
build_image() {
    log_info "构建 E2E 测试镜像..."
    podman build -t "$IMAGE_NAME" -f "$SCRIPT_DIR/Dockerfile.e2e" "$SCRIPT_DIR"
}

# 编译 Go 测试二进制
build_go_binary() {
    log_info "编译 Go 测试二进制..."
    cd "$PROJECT_ROOT"
    
    # 交叉编译为 Linux amd64
    GOOS=linux GOARCH=amd64 go build -o "$SCRIPT_DIR/vps-tg-bot-test" ./cmd/vps-tg-bot/
    
    log_info "Go 二进制编译完成"
}

# 运行容器测试
run_container_tests() {
    log_info "启动测试容器..."
    
    # 停止并删除旧容器
    podman rm -f "$CONTAINER_NAME" 2>/dev/null || true
    
    # 启动容器
    podman run -d \
        --name "$CONTAINER_NAME" \
        --privileged \
        -v "$SCRIPT_DIR/vps-tg-bot-test:/app/vps-tg-bot-test:ro" \
        -v "$PROJECT_ROOT/test:/app/test:ro" \
        -e TG_TOKEN="test_token_123456789" \
        -e TG_CHAT_ID="123456789" \
        "$IMAGE_NAME" \
        sleep infinity
    
    log_info "容器已启动，开始运行测试..."
    
    # 运行测试套件
    run_system_command_tests
    run_permission_tests
    run_firewall_tests
    run_fail2ban_tests
    run_service_tests
}

# 测试 Linux 系统命令调用
run_system_command_tests() {
    log_info "=== 测试 Linux 系统命令调用 ==="
    
    # 测试 systemctl
    log_info "测试 systemctl 命令..."
    if podman exec "$CONTAINER_NAME" systemctl --version &>/dev/null; then
        log_info "✅ systemctl 可用"
    else
        log_warn "⚠️ systemctl 不可用 (容器内可能未启用 systemd)"
    fi
    
    # 测试 apt-get
    log_info "测试 apt-get 命令..."
    if podman exec "$CONTAINER_NAME" apt-get --version &>/dev/null; then
        log_info "✅ apt-get 可用"
    else
        log_error "❌ apt-get 不可用"
    fi
    
    # 测试 ufw
    log_info "测试 ufw 命令..."
    if podman exec "$CONTAINER_NAME" ufw version &>/dev/null; then
        log_info "✅ ufw 可用"
    else
        log_warn "⚠️ ufw 不可用"
    fi
    
    # 测试 iptables
    log_info "测试 iptables 命令..."
    if podman exec "$CONTAINER_NAME" iptables --version &>/dev/null; then
        log_info "✅ iptables 可用"
    else
        log_warn "⚠️ iptables 不可用"
    fi
    
    # 测试维护脚本
    log_info "测试核心维护脚本..."
    if podman exec "$CONTAINER_NAME" /opt/vps-maintain/core.sh; then
        log_info "✅ 核心维护脚本执行成功"
    else
        log_error "❌ 核心维护脚本执行失败"
    fi
    
    log_info "测试规则更新脚本..."
    if podman exec "$CONTAINER_NAME" /opt/vps-maintain/rules.sh; then
        log_info "✅ 规则更新脚本执行成功"
    else
        log_error "❌ 规则更新脚本执行失败"
    fi
}

# 测试权限与文件系统
run_permission_tests() {
    log_info "=== 测试权限与文件系统 ==="
    
    # 测试 /etc/systemd/system/ 写入权限
    log_info "测试 systemd 服务文件创建..."
    if podman exec "$CONTAINER_NAME" bash -c "echo '[Unit]' > /etc/systemd/system/test.service && rm /etc/systemd/system/test.service"; then
        log_info "✅ systemd 服务文件创建权限正常"
    else
        log_error "❌ systemd 服务文件创建权限不足"
    fi
    
    # 测试 /tmp/ 锁文件
    log_info "测试 /tmp/ 锁文件逻辑..."
    if podman exec "$CONTAINER_NAME" bash -c "touch /tmp/vps-bot.lock && rm /tmp/vps-bot.lock"; then
        log_info "✅ /tmp/ 锁文件创建正常"
    else
        log_error "❌ /tmp/ 锁文件创建失败"
    fi
    
    # 测试状态文件目录
    log_info "测试状态文件目录..."
    if podman exec "$CONTAINER_NAME" bash -c "echo '{}' > /var/lib/vps-bot/state.json"; then
        log_info "✅ 状态文件写入正常"
    else
        log_error "❌ 状态文件写入失败"
    fi
    
    # 测试日志目录
    log_info "测试日志目录..."
    if podman exec "$CONTAINER_NAME" bash -c "echo 'test log' >> /var/log/vps-bot/bot.log"; then
        log_info "✅ 日志文件写入正常"
    else
        log_error "❌ 日志文件写入失败"
    fi
}

# 测试网络防火墙逻辑
run_firewall_tests() {
    log_info "=== 测试网络防火墙逻辑 ==="
    
    # 测试 iptables 规则添加
    log_info "测试 iptables 规则添加..."
    if podman exec "$CONTAINER_NAME" iptables -A INPUT -p tcp --dport 12345 -j ACCEPT 2>/dev/null; then
        log_info "✅ iptables 规则添加成功"
        # 清理测试规则
        podman exec "$CONTAINER_NAME" iptables -D INPUT -p tcp --dport 12345 -j ACCEPT 2>/dev/null || true
    else
        log_warn "⚠️ iptables 规则添加失败 (可能需要 --privileged)"
    fi
    
    # 测试 ufw 状态
    log_info "测试 ufw 状态..."
    if podman exec "$CONTAINER_NAME" ufw status 2>/dev/null; then
        log_info "✅ ufw 状态查询成功"
    else
        log_warn "⚠️ ufw 状态查询失败"
    fi
}

# 测试 Fail2Ban 联动
run_fail2ban_tests() {
    log_info "=== 测试 Fail2Ban 联动 ==="
    
    # 检查 Fail2Ban 配置
    log_info "检查 Fail2Ban 配置..."
    if podman exec "$CONTAINER_NAME" test -f /etc/fail2ban/jail.d/sshd.local; then
        log_info "✅ Fail2Ban SSH 配置存在"
    else
        log_error "❌ Fail2Ban SSH 配置不存在"
    fi
    
    # 测试 Fail2Ban 客户端
    log_info "测试 Fail2Ban 客户端..."
    if podman exec "$CONTAINER_NAME" fail2ban-client --version &>/dev/null; then
        log_info "✅ fail2ban-client 可用"
    else
        log_warn "⚠️ fail2ban-client 不可用"
    fi
    
    # 模拟 SSH 登录失败日志
    log_info "模拟 SSH 登录失败..."
    podman exec "$CONTAINER_NAME" bash -c "
        mkdir -p /var/log
        for i in 1 2 3 4 5; do
            echo \"\$(date '+%b %d %H:%M:%S') vps sshd[12345]: Failed password for invalid user test from 192.168.1.100 port 22 ssh2\" >> /var/log/auth.log
        done
    "
    log_info "✅ SSH 登录失败日志已写入"
}

# 测试服务管理
run_service_tests() {
    log_info "=== 测试服务管理 ==="
    
    # 测试 x-ui 命令
    log_info "测试 x-ui 服务命令..."
    if podman exec "$CONTAINER_NAME" x-ui status; then
        log_info "✅ x-ui status 执行成功"
    else
        log_error "❌ x-ui status 执行失败"
    fi
    
    if podman exec "$CONTAINER_NAME" x-ui restart; then
        log_info "✅ x-ui restart 执行成功"
    else
        log_error "❌ x-ui restart 执行失败"
    fi
    
    # 测试 sing-box 命令
    log_info "测试 sing-box 服务命令..."
    if podman exec "$CONTAINER_NAME" sb status; then
        log_info "✅ sb status 执行成功"
    else
        log_error "❌ sb status 执行失败"
    fi
    
    if podman exec "$CONTAINER_NAME" sb restart; then
        log_info "✅ sb restart 执行成功"
    else
        log_error "❌ sb restart 执行失败"
    fi
}

# 运行 Go 二进制测试
run_go_binary_tests() {
    log_info "=== 运行 Go 二进制测试 ==="
    
    if [ -f "$SCRIPT_DIR/vps-tg-bot-test" ]; then
        log_info "测试 Go 二进制在容器内执行..."
        if podman exec "$CONTAINER_NAME" /app/vps-tg-bot-test --help 2>/dev/null; then
            log_info "✅ Go 二进制执行成功"
        else
            log_warn "⚠️ Go 二进制执行失败或无 --help 参数"
        fi
    else
        log_warn "⚠️ Go 二进制未编译，跳过此测试"
    fi
}

# 清理
cleanup() {
    log_info "清理测试环境..."
    podman rm -f "$CONTAINER_NAME" 2>/dev/null || true
    rm -f "$SCRIPT_DIR/vps-tg-bot-test" 2>/dev/null || true
    log_info "清理完成"
}

# 显示测试报告
show_report() {
    log_info "=== 测试报告 ==="
    echo ""
    echo "容器化 E2E 测试完成"
    echo "测试项目:"
    echo "  - Linux 系统命令调用"
    echo "  - 权限与文件系统"
    echo "  - 网络防火墙逻辑"
    echo "  - Fail2Ban 联动"
    echo "  - 服务管理"
    echo ""
}

# 主函数
main() {
    log_info "VPS Telegram Bot 容器化 E2E 测试"
    echo ""
    
    check_podman
    
    case "${1:-all}" in
        build)
            build_image
            ;;
        compile)
            build_go_binary
            ;;
        test)
            run_container_tests
            run_go_binary_tests
            show_report
            ;;
        clean)
            cleanup
            ;;
        all)
            build_image
            build_go_binary
            run_container_tests
            run_go_binary_tests
            show_report
            cleanup
            ;;
        *)
            echo "用法: $0 {build|compile|test|clean|all}"
            exit 1
            ;;
    esac
}

main "$@"
