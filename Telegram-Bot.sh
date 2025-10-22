#!/bin/bash
# -----------------------------------------------------------------------------
# VPS Telegram Bot 管理系统 - 一键部署脚本
#
# 版本: 5.0 (Bot 交互版)
# 功能: 通过 Telegram Bot 交互式管理 VPS 维护任务
# -----------------------------------------------------------------------------

set -e

# --- 变量定义 ---
BOT_SCRIPT="/usr/local/bin/vps-tg-bot.py"
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

# 检查并安装必要组件
echo "📦 检查必要组件..."
if ! command -v python3 &> /dev/null; then
    echo "正在安装 Python3..."
    apt-get update && apt-get install -y python3 python3-pip
fi

if ! command -v pip3 &> /dev/null; then
    echo "正在安装 pip3..."
    apt-get install -y python3-pip
fi

# 安装 Python 依赖
echo "📦 安装 Python 依赖库..."
# 方案1: 使用系统包管理器安装（推荐）
if apt-cache show python3-telegram-bot &> /dev/null; then
    apt-get install -y python3-telegram-bot python3-apscheduler python3-requests python3-tz
    echo "✅ 使用系统包安装依赖"
else
    # 方案2: 使用 --break-system-packages（如果系统包不可用）
    echo "⚠️  系统包不可用，使用 pip 安装..."
    pip3 install --break-system-packages python-telegram-bot==13.15 APScheduler requests pytz -q
    echo "✅ 使用 pip 安装依赖"
fi

# 清理旧版本
print_message "清理旧版本文件"
rm -f "$BOT_SCRIPT" "$BOT_SERVICE"
rm -f "$CORE_MAINTAIN_SCRIPT" "$RULES_MAINTAIN_SCRIPT"
rm -f "/usr/local/bin/vps-maintain.sh"
rm -f "/usr/local/bin/vps-reboot-notify.sh"
(crontab -l 2>/dev/null | grep -v "vps-maintain" || true) | crontab -
systemctl stop vps-tg-bot 2>/dev/null || true
systemctl disable vps-tg-bot 2>/dev/null || true

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

# 执行系统更新
echo "开始系统更新..." > "$RESULT_FILE"
if sudo -n apt-get update && sudo apt-get upgrade -y && sudo apt-get autoremove -y && sudo apt-get clean; then
    echo "✅ 系统更新成功" >> "$RESULT_FILE"
else
    echo "❌ 系统更新失败" >> "$RESULT_FILE"
fi

# 更新 Xray
if command -v xray &> /dev/null; then
    if xray up 2>&1; then
        echo "✅ Xray 核心更新成功" >> "$RESULT_FILE"
    else
        echo "❌ Xray 核心更新失败" >> "$RESULT_FILE"
    fi
else
    echo "ℹ️ Xray 未安装" >> "$RESULT_FILE"
fi

# 更新 Sing-box
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

# --- 步骤 4: 创建 Telegram Bot 主程序 ---
print_message "步骤 4: 创建 Telegram Bot 主程序"

cat > "$BOT_SCRIPT" <<'BOTPY_EOF'
#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
VPS Telegram Bot 管理系统
通过 Telegram 交互式管理 VPS 维护任务
"""

import logging
import subprocess
import os
import time
from datetime import datetime, timedelta
from telegram import Update, InlineKeyboardButton, InlineKeyboardMarkup, ParseMode
from telegram.ext import Updater, CommandHandler, CallbackQueryHandler, CallbackContext
from apscheduler.schedulers.background import BackgroundScheduler
from apscheduler.triggers.cron import CronTrigger
import pytz

# 配置日志
logging.basicConfig(
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s',
    level=logging.INFO
)
logger = logging.getLogger(__name__)

# 配置信息（将被替换）
TOKEN = '__TG_TOKEN__'
ADMIN_CHAT_ID = '__TG_CHAT_ID__'
CORE_SCRIPT = '/usr/local/bin/vps-maintain-core.sh'
RULES_SCRIPT = '/usr/local/bin/vps-maintain-rules.sh'

# 定时任务调度器
scheduler = BackgroundScheduler(timezone=pytz.UTC)

def get_system_info():
    """获取系统信息"""
    try:
        timezone = subprocess.check_output(
            "timedatectl | grep 'Time zone' | awk '{print $3}'",
            shell=True
        ).decode().strip()
    except:
        timezone = "UTC"
    
    current_time = datetime.now().strftime('%Y-%m-%d %H:%M:%S')
    
    # 检查已安装的工具
    xray_installed = os.path.exists('/usr/local/bin/xray')
    sb_installed = os.path.exists('/usr/local/bin/sb')
    
    return {
        'timezone': timezone,
        'time': current_time,
        'xray': xray_installed,
        'singbox': sb_installed
    }

def is_admin(update: Update) -> bool:
    """验证是否为管理员"""
    return str(update.effective_chat.id) == ADMIN_CHAT_ID

def start(update: Update, context: CallbackContext):
    """处理 /start 命令"""
    if not is_admin(update):
        update.message.reply_text("❌ 无权限访问此 Bot")
        return
    
    keyboard = [
        [InlineKeyboardButton("📊 系统状态", callback_data='status')],
        [InlineKeyboardButton("🔧 立即维护", callback_data='maintain_now')],
        [InlineKeyboardButton("⚙️ 定时设置", callback_data='schedule_menu')],
        [InlineKeyboardButton("📋 查看日志", callback_data='view_logs')],
        [InlineKeyboardButton("🔄 重启 VPS", callback_data='reboot_confirm')]
    ]
    reply_markup = InlineKeyboardMarkup(keyboard)
    
    update.message.reply_text(
        "🤖 *VPS 管理 Bot*\n\n"
        "欢迎使用 VPS 自动化管理系统\n"
        "请选择操作：",
        reply_markup=reply_markup,
        parse_mode=ParseMode.MARKDOWN
    )

def button_callback(update: Update, context: CallbackContext):
    """处理按钮回调"""
    query = update.callback_query
    query.answer()
    
    if not is_admin(update):
        query.edit_message_text("❌ 无权限访问")
        return
    
    data = query.data
    
    if data == 'status':
        show_status(query, context)
    elif data == 'maintain_now':
        maintain_menu(query, context)
    elif data == 'maintain_core':
        run_core_maintain(query, context)
    elif data == 'maintain_rules':
        run_rules_maintain(query, context)
    elif data == 'maintain_full':
        run_full_maintain(query, context)
    elif data == 'schedule_menu':
        schedule_menu(query, context)
    elif data.startswith('schedule_'):
        handle_schedule(query, context, data)
    elif data == 'view_logs':
        view_logs(query, context)
    elif data == 'reboot_confirm':
        reboot_confirm(query, context)
    elif data == 'reboot_now':
        reboot_vps(query, context)
    elif data == 'back_main':
        back_to_main(query, context)

def show_status(query, context):
    """显示系统状态"""
    info = get_system_info()
    
    jobs = scheduler.get_jobs()
    schedule_info = "未设置定时任务"
    if jobs:
        schedule_info = "\n".join([
            f"• {job.name}: {job.trigger}" for job in jobs
        ])
    
    status_text = (
        f"📊 *系统状态*\n\n"
        f"🕐 时区: `{info['timezone']}`\n"
        f"⏰ 时间: `{info['time']}`\n\n"
        f"📦 已安装组件:\n"
        f"  • Xray: {'✅' if info['xray'] else '❌'}\n"
        f"  • Sing-box: {'✅' if info['singbox'] else '❌'}\n\n"
        f"⏲️ 定时任务:\n{schedule_info}"
    )
    
    keyboard = [[InlineKeyboardButton("🔙 返回", callback_data='back_main')]]
    reply_markup = InlineKeyboardMarkup(keyboard)
    
    query.edit_message_text(
        status_text,
        reply_markup=reply_markup,
        parse_mode=ParseMode.MARKDOWN
    )

def maintain_menu(query, context):
    """维护菜单"""
    keyboard = [
        [InlineKeyboardButton("🔧 核心维护（含重启）", callback_data='maintain_core')],
        [InlineKeyboardButton("📜 规则更新", callback_data='maintain_rules')],
        [InlineKeyboardButton("🔄 完整维护", callback_data='maintain_full')],
        [InlineKeyboardButton("🔙 返回", callback_data='back_main')]
    ]
    reply_markup = InlineKeyboardMarkup(keyboard)
    
    query.edit_message_text(
        "🔧 *维护操作*\n\n"
        "请选择维护类型：\n"
        "• 核心维护：更新系统和代理核心，完成后重启\n"
        "• 规则更新：仅更新 Xray 规则文件\n"
        "• 完整维护：执行所有维护操作",
        reply_markup=reply_markup,
        parse_mode=ParseMode.MARKDOWN
    )

def run_core_maintain(query, context):
    """执行核心维护"""
    query.edit_message_text("⏳ 正在执行核心维护，请稍候...")
    
    try:
        subprocess.run([CORE_SCRIPT], check=True)
        time.sleep(2)
        
        result = ""
        if os.path.exists('/tmp/vps_maintain_result.txt'):
            with open('/tmp/vps_maintain_result.txt', 'r') as f:
                result = f.read()
        
        query.edit_message_text(
            f"🔧 *核心维护完成*\n\n```\n{result}\n```\n\n"
            f"⚠️ 系统将在 5 秒后重启",
            parse_mode=ParseMode.MARKDOWN
        )
        
        time.sleep(5)
        subprocess.run(['/sbin/reboot'])
        
    except Exception as e:
        query.edit_message_text(f"❌ 维护失败: {str(e)}")

def run_rules_maintain(query, context):
    """执行规则更新"""
    query.edit_message_text("⏳ 正在更新规则文件，请稍候...")
    
    try:
        subprocess.run([RULES_SCRIPT], check=True)
        
        result = ""
        if os.path.exists('/tmp/vps_rules_result.txt'):
            with open('/tmp/vps_rules_result.txt', 'r') as f:
                result = f.read()
        
        keyboard = [[InlineKeyboardButton("🔙 返回", callback_data='back_main')]]
        reply_markup = InlineKeyboardMarkup(keyboard)
        
        query.edit_message_text(
            f"📜 *规则更新完成*\n\n```\n{result}\n```",
            reply_markup=reply_markup,
            parse_mode=ParseMode.MARKDOWN
        )
        
    except Exception as e:
        query.edit_message_text(f"❌ 更新失败: {str(e)}")

def run_full_maintain(query, context):
    """执行完整维护"""
    query.edit_message_text("⏳ 正在执行完整维护...")
    run_rules_maintain(query, context)
    time.sleep(3)
    run_core_maintain(query, context)

def schedule_menu(query, context):
    """定时设置菜单"""
    keyboard = [
        [InlineKeyboardButton("⏰ 设置核心维护", callback_data='schedule_core')],
        [InlineKeyboardButton("📅 设置规则更新", callback_data='schedule_rules')],
        [InlineKeyboardButton("🗑️ 清除所有定时", callback_data='schedule_clear')],
        [InlineKeyboardButton("🔙 返回", callback_data='back_main')]
    ]
    reply_markup = InlineKeyboardMarkup(keyboard)
    
    query.edit_message_text(
        "⚙️ *定时任务设置*\n\n"
        "当前支持的定时任务：\n"
        "• 核心维护：每日执行（含重启）\n"
        "• 规则更新：每周执行",
        reply_markup=reply_markup,
        parse_mode=ParseMode.MARKDOWN
    )

def handle_schedule(query, context, data):
    """处理定时设置"""
    if data == 'schedule_core':
        # 默认每日凌晨 4 点（本地时间）
        scheduler.add_job(
            scheduled_core_maintain,
            CronTrigger(hour=4, minute=0),
            id='core_maintain',
            replace_existing=True,
            name='核心维护'
        )
        query.edit_message_text(
            "✅ 已设置核心维护定时任务\n"
            "执行时间：每日 04:00（本地时间）"
        )
    
    elif data == 'schedule_rules':
        # 默认每周日早上 7 点
        scheduler.add_job(
            scheduled_rules_maintain,
            CronTrigger(day_of_week='sun', hour=7, minute=0),
            id='rules_maintain',
            replace_existing=True,
            name='规则更新'
        )
        query.edit_message_text(
            "✅ 已设置规则更新定时任务\n"
            "执行时间：每周日 07:00（本地时间）"
        )
    
    elif data == 'schedule_clear':
        scheduler.remove_all_jobs()
        query.edit_message_text("✅ 已清除所有定时任务")

def scheduled_core_maintain():
    """定时执行核心维护"""
    try:
        subprocess.run([CORE_SCRIPT], check=True)
        time.sleep(2)
        
        result = ""
        if os.path.exists('/tmp/vps_maintain_result.txt'):
            with open('/tmp/vps_maintain_result.txt', 'r') as f:
                result = f.read()
        
        send_message(f"🔧 *定时核心维护完成*\n\n```\n{result}\n```")
        time.sleep(5)
        subprocess.run(['/sbin/reboot'])
    except Exception as e:
        send_message(f"❌ 定时维护失败: {str(e)}")

def scheduled_rules_maintain():
    """定时执行规则更新"""
    try:
        subprocess.run([RULES_SCRIPT], check=True)
        
        result = ""
        if os.path.exists('/tmp/vps_rules_result.txt'):
            with open('/tmp/vps_rules_result.txt', 'r') as f:
                result = f.read()
        
        send_message(f"📜 *定时规则更新完成*\n\n```\n{result}\n```")
    except Exception as e:
        send_message(f"❌ 定时更新失败: {str(e)}")

def view_logs(query, context):
    """查看系统日志"""
    try:
        logs = subprocess.check_output(
            "journalctl -u vps-tg-bot -n 50 --no-pager",
            shell=True
        ).decode()
        
        query.edit_message_text(
            f"📋 *系统日志（最近50条）*\n\n```\n{logs[-3000:]}\n```",
            parse_mode=ParseMode.MARKDOWN
        )
    except Exception as e:
        query.edit_message_text(f"❌ 获取日志失败: {str(e)}")

def reboot_confirm(query, context):
    """重启确认"""
    keyboard = [
        [InlineKeyboardButton("✅ 确认重启", callback_data='reboot_now')],
        [InlineKeyboardButton("❌ 取消", callback_data='back_main')]
    ]
    reply_markup = InlineKeyboardMarkup(keyboard)
    
    query.edit_message_text(
        "⚠️ *确认重启 VPS？*\n\n"
        "此操作将立即重启服务器",
        reply_markup=reply_markup,
        parse_mode=ParseMode.MARKDOWN
    )

def reboot_vps(query, context):
    """重启 VPS"""
    query.edit_message_text("🔄 正在重启 VPS...")
    time.sleep(2)
    subprocess.run(['/sbin/reboot'])

def back_to_main(query, context):
    """返回主菜单"""
    keyboard = [
        [InlineKeyboardButton("📊 系统状态", callback_data='status')],
        [InlineKeyboardButton("🔧 立即维护", callback_data='maintain_now')],
        [InlineKeyboardButton("⚙️ 定时设置", callback_data='schedule_menu')],
        [InlineKeyboardButton("📋 查看日志", callback_data='view_logs')],
        [InlineKeyboardButton("🔄 重启 VPS", callback_data='reboot_confirm')]
    ]
    reply_markup = InlineKeyboardMarkup(keyboard)
    
    query.edit_message_text(
        "🤖 *VPS 管理 Bot*\n\n请选择操作：",
        reply_markup=reply_markup,
        parse_mode=ParseMode.MARKDOWN
    )

def send_message(text):
    """发送消息给管理员"""
    try:
        updater = Updater(TOKEN, use_context=True)
        updater.bot.send_message(
            chat_id=ADMIN_CHAT_ID,
            text=text,
            parse_mode=ParseMode.MARKDOWN
        )
    except Exception as e:
        logger.error(f"发送消息失败: {e}")

def main():
    """主函数"""
    updater = Updater(TOKEN, use_context=True)
    dp = updater.dispatcher
    
    dp.add_handler(CommandHandler("start", start))
    dp.add_handler(CallbackQueryHandler(button_callback))
    
    # 启动定时任务调度器
    scheduler.start()
    
    # 发送启动通知
    send_message("🤖 *VPS 管理 Bot 已启动*\n\n使用 /start 打开管理面板")
    
    logger.info("Bot 启动成功")
    updater.start_polling()
    updater.idle()

if __name__ == '__main__':
    main()
BOTPY_EOF

# 替换配置信息
sed -i "s|__TG_TOKEN__|$TG_TOKEN|g" "$BOT_SCRIPT"
sed -i "s|__TG_CHAT_ID__|$TG_CHAT_ID|g" "$BOT_SCRIPT"

chmod +x "$BOT_SCRIPT"

echo "✅ Bot 主程序创建完成"

# --- 步骤 5: 创建 systemd 服务 ---
print_message "步骤 5: 配置系统服务"

cat > "$BOT_SERVICE" <<EOF
[Unit]
Description=VPS Telegram Bot Management System
After=network.target

[Service]
Type=simple
User=root
WorkingDirectory=/usr/local/bin
ExecStart=/usr/bin/python3 $BOT_SCRIPT
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
EOF

# 重载 systemd 并启动服务
systemctl daemon-reload
systemctl enable vps-tg-bot
systemctl start vps-tg-bot

echo "✅ 系统服务配置完成"

# --- 步骤 6: 完成部署 ---
print_message "🎉 部署完成！"

echo ""
echo "✅ VPS Telegram Bot 管理系统已成功部署"
echo ""
echo "📱 使用方法："
echo "   1. 在 Telegram 中打开你的 Bot"
echo "   2. 发送 /start 命令"
echo "   3. 通过按钮菜单管理你的 VPS"
echo ""
echo "🔧 管理命令："
echo "   • 查看服务状态: systemctl status vps-tg-bot"
echo "   • 重启服务:     systemctl restart vps-tg-bot"
echo "   • 查看日志:     journalctl -u vps-tg-bot -f"
echo ""
echo "⚙️ Bot 功能："
echo "   • 📊 实时查看系统状态"
echo "   • 🔧 一键执行维护任务"
echo "   • ⏰ 设置定时维护计划"
echo "   • 🔄 远程重启 VPS"
echo "   • 📋 查看运行日志"
echo ""
echo "------------------------------------------------------------"
