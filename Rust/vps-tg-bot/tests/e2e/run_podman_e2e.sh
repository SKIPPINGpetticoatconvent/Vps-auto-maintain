#!/bin/bash
# Rust VPS Telegram Bot - Podman E2E 测试脚本
# 模拟真实用户与 Bot 按钮交互，验证程序行为

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
IMAGE_NAME="vps-tg-bot-rust-e2e"
CONTAINER_NAME="vps-tg-bot-rust-e2e-test"

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

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

# 检查 Podman
check_podman() {
    if ! command -v podman &> /dev/null; then
        log_error "Podman 未安装"
        log_info "安装命令:"
        log_info "  - Debian/Ubuntu: sudo apt-get install podman"
        log_info "  - Fedora: sudo dnf install podman"
        log_info "  - Arch: sudo pacman -S podman"
        exit 1
    fi
    log_success "Podman 已安装: $(podman --version)"
}

# 运行 Rust 单元测试
run_rust_tests() {
    log_info "运行 Rust E2E 单元测试..."
    cd "$PROJECT_DIR"
    
    cargo test e2e_test --release -- --test-threads=1 --nocapture
    
    if [ $? -eq 0 ]; then
        log_success "Rust E2E 测试全部通过"
    else
        log_error "Rust E2E 测试失败"
        exit 1
    fi
}

# 构建 Release 版本
build_binary() {
    log_info "构建 Rust Release 版本..."
    cd "$PROJECT_DIR"
    
    cargo build --release
    
    if [ -f "target/release/vps-tg-bot" ]; then
        log_success "二进制文件构建成功"
    else
        log_error "二进制文件构建失败"
        exit 1
    fi
}

# 构建 Podman 镜像
build_image() {
    log_info "构建 Podman E2E 测试镜像..."
    cd "$PROJECT_DIR"
    
    podman build -t "$IMAGE_NAME" -f tests/e2e/Dockerfile.e2e .
    
    if [ $? -eq 0 ]; then
        log_success "镜像构建成功: $IMAGE_NAME"
    else
        log_error "镜像构建失败"
        exit 1
    fi
}

# 运行容器测试
run_container_tests() {
    log_info "运行容器化 E2E 测试..."
    
    # 清理现有容器
    if podman ps -a --format "{{.Names}}" | grep -q "^${CONTAINER_NAME}$"; then
        log_info "停止并删除现有容器..."
        podman rm -f "$CONTAINER_NAME" 2>/dev/null || true
    fi
    
    # 创建测试网络
    if ! podman network exists e2e-rust-network 2>/dev/null; then
        podman network create e2e-rust-network 2>/dev/null || true
    fi
    
    # 运行测试容器
    log_info "启动测试容器..."
    podman run -d \
        --name "$CONTAINER_NAME" \
        --network e2e-rust-network \
        -e TELOXIDE_TOKEN="${TELOXIDE_TOKEN:-test_token_123456789:ABCdefGHIjklMNOpqrsTUVwxyz}" \
        -e CHAT_ID="${CHAT_ID:-123456789}" \
        -e RUST_LOG=info \
        "$IMAGE_NAME"
    
    sleep 3
    
    # 检查容器状态
    if podman ps --format "{{.Names}}" | grep -q "^${CONTAINER_NAME}$"; then
        log_success "容器已启动"
        log_info "容器日志:"
        podman logs "$CONTAINER_NAME" 2>&1 | head -20
        
        # 执行容器内脚本测试
        run_script_tests
    else
        log_error "容器启动失败"
        podman logs "$CONTAINER_NAME" 2>&1
        exit 1
    fi
}

# 在容器内测试脚本
run_script_tests() {
    log_info "在容器内执行脚本测试..."
    
    # 测试核心维护脚本
    log_info "测试核心维护脚本..."
    if podman exec "$CONTAINER_NAME" /usr/local/bin/vps-maintain-core.sh; then
        log_success "核心维护脚本执行成功"
    else
        log_error "核心维护脚本执行失败"
    fi
    
    # 测试规则维护脚本
    log_info "测试规则维护脚本..."
    if podman exec "$CONTAINER_NAME" /usr/local/bin/vps-maintain-rules.sh; then
        log_success "规则维护脚本执行成功"
    else
        log_error "规则维护脚本执行失败"
    fi
    
    # 测试 Xray 命令
    log_info "测试 Xray 命令..."
    if podman exec "$CONTAINER_NAME" /usr/local/bin/x-ui restart; then
        log_success "Xray 重启命令执行成功"
    else
        log_error "Xray 重启命令执行失败"
    fi
    
    if podman exec "$CONTAINER_NAME" /usr/local/bin/x-ui update; then
        log_success "Xray 更新命令执行成功"
    else
        log_error "Xray 更新命令执行失败"
    fi
    
    # 测试 Sing-box 命令
    log_info "测试 Sing-box 命令..."
    if podman exec "$CONTAINER_NAME" /usr/local/bin/sb restart; then
        log_success "Sing-box 重启命令执行成功"
    else
        log_error "Sing-box 重启命令执行失败"
    fi
    
    if podman exec "$CONTAINER_NAME" /usr/local/bin/sb update; then
        log_success "Sing-box 更新命令执行成功"
    else
        log_error "Sing-box 更新命令执行失败"
    fi
    
    # 检查系统状态
    log_info "检查容器内系统状态..."
    podman exec "$CONTAINER_NAME" bash -c 'echo "Hostname: $(hostname)"; echo "Uptime: $(uptime 2>/dev/null || echo "N/A")"; echo "Memory: $(free -h 2>/dev/null || echo "N/A")"; echo "Disk: $(df -h / 2>/dev/null | tail -1)"'
}

# 清理资源
cleanup() {
    log_info "清理资源..."
    
    podman rm -f "$CONTAINER_NAME" 2>/dev/null || true
    # podman rmi "$IMAGE_NAME" 2>/dev/null || true
    
    log_success "清理完成"
}

# 帮助信息
show_help() {
    echo "Rust VPS Telegram Bot E2E 测试脚本 (Podman)"
    echo ""
    echo "用法: $0 [选项]"
    echo ""
    echo "选项:"
    echo "  --build-only    仅构建镜像"
    echo "  --run-only      仅运行容器测试"
    echo "  --rust-tests    仅运行 Rust 单元测试"
    echo "  --cleanup       清理容器和镜像"
    echo "  --help          显示帮助"
    echo ""
    echo "环境变量:"
    echo "  TELOXIDE_TOKEN  Telegram Bot Token"
    echo "  CHAT_ID         管理员 Chat ID"
    echo ""
    echo "示例:"
    echo "  $0                    # 完整 E2E 测试"
    echo "  $0 --rust-tests       # 仅 Rust 测试"
    echo "  $0 --build-only       # 仅构建镜像"
}

# 主函数
main() {
    echo "=========================================="
    echo "  Rust VPS Telegram Bot E2E 测试 (Podman)"
    echo "=========================================="
    echo ""
    
    case "${1:-}" in
        --help)
            show_help
            exit 0
            ;;
        --build-only)
            check_podman
            build_image
            ;;
        --run-only)
            check_podman
            run_container_tests
            ;;
        --rust-tests)
            run_rust_tests
            ;;
        --cleanup)
            cleanup
            ;;
        *)
            # 完整测试
            check_podman
            run_rust_tests
            build_image
            run_container_tests
            cleanup
            ;;
    esac
    
    echo ""
    echo "=========================================="
    echo "✅ E2E 测试完成"
    echo "=========================================="
}

trap cleanup EXIT
main "$@"
