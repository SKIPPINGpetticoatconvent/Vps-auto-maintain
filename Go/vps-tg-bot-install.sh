#!/bin/bash
# -----------------------------------------------------------------------------
# VPS Telegram Bot 管理系统 - Go 版本一键部署脚本
#
# 版本: 1.0.0
# 功能:
#   ✅ 自动同步 VPS 时区
#   ✅ 每周日 04:00 自动维护 (系统+规则更新+重启)
#   ✅ 使用 Go 编译二进制文件
#   ✅ 创建 systemd 服务
# -----------------------------------------------------------------------------

set -e

BOT_DIR="/opt/vps-tg-bot"
BOT_BINARY="$BOT_DIR/vps-tg-bot"
BOT_SERVICE="/etc/systemd/system/vps-tg-bot.service"
CORE_MAINTAIN_SCRIPT="/usr/local/bin/vps-maintain-core.sh"
RULES_MAINTAIN_SCRIPT="/usr/local/bin/vps-maintain-rules.sh"

print_message() {
  echo ""
  echo "============================================================"
  echo "$1"
  echo "============================================================"
}

# --- 自动同步 VPS 时区 ---
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

  if [ -z "$tz" ] || [ ! -f "/usr/share/zoneinfo/$tz" ]; then
    tz="Etc/UTC"
  fi

  ln -sf "/usr/share/zoneinfo/$tz" /etc/localtime
  echo "$tz" > /etc/timezone
  echo "✅ 当前 VPS 时区: $tz"
}

# --- 检查 root 权限 ---
if [ "$EUID" -ne 0 ]; then
  echo "❌ 请使用 root 用户或 sudo 执行此脚本"
  exit 1
fi

sync_timezone

# --- 步骤 0: 环境检查 ---
print_message "步骤 0: 检查系统环境"

if ! command -v go &>/dev/null; then
  echo "📦 安装 Go..."
  apt-get update -o Acquire::ForceIPv4=true && apt-get install -y golang-go
fi

GO_VERSION=$(go version)
echo "✅ Go 已安装: $GO_VERSION"

# --- 清理旧版本 ---
print_message "清理旧版本文件与服务"
systemctl stop vps-tg-bot 2>/dev/null || true
systemctl disable vps-tg-bot 2>/dev/null || true
rm -rf "$BOT_DIR" "$BOT_SERVICE" "$CORE_MAINTAIN_SCRIPT" "$RULES_MAINTAIN_SCRIPT"
(crontab -l 2>/dev/null | grep -v "vps-maintain" || true) | crontab -
echo "✅ 清理完成"

# --- 步骤 1: 配置 Telegram Bot ---
print_message "步骤 1: 配置 Telegram Bot"
read -p "请输入你的 Telegram Bot Token: " TG_TOKEN
read -p "请输入你的 Telegram Chat ID (管理员): " TG_CHAT_ID
if [ -z "$TG_TOKEN" ] || [ -z "$TG_CHAT_ID" ]; then
  echo "❌ 错误：Token 和 Chat ID 不能为空"
  exit 1
fi

# --- 步骤 2: journald 内存化 ---
print_message "步骤 2: 配置 journald 内存日志"
mkdir -p /etc/systemd/journald.conf.d
cat > /etc/systemd/journald.conf.d/memory.conf <<'EOF'
[Journal]
Storage=volatile
RuntimeMaxUse=50M
Compress=yes
EOF
systemctl restart systemd-journald 2>/dev/null || true
echo "✅ journald 内存化完成"

# --- 步骤 3: 创建维护脚本 ---
print_message "步骤 3: 创建维护脚本"

cat > "$CORE_MAINTAIN_SCRIPT" <<'EOF'
#!/bin/bash
set -e
TIMEZONE=$(timedatectl show -p Timezone --value 2>/dev/null || cat /etc/timezone)
TIME_NOW=$(date '+%Y-%m-%d %H:%M:%S')
RESULT_FILE="/tmp/vps_maintain_result.txt"
export DEBIAN_FRONTEND=noninteractive

echo "开始系统更新..." > "$RESULT_FILE"
if command -v apt-get &>/dev/null; then
  apt-get update -o Acquire::ForceIPv4=true && apt-get -y upgrade && apt-get -y autoremove && apt-get clean \
    && echo "✅ 系统更新成功" >> "$RESULT_FILE" \
    || echo "❌ 系统更新失败" >> "$RESULT_FILE"
fi

if command -v xray &>/dev/null; then
  xray up 2>&1 && echo "✅ Xray 更新成功" >> "$RESULT_FILE" || echo "❌ Xray 更新失败" >> "$RESULT_FILE"
else
  echo "ℹ️ Xray 未安装" >> "$RESULT_FILE"
fi

if command -v sb &>/dev/null; then
  sb up 2>&1 && echo "✅ Sing-box 更新成功" >> "$RESULT_FILE" || echo "❌ Sing-box 更新失败" >> "$RESULT_FILE"
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
TIMEZONE=$(timedatectl show -p Timezone --value 2>/dev/null || cat /etc/timezone)
TIME_NOW=$(date '+%Y-%m-%d %H:%M:%S')
RESULT_FILE="/tmp/vps_rules_result.txt"

if ! command -v xray &>/dev/null; then
  echo "ℹ️ Xray 未安装" > "$RESULT_FILE"
  exit 0
fi

xray up dat 2>&1 && echo "✅ Xray 规则文件更新成功" > "$RESULT_FILE" || echo "❌ Xray 规则文件更新失败" > "$RESULT_FILE"
echo "时区: $TIMEZONE" >> "$RESULT_FILE"
echo "时间: $TIME_NOW" >> "$RESULT_FILE"
EOF
chmod +x "$RULES_MAINTAIN_SCRIPT"
echo "✅ 维护脚本创建完成"

# --- 步骤 4: 获取或编译 Go 程序 ---
print_message "步骤 4: 获取或编译 Go 程序"
mkdir -p "$BOT_DIR"

# 获取脚本所在目录
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SOURCE_DIR="$SCRIPT_DIR"

# 检查是否已有预编译二进制文件（优先检查多个位置）
if [ -f "../vps-tg-bot-linux-amd64" ]; then
  echo "✅ 发现预编译二进制文件 ../vps-tg-bot-linux-amd64，使用现有文件"
  cp ../vps-tg-bot-linux-amd64 "$BOT_BINARY"
elif [ -f "vps-tg-bot-linux-amd64" ]; then
  echo "✅ 发现预编译二进制文件 vps-tg-bot-linux-amd64，使用现有文件"
  cp vps-tg-bot-linux-amd64 "$BOT_BINARY"
elif [ -f "$SOURCE_DIR/../vps-tg-bot-linux-amd64" ]; then
  echo "✅ 发现预编译二进制文件在上级目录，使用现有文件"
  cp "$SOURCE_DIR/../vps-tg-bot-linux-amd64" "$BOT_BINARY"
elif [ -f "$SOURCE_DIR/dist/vps-tg-bot" ]; then
  echo "✅ 发现预编译二进制文件，使用现有文件"
  cp "$SOURCE_DIR/dist/vps-tg-bot" "$BOT_BINARY"
elif [ -f "$SOURCE_DIR/vps-tg-bot" ]; then
  echo "✅ 发现二进制文件，使用现有文件"
  cp "$SOURCE_DIR/vps-tg-bot" "$BOT_BINARY"
else
  echo "📦 未发现预编译文件，开始编译 Go 程序"

  # 检查源代码目录是否存在
  if [ ! -f "$SOURCE_DIR/cmd/vps-tg-bot/main.go" ]; then
    echo "❌ 错误：找不到源代码文件 $SOURCE_DIR/cmd/vps-tg-bot/main.go"
    echo "请确保在 Go 项目根目录下运行此脚本"
    exit 1
  fi

  cd "$SOURCE_DIR"

  # 下载依赖
  echo "📦 下载 Go 依赖..."
  go mod download

  # 编译二进制文件
  echo "🔨 编译二进制文件..."
  GOOS=linux GOARCH=amd64 go build -o "$BOT_BINARY" ./cmd/vps-tg-bot

  if [ ! -f "$BOT_BINARY" ]; then
    echo "❌ 编译失败"
    exit 1
  fi
fi

chmod +x "$BOT_BINARY"
echo "✅ Go 程序准备完成"

# --- 步骤 5: 创建 systemd 服务 ---
print_message "步骤 5: 创建 systemd 服务"

cat > "$BOT_SERVICE" <<EOF
[Unit]
Description=VPS Telegram Bot Management System (Go)
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
  echo "✅ 服务启动成功"
else
  echo "❌ 服务启动失败，请查看日志: journalctl -u vps-tg-bot -n 50"
fi

print_message "🎉 部署完成！"
echo "✅ 每周维护任务已自动设置 (每周日 04:00)"
echo "📱 前往 Telegram 发送 /start 开始使用"
echo "♻️ 支持功能：系统状态、立即维护、查看日志、重启 VPS"
