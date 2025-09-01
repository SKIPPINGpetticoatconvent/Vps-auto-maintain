#!/bin/bash
# -----------------------------------------------------------------------------
# 一键部署 VPS 自动维护脚本
#
# 版本: 2.6 (最终版 - 兼容无 systemd 环境并修复所有已知问题)
# -----------------------------------------------------------------------------

set -e

# --- 函数定义 ---
print_message() {
    echo ""
    echo "------------------------------------------------------------"
    echo "$1"
    echo "------------------------------------------------------------"
}

# 【新增】健壮的时区获取函数，兼容无 systemd 的环境
get_timezone() {
    local tz
    # 优先尝试 systemd 的命令
    if command -v timedatectl &> /dev/null; then
        tz=$(timedatectl | grep "Time zone" | awk '{print $3}')
    fi
    # 如果失败，尝试读取传统文件
    if [ -z "$tz" ] && [ -f /etc/timezone ]; then
        tz=$(cat /etc/timezone)
    fi
    # 如果还失败，使用 UTC 作为默认值
    if [ -z "$tz" ]; then
        tz="Etc/UTC"
    fi
    echo "$tz"
}


# --- 步骤 1: 用户输入 ---
print_message "步骤 1: 请输入您的 Telegram 配置信息"
read -p "请输入你的 Telegram Bot Token: " TG_TOKEN
read -p "请输入你的 Telegram Chat ID: " TG_CHAT_ID

if [ -z "$TG_TOKEN" ] || [ -z "$TG_CHAT_ID" ]; then
    echo "❌ 错误：Telegram Bot Token 和 Chat ID 不能为空。"
    exit 1
fi

MAINTAIN_SCRIPT="/usr/local/bin/vps-maintain.sh"
REBOOT_NOTIFY_SCRIPT="/usr/local/bin/vps-reboot-notify.sh"

# --- 步骤 2: 创建重启后通知脚本 ---
print_message "步骤 2: 创建重启后通知脚本 ($REBOOT_NOTIFY_SCRIPT)"
cat > "$REBOOT_NOTIFY_SCRIPT" <<'EOF'
#!/bin/bash
sleep 20

# 【新增】嵌入健壮的时区获取函数
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

TG_TOKEN="__TG_TOKEN__"
TG_CHAT_ID="__TG_CHAT_ID__"

TIMEZONE=$(get_timezone)
TIME_NOW=$(date '+%Y-%m-%d %H:%M:%S')

curl --connect-timeout 10 --retry 5 -s -X POST "https://api.telegram.org/bot$TG_TOKEN/sendMessage" \
    -d chat_id="$TG_CHAT_ID" \
    -d text="🔄 *VPS 已重启完成*
> *系统时区*: \`$TIMEZONE\`
> *当前时间*: \`$TIME_NOW\`" \
    -d parse_mode="Markdown" > /dev/null

(crontab -l | grep -v "__REBOOT_NOTIFY_SCRIPT_PATH__" || true) | crontab -
EOF

sed -i "s|__TG_TOKEN__|$TG_TOKEN|g" "$REBOOT_NOTIFY_SCRIPT"
sed -i "s|__TG_CHAT_ID__|$TG_CHAT_ID|g" "$REBOOT_NOTIFY_SCRIPT"
sed -i "s|__REBOOT_NOTIFY_SCRIPT_PATH__|$REBOOT_NOTIFY_SCRIPT|g" "$REBOOT_NOTIFY_SCRIPT"
chmod +x "$REBOOT_NOTIFY_SCRIPT"
echo "✅ 重启后通知脚本创建成功。"

# --- 步骤 3: 创建核心维护脚本 ---
print_message "步骤 3: 创建核心维护脚本 ($MAINTAIN_SCRIPT)"
cat > "$MAINTAIN_SCRIPT" <<'EOF'
#!/bin/bash
set -e

# 【新增】嵌入健壮的时区获取函数
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

TG_TOKEN="__TG_TOKEN__"
TG_CHAT_ID="__TG_CHAT_ID__"
REBOOT_NOTIFY_SCRIPT="__REBOOT_NOTIFY_SCRIPT_PATH__"

send_telegram() {
    local message="$1"
    sleep 5
    curl --connect-timeout 10 --retry 3 -s -X POST "https://api.telegram.org/bot$TG_TOKEN/sendMessage" \
        -d chat_id="$TG_CHAT_ID" \
        -d text="$message" \
        -d parse_mode="Markdown" > /dev/null
}

TIMEZONE=$(get_timezone)
TIME_NOW=$(date '+%Y-%m-%d %H:%M:%S')

export DEBIAN_FRONTEND=noninteractive
apt-get update -y && apt-get upgrade -y && apt-get autoremove -y && apt-get clean

XRAY_STATUS="*Xray*: 未安装"
if command -v xray &> /dev/null; then
    XRAY_OUTPUT=$(xray up 2>&1)
    XRAY_STATUS=$(echo "$XRAY_OUTPUT" | grep -q "当前已经是最新版本" && echo "*Xray*: ✅ 最新版本" || echo "*Xray*: ⚠️ 已更新")
fi

SB_STATUS="*Sing-box*: 未安装"
if command -v sb &> /dev/null; then
    SB_OUTPUT=$(sb up 2>&1)
    SB_STATUS=$(echo "$SB_OUTPUT" | grep -q "当前已经是最新版本" && echo "*Sing-box*: ✅ 最新版本" || echo "*Sing-box*: ⚠️ 已更新")
fi

send_telegram "🛠 *VPS 维护完成 (即将重启)*
> *系统时区*: \`$TIMEZONE\`
> *当前时间*: \`$TIME_NOW\`
>
> $XRAY_STATUS
> $SB_STATUS"

(crontab -l 2>/dev/null | grep -v "$REBOOT_NOTIFY_SCRIPT" || true; echo "@reboot $REBOOT_NOTIFY_SCRIPT") | crontab -

sleep 3
/sbin/reboot
EOF

sed -i "s|__TG_TOKEN__|$TG_TOKEN|g" "$MAINTAIN_SCRIPT"
sed -i "s|__TG_CHAT_ID__|$TG_CHAT_ID|g" "$MAINTAIN_SCRIPT"
sed -i "s|__REBOOT_NOTIFY_SCRIPT_PATH__|$REBOOT_NOTIFY_SCRIPT|g" "$MAINTAIN_SCRIPT"
chmod +x "$MAINTAIN_SCRIPT"
echo "✅ 核心维护脚本创建成功。"

# --- 步骤 4: 设置每日定时任务 ---
print_message "步骤 4: 设置每日执行的定时任务 (Cron)"
SYS_TZ=$(get_timezone)
TOKYO_HOUR=4
LOCAL_HOUR=$(TZ="$SYS_TZ" date -d "TZ=\"Asia/Tokyo\" $TOKYO_HOUR:00" +%H)
LOCAL_MINUTE=$(TZ="$SYS_TZ" date -d "TZ=\"Asia/Tokyo\" $TOKYO_HOUR:00" +%M)
if [ -z "$LOCAL_HOUR" ] || [ -z "$LOCAL_MINUTE" ]; then
    echo "⚠️ 警告：时区计算失败，将设置为服务器本地时间 04:00 执行。"
    LOCAL_HOUR="4"; LOCAL_MINUTE="0"
fi

(crontab -l 2>/dev/null | grep -v "$MAINTAIN_SCRIPT" || true; echo "$LOCAL_MINUTE $LOCAL_HOUR * * * $MAINTAIN_SCRIPT") | crontab -

echo "✅ Cron 设置完成: VPS 将在本地时间 $LOCAL_HOUR:$LOCAL_MINUTE 自动执行维护。"

# --- 步骤 5: 立即执行一次 ---
print_message "步骤 5: 准备首次执行维护与重启"
read -p "    所有设置已完成，按 Enter 键立即执行一次，或按 Ctrl+C 取消..."

"$MAINTAIN_SCRIPT"
