#!/bin/bash
# ----------------------------------------------------------
# Debian RSS 安全更新自动触发 (内存日志版)
# ----------------------------------------------------------

RSS_URL="https://www.debian.org/security/dsa-long.en.rdf"
STATE_FILE="/run/rss-security-last-hash.txt"        # 存在内存中
LOG_FILE="/dev/shm/rss-security-auto-update.log"    # 存在内存中

# Telegram 配置（可选）
TG_TOKEN="替换为你的BotToken"
TG_CHAT_ID="替换为你的ChatID"

send_telegram() {
    local msg="$1"
    if [[ -n "$TG_TOKEN" && -n "$TG_CHAT_ID" ]]; then
        curl -s -X POST "https://api.telegram.org/bot${TG_TOKEN}/sendMessage" \
            -d chat_id="${TG_CHAT_ID}" \
            -d text="${msg}" >/dev/null
    fi
}

mkdir -p /run /dev/shm

# 获取 RSS 哈希
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
