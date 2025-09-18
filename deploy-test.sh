#!/bin/bash
# -----------------------------------------------------------------------------
# 一键部署 VPS 自动维护脚本
#
# 版本: 3.0 (增强版 - 优化 Xray 状态判断，可识别规则文件更新成功状态)
# -----------------------------------------------------------------------------

set -e

# --- 变量定义 ---
MAINTAIN_SCRIPT="/usr/local/bin/vps-maintain.sh"
REBOOT_NOTIFY_SCRIPT="/usr/local/bin/vps-reboot-notify.sh"

# --- 函数定义 ---
print_message() {
    echo ""
    echo "------------------------------------------------------------"
    echo "$1"
    echo "------------------------------------------------------------"
}

# 健壮的时区获取函数，兼容无 systemd 的环境
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

# --- 步骤 0: 清理旧版本 (如果存在) ---
print_message "步骤 0: 清理旧的脚本和定时任务（如果存在）"
# 使用 rm -f 避免在文件不存在时报错
rm -f "$MAINTAIN_SCRIPT"
rm -f "$REBOOT_NOTIFY_SCRIPT"
# 从 crontab 中移除相关的定时任务，同时保留其他任务
(crontab -l 2>/dev/null | grep -v "$MAINTAIN_SCRIPT" | grep -v "$REBOOT_NOTIFY_SCRIPT" || true) | crontab -
echo "✅ 旧版本清理完成。"


# --- 步骤 1: 用户输入 ---
print_message "步骤 1: 请输入您的 Telegram 配置信息"
read -p "请输入你的 Telegram Bot Token: " TG_TOKEN
read -p "请输入你的 Telegram Chat ID: " TG_CHAT_ID

if [ -z "$TG_TOKEN" ] || [ -z "$TG_CHAT_ID" ]; then
    echo "❌ 错误：Telegram Bot Token 和 Chat ID 不能为空。"
    exit 1
fi

# --- 步骤 2: 创建重启后通知脚本 ---
print_message "步骤 2: 创建重启后通知脚本 ($REBOOT_NOTIFY_SCRIPT)"
cat > "$REBOOT_NOTIFY_SCRIPT" <<'EOF'
#!/bin/bash
sleep 20

# 嵌入健壮的时区获取函数
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

# 执行后自动从 crontab 中删除 @reboot 任务
(crontab -l | grep -v "__REBOOT_NOTIFY_SCRIPT_PATH__" || true) | crontab -
EOF

sed -i "s|__TG_TOKEN__|$TG_TOKEN|g" "$REBOOT_NOTIFY_SCRIPT"
sed -i "s|__TG_CHAT_ID__|$TG_CHAT_ID|g" "$REBOOT_NOTIFY_SCRIPT"
sed -i "s|__REBOOT_NOTIFY_SCRIPT_PATH__|$REBOOT_NOTIFY_SCRIPT|g" "$REBOOT_NOTIFY_SCRIPT"
chmod +x "$REBOOT_NOTIFY_SCRIPT"
echo "✅ 重启后通知脚本创建成功。"

# --- 步骤 2.5: 配置日志存储到内存 ---
print_message "步骤 2.5: 配置日志存储到内存"

if systemctl is-active --quiet systemd-journald; then
    echo "检测到 systemd-journald 正在运行，正在配置日志存储到内存..."
    # 备份配置文件
    if [ -f /etc/systemd/journald.conf ]; then
        sudo cp /etc/systemd/journald.conf /etc/systemd/journald.conf.backup
    fi
    # 修改或添加 Storage=volatile
    if grep -q '^#\?Storage=' /etc/systemd/journald.conf 2>/dev/null; then
        sudo sed -i 's/^#\?Storage=.*/Storage=volatile/' /etc/systemd/journald.conf
    else
        echo "Storage=volatile" | sudo tee -a /etc/systemd/journald.conf
    fi
    # 重启服务
    sudo systemctl restart systemd-journald
    echo "✅ 日志配置已更新，存储到内存。"
    # 验证配置
    if grep -q '^Storage=volatile' /etc/systemd/journald.conf; then
        echo "✅ 验证成功：日志存储已配置为内存。"
    else
        echo "❌ 验证失败：日志存储配置未正确应用。"
    fi
else
    echo "systemd-journald 未运行，跳过日志配置。"
fi

# --- 步骤 3: 创建核心维护脚本 ---
print_message "步骤 3: 创建核心维护脚本 ($MAINTAIN_SCRIPT)"
cat > "$MAINTAIN_SCRIPT" <<'EOF'
#!/bin/bash
set -e

# 嵌入健壮的时区获取函数
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
sudo apt-get update && sudo apt-get upgrade -y && sudo apt-get autoremove -y && sudo apt-get clean

# ----------------- [修改区域开始] -----------------
XRAY_STATUS="*Xray*: 未安装"
if command -v xray &> /dev/null; then
    # 分别更新核心和规则文件，并捕获它们的输出
    XRAY_CORE_OUTPUT=$(xray up 2>&1 || true)
    XRAY_DAT_OUTPUT=$(xray up dat 2>&1 || true)

    # 判断 Xray 核心的更新状态
    if echo "$XRAY_CORE_OUTPUT" | grep -q "当前已经是最新版本"; then
        XRAY_CORE_STATUS="✅ 核心最新"
    else
        # 任何其他输出都意味着执行了更新或出现错误，统一标记为“已更新”
        XRAY_CORE_STATUS="⚠️ 核心已更新"
    fi

    # 判断规则文件的更新状态
    if echo "$XRAY_DAT_OUTPUT" | grep -q "已经是最新版本"; then
        XRAY_DAT_STATUS="✅ 规则最新"
    elif echo "$XRAY_DAT_OUTPUT" | grep -q "更新 geoip.dat geosite.dat 成功"; then
        XRAY_DAT_STATUS="⚠️ 规则已更新"
    else
        XRAY_DAT_STATUS="❌ 规则更新失败"
    fi
    
    # 组合成最终的 Telegram 消息
    XRAY_STATUS="*Xray*: $XRAY_CORE_STATUS, $XRAY_DAT_STATUS"
fi
# ----------------- [修改区域结束] -----------------

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

# 添加 @reboot 任务，用于重启后发送通知
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
print_message "步骤 4: 设置每日维护时间"
echo "请选择维护执行时间："
echo "  [1] 默认时间: 每天东京时间凌晨 4 点 (推荐)"
echo "  [2] 自定义时间: 手动输入服务器本地时间的小时和分钟"
read -p "请输入选项 [1-2]，直接回车默认为 1: " TIME_CHOICE

LOCAL_HOUR=""
LOCAL_MINUTE=""

case "$TIME_CHOICE" in
    2)
        echo "--> 您选择了自定义时间。"
        # 循环输入小时，直到格式正确
        while true; do
            read -p "请输入执行的小时 (0-23): " CUSTOM_HOUR
            if [[ "$CUSTOM_HOUR" =~ ^([0-9]|1[0-9]|2[0-3])$ ]]; then
                LOCAL_HOUR=$CUSTOM_HOUR
                break
            else
                echo "❌ 格式错误，请输入 0 到 23 之间的数字。"
            fi
        done
        # 循环输入分钟，直到格式正确
        while true; do
            read -p "请输入执行的分钟 (0-59): " CUSTOM_MINUTE
            if [[ "$CUSTOM_MINUTE" =~ ^([0-9]|[1-5][0-9])$ ]]; then
                LOCAL_MINUTE=$CUSTOM_MINUTE
                break
            else
                echo "❌ 格式错误，请输入 0 到 59 之间的数字。"
            fi
        done
        ;;
    *)
        echo "--> 您选择了默认时间 (东京时间 4:00)。"
        SYS_TZ=$(get_timezone)
        TOKYO_HOUR=4
        # 使用 date 命令进行精确的时区转换
        LOCAL_HOUR=$(TZ="$SYS_TZ" date -d "TZ=\"Asia/Tokyo\" $TOKYO_HOUR:00" +%H)
        LOCAL_MINUTE=$(TZ="$SYS_TZ" date -d "TZ=\"Asia/Tokyo\" $TOKYO_HOUR:00" +%M)

        if [ -z "$LOCAL_HOUR" ] || [ -z "$LOCAL_MINUTE" ]; then
            echo "⚠️ 警告：时区自动计算失败，将使用服务器本地时间 04:00 作为备用方案。"
            LOCAL_HOUR="4"
            LOCAL_MINUTE="0"
        fi
        ;;
esac

# 写入 Crontab
(crontab -l 2>/dev/null | grep -v "$MAINTAIN_SCRIPT" || true; echo "$LOCAL_MINUTE $LOCAL_HOUR * * * $MAINTAIN_SCRIPT") | crontab -
echo "✅ Cron 设置完成: VPS 将在服务器本地时间 $LOCAL_HOUR:$LOCAL_MINUTE 自动执行维护。"

# --- 步骤 5: 立即执行一次 ---
print_message "步骤 5: 准备首次执行维护与重启"
read -p "    所有设置已完成，按 Enter 键立即执行一次，或按 Ctrl+C 取消..."

"$MAINTAIN_SCRIPT"
