#!/bin/bash
# -----------------------------------------------------------------------------
# 一键部署 VPS 自动维护脚本
#
# 版本: 4.2 (稳定版 - 使用 case 语句重构定时任务逻辑，彻底修复语法错误)
# -----------------------------------------------------------------------------

set -e

# --- 变量定义 ---
CORE_MAINTAIN_SCRIPT="/usr/local/bin/vps-maintain-core.sh"
RULES_MAINTAIN_SCRIPT="/usr/local/bin/vps-maintain-rules.sh"
REBOOT_NOTIFY_SCRIPT="/usr/local/bin/vps-reboot-notify.sh"

# --- 函数定义 ---
print_message() {
    echo ""
    echo "------------------------------------------------------------"
    echo "$1"
    echo "------------------------------------------------------------"
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

# --- 步骤 0: 清理旧版本 ---
print_message "步骤 0: 清理旧的脚本和定时任务（如果存在）"
rm -f "$CORE_MAINTAIN_SCRIPT"
rm -f "$RULES_MAINTAIN_SCRIPT"
rm -f "$REBOOT_NOTIFY_SCRIPT"
rm -f "/usr/local/bin/vps-maintain.sh" # 清理旧的单文件脚本
(crontab -l 2>/dev/null | grep -v "vps-maintain" || true) | crontab -
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
print_message "步骤 2: 创建重启后通知脚本"
cat > "$REBOOT_NOTIFY_SCRIPT" <<'EOF'
#!/bin/bash
sleep 20
get_timezone() {
    local tz
    if command -v timedatectl &> /dev/null; then tz=$(timedatectl | grep "Time zone" | awk '{print $3}'); fi
    if [ -z "$tz" ] && [ -f /etc/timezone ]; then tz=$(cat /etc/timezone); fi
    if [ -z "$tz" ]; then tz="Etc/UTC"; fi
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


# --- 步骤 3: 创建两个独立的维护脚本 ---

# 3.1 创建核心更新脚本
print_message "步骤 3.1: 创建核心更新脚本 ($CORE_MAINTAIN_SCRIPT)"
cat > "$CORE_MAINTAIN_SCRIPT" <<'EOF'
#!/bin/bash
set -e
get_timezone() {
    local tz
    if command -v timedatectl &> /dev/null; then tz=$(timedatectl | grep "Time zone" | awk '{print $3}'); fi
    if [ -z "$tz" ] && [ -f /etc/timezone ]; then tz=$(cat /etc/timezone); fi
    if [ -z "$tz" ]; then tz="Etc/UTC"; fi
    echo "$tz"
}
TG_TOKEN="__TG_TOKEN__"
TG_CHAT_ID="__TG_CHAT_ID__"
REBOOT_NOTIFY_SCRIPT="__REBOOT_NOTIFY_SCRIPT_PATH__"
send_telegram() {
    local message="$1"
    sleep 5
    curl --connect-timeout 10 --retry 3 -s -X POST "https://api.telegram.org/bot$TG_TOKEN/sendMessage" \
        -d chat_id="$TG_CHAT_ID" -d text="$message" -d parse_mode="Markdown" > /dev/null
}
TIMEZONE=$(get_timezone)
TIME_NOW=$(date '+%Y-%m-%d %H:%M:%S')

export DEBIAN_FRONTEND=noninteractive
sudo apt-get update && sudo apt-get upgrade -y && sudo apt-get autoremove -y && sudo apt-get clean

XRAY_STATUS="*Xray 核心*: 未安装"
if command -v xray &> /dev/null; then
    XRAY_CORE_OUTPUT=$(xray up 2>&1 || true)
    XRAY_STATUS=$(echo "$XRAY_CORE_OUTPUT" | grep -q "当前已经是最新版本" && echo "*Xray 核心*: ✅ 最新版本" || echo "*Xray 核心*: ⚠️ 已更新")
fi

SB_STATUS="*Sing-box*: 未安装"
if command -v sb &> /dev/null; then
    SB_OUTPUT=$(sb up 2>&1)
    SB_STATUS=$(echo "$SB_OUTPUT" | grep -q "当前已经是最新版本" && echo "*Sing-box*: ✅ 最新版本" || echo "*Sing-box*: ⚠️ 已更新")
fi

send_telegram "🛠️ *VPS 核心维护完成 (即将重启)*
> *系统时区*: \`$TIMEZONE\`
> *当前时间*: \`$TIME_NOW\`
>
> $XRAY_STATUS
> $SB_STATUS"
(crontab -l 2>/dev/null | grep -v "$REBOOT_NOTIFY_SCRIPT" || true; echo "@reboot $REBOOT_NOTIFY_SCRIPT") | crontab -
sleep 3
/sbin/reboot
EOF
sed -i "s|__TG_TOKEN__|$TG_TOKEN|g" "$CORE_MAINTAIN_SCRIPT"
sed -i "s|__TG_CHAT_ID__|$TG_CHAT_ID|g" "$CORE_MAINTAIN_SCRIPT"
sed -i "s|__REBOOT_NOTIFY_SCRIPT_PATH__|$REBOOT_NOTIFY_SCRIPT|g" "$CORE_MAINTAIN_SCRIPT"
chmod +x "$CORE_MAINTAIN_SCRIPT"
echo "✅ 核心更新脚本创建成功。"

# 3.2 创建规则更新脚本
print_message "步骤 3.2: 创建规则文件更新脚本 ($RULES_MAINTAIN_SCRIPT)"
cat > "$RULES_MAINTAIN_SCRIPT" <<'EOF'
#!/bin/bash
set -e
get_timezone() {
    local tz
    if command -v timedatectl &> /dev/null; then tz=$(timedatectl | grep "Time zone" | awk '{print $3}'); fi
    if [ -z "$tz" ] && [ -f /etc/timezone ]; then tz=$(cat /etc/timezone); fi
    if [ -z "$tz" ]; then tz="Etc/UTC"; fi
    echo "$tz"
}
TG_TOKEN="__TG_TOKEN__"
TG_CHAT_ID="__TG_CHAT_ID__"
send_telegram() {
    local message="$1"
    curl --connect-timeout 10 --retry 3 -s -X POST "https://api.telegram.org/bot$TG_TOKEN/sendMessage" \
        -d chat_id="$TG_CHAT_ID" -d text="$message" -d parse_mode="Markdown" > /dev/null
}

if ! command -v xray &> /dev/null; then
    exit 0
fi

XRAY_DAT_OUTPUT=$(xray up dat 2>&1 || true)
if echo "$XRAY_DAT_OUTPUT" | grep -q "已经是最新版本"; then
    exit 0
elif echo "$XRAY_DAT_OUTPUT" | grep -q "更新 geoip.dat geosite.dat 成功"; then
    XRAY_DAT_STATUS="⚠️ 已更新成功"
else
    XRAY_DAT_STATUS="❌ 更新失败"
fi

TIMEZONE=$(get_timezone)
TIME_NOW=$(date '+%Y-%m-%d %H:%M:%S')

send_telegram "📜 *Xray 规则文件维护*
> *状态*: $XRAY_DAT_STATUS
> *时间*: \`$TIME_NOW ($TIMEZONE)\`"
EOF
sed -i "s|__TG_TOKEN__|$TG_TOKEN|g" "$RULES_MAINTAIN_SCRIPT"
sed -i "s|__TG_CHAT_ID__|$TG_CHAT_ID|g" "$RULES_MAINTAIN_SCRIPT"
chmod +x "$RULES_MAINTAIN_SCRIPT"
echo "✅ 规则文件更新脚本创建成功。"

# --- 步骤 4: 设置两个独立的定时任务 ---
# ----------------- [修改区域开始] -----------------
print_message "步骤 4: 设置每日维护时间"
echo "我们将为您设置两个独立的定时任务："
echo "  - 任务 A (核心维护与重启): 默认在 东京时间 凌晨 4 点"
echo "  - 任务 B (规则文件更新):   默认在 北京时间 早上 7 点"
echo ""
echo "请选择："
echo "  [1] 使用以上默认时间 (推荐)"
echo "  [2] 手动为两个任务分别自定义时间"
read -p "请输入选项 [1-2]，直接回车默认为 1: " TIME_CHOICE

CORE_H=""
CORE_M=""
RULES_H=""
RULES_M=""

# 使用 case 语句重构，更加稳健
case "$TIME_CHOICE" in
    2)
        echo "--> 设置任务 A (核心维护与重启) 的时间..."
        read -p "请输入执行的小时 (0-23): " CORE_H
        read -p "请输入执行的分钟 (0-59): " CORE_M
        echo "--> 设置任务 B (规则文件更新) 的时间..."
        read -p "请输入执行的小时 (0-23): " RULES_H
        read -p "请输入执行的分钟 (0-59): " RULES_M
        ;;
    *) # 捕获选项 1 或直接回车
        echo "--> 正在为您计算默认时间..."
        SYS_TZ=$(get_timezone)
        CORE_H=$(TZ="$SYS_TZ" date -d "TZ=\"Asia/Tokyo\" 04:00" +%H)
        CORE_M=$(TZ="$SYS_TZ" date -d "TZ=\"Asia/Tokyo\" 04:00" +%M)
        RULES_H=$(TZ="$SYS_TZ" date -d "TZ=\"Asia/Shanghai\" 07:00" +%H)
        RULES_M=$(TZ="$SYS_TZ" date -d "TZ=\"Asia/Shanghai\" 07:00" +%M)
        ;;
esac
# ----------------- [修改区域结束] -----------------

# 写入 Crontab
(crontab -l 2>/dev/null; \
 echo "$CORE_M $CORE_H * * * $CORE_MAINTAIN_SCRIPT"; \
 echo "$RULES_M $RULES_H * * * $RULES_MAINTAIN_SCRIPT"; \
) | crontab -

echo "✅ Cron 设置完成:"
echo "   - 核心维护与重启: 服务器本地时间 $CORE_H:$CORE_M"
echo "   - 规则文件更新:   服务器本地时间 $RULES_H:$RULES_M"

# --- 步骤 5: 立即执行一次核心维护 ---
print_message "步骤 5: 准备首次执行核心维护与重启"
read -p "    所有设置已完成，按 Enter 键立即执行一次核心维护，或按 Ctrl+C 取消..."

"$CORE_MAINTAIN_SCRIPT"
