#!/bin/bash
# -----------------------------------------------------------------------------
# VPS Telegram Bot 管理系统 - 一键部署脚本 (使用 uv)
#
# 版本: 5.3.3-stable
# 作者: FTDRTD
# 功能:
#   ✅ 自动同步 VPS 时区
#   ✅ 每周日 04:00 自动维护 (系统+规则更新+重启)
#   ✅ 使用 uv 包管理器 (支持 0.9+)
#   ✅ 使用 .venv/bin/python 启动
#   ✅ 新增 ♻️ 一键重启 功能
#   ✅ 新增 🧹 一键卸载模式 (--uninstall)
# -----------------------------------------------------------------------------

set -e

BOT_DIR="/opt/vps-tg-bot"
BOT_SCRIPT="$BOT_DIR/vps-tg-bot.py"
BOT_SERVICE="/etc/systemd/system/vps-tg-bot.service"
CORE_MAINTAIN_SCRIPT="/usr/local/bin/vps-maintain-core.sh"
RULES_MAINTAIN_SCRIPT="/usr/local/bin/vps-maintain-rules.sh"

# --- 检查是否执行卸载模式 ---
if [[ "$1" == "--uninstall" || "$1" == "uninstall" ]]; then
  echo ""
  echo "============================================================"
  echo "🧹 VPS Telegram Bot 管理系统 - 卸载模式"
  echo "============================================================"
  echo ""
  read -p "⚠️ 确认要卸载 VPS Bot 管理系统吗？(y/N): " confirm
  if [[ ! "$confirm" =~ ^[Yy]$ ]]; then
    echo "❎ 已取消卸载操作。"
    exit 0
  fi

  echo ""
  echo "🧩 正在执行卸载操作..."

  systemctl stop vps-tg-bot 2>/dev/null || true
  systemctl disable vps-tg-bot 2>/dev/null || true

  rm -rf "$BOT_DIR" "$BOT_SERVICE" "$CORE_MAINTAIN_SCRIPT" "$RULES_MAINTAIN_SCRIPT"
  (crontab -l 2>/dev/null | grep -v "vps-maintain" || true) | crontab -

  if [ -f /etc/systemd/journald.conf.d/memory.conf ]; then
    rm -f /etc/systemd/journald.conf.d/memory.conf
    systemctl restart systemd-journald 2>/dev/null || true
  fi

  rm -f /tmp/vps_maintain_result.txt /tmp/vps_rules_result.txt /var/log/vps-tg-bot.log 2>/dev/null || true

  echo ""
  echo "✅ 卸载完成！"
  echo "所有相关服务与文件已清理干净。"
  echo "如需重新安装，请重新执行部署脚本。"
  echo "============================================================"
  exit 0
fi

print_message() {
  echo ""
  echo "============================================================"
  echo "$1"
  echo "============================================================"
}

# --- 检查 root 权限 ---
if [ "$EUID" -ne 0 ]; then
  echo "❌ 请使用 root 用户或 sudo 执行此脚本"
  exit 1
fi

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
sync_timezone

# --- 步骤 0: 检查系统环境 ---
print_message "步骤 0: 检查系统环境"
if ! command -v curl &>/dev/null; then
  echo "📦 安装 curl..."
  apt-get update -o Acquire::ForceIPv4=true && apt-get install -y curl
fi
if ! command -v uv &>/dev/null; then
  echo "📦 安装 uv 包管理器..."
  curl -LsSf https://astral.sh/uv/install.sh | sh
  export PATH="$HOME/.local/bin:$HOME/.cargo/bin:$PATH"
fi
UV_BIN=$(command -v uv)
echo "✅ uv 已安装: $UV_BIN"

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
  sudo apt update && sudo apt full-upgrade -y && sudo apt autoremove -y && sudo apt autoclean \
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

# --- 步骤 4: 创建 Python 环境 ---
print_message "步骤 4: 初始化 Python 项目"
mkdir -p "$BOT_DIR"
cd "$BOT_DIR"

"$UV_BIN" init --no-readme --name vps-tg-bot
"$UV_BIN" venv --python 3.12 .venv
"$UV_BIN" add --frozen \
  "python-telegram-bot==13.15" \
  "urllib3<2.0" \
  "tzlocal<3.0" \
  "requests" \
  "pytz" \
  "SQLAlchemy<2.0" \
  "apscheduler==3.6.3"
"$UV_BIN" sync
echo "✅ Python 环境安装完成"

# --- 步骤 5: 创建 Bot 主程序 ---
print_message "步骤 5: 创建 Telegram Bot 主程序"

cat > "$BOT_SCRIPT" <<'EOF'
#!/usr/bin/env python3
# -*- coding: utf-8 -*-
import logging, subprocess, os, time, pytz
from datetime import datetime
from telegram import Update, InlineKeyboardButton, InlineKeyboardMarkup, ParseMode
from telegram.ext import Updater, CommandHandler, CallbackQueryHandler, CallbackContext
from telegram.helpers import escape_markdown
from apscheduler.schedulers.background import BackgroundScheduler
from apscheduler.triggers.cron import CronTrigger
from apscheduler.jobstores.sqlalchemy import SQLAlchemyJobStore

logging.basicConfig(format='%(asctime)s - %(name)s - %(levelname)s - %(message)s', level=logging.INFO)
logger = logging.getLogger(__name__)

TOKEN = '__TG_TOKEN__'
ADMIN_CHAT_ID = '__TG_CHAT_ID__'
CORE_SCRIPT = '/usr/local/bin/vps-maintain-core.sh'
RULES_SCRIPT = '/usr/local/bin/vps-maintain-rules.sh'
jobstores = {'default': SQLAlchemyJobStore(url='sqlite:///jobs.sqlite')}

try:
    tz_name = subprocess.getoutput("timedatectl show -p Timezone --value").strip()
    if not tz_name:
        tz_name = open("/etc/timezone").read().strip()
    SYSTEM_TZ = pytz.timezone(tz_name)
except Exception:
    SYSTEM_TZ = pytz.UTC

scheduler = BackgroundScheduler(jobstores=jobstores, timezone=SYSTEM_TZ)

def send_message(text):
    try:
        updater = Updater(TOKEN, use_context=True)
        updater.bot.send_message(chat_id=ADMIN_CHAT_ID, text=text, parse_mode=ParseMode.MARKDOWN)
    except Exception as e:
        logger.error(f"发送消息失败: {e}")

def start(update: Update, context: CallbackContext):
    if str(update.effective_chat.id) != ADMIN_CHAT_ID:
        update.message.reply_text("❌ 无权限访问此 Bot"); return
    keyboard = [
        [InlineKeyboardButton("📊 系统状态", callback_data='status')],
        [InlineKeyboardButton("🔧 立即维护", callback_data='maintain_core')],
        [InlineKeyboardButton("📋 查看日志", callback_data='logs')],
        [InlineKeyboardButton("♻️ 重启 VPS", callback_data='reboot')]
    ]
    update.message.reply_text("🤖 *VPS 管理 Bot*\n\n请选择操作：", reply_markup=InlineKeyboardMarkup(keyboard), parse_mode=ParseMode.MARKDOWN)

def button(update: Update, context: CallbackContext):
    query = update.callback_query; query.answer()
    if str(query.message.chat.id) != ADMIN_CHAT_ID:
        query.edit_message_text("❌ 无权限访问"); return
    if query.data == 'status':
        info = subprocess.getoutput("uptime && date")
        query.edit_message_text(f"📊 *系统状态*\n\n```\n{escape_markdown(info, version=2)}\n```", parse_mode=ParseMode.MARKDOWN_V2)
    elif query.data == 'maintain_core':
        query.edit_message_text("⏳ 正在执行维护，请稍候...")
        subprocess.run([CORE_SCRIPT], check=False)
        result = open("/tmp/vps_maintain_result.txt").read()
        query.edit_message_text(f"✅ *维护完成*\n\n```\n{escape_markdown(result, version=2)}\n```\n\n⚠️ 系统将在 5 秒后重启", parse_mode=ParseMode.MARKDOWN_V2)
        time.sleep(5); reboot_system()
    elif query.data == 'logs':
        logs = subprocess.getoutput("journalctl -u vps-tg-bot -n 20 --no-pager")
        query.edit_message_text(f"📋 *日志*\n\n```\n{escape_markdown(logs[-2000:], version=2)}\n```", parse_mode=ParseMode.MARKDOWN_V2)
    elif query.data == 'reboot':
        query.edit_message_text("⚠️ 系统将在 5 秒后重启...")
        time.sleep(5); reboot_system()

def reboot_system():
    if os.path.exists("/sbin/reboot"):
        subprocess.run(["/sbin/reboot"], check=False)
    else:
        subprocess.run(["shutdown", "-r", "now"], check=False)

def scheduled_task():
    subprocess.run([RULES_SCRIPT], check=False)
    subprocess.run([CORE_SCRIPT], check=False)
    send_message("🕒 定时维护已执行，系统将在 5 秒后自动重启")
    time.sleep(5); reboot_system()

def main():
    updater = Updater(TOKEN, use_context=True)
    dp = updater.dispatcher
    dp.add_handler(CommandHandler("start", start))
    dp.add_handler(CallbackQueryHandler(button))
    scheduler.add_job(scheduled_task, CronTrigger(day_of_week='sun', hour=4, minute=0), id='weekly_task', replace_existing=True)
    scheduler.start()
    send_message("🤖 *VPS 管理 Bot 已启动*\n\n使用 /start 打开管理面板")
    updater.start_polling(); updater.idle()

if __name__ == '__main__':
    main()
EOF

sed -i "s|__TG_TOKEN__|$TG_TOKEN|g" "$BOT_SCRIPT"
sed -i "s|__TG_CHAT_ID__|$TG_CHAT_ID|g" "$BOT_SCRIPT"
chmod +x "$BOT_SCRIPT"
echo "✅ Bot 主程序创建完成"

# --- 步骤 6: 创建 systemd 服务 ---
print_message "步骤 6: 创建 systemd 服务"

cat > "$BOT_SERVICE" <<EOF
[Unit]
Description=VPS Telegram Bot Management System
After=network.target

[Service]
Type=simple
User=root
WorkingDirectory=$BOT_DIR
ExecStart=$BOT_DIR/.venv/bin/python $BOT_SCRIPT
Restart=always
RestartSec=10
StandardOutput=journal
StandardError=journal

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
echo "♻️ 新增按钮：重启 VPS"
echo "🧹 支持 --uninstall 模式安全卸载"
