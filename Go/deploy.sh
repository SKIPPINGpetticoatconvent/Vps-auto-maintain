#!/bin/bash
# ----------------------------------------------------------------------------
# VPS Telegram Bot Go 版本 - 一键部署脚本 (纯部署 + 自动修复环境)
#
# 版本: 2.0.6
# 作者: FTDRTD
# 功能:
#   ✅ 检测到 Go 自动卸载 Go 与旧版本 Bot
#   ✅ 精确卸载 golang，不再误删 /usr
#   ✅ 自动检测并修复 coreutils / apt / dpkg 缺失
#   ✅ 自动下载 GitHub Release 二进制文件
#   ✅ 自动同步时区、配置 journald、创建 systemd 服务与定时任务
# ----------------------------------------------------------------------------

set -e

# ========== 彩色输出 ==========
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

print_message() {
  echo ""
  echo "============================================================"
  echo "$1"
  echo "============================================================"
}
print_success() { echo -e "${GREEN}✅ $1${NC}"; }
print_error()   { echo -e "${RED}❌ $1${NC}"; }
print_warning() { echo -e "${YELLOW}⚠️  $1${NC}"; }

# ========== 全局路径 ==========
BOT_DIR="/opt/vps-tg-bot"
BOT_BINARY="$BOT_DIR/vps-tg-bot"
BOT_SERVICE="/etc/systemd/system/vps-tg-bot.service"
CORE_MAINTAIN_SCRIPT="/usr/local/bin/vps-maintain-core.sh"
RULES_MAINTAIN_SCRIPT="/usr/local/bin/vps-maintain-rules.sh"

# ========== 环境自愈 ==========
ensure_coreutils() {
  if ! command -v mkdir >/dev/null 2>&1; then
    print_warning "检测到 coreutils 缺失，正在自动修复..."
    if command -v apt-get >/dev/null 2>&1; then
      apt-get update -o Acquire::ForceIPv4=true >/dev/null 2>&1 || true
      apt-get install -y coreutils >/dev/null 2>&1 || true
    elif command -v apt >/dev/null 2>&1; then
      apt install -y coreutils >/dev/null 2>&1 || true
    elif command -v apk >/dev/null 2>&1; then
      apk add coreutils >/dev/null 2>&1 || true
    elif command -v yum >/dev/null 2>&1; then
      yum install -y coreutils >/dev/null 2>&1 || true
    else
      print_warning "未找到包管理器，尝试下载 busybox 临时修复..."
      curl -L -o /bin/busybox https://busybox.net/downloads/binaries/1.36.1-x86_64-linux-musl/busybox
      chmod +x /bin/busybox
      cd /bin
      for i in mkdir cp mv rm ls echo cat ln date; do ln -sf busybox $i; done
    fi
    print_success "coreutils 环境已修复"
  fi

  if ! command -v apt-get >/dev/null 2>&1; then
    print_warning "检测到 apt-get 缺失，尝试恢复..."
    curl -L -o /tmp/apt.deb http://ftp.us.debian.org/debian/pool/main/a/apt/apt_2.6.3_amd64.deb
    dpkg -i /tmp/apt.deb || dpkg-deb -x /tmp/apt.deb /
    rm -f /tmp/apt.deb
    print_success "apt-get 已恢复"
  fi
}

# ========== 卸载旧 Bot ==========
uninstall_bot() {
  print_message "卸载旧版 VPS Telegram Bot"

  if systemctl is-active --quiet vps-tg-bot 2>/dev/null; then
    print_warning "停止旧服务..."
    systemctl stop vps-tg-bot
  fi
  if systemctl is-enabled --quiet vps-tg-bot 2>/dev/null; then
    systemctl disable vps-tg-bot
  fi
  rm -f "$BOT_SERVICE"
  systemctl daemon-reload
  rm -rf "$BOT_DIR" "$CORE_MAINTAIN_SCRIPT" "$RULES_MAINTAIN_SCRIPT"
  (crontab -l 2>/dev/null | grep -v "vps-maintain" || true) | crontab -
  print_success "旧版本已清理完毕"
}

# ========== 卸载 Go ==========
uninstall_go() {
  if command -v go >/dev/null 2>&1; then
    print_message "检测到 Go 环境，开始安全卸载..."
    GO_PATH=$(which go || true)
    GO_DIR=$(dirname "$(dirname "$GO_PATH")")
    print_warning "Go 安装路径: $GO_DIR"

    if dpkg -l | grep -q "golang-go"; then
      apt-get remove -y golang-go golang >/dev/null 2>&1 || true
      apt-get purge -y golang-go golang >/dev/null 2>&1 || true
    fi

    rm -rf /usr/local/go /usr/lib/go "$GO_DIR/go" >/dev/null 2>&1 || true
    sed -i '/GOPATH/d' ~/.bashrc ~/.profile 2>/dev/null || true
    hash -r 2>/dev/null || true
    print_success "Go 已安全卸载"
  fi
}

# ========== 权限检查 ==========
if [ "$EUID" -ne 0 ]; then
  print_error "请使用 root 用户执行此脚本"
  exit 1
fi

# ========== 修复基础环境 ==========
ensure_coreutils

# ========== 时区同步 ==========
sync_timezone() {
  print_message "同步 VPS 时区"
  local tz
  if command -v timedatectl &>/dev/null; then
    tz=$(timedatectl show -p Timezone --value)
  elif [ -f /etc/timezone ]; then
    tz=$(cat /etc/timezone)
  else
    tz="Etc/UTC"
  fi
  ln -sf "/usr/share/zoneinfo/$tz" /etc/localtime
  echo "$tz" > /etc/timezone
  print_success "当前 VPS 时区: $tz"
}
sync_timezone

# ========== 检查并卸载 Go ==========
print_message "步骤 0: 检查系统环境"
if command -v go &>/dev/null; then
  print_warning "检测到 Go 环境，自动卸载 Go 与旧 Bot..."
  uninstall_bot
  uninstall_go
else
  print_success "未检测到 Go，继续安装"
fi

# ========== Telegram 配置 ==========
print_message "步骤 1: 配置 Telegram Bot"
read -p "请输入 Telegram Bot Token: " TG_TOKEN
read -p "请输入 Telegram Chat ID (管理员): " TG_CHAT_ID
if [ -z "$TG_TOKEN" ] || [ -z "$TG_CHAT_ID" ]; then
  print_error "Token 和 Chat ID 不能为空"
  exit 1
fi

# ========== journald ==========
print_message "步骤 2: 配置 journald 内存日志"
mkdir -p /etc/systemd/journald.conf.d
cat > /etc/systemd/journald.conf.d/memory.conf <<'EOF'
[Journal]
Storage=volatile
RuntimeMaxUse=50M
Compress=yes
EOF
systemctl restart systemd-journald 2>/dev/null || true
print_success "journald 内存化完成"

# ========== 维护脚本 ==========
print_message "步骤 3: 创建维护脚本"
cat > "$CORE_MAINTAIN_SCRIPT" <<'EOF'
#!/bin/bash
set -e
export DEBIAN_FRONTEND=noninteractive
RESULT_FILE="/tmp/vps_maintain_result.txt"
TIME_NOW=$(date '+%Y-%m-%d %H:%M:%S')
apt-get update -o Acquire::ForceIPv4=true && apt-get -y upgrade && apt-get -y autoremove && apt-get clean
echo "✅ 系统更新完成于 $TIME_NOW" > "$RESULT_FILE"
EOF
chmod +x "$CORE_MAINTAIN_SCRIPT"
cat > "$RULES_MAINTAIN_SCRIPT" <<'EOF'
#!/bin/bash
set -e
RESULT_FILE="/tmp/vps_rules_result.txt"
TIME_NOW=$(date '+%Y-%m-%d %H:%M:%S')
if command -v xray &>/dev/null; then
  xray up dat && echo "✅ 规则更新完成 $TIME_NOW" > "$RESULT_FILE"
else
  echo "ℹ️ 未检测到 Xray" > "$RESULT_FILE"
fi
EOF
chmod +x "$RULES_MAINTAIN_SCRIPT"
print_success "维护脚本创建完成"

# ========== 下载预编译二进制 ==========
print_message "步骤 4: 下载预编译二进制"
mkdir -p "$BOT_DIR"
if [ -f "./vps-tg-bot-linux-amd64" ]; then
  cp ./vps-tg-bot-linux-amd64 "$BOT_BINARY"
  print_success "使用本地二进制文件"
else
  LATEST_URL=$(curl -s https://api.github.com/repos/SKIPPINGpetticoatconvent/Vps-auto-maintain/releases/latest | grep "browser_download_url.*vps-tg-bot-linux-amd64" | cut -d '"' -f 4)
  if [ -z "$LATEST_URL" ]; then
    LATEST_URL=$(curl -s https://ghproxy.com/https://api.github.com/repos/SKIPPINGpetticoatconvent/Vps-auto-maintain/releases/latest | grep "browser_download_url.*vps-tg-bot-linux-amd64" | cut -d '"' -f 4)
  fi
  curl -L -o "$BOT_BINARY" "$LATEST_URL"
  print_success "从 GitHub 下载最新版本成功"
fi
chmod +x "$BOT_BINARY"

# ========== 创建 systemd 服务 ==========
print_message "步骤 5: 创建 systemd 服务"
cat > "$BOT_SERVICE" <<EOF
[Unit]
Description=VPS Telegram Bot (Go)
After=network.target

[Service]
Type=simple
User=root
WorkingDirectory=$BOT_DIR
Environment="TG_TOKEN=$TG_TOKEN"
Environment="TG_CHAT_ID=$TG_CHAT_ID"
Environment="CORE_SCRIPT=$CORE_MAINTAIN_SCRIPT"
Environment="RULES_SCRIPT=$RULES_MAINTAIN_SCRIPT"
ExecStart=$BOT_BINARY
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
EOF
systemctl daemon-reload
systemctl enable vps-tg-bot
systemctl start vps-tg-bot
sleep 3

if systemctl is-active --quiet vps-tg-bot; then
  print_success "服务启动成功"
else
  print_error "服务启动失败，请执行: journalctl -u vps-tg-bot -n 50"
fi

# ========== 添加自动维护任务 ==========
print_message "步骤 6: 添加自动维护任务"
(crontab -l 2>/dev/null | grep -v "vps-maintain" ; echo "0 4 * * 0 bash $CORE_MAINTAIN_SCRIPT && bash $RULES_MAINTAIN_SCRIPT && reboot") | crontab -
print_success "已添加每周日 04:00 自动维护任务"

# ========== 完成提示 ==========
print_message "🎉 部署完成！"
print_success "Go 环境已安全清理，Bot 已重新部署"
print_success "服务后台运行中（SSH 关闭不影响）"
print_success "每周日 04:00 自动维护与重启"
print_warning "查看日志: journalctl -u vps-tg-bot -n 50 --no-pager"
print_warning "卸载命令: ./deploy.sh remove"
