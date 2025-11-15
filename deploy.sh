#!/bin/bash
# -----------------------------------------------------------------------------
# 一键部署 VPS 自动维护脚本
# 此脚本只适用于233Boy的脚本
# 版本: 4.4 (稳定版 - 智能检测，按需配置)
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
# --- 步骤 0.5: 配置系统日志内存化 ---
print_message "步骤 0.5: 配置系统日志使用内存存储"

# 配置 systemd journald 使用内存存储
mkdir -p /etc/systemd/journald.conf.d

cat > /etc/systemd/journald.conf.d/memory.conf <<'EOF'
[Journal]
# 使用内存存储日志，避免写入磁盘
Storage=volatile
# 设置内存日志大小限制 (10MB)
RuntimeMaxUse=10M
# 压缩日志
Compress=yes
EOF

# 重启 journald 服务
systemctl restart systemd-journald 2>/dev/null || true

# 配置 rsyslog 使用内存缓冲（如果安装了 rsyslog）
if command -v rsyslogd &> /dev/null; then
    # 创建 rsyslog 内存配置
    cat > /etc/rsyslog.d/memory.conf <<'EOF'
# 使用内存缓冲区存储日志
$SystemLogRateLimitInterval 0
$SystemLogRateLimitBurst 0

# 将所有日志输出到内存缓冲区
*.* :ommem:;RSYSLOG_MemoryBuffer
EOF

    # 重启 rsyslog 服务
    systemctl restart rsyslog 2>/dev/null || service rsyslog restart 2>/dev/null || true
fi

echo "✅ 系统日志已配置为内存存储模式。"
echo "✅ 旧版本清理完成。"


# --- 步骤 1: 用户输入 ---
print_message "步骤 1: 请输入您的 Telegram 配置信息"
read -p "请输入你的 Telegram Bot Token: " TG_TOKEN
read -p "请输入你的 Telegram Chat ID: " TG_CHAT_ID

if [ -z "$TG_TOKEN" ] || [ -z "$TG_CHAT_ID" ]; then
    echo "❌ 错误：Telegram Bot Token 和 Chat ID 不能为空。"
    exit 1
fi

# --- 步骤 1.5: 检测 Xray/Sing-box 安装情况 ---
# ----------------- [新增区域开始] -----------------
print_message "步骤 1.5: 检测 Xray/Sing-box 安装情况"
XRAY_INSTALLED=false
if command -v xray &> /dev/null; then
    XRAY_INSTALLED=true
    echo "✅ 检测到 Xray 已安装。"
else
    echo "ℹ️ 未检测到 Xray。"
fi
if command -v sb &> /dev/null; then
    echo "✅ 检测到 Sing-box 已安装。"
else
    echo "ℹ️ 未检测到 Sing-box。"
fi
# ----------------- [新增区域结束] -----------------


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
# 清理 @reboot 任务（使用更安全的方式）
(crontab -l 2>/dev/null | sed "/$REBOOT_NOTIFY_SCRIPT/d" || true) | crontab -
EOF
sed -i "s|__TG_TOKEN__|$TG_TOKEN|g" "$REBOOT_NOTIFY_SCRIPT"
sed -i "s|__TG_CHAT_ID__|$TG_CHAT_ID|g" "$REBOOT_NOTIFY_SCRIPT"
sed -i "s|__REBOOT_NOTIFY_SCRIPT_PATH__|$REBOOT_NOTIFY_SCRIPT|g" "$REBOOT_NOTIFY_SCRIPT"
chmod +x "$REBOOT_NOTIFY_SCRIPT"
echo "✅ 重启后通知脚本创建成功。"


# --- 步骤 3: 创建两个独立的维护脚本 ---

# 3.1 创建核心更新脚本 (此脚本总是创建，其内部逻辑会自行判断是否更新 xray/sb)
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
# 检查 sudo 是否需要密码
if sudo -n true 2>/dev/null; then
    echo "✅ sudo 无需密码，继续执行..."
    sudo apt update && sudo apt full-upgrade -y && sudo apt autoremove -y && sudo apt autoclean
else
    echo "❌ 警告：sudo 需要密码。系统更新可能失败。请考虑配置无密码 sudo 或手动运行："
    echo "    sudo apt update && sudo apt full-upgrade -y && sudo apt autoremove -y && sudo apt autoclean"
    # 尝试执行，如果失败则发送错误通知
    if ! sudo apt update && sudo apt full-upgrade -y && sudo apt autoremove -y && sudo apt autoclean; then
        send_telegram "❌ *系统更新失败*
> *原因*: sudo 需要密码
> *建议*: 配置无密码 sudo 或手动更新系统"
    fi
fi

XRAY_STATUS="*Xray 核心*: 未安装"
if command -v xray &> /dev/null; then
    if xray up 2>&1; then
        XRAY_STATUS="*Xray 核心*: ✅ 更新成功或已是最新版本"
    else
        XRAY_STATUS="*Xray 核心*: ❌ 更新失败"
    fi
fi

SB_STATUS="*Sing-box*: 未安装"
if command -v sb &> /dev/null; then
    if sb up 2>&1; then
        SB_STATUS="*Sing-box*: ✅ 更新成功或已是最新版本"
    else
        SB_STATUS="*Sing-box*: ❌ 更新失败"
    fi
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

# 3.2 创建规则更新脚本 (仅在 Xray 安装时创建)
# ----------------- [修改区域开始] -----------------
if [ "$XRAY_INSTALLED" = true ]; then
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

if ! xray up dat 2>&1; then
    XRAY_DAT_STATUS="❌ 更新失败"
    send_telegram "❌ *Xray 规则文件更新失败*
> *时间*: \`$TIME_NOW ($TIMEZONE)\`"
    exit 1
else
    XRAY_DAT_STATUS="✅ 更新成功"
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
else
    print_message "步骤 3.2: 跳过规则文件更新脚本"
    echo "未检测到 Xray, 无需创建规则文件更新脚本。"
fi
# ----------------- [修改区域结束] -----------------

# --- 步骤 4: 设置定时任务 (根据是否安装 Xray 进行调整) ---
get_offset_hours() {
    local tz="$1"
    local offset_str
    offset_str=$(TZ="$tz" date +%z 2>/dev/null)
    if [[ ! "$offset_str" =~ ^[+-][0-9]{4}$ ]]; then
        offset_str="+0000"
    fi
    local sign="${offset_str:0:1}"
    local hours=$((10#${offset_str:1:2}))
    if [ "$sign" = "+" ]; then
        echo "$hours"
    else
        echo "-$hours"
    fi
}

print_message "步骤 4: 设置维护时间"

# 初始化时间变量
CORE_H=""
CORE_M=""
RULES_H=""
RULES_M=""

# ----------------- [修改区域开始] -----------------
if [ "$XRAY_INSTALLED" = true ]; then
    # --- 场景一：安装了 Xray，设置两个任务 ---
    echo "检测到 Xray, 将为您设置两个独立的定时任务："
    echo "  - 任务 A (核心维护与重启): 每日 默认在 东京时间 凌晨 4 点"
    echo "  - 任务 B (规则文件更新): 每周 默认在 北京时间 早上 7 点"
    echo ""
    echo "请选择："
    echo "  [1] 使用以上默认时间 (推荐)"
    echo "  [2] 手动为两个任务分别自定义时间"
    read -p "请输入选项 [1-2]，直接回车默认为 1: " TIME_CHOICE

    case "$TIME_CHOICE" in
        2)
            echo "--> 设置任务 A (核心维护与重启) 的时间..."
            while true; do read -p "请输入执行的小时 (0-23): " CORE_H; if [[ "$CORE_H" =~ ^[0-9]+$ ]] && [ "$CORE_H" -ge 0 ] && [ "$CORE_H" -le 23 ]; then break; else echo "❌ 错误：小时必须是 0-23 之间的整数。"; fi; done
            while true; do read -p "请输入执行的分钟 (0-59): " CORE_M; if [[ "$CORE_M" =~ ^[0-9]+$ ]] && [ "$CORE_M" -ge 0 ] && [ "$CORE_M" -le 59 ]; then break; else echo "❌ 错误：分钟必须是 0-59 之间的整数。"; fi; done
            echo "--> 设置任务 B (规则文件更新) 的时间..."
            while true; do read -p "请输入执行的小时 (0-23): " RULES_H; if [[ "$RULES_H" =~ ^[0-9]+$ ]] && [ "$RULES_H" -ge 0 ] && [ "$RULES_H" -le 23 ]; then break; else echo "❌ 错误：小时必须是 0-23 之间的整数。"; fi; done
            while true; do read -p "请输入执行的分钟 (0-59): " RULES_M; if [[ "$RULES_M" =~ ^[0-9]+$ ]] && [ "$RULES_M" -ge 0 ] && [ "$RULES_M" -le 59 ]; then break; else echo "❌ 错误：分钟必须是 0-59 之间的整数。"; fi; done
            ;;
        *)
            echo "--> 正在为您计算默认时间..."
            SYS_TZ=$(get_timezone); LOCAL_OFFSET=$(get_offset_hours "$SYS_TZ")
            TOKYO_OFFSET=$(get_offset_hours "Asia/Tokyo"); OFFSET_DIFF_CORE=$((TOKYO_OFFSET - LOCAL_OFFSET)); CORE_H=$(( (4 - OFFSET_DIFF_CORE % 24 + 24) % 24 )); CORE_M=0
            SHANGHAI_OFFSET=$(get_offset_hours "Asia/Shanghai"); OFFSET_DIFF_RULES=$((SHANGHAI_OFFSET - LOCAL_OFFSET)); RULES_H=$(( (7 - OFFSET_DIFF_RULES % 24 + 24) % 24 )); RULES_M=0
            ;;
    esac

    # 写入两个 Crontab 任务
    (crontab -l 2>/dev/null; \
    echo "$CORE_M $CORE_H * * * $CORE_MAINTAIN_SCRIPT"; \
    echo "$RULES_M $RULES_H * * 0 $RULES_MAINTAIN_SCRIPT"; \
    ) | crontab -

    echo "✅ Cron 设置完成:"
    echo "   - 核心维护与重启: 服务器本地时间 $CORE_H:$CORE_M"
    echo "   - 规则文件更新:   服务器本地时间 $RULES_H:$RULES_M (每周日)"

else
    # --- 场景二：未安装 Xray，只设置一个任务 ---
    echo "未检测到 Xray, 将仅设置核心系统维护定时任务。"
    echo "  - 任务 (核心维护与重启): 每日 默认在 东京时间 凌晨 4 点"
    echo ""
    echo "请选择："
    echo "  [1] 使用以上默认时间 (推荐)"
    echo "  [2] 手动自定义时间"
    read -p "请输入选项 [1-2]，直接回车默认为 1: " TIME_CHOICE

    case "$TIME_CHOICE" in
        2)
            echo "--> 设置核心维护与重启的时间..."
            while true; do read -p "请输入执行的小时 (0-23): " CORE_H; if [[ "$CORE_H" =~ ^[0-9]+$ ]] && [ "$CORE_H" -ge 0 ] && [ "$CORE_H" -le 23 ]; then break; else echo "❌ 错误：小时必须是 0-23 之间的整数。"; fi; done
            while true; do read -p "请输入执行的分钟 (0-59): " CORE_M; if [[ "$CORE_M" =~ ^[0-9]+$ ]] && [ "$CORE_M" -ge 0 ] && [ "$CORE_M" -le 59 ]; then break; else echo "❌ 错误：分钟必须是 0-59 之间的整数。"; fi; done
            ;;
        *)
            echo "--> 正在为您计算默认时间..."
            SYS_TZ=$(get_timezone); LOCAL_OFFSET=$(get_offset_hours "$SYS_TZ")
            TOKYO_OFFSET=$(get_offset_hours "Asia/Tokyo"); OFFSET_DIFF_CORE=$((TOKYO_OFFSET - LOCAL_OFFSET)); CORE_H=$(( (4 - OFFSET_DIFF_CORE % 24 + 24) % 24 )); CORE_M=0
            ;;
    esac

    # 仅写入一个 Crontab 任务
    (crontab -l 2>/dev/null; \
    echo "$CORE_M $CORE_H * * * $CORE_MAINTAIN_SCRIPT"; \
    ) | crontab -

    echo "✅ Cron 设置完成:"
    echo "   - 核心维护与重启: 服务器本地时间 $CORE_H:$CORE_M"
fi
# ----------------- [修改区域结束] -----------------


# --- 步骤 5: 立即执行一次核心维护 ---
print_message "步骤 5: 准备首次执行核心维护与重启"
read -p "    所有设置已完成，按 Enter 键立即执行一次核心维护，或按 Ctrl+C 取消..."

"$CORE_MAINTAIN_SCRIPT"
