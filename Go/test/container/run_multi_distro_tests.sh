#!/bin/bash
# 多发行版兼容性测试脚本
# 同时在 Debian、Ubuntu、CentOS 上运行测试

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() { echo -e "${GREEN}[INFO]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }
log_distro() { echo -e "${BLUE}[$1]${NC} $2"; }

# 创建结果目录
mkdir -p "$SCRIPT_DIR/results" "$SCRIPT_DIR/scripts"

# 创建通用测试脚本
cat > "$SCRIPT_DIR/scripts/run_tests.sh" << 'EOF'
#!/bin/bash
DISTRO="${DISTRO:-unknown}"
RESULT_FILE="/app/results/${DISTRO}_results.txt"

echo "=== $DISTRO 兼容性测试 ===" > "$RESULT_FILE"
echo "测试时间: $(date)" >> "$RESULT_FILE"
echo "" >> "$RESULT_FILE"

# 测试系统命令
test_command() {
    local cmd="$1"
    local desc="$2"
    if command -v "$cmd" &>/dev/null; then
        echo "✅ $desc ($cmd) - 可用" >> "$RESULT_FILE"
        return 0
    else
        echo "❌ $desc ($cmd) - 不可用" >> "$RESULT_FILE"
        return 1
    fi
}

echo "--- 系统命令测试 ---" >> "$RESULT_FILE"
test_command "bash" "Bash Shell"
test_command "curl" "cURL"
test_command "wget" "Wget"
test_command "jq" "JSON 处理器"

echo "" >> "$RESULT_FILE"
echo "--- 网络工具测试 ---" >> "$RESULT_FILE"
test_command "ip" "IP 工具"
test_command "netstat" "网络状态"
test_command "ss" "Socket 统计"

echo "" >> "$RESULT_FILE"
echo "--- 防火墙测试 ---" >> "$RESULT_FILE"
test_command "iptables" "iptables"
test_command "ufw" "UFW" || test_command "firewall-cmd" "firewalld"

echo "" >> "$RESULT_FILE"
echo "--- 安全工具测试 ---" >> "$RESULT_FILE"
test_command "fail2ban-client" "Fail2Ban"
test_command "sshd" "SSH 服务"

echo "" >> "$RESULT_FILE"
echo "--- 维护脚本测试 ---" >> "$RESULT_FILE"
if /opt/vps-maintain/core.sh &>/dev/null; then
    echo "✅ 核心维护脚本 - 执行成功" >> "$RESULT_FILE"
else
    echo "❌ 核心维护脚本 - 执行失败" >> "$RESULT_FILE"
fi

if /opt/vps-maintain/rules.sh &>/dev/null; then
    echo "✅ 规则更新脚本 - 执行成功" >> "$RESULT_FILE"
else
    echo "❌ 规则更新脚本 - 执行失败" >> "$RESULT_FILE"
fi

echo "" >> "$RESULT_FILE"
echo "--- 服务命令测试 ---" >> "$RESULT_FILE"
if x-ui status &>/dev/null; then
    echo "✅ x-ui 命令 - 可用" >> "$RESULT_FILE"
else
    echo "❌ x-ui 命令 - 不可用" >> "$RESULT_FILE"
fi

if sb status &>/dev/null; then
    echo "✅ sb 命令 - 可用" >> "$RESULT_FILE"
else
    echo "❌ sb 命令 - 不可用" >> "$RESULT_FILE"
fi

echo "" >> "$RESULT_FILE"
echo "--- 文件系统测试 ---" >> "$RESULT_FILE"
if touch /tmp/test_lock && rm /tmp/test_lock; then
    echo "✅ /tmp 写入 - 正常" >> "$RESULT_FILE"
else
    echo "❌ /tmp 写入 - 失败" >> "$RESULT_FILE"
fi

if echo '{}' > /var/lib/vps-bot/test.json && rm /var/lib/vps-bot/test.json; then
    echo "✅ 状态目录写入 - 正常" >> "$RESULT_FILE"
else
    echo "❌ 状态目录写入 - 失败" >> "$RESULT_FILE"
fi

echo "" >> "$RESULT_FILE"
echo "=== 测试完成 ===" >> "$RESULT_FILE"

cat "$RESULT_FILE"
EOF
chmod +x "$SCRIPT_DIR/scripts/run_tests.sh"

# 构建并运行测试
run_distro_test() {
    local distro="$1"
    local dockerfile="Dockerfile.$distro"
    local container="vps-bot-${distro}-test"
    
    log_distro "$distro" "构建测试镜像..."
    if podman build -t "vps-bot-$distro" -f "$SCRIPT_DIR/$dockerfile" "$SCRIPT_DIR" &>/dev/null; then
        log_distro "$distro" "镜像构建成功"
    else
        log_error "$distro 镜像构建失败"
        return 1
    fi
    
    log_distro "$distro" "运行测试容器..."
    podman rm -f "$container" &>/dev/null || true
    
    podman run --rm \
        --name "$container" \
        -e DISTRO="$distro" \
        -v "$SCRIPT_DIR/scripts:/app/scripts:ro" \
        -v "$SCRIPT_DIR/results:/app/results" \
        "vps-bot-$distro" \
        /app/scripts/run_tests.sh
    
    log_distro "$distro" "测试完成"
}

# 主函数
main() {
    log_info "VPS Telegram Bot 多发行版兼容性测试"
    echo ""
    
    # 检查 Podman
    if ! command -v podman &>/dev/null; then
        log_error "Podman 未安装"
        exit 1
    fi
    
    # 运行各发行版测试
    for distro in debian ubuntu centos; do
        echo ""
        log_info "========== $distro 测试 =========="
        run_distro_test "$distro" || log_warn "$distro 测试失败"
    done
    
    # 汇总报告
    echo ""
    log_info "========== 测试汇总 =========="
    for result in "$SCRIPT_DIR/results"/*_results.txt; do
        if [ -f "$result" ]; then
            echo ""
            cat "$result"
        fi
    done
}

main "$@"
