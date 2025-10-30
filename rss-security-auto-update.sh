#!/bin/bash
# ----------------------------------------------------------
# Debian RSS 安全更新自动触发 (内存日志 + Telegram 交互配置)
# ----------------------------------------------------------

RSS_URL="https://www.debian.org/security/dsa-long.en.rdf"
STATE_FILE="/run/rss-security-last-hash.txt"        # 内存中保存上次 RSS 哈希
CONFIG_FILE="/etc/rss-security.conf"                # Telegram 配置保存处
LOG_FILE="/dev/shm/rss-security-auto-update.log"    # 日志写内存中 (tmpfs)

# --- 函数定义 ---
print_message() {
    echo ""
    echo "------------------------------------------------------------"
    echo "$1"
    echo "------------------------------------------------------------"
}

send_telegram() {
    local msg="$1"
    if [[ -n "$TG_TOKEN" && -n "$TG_CHAT_ID" ]]; then
        curl -s -X POST "https://api.telegram.org/bot${TG_TOKEN}/sendMessage" \
            -d chat_id="${TG_CHAT_ID}" \
            -d text="${msg}" >/dev/null
    fi
}

# --- 配置交互 ---
if [ ! -f "$CONFIG_FILE" ]; then
    print_message "首次运行配置 Telegram 通知"
    read -p "请输入你的 Telegram Bot Token: " TG_TOKEN
    read -p "请输入你的 Telegram Chat ID (管理员): " TG_CHAT_ID

    if [ -z "$TG_TOKEN" ] || [ -z "$TG_CHAT_ID" ]; then
        echo "❌ 错误：Telegram Bot Token 和 Chat ID 不能为空"
        exit 1
    fi

    mkdir -p /etc
    cat > "$CONFIG_FILE" <<EOF
TG_TOKEN="$TG_TOKEN"
TG_CHAT_ID="$TG_CHAT_ID"
EOF

    chmod 600 "$CONFIG_FILE"
    echo "✅ Telegram 配置已保存到 $CONFIG_FILE"
else
    source "$CONFIG_FILE"
fi

# --- 内存目录确保存在 ---
mkdir -p /run /dev/shm

# --- RSS 检测 ---
RSS_HASH=$(curl -fsSL "$RSS_URL" | sha256sum | awk '{print $1}')
if [ -f "$STATE_FILE" ]; then
    LAST_HASH=$(cat "$STATE_FILE")
else
    LAST_HASH=""
fi

if [ "$RSS_HASH" != "$LAST_HASH" ]; then
    echo "$RSS_HASH" > "$STATE_FILE"
    echo "$(date '+%F %T') 🔄 检测到安全 RSS 更新，执行 unattended-upgrade" | tee "$LOG_FILE"
    RESULT=$(unattended-upgrade -d 2>&1)
    echo "$RESULT" >> "$LOG_FILE"
    send_telegram "🚨 Debian 安全 RSS 检测到更新，系统已执行 unattended-upgrade ✅
日志摘要：
$(echo "$RESULT" | tail -n 10)"
else
    echo "$(date '+%F %T') ✅ 无新安全更新" > "$LOG_FILE"
fi
