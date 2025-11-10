#!/bin/bash
# ----------------------------------------------------------------------------
# VPS Telegram Bot 管理系统 - 一键部署脚本 (Go + Python)
#
# 版本: 2.0.0
# 功能:
#   ✅ 自动检测并部署 Go 或 Python 版本
#   ✅ 自动同步 VPS 时区
#   ✅ 每周日 04:00 自动维护 (系统+规则更新+重启)
#   ✅ 创建 systemd 服务 (后台运行)
#   ✅ SSH 终端关闭后程序继续运行
# ----------------------------------------------------------------------------

set -e

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

# --- 检测部署类型 ---
print_message "检测部署类型"

if [ -f "vps-tg-bot-linux-amd64" ] || [ -f "Go/dist/vps-tg-bot" ] || [ -f "Go/cmd/vps-tg-bot/main.go" ]; then
  DEPLOY_TYPE="go"
  echo "✅ 检测到 Go 版本，将部署 Go 版本"
elif [ -f "bot.py" ] || [ -f "main.py" ] || [ -d "src" ]; then
  DEPLOY_TYPE="python"
  echo "✅ 检测到 Python 版本，将部署 Python 版本"
else
  echo "❌ 未检测到有效的项目文件"
  echo "请确保当前目录包含以下文件之一："
  echo "  - Go 版本: vps-tg-bot-linux-amd64, Go/dist/vps-tg-bot, 或 Go/cmd/vps-tg-bot/main.go"
  echo "  - Python 版本: bot.py, main.py, 或 src 目录"
  exit 1
fi

# --- 根据类型执行相应部署 ---
if [ "$DEPLOY_TYPE" = "go" ]; then
  # Go 版本部署
  if [ ! -f "Go/vps-tg-bot-install.sh" ]; then
    echo "❌ 找不到 Go 版本部署脚本: Go/vps-tg-bot-install.sh"
    exit 1
  fi

  echo "🚀 开始部署 Go 版本..."
  chmod +x Go/vps-tg-bot-install.sh
  ./Go/vps-tg-bot-install.sh

elif [ "$DEPLOY_TYPE" = "python" ]; then
  # Python 版本部署
  if [ ! -f "vps-tg-bot-install.sh" ]; then
    echo "❌ 找不到 Python 版本部署脚本: vps-tg-bot-install.sh"
    exit 1
  fi

  echo "🚀 开始部署 Python 版本..."
  chmod +x vps-tg-bot-install.sh
  ./vps-tg-bot-install.sh
fi

print_message "🎉 部署完成！"
echo "✅ 服务已在后台运行，即使 SSH 终端关闭也不会停止"
echo "✅ 每周日 04:00 会自动执行系统维护"
echo "📱 前往 Telegram 发送 /start 开始使用"