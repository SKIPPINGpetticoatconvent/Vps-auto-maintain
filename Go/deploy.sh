#!/bin/bash
# ----------------------------------------------------------------------------
# VPS Telegram Bot Go 版本 - 一键部署脚本 (纯部署 + 自动清理 Go)
#
# 版本: 2.0.5
# 作者: FTDRTD
# 功能:
#   ✅ 检测到 Go 自动卸载 Go 及旧版本
#   ✅ 优先使用预编译二进制文件（无需本地构建）
#   ✅ 自动下载 GitHub Release（含 ghproxy 备用）
#   ✅ 自动同步 VPS 时区
#   ✅ 每周日 04:00 自动维护 (系统+规则更新+重启)
#   ✅ 创建 systemd 服务 (后台运行)
#   ✅ SSH 关闭后持续运行
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

# ========== 卸载旧版本 ==========
uninstall_bot() {
  print_message "卸载旧版本 VPS Telegram Bot"

  if systemctl is-active --quiet vps-tg-bot 2>/dev/null; then
    print_warning "停止 vps-tg-bot 服务..."
    systemctl stop vps-tg-bot
  fi

  if systemctl is-enabled --quiet vps-tg-bot 2>/dev/null; then
    print_warning "禁用 vps-tg-bot 开机启动..."
    systemctl disable vps-tg-bot
  fi

  rm -f "$BOT_SERVICE"
  systemctl daemon-reload

  rm -rf "$BOT_DIR" "$CORE_MAINTAIN_SCRIPT" "$RULES_MAINTAIN_SCRIPT"
  rm -f "/tmp/vps_maintain_result.txt" "/tmp/vps_rules_result.txt"
  (crontab -l 2>/dev/null | grep -v "vps-maintain" || true) | crontab -

  print_success "VPS Telegram Bot 已完全卸载"
}

# ========== 卸载 Go ==========
uninstall_go() {
  if command -v go &>/dev/null; then
    print_message "检测到 Go 环境，开始卸载 Go..."
    GO_PATH=$(which go || true)
    GO_DIR=$(dirname "$(dirname "$GO_PATH")")
    print_warning "检测到 Go 安装路径: $GO_DIR"

    # Debian / Ubuntu 系统包卸载
    if dpkg -l | grep -q golang; then
      print_warning "检测到 golang 软件包，正在卸载..."
      apt-get remove -y golang golang-go golang-* >/dev/null 2>&1 || true
      apt-get purge -y golang* >/dev/null 2>&1 || true
    fi

    # 删除 /usr/local/go 或 /usr/lib/go
    rm -rf /usr/local/go /usr/lib/go "$GO_DIR" >/dev/null 2>&1 || true

    # 清理环境变量
    sed -i '/\/go/d' ~/.bashrc ~/.profile 2>/dev/null || true
    sed -i '/GOPATH/d' ~/.bashrc ~/.profile 2>/dev/null || true
    hash -r 2>/dev/null || true

    print_success "Go 已成功卸载"
  fi
}

# ========== 参数检查 ==========
if [ "$1" = "remove" ] || [ "$1" = "uninstall" ]; then
  if [ "$EUID" -ne 0 ]; then
    print_error "请使用 root 用户执行"
    exit 1
  fi
  uninstall_bot
  uninstall_go
  print_success "已清理 Go 与 Bot 环境"
  exit 0
fi

# ========== 权限检查 ==========
if [ "$EUID" -ne 0 ]; then
  print_error "请使用 root 用户或 sudo 执行此脚本"
  exit 1
fi

# ========== 同步时区 ==========
sync_timezone() {
  print_message "同步 VPS 时区配置"
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

# ========== 检测并清理 Go ==========
print_message "步骤 0: 检查系统环境"
if command -v go &>/dev/null; then
  print_warning "检测到 Go 环境，自动卸载 Go 与旧版本..."
  uninstall_bot
  uninstall_go
else
  print_success "未检测到 Go，继续安装流程"
fi

# ========== Telegram 配置 ==========
print_message "步骤 1: 配置 Telegram Bot"
read -p "请输入 Telegram Bot Token: " TG_TOKEN
read -p "请输入 Telegram Chat ID (管理员): " TG_CHAT_ID
if [ -z "$TG_TOKEN" ] || [ -z "$TG_CHAT_ID" ]; then
  print_error "Token 和 Chat ID 不能为空"
  exit 1
fi

# ========== journald 内存日志 ==========
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

# ========== 创建维护脚本 ==========
print_message "步骤 3: 创建维护脚本"

cat > "$CORE_MAINTAIN_SCRIPT" <<'EOF'
#!/bin/bash
set -e
export DEBIAN_FRONTEND=noninteractive
RESULT_FILE="/tmp/vps_maintain_result.txt"
TIMEZONE=$(timedatectl show -p Timezone --value 2>/dev/null || cat /etc/timezone)
TIME_NOW=$(date '+%Y-%m-%d %H:%M:%S')

echo "开始系统更新..." > "$RESULT_FILE"
if command -v apt-get &>/dev/null; then
  apt-get update -o Acquire::ForceIPv4=true && apt-get -y upgrade && apt-get -y autoremove && apt-get clean \
    && echo "✅ 系统更新成功" >> "$RESULT_FILE" \
    || echo "❌ 系统更新失败" >> "$RESULT_FILE"
fi

if command -v xray &>/dev/null; then
  xray up && echo "✅ Xray 更新成功" >> "$RESULT_FILE" || echo "❌ Xray 更新失败" >> "$RESULT_FILE"
else
  echo "ℹ️ Xray 未安装" >> "$RESULT_FILE"
fi

if command -v sb &>/dev/null; then
  sb up && echo "✅ Sing-box 更新成功" >> "$RESULT_FILE" || echo "❌ Sing-box 更新失败" >> "$RESULT_FILE"
else
  echo "ℹ️ Sing-box 未安装" >> "$RESULT_FILE"
fi

echo "时区: $TIMEZONE" >> "$RESULT_FILE"
echo "时间: $TIME_NOW" >> "$RESULT_FILE"
EOF
chmod +x "$CORE_MAINTAIN_SCRIPT"

cat > "$RULES_MAINTAIN_SCRIPT" <<'EOF'
#!/bin/bash
set -e
RESULT_FILE="/tmp/vps_rules_result.txt"
TIMEZONE=$(timedatectl show -p Timezone --value 2>/dev/null || cat /etc/timezone)
TIME_NOW=$(date '+%Y-%m-%d %H:%M:%S')

if command -v xray &>/dev/null; then
  xray up dat && echo "✅ Xray 规则文件更新成功" > "$RESULT_FILE" || echo "❌ Xray 规则文件更新失败" > "$RESULT_FILE"
else
  echo "ℹ️ Xray 未安装" > "$RESULT_FILE"
fi

echo "时区: $TIMEZONE" >> "$RESULT_FILE"
echo "时间: $TIME_NOW" >> "$RESULT_FILE"
EOF
chmod +x "$RULES_MAINTAIN_SCRIPT"
print_success "维护脚本创建完成"

# ========== 下载预编译二进制 ==========
print_message "步骤 4: 下载预编译二进制文件"
mkdir -p "$BOT_DIR"

echo "📦 正在从 GitHub 获取最新版本..."
LATEST_URL=$(curl -s https://api.github.com/repos/SKIPPINGpetticoatconvent/Vps-auto-maintain/releases/latest | grep "browser_download_url.*vps-tg-bot-linux-amd64" | cut -d '"' -f 4)
if [ -z "$LATEST_URL" ]; then
  print_warning "GitHub API 获取失败，尝试 ghproxy 镜像..."
  LATEST_URL=$(curl -s https://ghproxy.com/https://api.github.com/repos/SKIPPINGpetticoatconvent/Vps-auto-maintain/releases/latest | grep "browser_download_url.*vps-tg-bot-linux-amd64" | cut -d '"' -f 4)
fi

if [ -n "$LATEST_URL" ]; then
  curl -L -o "$BOT_BINARY" "$LATEST_URL"
else
  print_error "无法获取下载地址，请检查网络或手动提供二进制文件"
  exit 1
fi

chmod +x "$BOT_BINARY"
print_success "二进制文件下载完成"

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
print_success "Go 已清理干净，Bot 已重新部署"
print_success "服务后台运行中（SSH 关闭不影响）"
print_success "每周日 04:00 自动维护与重启"
print_success "Telegram 发送 /start 开始使用"
print_warning "查看日志: journalctl -u vps-tg-bot -n 50 --no-pager"
print_warning "卸载命令: ./deploy.sh remove"
