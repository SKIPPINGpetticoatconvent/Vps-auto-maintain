#!/bin/bash
# LoadCredential 凭证加载诊断脚本

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

print_header() {
    echo
    echo -e "${BLUE}========================================${NC}"
    echo -e "${BLUE}         $1${NC}"
    echo -e "${BLUE}========================================${NC}"
    echo
}

print_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

print_error() {
    echo -e "${RED}❌ $1${NC}"
}

print_info() {
    echo -e "${CYAN}ℹ️  $1${NC}"
}

print_header "LoadCredential 凭证加载诊断"

BOT_NAME="vps-tg-bot-rust"
CREDSTORE_DIR="/etc/credstore"
BOT_TOKEN_CRED="$CREDSTORE_DIR/$BOT_NAME.bot-token"
CHAT_ID_CRED="$CREDSTORE_DIR/$BOT_NAME.chat-id"
CRED_DIR="/run/credentials/$BOT_NAME.service"

print_info "检查系统信息..."
echo "  系统: $(uname -a)"
echo "  Systemd 版本: $(systemctl --version | head -1)"
echo

print_info "检查原始凭证文件..."
if [ -f "$BOT_TOKEN_CRED" ]; then
    print_success "BOT_TOKEN 凭证文件存在: $BOT_TOKEN_CRED"
    echo "  文件大小: $(stat -c%s "$BOT_TOKEN_CRED") 字节"
    echo "  文件权限: $(stat -c%a "$BOT_TOKEN_CRED")"
    echo "  内容前缀: $(head -c 20 "$BOT_TOKEN_CRED" | tr -d '\n')..."
else
    print_error "BOT_TOKEN 凭证文件不存在: $BOT_TOKEN_CRED"
fi

if [ -f "$CHAT_ID_CRED" ]; then
    print_success "CHAT_ID 凭证文件存在: $CHAT_ID_CRED"
    echo "  文件大小: $(stat -c%s "$CHAT_ID_CRED") 字节"
    echo "  文件权限: $(stat -c%a "$CHAT_ID_CRED")"
    echo "  内容: $(cat "$CHAT_ID_CRED" | tr -d '\n')"
else
    print_error "CHAT_ID 凭证文件不存在: $CHAT_ID_CRED"
fi

echo

print_info "检查 systemd 服务状态..."
if systemctl is-active --quiet "$BOT_NAME"; then
    print_success "服务正在运行"
else
    print_warning "服务未运行"
    print_info "服务状态: $(systemctl is-active "$BOT_NAME" 2>/dev/null || echo 'unknown')"
fi

echo

print_info "检查 systemd LoadCredential 挂载点..."
if [ -d "$CRED_DIR" ]; then
    print_success "凭证目录存在: $CRED_DIR"
    echo "  目录内容:"
    ls -la "$CRED_DIR" | grep -E "bot-token|chat-id" | while read line; do
        echo "    $line"
    done
    
    if [ -f "$CRED_DIR/bot-token" ]; then
        print_success "LoadCredential 挂载的 bot-token 文件存在"
        echo "  挂载文件大小: $(stat -c%s "$CRED_DIR/bot-token") 字节"
        echo "  挂载文件权限: $(stat -c%a "$CRED_DIR/bot-token")"
        echo "  挂载文件内容前缀: $(head -c 20 "$CRED_DIR/bot-token" | tr -d '\n')..."
    else
        print_error "LoadCredential 挂载的 bot-token 文件不存在"
    fi
    
    if [ -f "$CRED_DIR/chat-id" ]; then
        print_success "LoadCredential 挂载的 chat-id 文件存在"
        echo "  挂载文件大小: $(stat -c%s "$CRED_DIR/chat-id") 字节"
        echo "  挂载文件权限: $(stat -c%a "$CRED_DIR/chat-id")"
        echo "  挂载文件内容: $(cat "$CRED_DIR/chat-id" | tr -d '\n')"
    else
        print_error "LoadCredential 挂载的 chat-id 文件不存在"
    fi
else
    print_error "LoadCredential 凭证目录不存在: $CRED_DIR"
    print_info "这可能意味着:"
    echo "  1. 服务未正确配置 LoadCredential"
    echo "  2. 服务未运行"
    echo "  3. Systemd 版本不支持 LoadCredential"
fi

echo

print_info "检查服务配置文件..."
SERVICE_FILE="/etc/systemd/system/$BOT_NAME.service"
if [ -f "$SERVICE_FILE" ]; then
    print_success "服务文件存在: $SERVICE_FILE"
    echo "  LoadCredential 配置:"
    grep -n "LoadCredential" "$SERVICE_FILE" || print_warning "未找到 LoadCredential 配置"
else
    print_error "服务文件不存在: $SERVICE_FILE"
fi

echo

print_info "检查最近的服务日志..."
if systemctl is-active --quiet "$BOT_NAME" 2>/dev/null; then
    echo "  最近 10 行日志:"
    journalctl -u "$BOT_NAME" -n 10 --no-pager | while read line; do
        echo "    $line"
    done
else
    print_warning "服务未运行，无法查看日志"
    echo "  最后的服务日志:"
    journalctl -u "$BOT_NAME" -n 10 --no-pager | tail -5 | while read line; do
        echo "    $line"
    done
fi

echo

print_header "诊断建议"
print_info "如果凭证文件加载失败，请检查:"
echo "  1. 确保 systemd 版本 >= 235"
echo "  2. 确保服务配置正确设置了 LoadCredential"
echo "  3. 确保原始凭证文件权限为 400"
echo "  4. 查看详细日志: journalctl -u $BOT_NAME -f"
echo

print_info "测试手动配置加载:"
if [ -f "$CRED_DIR/bot-token" ] && [ -f "$CRED_DIR/chat-id" ]; then
    print_success "凭证文件已正确挂载，可以测试应用"
    echo "  手动测试命令: $BOT_NAME check-config"
else
    print_error "凭证文件未正确挂载"
fi

echo