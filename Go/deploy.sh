#!/bin/bash
# ----------------------------------------------------------------------------
# VPS Telegram Bot Go 版本 - 修复版部署脚本
#
# 版本: 2.0.7 (修复 "Text file busy" 错误)
# 修复内容:
#   ✅ 下载到临时文件再移动，避免覆盖运行中的二进制
#   ✅ 先停止服务再替换二进制文件
#   ✅ 添加下载重试机制
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
BOT_BINARY_TMP="$BOT_DIR/vps-tg-bot.tmp"
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
# ========== 参数处理 ==========
usage() {
  echo "Usage: $0 [install|uninstall]"
  echo "  install:    Installs or updates the VPS Telegram Bot."
  echo "  uninstall:  Uninstalls the VPS Telegram Bot and Go environment."
}

if [ "$1" == "uninstall" ]; then
  print_message "开始卸载 VPS Telegram Bot"
  if [ "$EUID" -ne 0 ]; then
    print_error "请使用 root 用户或 sudo 执行此脚本进行卸载"
    exit 1
  fi
  read -p "⚠️  您确定要卸载 VPS Telegram Bot 及其相关环境吗? (y/N): " confirm
  if [[ "$confirm" =~ ^[Yy]$ ]]; then
    uninstall_bot
    uninstall_go
    print_success "VPS Telegram Bot 已成功卸载。"
    print_success "Go 环境也已尝试卸载。"
    print_warning "请手动检查并删除剩余的配置文件（如果需要）"
    exit 0
  else
    print_warning "卸载已取消。"
    exit 0
  fi
elif [ "$1" == "install" ] || [ -z "$1" ]; then
  # Proceed with installation
  : # No-op, continue script
else
  usage
  exit 1
fi

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
sudo apt update && sudo apt full-upgrade -y && sudo apt autoremove -y && sudo apt autoclean
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

# ========== 停止旧服务（关键修复点）==========
print_message "步骤 4: 准备安装新版本"
if systemctl is-active --quiet vps-tg-bot 2>/dev/null; then
  print_warning "停止现有服务..."
  systemctl stop vps-tg-bot
  sleep 2
fi

# ========== 下载预编译二进制（修复版）==========
print_message "步骤 5: 下载预编译二进制"
mkdir -p "$BOT_DIR"

# 检查本地文件
if [ -f "./vps-tg-bot-linux-amd64" ]; then
  cp ./vps-tg-bot-linux-amd64 "$BOT_BINARY"
  print_success "使用本地二进制文件"
else
  print_warning "从 GitHub 下载最新版本..."
  
  # 获取下载链接
  print_warning "从 GitHub 下载最新版本..."
  
  # 尝试多个仓库和镜像源
  REPOS=("FTDRTD/Vps-auto-maintain" "SKIPPINGpetticoatconvent/Vps-auto-maintain")
  MIRRORS=("" "https://ghproxy.com/https://" "https://mirror.ghproxy.com/https://" "https://pd.zwc365.com/https://")
  
  LATEST_URL=""
  
  for REPO in "${REPOS[@]}"; do
    for MIRROR in "${MIRRORS[@]}"; do
      print_warning "尝试从 $MIRROR$REPO 获取下载链接..."
      
      API_URL="${MIRROR}api.github.com/repos/${REPO}/releases/latest"
      TEMP_URL=$(curl -s --max-time 10 "$API_URL" | grep -oE '"browser_download_url":\s*"([^"]+vps-tg-bot-go-linux-amd64[^"]*)' | cut -d'"' -f4 | head -n1)
      
      if [ -n "$TEMP_URL" ]; then
        LATEST_URL="$TEMP_URL"
        print_success "找到下载链接: $LATEST_URL"
        break 2
      fi
    done
  done
  
  if [ -z "$LATEST_URL" ]; then
    print_error "无法从任何源获取下载链接"
    print_error "请检查网络连接或手动下载二进制文件"
    exit 1
  fi
  
  print_warning "下载地址: $LATEST_URL"
  
  # 下载到临时文件（关键修复点）
  rm -f "$BOT_BINARY_TMP"
  
  # 重试下载
  MAX_RETRY=3
  RETRY=0
  while [ $RETRY -lt $MAX_RETRY ]; do
    if curl -L -o "$BOT_BINARY_TMP" "$LATEST_URL"; then
      print_success "下载成功"
      break
    else
      RETRY=$((RETRY+1))
      if [ $RETRY -lt $MAX_RETRY ]; then
        print_warning "下载失败，重试 $RETRY/$MAX_RETRY..."
        sleep 2
      else
        print_error "下载失败，已重试 $MAX_RETRY 次"
        exit 1
      fi
    fi
  done
  
  # 验证文件
  if [ ! -f "$BOT_BINARY_TMP" ]; then
    print_error "下载的文件不存在"
    exit 1
  fi
  
  if [ ! -s "$BOT_BINARY_TMP" ]; then
    print_error "下载的文件为空"
    rm -f "$BOT_BINARY_TMP"
    exit 1
  fi
  
  # 移动文件（关键修复点）
  rm -f "$BOT_BINARY"
  mv "$BOT_BINARY_TMP" "$BOT_BINARY"
  print_success "文件安装完成"
fi

chmod +x "$BOT_BINARY"

# ========== 创建 systemd 服务 ==========
print_message "步骤 6: 创建 systemd 服务"
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
  exit 1
fi

# ========== 添加自动维护任务 ==========
print_message "步骤 7: 添加自动维护任务"
(crontab -l 2>/dev/null | grep -v "vps-maintain" ; echo "0 4 * * 0 bash $CORE_MAINTAIN_SCRIPT && bash $RULES_MAINTAIN_SCRIPT && reboot") | crontab -
print_success "已添加每周日 04:00 自动维护任务"

# ========== 完成提示 ==========
print_message "🎉 部署完成！"
print_success "Go 环境已安全清理，Bot 已重新部署"
print_success "服务后台运行中（SSH 关闭不影响）"
print_success "每周日 04:00 自动维护与重启"
print_warning "查看日志: journalctl -u vps-tg-bot -f"
print_warning "查看状态: systemctl status vps-tg-bot"
print_warning "重启服务: systemctl restart vps-tg-bot"
print_warning "卸载命令: (待添加)"

echo ""
echo "============================================================"
echo "📱 现在可以在 Telegram 中发送 /start 测试 Bot"
echo "============================================================"
