#!/bin/bash
# ---------------------------------------------------------------------------
# Debian 自动安全更新 (RSS + Telegram + 内存日志)
# V2.0 - 使用稳定源 https://security-tracker.debian.org/tracker/data/rss
# ---------------------------------------------------------------------------

RSS_URL="https://security-tracker.debian.org/tracker/data/rss"
STATE_FILE="/run/rss-security-last-hash.txt"
CONFIG_FILE="/etc/rss-security.conf"
LOG_FILE="/dev/shm/rss-security-auto.log"
REBOOT_TIME="03:00"

# --- 日志输出 ---
log() {
    echo "$(date '+%F %T') $1" | tee -a "$LOG_FILE"
}

# --- Telegram 发送 ---
send_tg() {
    local msg="$1"
    if [[ -n "$TG_TOKEN" && -n "$TG_CHAT_ID" ]]; then
        curl -s -X POST "https://api.telegram.org/bot${TG_TOKEN}/sendMessage" \
            -d chat_id="$TG_CHAT_ID" \
            -d text="$msg" >/dev/null
    fi
}

# --- 首次配置 ---
if [ ! -f "$CONFIG_FILE" ]; then
    echo "📩 首次运行：配置 Telegram 通知"
    read -p "请输入 Telegram Bot Token: " TG_TOKEN
    read -p "请输入 Telegram Chat ID (管理员): " TG_CHAT_ID
    read -p "需要自动重启系统吗？(y/N): " auto_reboot
    [[ "$auto_reboot" =~ ^[Yy]$ ]] && AUTO_REBOOT=true || AUTO_REBOOT=false

    cat > "$CONFIG_FILE" <<EOF
TG_TOKEN="$TG_TOKEN"
TG_CHAT_ID="$TG_CHAT_ID"
AUTO_REBOOT=$AUTO_REBOOT
EOF

    chmod 600 "$CONFIG_FILE"
    log "✅ Telegram 已配置。"
else
    source "$CONFIG_FILE"
fi

# --- 获取 RSS 哈希 ---
RSS_HASH=$(curl -fsSL "$RSS_URL" | sha256sum | awk '{print $1}')
if [[ -z "$RSS_HASH" ]]; then
    log "❌ 无法访问 RSS 源：$RSS_URL"
    send_tg "⚠️ 无法访问 Debian 安全更新源。"
    exit 1
fi

mkdir -p /run /dev/shm

# --- 首次运行 ---
if [ ! -f "$STATE_FILE" ]; then
    echo "$RSS_HASH" > "$STATE_FILE"
    log "📦 首次运行，创建状态文件。"
    send_tg "🤖 Debian 自动安全更新已启用。\n检测源：$RSS_URL"
    exit 0
fi

LAST_HASH=$(cat "$STATE_FILE")

# --- 检测更新 ---
if [ "$RSS_HASH" != "$LAST_HASH" ]; then
    echo "$RSS_HASH" > "$STATE_FILE"
    log "🔄 检测到安全公告更新，开始执行 unattended-upgrade"

    unattended-upgrade -d >> "$LOG_FILE" 2>&1

    if [ -f /var/run/reboot-required ]; then
        log "⚠️ 更新完成，需要重启系统"
        send_tg "🚨 Debian 安全更新完成，需要重启系统。"
        if [ "$AUTO_REBOOT" = true ]; then
            log "⏰ 将在 $REBOOT_TIME 自动重启"
            shutdown -r "$REBOOT_TIME"
        fi
    else
        log "✅ 安全更新完成，无需重启"
        send_tg "✅ Debian 安全更新完成，无需重启。"
    fi
else
    log "✅ 无新安全公告，无需更新。"
fi
