#!/bin/bash
# Podman E2E 测试脚本
# 使用容器模拟真实 VPS 环境进行端到端测试

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
IMAGE_NAME="vps-tg-bot-e2e"
CONTAINER_NAME="vps-tg-bot-e2e-test"

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

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

# 检查 Podman 是否安装
check_podman() {
    if ! command -v podman &> /dev/null; then
        log_error "Podman 未安装，请先安装 Podman"
        log_info "安装命令: "
        log_info "  - Debian/Ubuntu: sudo apt-get install podman"
        log_info "  - Fedora: sudo dnf install podman"
        log_info "  - Arch: sudo pacman -S podman"
        exit 1
    fi
    log_success "Podman 已安装: $(podman --version)"
}

# 构建 Go 二进制
build_binary() {
    log_info "构建 Go 二进制文件..."
    cd "$PROJECT_DIR"
    
    # 为 Linux 构建（容器环境）
    CGO_ENABLED=0 GOOS=linux GOARCH=amd64 go build -ldflags="-s -w" -o vps-tg-bot ./cmd/vps-tg-bot
    
    if [ -f "vps-tg-bot" ]; then
        log_success "二进制文件构建成功"
    else
        log_error "二进制文件构建失败"
        exit 1
    fi
}

# 构建 E2E 测试镜像
build_image() {
    log_info "构建 E2E 测试镜像..."
    cd "$PROJECT_DIR"
    
    podman build -t "$IMAGE_NAME" -f test/e2e/Dockerfile.e2e .
    
    if [ $? -eq 0 ]; then
        log_success "镜像构建成功: $IMAGE_NAME"
    else
        log_error "镜像构建失败"
        exit 1
    fi
}

# 运行容器化 E2E 测试
run_container_tests() {
    log_info "运行容器化 E2E 测试..."
    
    # 检查是否有正在运行的容器
    if podman ps -a --format "{{.Names}}" | grep -q "^${CONTAINER_NAME}$"; then
        log_info "停止并删除现有容器..."
        podman rm -f "$CONTAINER_NAME" 2>/dev/null || true
    fi
    
    # 创建测试网络（如果不存在）
    if ! podman network exists e2e-test-network 2>/dev/null; then
        podman network create e2e-test-network 2>/dev/null || true
    fi
    
    # 运行测试容器
    log_info "启动测试容器..."
    podman run -d \
        --name "$CONTAINER_NAME" \
        --network e2e-test-network \
        -e TG_TOKEN="${TG_TOKEN:-test_token_123456789:ABCdefGHIjklMNOpqrsTUVwxyz}" \
        -e TG_CHAT_ID="${TG_CHAT_ID:-123456789}" \
        -e TEST_MODE=true \
        "$IMAGE_NAME"
    
    # 等待容器启动
    log_info "等待容器启动..."
    sleep 3
    
    # 检查容器状态
    if podman ps --format "{{.Names}}" | grep -q "^${CONTAINER_NAME}$"; then
        log_success "容器已启动"
        
        # 获取容器日志
        log_info "容器日志:"
        podman logs "$CONTAINER_NAME" 2>&1 | head -20
        
        # 执行容器内测试
        run_in_container_tests
    else
        log_error "容器启动失败"
        podman logs "$CONTAINER_NAME" 2>&1
        exit 1
    fi
}

# 在容器内执行测试
run_in_container_tests() {
    log_info "在容器内执行脚本测试..."
    
    # 测试核心维护脚本
    log_info "测试核心维护脚本..."
    podman exec "$CONTAINER_NAME" /usr/local/bin/vps-maintain-core.sh
    if [ $? -eq 0 ]; then
        log_success "核心维护脚本执行成功"
    else
        log_error "核心维护脚本执行失败"
    fi
    
    # 测试规则维护脚本
    log_info "测试规则维护脚本..."
    podman exec "$CONTAINER_NAME" /usr/local/bin/vps-maintain-rules.sh
    if [ $? -eq 0 ]; then
        log_success "规则维护脚本执行成功"
    else
        log_error "规则维护脚本执行失败"
    fi
    
    # 测试 Xray 命令
    log_info "测试 Xray 重启命令..."
    podman exec "$CONTAINER_NAME" /usr/local/bin/x-ui restart
    if [ $? -eq 0 ]; then
        log_success "Xray 重启命令执行成功"
    else
        log_error "Xray 重启命令执行失败"
    fi
    
    # 测试 Sing-box 命令
    log_info "测试 Sing-box 重启命令..."
    podman exec "$CONTAINER_NAME" /usr/local/bin/sb restart
    if [ $? -eq 0 ]; then
        log_success "Sing-box 重启命令执行成功"
    else
        log_error "Sing-box 重启命令执行失败"
    fi
    
    # 检查系统状态
    log_info "检查容器内系统状态..."
    podman exec "$CONTAINER_NAME" bash -c 'echo "Uptime: $(uptime)"; echo "Memory: $(free -h 2>/dev/null || echo "N/A")"; echo "Disk: $(df -h / 2>/dev/null | tail -1)"'
}

# 运行 Go 单元测试
run_go_tests() {
    log_info "运行 Go E2E 单元测试..."
    cd "$PROJECT_DIR"
    
    go test -v -run TestE2E_ ./test/e2e/... -count=1 -timeout=5m
    
    if [ $? -eq 0 ]; then
        log_success "Go E2E 测试全部通过"
    else
        log_error "Go E2E 测试失败"
        exit 1
    fi
}

# 清理资源
cleanup() {
    log_info "清理资源..."
    
    # 停止并删除容器
    podman rm -f "$CONTAINER_NAME" 2>/dev/null || true
    
    # 可选：删除镜像
    # podman rmi "$IMAGE_NAME" 2>/dev/null || true
    
    # 删除构建的二进制文件
    rm -f "$PROJECT_DIR/vps-tg-bot"
    
    log_success "清理完成"
}

# 显示帮助
show_help() {
    echo "VPS Telegram Bot E2E 测试脚本 (Podman)"
    echo ""
    echo "用法: $0 [选项]"
    echo ""
    echo "选项:"
    echo "  --build-only    仅构建镜像，不运行测试"
    echo "  --run-only      仅运行测试（需要已构建的镜像）"
    echo "  --go-tests      仅运行 Go 单元测试"
    echo "  --cleanup       清理容器和镜像"
    echo "  --help          显示此帮助信息"
    echo ""
    echo "环境变量:"
    echo "  TG_TOKEN        Telegram Bot Token（测试用）"
    echo "  TG_CHAT_ID      管理员 Chat ID（测试用）"
    echo ""
    echo "示例:"
    echo "  $0                    # 完整 E2E 测试"
    echo "  $0 --go-tests         # 仅运行 Go 测试"
    echo "  $0 --build-only       # 仅构建镜像"
}

# 主函数
main() {
    echo "=========================================="
    echo "  VPS Telegram Bot E2E 测试 (Podman)"
    echo "=========================================="
    echo ""
    
    case "${1:-}" in
        --help)
            show_help
            exit 0
            ;;
        --build-only)
            check_podman
            build_binary
            build_image
            ;;
        --run-only)
            check_podman
            run_container_tests
            ;;
        --go-tests)
            run_go_tests
            ;;
        --cleanup)
            cleanup
            ;;
        *)
            # 完整测试流程
            check_podman
            
            # 1. 运行 Go 单元测试
            run_go_tests
            
            # 2. 构建并运行容器测试
            build_binary
            build_image
            run_container_tests
            
            # 3. 清理
            cleanup
            ;;
    esac
    
    echo ""
    echo "=========================================="
    echo "✅ E2E 测试完成"
    echo "=========================================="
}

# 捕获错误并清理
trap cleanup EXIT

main "$@"
