#!/bin/bash
# -----------------------------------------------------------------------------
# VPS Telegram Bot 管理系统 - 一键部署脚本 (使用 uv)
#
# 版本: 5.3 (持久化兼容性修复版)
# 功能: 通过 Telegram Bot 交互式管理 VPS 维护任务
# -----------------------------------------------------------------------------

set -e

# --- 变量定义 ---
BOT_DIR="/opt/vps-tg-bot"
BOT_SCRIPT="$BOT_DIR/vps-tg-bot.py"
BOT_SERVICE="/etc/systemd/system/vps-tg-bot.service"
CORE_MAINTAIN_SCRIPT="/usr/local/bin/vps-maintain-core.sh"
RULES_MAINTAIN_SCRIPT="/usr/local/bin/vps-maintain-rules.sh"

# --- 函数定义 ---
print_message() {
    echo ""
    echo "============================================================"
    echo "$1"
    echo "============================================================"
}

get_timezone() {
    local tz
    if command -v timedatectl &> /dev/null; then
        tz=$(timedatectl | grep "Time zone" | awk '{print $3}')
    fi
    if [ -z "$tz" ] && [ -f /etc/timezone ]; then
        tz=$(cat /etc/timezone)
    fi
    if [ -z "$tz" ]; then
        tz="Etc/UTC"
    fi
    echo "$tz"
}

# --- 步骤 0: 环境检查与准备 ---
print_message "步骤 0: 检查系统环境"

# 检查是否为 root 用户
if [ "$EUID" -ne 0 ]; then
    echo "❌ 请使用 root 用户或 sudo 运行此脚本"
    exit 1
fi

# 安装 uv（如果未安装）
echo "📦 检查 uv 包管理器..."
if ! command -v uv &> /dev/null; then
    echo "正在安装 uv..."
    curl -LsSf https://astral.sh/uv/install.sh | sh
    
    # 立即加载 uv 到当前 shell
    if [ -f "$HOME/.local/bin/uv" ]; then
        export PATH="$HOME/.local/bin:$PATH"
        UV_BIN="$HOME/.local/bin/uv"
    elif [ -f "$HOME/.cargo/bin/uv" ]; then
        export PATH="$HOME/.cargo/bin:$PATH"
        UV_BIN="$HOME/.cargo/bin/uv"
    else
        echo "❌ uv 安装失败，未找到可执行文件"
        exit 1
    fi
    
    # 添加到系统 PATH（持久化）
    if ! grep -q '.local/bin' /root/.bashrc 2>/dev/null; then
        echo 'export PATH="$HOME/.local/bin:$PATH"' >> /root/.bashrc
    fi
    
    echo "✅ uv 安装完成: $UV_BIN"
else
    UV_BIN=$(command -v uv)
    echo "✅ uv 已安装: $UV_BIN"
fi

# 清理旧版本
print_message "清理旧版本文件"
systemctl stop vps-tg-bot 2>/dev/null || true
systemctl disable vps-tg-bot 2>/dev/null || true
rm -rf "$BOT_DIR"
rm -f "$BOT_SERVICE"
rm -f "$CORE_MAINTAIN_SCRIPT" "$RULES_MAINTAIN_SCRIPT"
rm -f "/usr/local/bin/vps-maintain.sh"
rm -f "/usr/local/bin/vps-reboot-notify.sh"
(crontab -l 2>/dev/null | grep -v "vps-maintain" || true) | crontab -

echo "✅ 环境准备完成"

# --- 步骤 1: 用户输入 ---
print_message "步骤 1: 配置 Telegram Bot"
read -p "请输入你的 Telegram Bot Token: " TG_TOKEN
read -p "请输入你的 Telegram Chat ID (管理员): " TG_CHAT_ID

if [ -z "$TG_TOKEN" ] || [ -z "$TG_CHAT_ID" ]; then
    echo "❌ 错误：Telegram Bot Token 和 Chat ID 不能为空"
    exit 1
fi

# --- 步骤 2: 配置系统日志内存化 ---
print_message "步骤 2: 配置系统日志内存存储"

mkdir -p /etc/systemd/journald.conf.d

cat > /etc/systemd/journald.conf.d/memory.conf <<'EOF'
[Journal]
Storage=volatile
RuntimeMaxUse=10M
Compress=yes
EOF

systemctl restart systemd-journald 2>/dev/null || true

if command -v rsyslogd &> /dev/null; then
    cat > /etc/rsyslog.d/memory.conf <<'EOF'
$SystemLogRateLimitInterval 0
$SystemLogRateLimitBurst 0
*.* :ommem:;RSYSLOG_MemoryBuffer
EOF
    systemctl restart rsyslog 2>/dev/null || service rsyslog restart 2>/dev/null || true
fi

echo "✅ 系统日志配置完成"

# --- 步骤 3: 创建维护脚本 ---
print_message "步骤 3: 创建维护脚本"

# 3.1 核心更新脚本
cat > "$CORE_MAINTAIN_SCRIPT" <<'CORE_EOF'
#!/bin/bash
set -e
# ... (维护脚本内容与之前相同，此处省略以节约篇幅)
get_timezone() {
    local tz
    if command -v timedatectl &> /dev/null; then 
        tz=$(timedatectl | grep "Time zone" | awk '{print $3}')
    fi
    if [ -z "$tz" ] && [ -f /etc/timezone ]; then 
        tz=$(cat /etc/timezone)
    fi
    if [ -z "$tz" ]; then 
        tz="Etc/UTC"
    fi
    echo "$tz"
}
TIMEZONE=$(get_timezone)
TIME_NOW=$(date '+%Y-%m-%d %H:%M:%S')
RESULT_FILE="/tmp/vps_maintain_result.txt"
export DEBIAN_FRONTEND=noninteractive
echo "开始系统更新..." > "$RESULT_FILE"
if sudo -n apt-get update && sudo apt-get upgrade -y && sudo apt-get autoremove -y && sudo apt-get clean; then
    echo "✅ 系统更新成功" >> "$RESULT_FILE"
else
    echo "❌ 系统更新失败" >> "$RESULT_FILE"
fi
if command -v xray &> /dev/null; then
    if xray up 2>&1; then
        echo "✅ Xray 核心更新成功" >> "$RESULT_FILE"
    else
        echo "❌ Xray 核心更新失败" >> "$RESULT_FILE"
    fi
else
    echo "ℹ️ Xray 未安装" >> "$RESULT_FILE"
fi
if command -v sb &> /dev/null; then
    if sb up 2>&1; then
        echo "✅ Sing-box 更新成功" >> "$RESULT_FILE"
    else
        echo "❌ Sing-box 更新失败" >> "$RESULT_FILE"
    fi
else
    echo "ℹ️ Sing-box 未安装" >> "$RESULT_FILE"
fi
echo "时区: $TIMEZONE" >> "$RESULT_FILE"
echo "时间: $TIME_NOW" >> "$RESULT_FILE"
CORE_EOF
chmod +x "$CORE_MAINTAIN_SCRIPT"

# 3.2 规则更新脚本
cat > "$RULES_MAINTAIN_SCRIPT" <<'RULES_EOF'
#!/bin/bash
set -e
# ... (维护脚本内容与之前相同，此处省略以节约篇幅)
get_timezone() {
    local tz
    if command -v timedatectl &> /dev/null; then 
        tz=$(timedatectl | grep "Time zone" | awk '{print $3}')
    fi
    if [ -z "$tz" ] && [ -f /etc/timezone ]; then 
        tz=$(cat /etc/timezone)
    fi
    if [ -z "$tz" ]; then 
        tz="Etc/UTC"
    fi
    echo "$tz"
}
RESULT_FILE="/tmp/vps_rules_result.txt"
TIMEZONE=$(get_timezone)
TIME_NOW=$(date '+%Y-%m-%d %H:%M:%S')
if ! command -v xray &> /dev/null; then
    echo "ℹ️ Xray 未安装" > "$RESULT_FILE"
    exit 0
fi
if xray up dat 2>&1; then
    echo "✅ Xray 规则文件更新成功" > "$RESULT_FILE"
else
    echo "❌ Xray 规则文件更新失败" > "$RESULT_FILE"
fi
echo "时区: $TIMEZONE" >> "$RESULT_FILE"
echo "时间: $TIME_NOW" >> "$RESULT_FILE"
RULES_EOF
chmod +x "$RULES_MAINTAIN_SCRIPT"

echo "✅ 维护脚本创建完成"

# --- 步骤 4: 使用 uv 创建项目 ---
print_message "步骤 4: 使用 uv 创建 Python 项目"

mkdir -p "$BOT_DIR"
cd "$BOT_DIR"

# 初始化 uv 项目
echo "📦 初始化 uv 项目..."
"$UV_BIN" init --no-readme --name vps-tg-bot

# 添加依赖（固定兼容版本）
echo "📦 添加所有 Python 依赖并锁定兼容版本..."
"$UV_BIN" add \
    "python-telegram-bot==13.15" \
    "urllib3<2.0" \
    "APScheduler==3.10.4" \
    "tzlocal<3.0" \
    "requests" \
    "pytz" \
    "SQLAlchemy<2.0"

echo "✅ Python 环境配置完成"

# --- 步骤 5: 创建 Telegram Bot 主程序 ---
print_message "步骤 5: 创建 Telegram Bot 主程序"

# Python 脚本内容与之前版本相同，此处省略以节约篇幅
# 它已经包含了持久化的逻辑
cat > "$BOT_SCRIPT" <<'BOTPY_EOF'
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
def get_system_timezone_name():
    try:
        tz_name = subprocess.check_output("timedatectl show -p Timezone --value 2>/dev/null || cat /etc/timezone 2>/dev/null || echo UTC", shell=True).decode().strip()
        return tz_name if tz_name else 'UTC'
    except: return 'UTC'
jobstores = {'default': SQLAlchemyJobStore(url='sqlite:///jobs.sqlite')}
SYSTEM_TZ_NAME = get_system_timezone_name()
SYSTEM_TZ = pytz.timezone(SYSTEM_TZ_NAME)
scheduler = BackgroundScheduler(jobstores=jobstores, timezone=SYSTEM_TZ)
logger.info(f"系统时区: {SYSTEM_TZ_NAME}")
def get_local_timezone(): return SYSTEM_TZ
def get_system_info():
    current_time = datetime.now(SYSTEM_TZ).strftime('%Y-%m-%d %H:%M:%S')
    xray_installed = os.path.exists('/usr/local/bin/xray')
    sb_installed = os.path.exists('/usr/local/bin/sb')
    return {'timezone': SYSTEM_TZ_NAME, 'time': current_time, 'xray': xray_installed, 'singbox': sb_installed}
def is_admin(update: Update) -> bool: return str(update.effective_chat.id) == ADMIN_CHAT_ID
def start(update: Update, context: CallbackContext):
    if not is_admin(update):
        update.message.reply_text("❌ 无权限访问此 Bot"); return
    keyboard = [[InlineKeyboardButton("📊 系统状态", callback_data='status')], [InlineKeyboardButton("🔧 立即维护", callback_data='maintain_now')], [InlineKeyboardButton("⚙️ 定时设置", callback_data='schedule_menu')], [InlineKeyboardButton("📋 查看日志", callback_data='view_logs')], [InlineKeyboardButton("🔄 重启 VPS", callback_data='reboot_confirm')]]
    reply_markup = InlineKeyboardMarkup(keyboard)
    update.message.reply_text("🤖 *VPS 管理 Bot*\n\n欢迎使用 VPS 自动化管理系统\n请选择操作：", reply_markup=reply_markup, parse_mode=ParseMode.MARKDOWN)
def button_callback(update: Update, context: CallbackContext):
    query = update.callback_query; query.answer()
    if not is_admin(update):
        query.edit_message_text("❌ 无权限访问"); return
    data = query.data
    if data == 'status': show_status(query, context)
    elif data == 'maintain_now': maintain_menu(query, context)
    elif data == 'maintain_core': run_core_maintain(query, context)
    elif data == 'maintain_rules': run_rules_maintain(query, context)
    elif data == 'maintain_full': run_full_maintain(query, context)
    elif data == 'schedule_menu': schedule_menu(query, context)
    elif data.startswith('schedule_'): handle_schedule(query, context, data)
    elif data == 'view_logs': view_logs(query, context)
    elif data == 'reboot_confirm': reboot_confirm(query, context)
    elif data == 'reboot_now': reboot_vps(query, context)
    elif data == 'back_main': back_to_main(query, context)
def show_status(query, context):
    info = get_system_info()
    jobs = scheduler.get_jobs()
    schedule_info = "未设置定时任务"
    if jobs:
        schedule_lines = []
        for job in jobs:
            if job.id == 'core_maintain': schedule_lines.append("• 核心维护: 每日 04:00 执行")
            elif job.id == 'rules_maintain': schedule_lines.append("• 规则更新: 每周日 07:00 执行")
            else: schedule_lines.append(f"• {job.name}: {job.trigger}")
        schedule_info = "\n".join(schedule_lines)
    status_text = (f"📊 *系统状态*\n\n" f"🕐 时区: `{info['timezone']}`\n" f"⏰ 时间: `{info['time']}`\n\n" f"📦 已安装组件:\n" f"  • Xray: {'✅' if info['xray'] else '❌'}\n" f"  • Sing-box: {'✅' if info['singbox'] else '❌'}\n\n" f"⏲️ 定时任务:\n{schedule_info}")
    keyboard = [[InlineKeyboardButton("🔙 返回", callback_data='back_main')]]
    reply_markup = InlineKeyboardMarkup(keyboard)
    query.edit_message_text(status_text, reply_markup=reply_markup, parse_mode=ParseMode.MARKDOWN)
def maintain_menu(query, context):
    keyboard = [[InlineKeyboardButton("🔧 核心维护（含重启）", callback_data='maintain_core')], [InlineKeyboardButton("📜 规则更新", callback_data='maintain_rules')], [InlineKeyboardButton("🔄 完整维护", callback_data='maintain_full')], [InlineKeyboardButton("🔙 返回", callback_data='back_main')]]
    reply_markup = InlineKeyboardMarkup(keyboard)
    query.edit_message_text("🔧 *维护操作*\n\n请选择维护类型：\n• 核心维护：更新系统和代理核心，完成后重启\n• 规则更新：仅更新 Xray 规则文件\n• 完整维护：执行所有维护操作", reply_markup=reply_markup, parse_mode=ParseMode.MARKDOWN)
def run_core_maintain(query, context):
    query.edit_message_text("⏳ 正在执行核心维护，请稍候...")
    try:
        subprocess.run([CORE_SCRIPT], check=True); time.sleep(2)
        result = "";
        if os.path.exists('/tmp/vps_maintain_result.txt'):
            with open('/tmp/vps_maintain_result.txt', 'r') as f: result = f.read()
        query.edit_message_text(f"🔧 *核心维护完成*\n\n```\n{result}\n```\n\n" f"⚠️ 系统将在 5 秒后重启", parse_mode=ParseMode.MARKDOWN)
        time.sleep(5); subprocess.run(['/sbin/reboot'])
    except Exception as e: query.edit_message_text(f"❌ 维护失败: {str(e)}")
def run_rules_maintain(query, context):
    query.edit_message_text("⏳ 正在更新规则文件，请稍候...")
    try:
        subprocess.run([RULES_SCRIPT], check=True)
        result = "";
        if os.path.exists('/tmp/vps_rules_result.txt'):
            with open('/tmp/vps_rules_result.txt', 'r') as f: result = f.read()
        keyboard = [[InlineKeyboardButton("🔙 返回", callback_data='back_main')]]
        reply_markup = InlineKeyboardMarkup(keyboard)
        query.edit_message_text(f"📜 *规则更新完成*\n\n```\n{result}\n```", reply_markup=reply_markup, parse_mode=ParseMode.MARKDOWN)
    except Exception as e: query.edit_message_text(f"❌ 更新失败: {str(e)}")
def run_full_maintain(query, context):
    query.edit_message_text("⏳ 正在执行完整维护..."); run_rules_maintain(query, context); time.sleep(3); run_core_maintain(query, context)
def schedule_menu(query, context):
    jobs = scheduler.get_jobs(); core_status = "❌ 未设置"; rules_status = "❌ 未设置"
    for job in jobs:
        if job.id == 'core_maintain': core_status = "✅ 每日 04:00"
        elif job.id == 'rules_maintain': rules_status = "✅ 每周日 07:00"
    keyboard = [[InlineKeyboardButton("⏰ 设置核心维护", callback_data='schedule_core')], [InlineKeyboardButton("📅 设置规则更新", callback_data='schedule_rules')], [InlineKeyboardButton("🗑️ 清除所有定时", callback_data='schedule_clear')], [InlineKeyboardButton("🔙 返回", callback_data='back_main')]]
    reply_markup = InlineKeyboardMarkup(keyboard)
    query.edit_message_text(f"⚙️ *定时任务设置*\n\n📍 当前时区: `{SYSTEM_TZ_NAME}`\n\n🔧 核心维护: {core_status}\n📜 规则更新: {rules_status}", reply_markup=reply_markup, parse_mode=ParseMode.MARKDOWN)
def handle_schedule(query, context, data):
    keyboard = [[InlineKeyboardButton("🔙 返回定时设置", callback_data='schedule_menu')]]
    reply_markup = InlineKeyboardMarkup(keyboard)
    if data == 'schedule_core':
        try:
            scheduler.add_job(scheduled_core_maintain, CronTrigger(hour=4, minute=0), id='core_maintain', replace_existing=True, name='核心维护')
            query.edit_message_text(f"✅ *核心维护定时任务已设置*\n\n🌍 时区: `{SYSTEM_TZ_NAME}`\n📅 执行频率: 每日\n⏰ 执行时间: 04:00（服务器本地时间）\n🔄 执行内容:\n  • 系统更新\n  • Xray 核心更新\n  • Sing-box 更新\n  • VPS 重启", reply_markup=reply_markup, parse_mode=ParseMode.MARKDOWN)
            logger.info(f"核心维护定时任务已设置: 每日 04:00 {SYSTEM_TZ_NAME}")
        except Exception as e:
            logger.error(f"设置核心维护定时任务失败: {e}", exc_info=True)
            query.edit_message_text(f"❌ 设置失败\n\n错误信息: `{str(e)}`\n\n请检查系统日志:\n`journalctl -u vps-tg-bot -n 30`", reply_markup=reply_markup, parse_mode=ParseMode.MARKDOWN)
    elif data == 'schedule_rules':
        try:
            scheduler.add_job(scheduled_rules_maintain, CronTrigger(day_of_week='sun', hour=7, minute=0), id='rules_maintain', replace_existing=True, name='规则更新')
            query.edit_message_text(f"✅ *规则更新定时任务已设置*\n\n🌍 时区: `{SYSTEM_TZ_NAME}`\n📅 执行频率: 每周日\n⏰ 执行时间: 07:00（服务器本地时间）\n📜 执行内容:\n  • Xray 规则文件更新\n  • 不会重启系统", reply_markup=reply_markup, parse_mode=ParseMode.MARKDOWN)
            logger.info(f"规则更新定时任务已设置: 每周日 07:00 {SYSTEM_TZ_NAME}")
        except Exception as e:
            logger.error(f"设置规则更新定时任务失败: {e}", exc_info=True)
            query.edit_message_text(f"❌ 设置失败\n\n错误信息: `{str(e)}`\n\n请检查系统日志:\n`journalctl -u vps-tg-bot -n 30`", reply_markup=reply_markup, parse_mode=ParseMode.MARKDOWN)
    elif data == 'schedule_clear':
        try:
            job_count = len(scheduler.get_jobs()); scheduler.remove_all_jobs()
            query.edit_message_text(f"✅ *已清除所有定时任务*\n\n共清除 {job_count} 个任务", reply_markup=reply_markup, parse_mode=ParseMode.MARKDOWN)
            logger.info(f"已清除 {job_count} 个定时任务")
        except Exception as e:
            logger.error(f"清除定时任务失败: {e}"); query.edit_message_text(f"❌ 清除失败: {str(e)}", reply_markup=reply_markup)
def scheduled_core_maintain():
    logger.info("开始执行定时核心维护")
    try:
        subprocess.run([CORE_SCRIPT], check=True, timeout=300); time.sleep(2)
        result = "";
        if os.path.exists('/tmp/vps_maintain_result.txt'):
            with open('/tmp/vps_maintain_result.txt', 'r') as f: result = f.read()
        send_message(f"🔧 *定时核心维护完成*\n\n```\n{result}\n```\n\n" f"⚠️ 系统将在 5 秒后重启")
        time.sleep(5); subprocess.run(['/sbin/reboot'])
    except subprocess.TimeoutExpired:
        send_message("❌ 定时维护超时（超过5分钟）"); logger.error("定时核心维护超时")
    except Exception as e:
        send_message(f"❌ 定时维护失败: {str(e)}"); logger.error(f"定时核心维护失败: {e}")
def scheduled_rules_maintain():
    logger.info("开始执行定时规则更新")
    try:
        subprocess.run([RULES_SCRIPT], check=True, timeout=120)
        result = "";
        if os.path.exists('/tmp/vps_rules_result.txt'):
            with open('/tmp/vps_rules_result.txt', 'r') as f: result = f.read()
        send_message(f"📜 *定时规则更新完成*\n\n```\n{result}\n```")
    except subprocess.TimeoutExpired:
        send_message("❌ 定时规则更新超时（超过2分钟）"); logger.error("定时规则更新超时")
    except Exception as e:
        send_message(f"❌ 定时更新失败: {str(e)}"); logger.error(f"定时规则更新失败: {e}")
def view_logs(query, context):
    try:
        logs = subprocess.check_output("journalctl -u vps-tg-bot -n 50 --no-pager", shell=True).decode()
        query.edit_message_text(f"📋 *系统日志（最近50条）*\n\n```\n{logs[-3000:]}\n```", parse_mode=ParseMode.MARKDOWN)
    except Exception as e: query.edit_message_text(f"❌ 获取日志失败: {str(e)}")
def reboot_confirm(query, context):
    keyboard = [[InlineKeyboardButton("✅ 确认重启", callback_data='reboot_now')], [InlineKeyboardButton("❌ 取消", callback_data='back_main')]]
    reply_markup = InlineKeyboardMarkup(keyboard)
    query.edit_message_text("⚠️ *确认重启 VPS？*\n\n此操作将立即重启服务器", reply_markup=reply_markup, parse_mode=ParseMode.MARKDOWN)
def reboot_vps(query, context):
    query.edit_message_text("🔄 正在重启 VPS..."); time.sleep(2); subprocess.run(['/sbin/reboot'])
def back_to_main(query, context):
    keyboard = [[InlineKeyboardButton("📊 系统状态", callback_data='status')], [InlineKeyboardButton("🔧 立即维护", callback_data='maintain_now')], [InlineKeyboardButton("⚙️ 定时设置", callback_data='schedule_menu')], [InlineKeyboardButton("📋 查看日志", callback_data='view_logs')], [InlineKeyboardButton("🔄 重启 VPS", callback_data='reboot_confirm')]]
    reply_markup = InlineKeyboardMarkup(keyboard)
    query.edit_message_text("🤖 *VPS 管理 Bot*\n\n请选择操作：", reply_markup=reply_markup, parse_mode=ParseMode.MARKDOWN)
def send_message(text):
    try:
        updater = Updater(TOKEN, use_context=True)
        updater.bot.send_message(chat_id=ADMIN_CHAT_ID, text=text, parse_mode=ParseMode.MARKDOWN)
    except Exception as e: logger.error(f"发送消息失败: {e}")
def main():
    updater = Updater(TOKEN, use_context=True); dp = updater.dispatcher
    dp.add_handler(CommandHandler("start", start)); dp.add_handler(CallbackQueryHandler(button_callback))
    scheduler.start()
    send_message("🤖 *VPS 管理 Bot 已启动*\n\n使用 /start 打开管理面板")
    logger.info("Bot 启动成功"); updater.start_polling(); updater.idle()
if __name__ == '__main__':
    main()
BOTPY_EOF

# 替换配置信息
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
ExecStart=$UV_BIN run $BOT_SCRIPT
Restart=always
RestartSec=10
Environment="PATH=$HOME/.local/bin:$HOME/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"

[Install]
WantedBy=multi-user.target
EOF

# 重载 systemd 并启动服务
systemctl daemon-reload
systemctl enable vps-tg-bot
systemctl start vps-tg-bot

# 等待服务启动
sleep 3

# 检查服务状态
if systemctl is-active --quiet vps-tg-bot; then
    echo "✅ 系统服务启动成功"
else
    echo "❌ 服务启动失败，请查看日志: journalctl -u vps-tg-bot -n 50"
fi

echo "✅ 系统服务配置完成"

# --- 步骤 7: 验证部署 ---
print_message "步骤 7: 验证部署状态"

echo "🔍 正在检查 Bot 运行状态..."
sleep 2

if systemctl is-active --quiet vps-tg-bot; then
    echo "✅ Bot 服务运行正常"
    
    # 检查日志中是否有启动成功的消息
    if journalctl -u vps-tg-bot -n 20 | grep -q "Bot 启动成功"; then
        echo "✅ Bot 已成功连接到 Telegram"
    else
        echo "⚠️  Bot 正在启动中，请稍后使用以下命令查看日志："
        echo "   journalctl -u vps-tg-bot -f"
    fi
else
    echo "❌ Bot 服务未正常运行"
    echo ""
    echo "📋 最近的错误日志："
    journalctl -u vps-tg-bot -n 30 --no-pager
fi

# --- 步骤 8: 完成部署 ---
print_message "🎉 部署完成！"

echo ""
echo "✅ VPS Telegram Bot 管理系统已成功部署"
echo "   (已集成定时任务持久化功能并修复兼容性问题)"
echo ""
echo "📱 请前往您的 Telegram Bot，发送 /start 命令开始使用吧！"
echo ""
