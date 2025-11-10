#!/bin/bash
# -----------------------------------------------------------------------------
# VPS Telegram Bot 管理系统 - 一键部署脚本 (使用 uv)
#
# 版本: 5.3.1-weekly-fix2
# 作者: FTDRTD
# 功能:
#   ✅ 自动兼容 VPS 时区 (同步 /etc/localtime 与 /etc/timezone)
#   ✅ 默认每周日 04:00 执行完整维护 (系统+规则更新+自动重启)
#   ✅ 使用 uv 包管理器 (支持 0.9+)
#   ✅ 固定 apscheduler==3.6.3 解决 PTB 兼容性冲突
#   ✅ 使用 .venv/bin/python 启动，完全离线运行
# -----------------------------------------------------------------------------

set -e

BOT_DIR="/opt/vps-tg-bot"
BOT_SCRIPT="$BOT_DIR/vps-tg-bot.py"
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

# --- 步骤 0: 检查环境 ---
print_message "步骤 0: 检查系统环境"

if ! command -v curl &>/dev/null; then
  echo "📦 安装 curl..."
  apt-get update -o Acquire::ForceIPv4=true && apt-get install -y curl
fi

echo "📦 检查 uv 包管理器..."
if ! command -v uv &>/dev/null; then
  echo "正在安装 uv..."
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
echo "✅ 环境准备完成"

# --- 步骤 1: 获取 Token ---
print_message "步骤 1: 配置 Telegram Bot"
read -p "请输入你的 Telegram Bot Token: " TG_TOKEN
read -p "请输入你的 Telegram Chat ID (管理员): " TG_CHAT_ID
if [ -z "$TG_TOKEN" ] || [ -z "$TG_CHAT_ID" ]; then
  echo "❌ 错误：Token 和 Chat ID 不能为空"
  exit 1
fi

# --- 步骤 2: 配置 journald ---
print_message "步骤 2: 配置系统日志内存存储"
mkdir -p /etc/systemd/journald.conf.d
cat > /etc/systemd/journald.conf.d/memory.conf <<'EOF'
[Journal]
Storage=volatile
RuntimeMaxUse=50M
Compress=yes
EOF
systemctl restart systemd-journald 2>/dev/null || true
echo "✅ journald 内存化配置完成"

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

# --- 步骤 4: 创建 Python 项目 ---
print_message "步骤 4: 使用 uv 创建 Python 项目"
mkdir -p "$BOT_DIR"
cd "$BOT_DIR"

"$UV_BIN" init --no-readme --name vps-tg-bot
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

# --- 步骤 5: 创建主程序 ---
print_message "步骤 5: 创建 Telegram Bot 主程序"

cat > "$BOT_SCRIPT" <<'EOF'
#!/usr/bin/env python3
# -*- coding: utf-8 -*-
import logging, subprocess, os, time, pytz
from datetime import datetime
from telegram import Update, InlineKeyboardButton, InlineKeyboardMarkup, ParseMode
from telegram.ext import Updater, CommandHandler, CallbackQueryHandler, CallbackContext
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
SYSTEM_TZ = pytz.timezone(subprocess.check_output("timedatectl show -p Timezone --value", shell=True).decode().strip())
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
        [InlineKeyboardButton("📋 查看日志", callback_data='logs')]
    ]
    update.message.reply_text("🤖 *VPS 管理 Bot*\n\n请选择操作：", reply_markup=InlineKeyboardMarkup(keyboard), parse_mode=ParseMode.MARKDOWN)

def button(update: Update, context: CallbackContext):
    query = update.callback_query; query.answer()
    if str(query.message.chat.id) != ADMIN_CHAT_ID:
        query.edit_message_text("❌ 无权限访问"); return
    if query.data == 'status':
        info = subprocess.getoutput("uptime && date")
        query.edit_message_text(f"📊 *系统状态*\n\n```\n{info}\n```", parse_mode=ParseMode.MARKDOWN)
    elif query.data == 'maintain_core':
        query.edit_message_text("⏳ 正在执行维护，请稍候...")
        subprocess.run([CORE_SCRIPT], check=False)
        result = open("/tmp/vps_maintain_result.txt").read()
        query.edit_message_text(f"✅ *维护完成*\n\n```\n{result}\n```\n\n⚠️ 系统将在 5 秒后重启", parse_mode=ParseMode.MARKDOWN)
        time.sleep(5); subprocess.run(["/sbin/reboot"])
    elif query.data == 'logs':
        logs = subprocess.getoutput("journalctl -u vps-tg-bot -n 20 --no-pager")
        query.edit_message_text(f"📋 *日志*\n\n```\n{logs[-2000:]}\n```", parse_mode=ParseMode.MARKDOWN)

def scheduled_task():
    subprocess.run([RULES_SCRIPT], check=False)
    subprocess.run([CORE_SCRIPT], check=False)
    send_message("🕒 定时维护已执行，系统将在 5 秒后自动重启")
    time.sleep(5); subprocess.run(["/sbin/reboot"])

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
print_message "步骤 6: 配置系统服务"

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

[Install]
WantedBy=multi-user.target
EOF

systemctl daemon-reload
systemctl enable vps-tg-bot
systemctl start vps-tg-bot
sleep 3

if systemctl is-active --quiet vps-tg-bot; 键，然后
  echo "✅ 服务启动成功"
else
  echo "❌ 服务启动失败，请查看日志: journalctl -u vps-tg-bot -n 50"
fi

print_message "🎉 部署完成！"
echo "✅ 每周维护任务已自动设置 (每周日 04:00)"
echo "📱 前往 Telegram 发送 /start 开始使用"
