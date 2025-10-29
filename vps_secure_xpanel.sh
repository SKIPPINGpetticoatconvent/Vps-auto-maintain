#!/bin/bash
# -----------------------------------------------------------------------------------------
# VPS 代理服务端口检测和防火墙配置脚本（终极一键安全版 V3 - 兼容 xeefei X-Panel）
#
# 新增功能：
# - 自动检测 xeefei/X-Panel (xpanel) 进程与 /etc/x-ui/x-ui.db
# - 自动安装 sqlite3 并提取入站端口
# - 兼容 Xray、Sing-box、X-Panel 的端口识别与防火墙放行
# - 维持 Fail2Ban、Telegram 通知、UFW/firewalld 自动配置
# -----------------------------------------------------------------------------------------

set -e

TG_TOKEN="7982836307:AAEU-ru2xLuuWFhNLqBgHQVaMmKTh4VF5Js"
TG_CHAT_ID="6103295147"
NOTIFY=true

print_message() {
    echo ""
    echo "------------------------------------------------------------"
    echo "$1"
    echo "------------------------------------------------------------"
}

send_telegram() {
    if [ "$NOTIFY" = true ] && [ -n "$TG_TOKEN" ] && [ -n "$TG_CHAT_ID" ]; then
        local message="$1"
        curl --connect-timeout 10 --retry 3 -s -X POST "https://api.telegram.org/bot$TG_TOKEN/sendMessage" \
            -d chat_id="$TG_CHAT_ID" \
            -d text="$message" \
            -d parse_mode="Markdown" > /dev/null
    fi
}

# 自动安装 sqlite3
if ! command -v sqlite3 &>/dev/null; then
    echo "ℹ️ 未检测到 sqlite3，正在安装..."
    if [ -f /etc/debian_version ]; then
        apt-get update -y >/dev/null 2>&1
        apt-get install -y sqlite3 >/dev/null 2>&1
    elif [ -f /etc/redhat-release ]; then
        yum install -y sqlite >/dev/null 2>&1
    fi
    echo "✅ sqlite3 安装完成。"
fi

get_timezone() {
    timedatectl 2>/dev/null | grep "Time zone" | awk '{print $3}' || cat /etc/timezone 2>/dev/null || echo "Etc/UTC"
}

detect_firewall() {
    if systemctl is-active --quiet firewalld 2>/dev/null; then
        echo "firewalld"
    elif command -v ufw &> /dev/null && ufw status 2>/dev/null | grep -q "Status: active"; then
        echo "ufw"
    else
        echo "none"
    fi
}

setup_firewall() {
    print_message "安装并启用防火墙"
    if [ -f /etc/os-release ]; then
        . /etc/os-release
        if [[ "$ID" =~ (debian|ubuntu) || "$ID_LIKE" =~ debian ]]; then
            apt-get install -y ufw >/dev/null 2>&1
            ufw reset -y >/dev/null 2>&1
            ufw default deny incoming >/dev/null 2>&1
            ufw default allow outgoing >/dev/null 2>&1
            ufw --force enable >/dev/null 2>&1
            echo "ufw"
        else
            yum install -y firewalld >/dev/null 2>&1 || dnf install -y firewalld >/dev/null 2>&1
            systemctl enable --now firewalld >/dev/null 2>&1
            echo "firewalld"
        fi
    else
        echo "none"
    fi
}

setup_fail2ban() {
    print_message "配置 Fail2Ban"
    if ! command -v fail2ban-client &> /dev/null; then
        apt-get install -y fail2ban >/dev/null 2>&1 || yum install -y fail2ban >/dev/null 2>&1
    fi
    cat >/etc/fail2ban/jail.local <<EOF
[DEFAULT]
bantime  = 1h
findtime = 10m
maxretry = 5

[sshd]
enabled = true
EOF
    systemctl enable --now fail2ban >/dev/null 2>&1
    echo "✅ Fail2Ban 已启用。"
}

remove_unused_rules() {
    local ports_to_keep="$1"
    local firewall="$2"
    print_message "清理防火墙规则"
    if [ "$firewall" = "ufw" ]; then
        echo "y" | ufw reset >/dev/null 2>&1
        ufw default deny incoming >/dev/null 2>&1
        ufw default allow outgoing >/dev/null 2>&1
        for p in $ports_to_keep; do ufw allow $p >/dev/null; done
        ufw --force enable >/dev/null 2>&1
        ufw status
    else
        for p in $ports_to_keep; do
            firewall-cmd --permanent --add-port=$p/tcp >/dev/null 2>&1
            firewall-cmd --permanent --add-port=$p/udp >/dev/null 2>&1
        done
        firewall-cmd --reload >/dev/null 2>&1
    fi
}

main() {
    setup_fail2ban

    local firewall_type
    firewall_type=$(detect_firewall)
    [ "$firewall_type" = "none" ] && firewall_type=$(setup_firewall)

    local ssh_port
    ssh_port=$(grep -i '^Port ' /etc/ssh/sshd_config | awk '{print $2}' | head -n1)
    [ -z "$ssh_port" ] && ssh_port=22

    echo "🛡️ SSH 端口: $ssh_port"

    local all_ports="$ssh_port"

    # Xray
    if command -v xray &>/dev/null && pgrep -f "xray" &>/dev/null; then
        xray_ports=$(ss -tlnp | grep xray | awk '{print $4}' | awk -F: '{print $NF}' | sort -u)
        [ -n "$xray_ports" ] && all_ports="$all_ports $xray_ports"
    fi

    # Sing-box
    if pgrep -f "sing-box" &>/dev/null; then
        sb_ports=$(ss -tlnp | grep sing-box | awk '{print $4}' | awk -F: '{print $NF}' | sort -u)
        [ -n "$sb_ports" ] && all_ports="$all_ports $sb_ports"
    fi

    # X-Panel 兼容 xeefei
    if pgrep -f "xpanel" >/dev/null || pgrep -f "x-ui" >/dev/null; then
        if [ -f /etc/x-ui/x-ui.db ]; then
            xpanel_ports=$(sqlite3 /etc/x-ui/x-ui.db "SELECT port FROM inbounds;" | grep -E '^[0-9]+$' | sort -u)
            [ -n "$xpanel_ports" ] && echo "✅ 检测到 X-Panel 入站端口: $xpanel_ports"
            all_ports="$all_ports $xpanel_ports"
        fi
    fi

    all_ports=$(echo "$all_ports" | tr ' ' '\n' | sort -u | tr '\n' ' ')
    echo "✅ 允许端口: $all_ports"

    remove_unused_rules "$all_ports" "$firewall_type"

    local msg="🔒 *X-Panel 兼容安全配置完成*
> *服务器*: \`$(hostname)\`
> *防火墙*: \`$firewall_type\`
> *保留端口*: \`$all_ports\`"
    send_telegram "$msg"
}

main
