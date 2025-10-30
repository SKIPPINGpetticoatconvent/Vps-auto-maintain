#!/bin/bash
# =============================================================
# 🧩 VPS 终极安全与自动维护脚本 (V4.0)
# 适配: Debian / Ubuntu / Rocky / AlmaLinux / xeefei X-Panel
# 作者: FTDRTD (优化整合版)
#
# 功能概述:
#   ✅ 自动安装与启用防火墙 (UFW / Firewalld)
#   ✅ 自动配置 Fail2Ban 防护 SSH 暴力破解
#   ✅ 自动检测并放行 Xray / Sing-box / X-Panel 端口
#   ✅ 可选 Telegram 通知系统安全状态
#   ✅ 自动启用无人值守安全更新与每日 03:00 自动重启
#   ✅ 将 systemd 日志改为内存模式 (减少写盘)
# =============================================================

set -e

# === 权限检测 ===
if [ "$(id -u)" -ne 0 ]; then
    echo "❌ 请以 root 权限运行此脚本。"
    exit 1
fi

# === Telegram 通知配置 ===
read -p "是否启用 Telegram 通知？(y/N): " enable_tg
if [[ "$enable_tg" =~ ^[Yy]$ ]]; then
    read -p "请输入 Telegram Bot Token: " TG_TOKEN
    read -p "请输入 Telegram Chat ID: " TG_CHAT_ID
    NOTIFY=true
else
    NOTIFY=false
fi

print_message() {
    echo ""
    echo "------------------------------------------------------------"
    echo "$1"
    echo "------------------------------------------------------------"
}

send_telegram() {
    if [ "$NOTIFY" = true ] && [ -n "$TG_TOKEN" ] && [ -n "$TG_CHAT_ID" ]; then
        local message="$1"
        message=$(echo "$message" | sed 's/`/\\`/g' | sed 's/\*/\\*/g' | sed 's/_/\\_/g')
        curl --connect-timeout 10 --retry 3 -s -X POST \
            "https://api.telegram.org/bot$TG_TOKEN/sendMessage" \
            -d chat_id="$TG_CHAT_ID" -d text="$message" -d parse_mode="MarkdownV2" >/dev/null
    fi
}

# === 依赖检测 ===
if ! command -v sqlite3 &>/dev/null; then
    echo "ℹ️ 安装 sqlite3..."
    apt-get update -y >/dev/null 2>&1
    apt-get install -y sqlite3 >/dev/null 2>&1
fi

# === 防火墙检测与安装 ===
detect_firewall() {
    if systemctl is-active --quiet firewalld 2>/dev/null; then
        echo "firewalld"
    elif command -v ufw &>/dev/null && ufw status 2>/dev/null | grep -q "Status: active"; then
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
            echo "y" | ufw reset >/dev/null 2>&1
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

# === Fail2Ban 配置 ===
detect_banaction() {
    local firewall_type="$1"
    local banaction=""
    if [ "$firewall_type" = "ufw" ]; then
        if [ -f "/etc/fail2ban/action.d/ufw-allports.conf" ]; then
            banaction="ufw-allports"
        elif [ -f "/etc/fail2ban/action.d/ufw.conf" ]; then
            banaction="ufw"
        else
            banaction="iptables-allports"
        fi
    elif [ "$firewall_type" = "firewalld" ]; then
        if [ -f "/etc/fail2ban/action.d/firewallcmd-ipset.conf" ]; then
            banaction="firewallcmd-ipset"
        else
            banaction="iptables-allports"
        fi
    else
        banaction="iptables-allports"
    fi
    echo "$banaction"
}

setup_fail2ban() {
    local firewall_type="$1"
    print_message "配置 Fail2Ban (SSH 防护)"

    if ! command -v fail2ban-client &>/dev/null; then
        apt-get install -y fail2ban >/dev/null 2>&1 || yum install -y fail2ban >/dev/null 2>&1
    fi

    local banaction=$(detect_banaction "$firewall_type")
    echo "ℹ️ Fail2Ban 将使用动作: $banaction"

    bantime="1h"; maxretry="3"; findtime="10m"
    cat >/etc/fail2ban/jail.local <<EOF
[DEFAULT]
banaction = ${banaction}
backend = systemd
bantime = ${bantime}
findtime = ${findtime}
maxretry = ${maxretry}

[sshd]
enabled = true
bantime.increment = true
bantime.factor = 2
bantime.max = 1w
EOF

    systemctl enable --now fail2ban >/dev/null 2>&1
    systemctl restart fail2ban
    echo "✅ Fail2Ban 已启用并防护 SSH 登录。"
}

# === 清理并应用防火墙规则 ===
remove_unused_rules() {
    local ports_to_keep="$1"
    local firewall="$2"
    print_message "应用新的防火墙规则"
    local ports_array=($ports_to_keep)

    if [ "$firewall" = "ufw" ]; then
        echo "y" | ufw reset >/dev/null 2>&1
        ufw default deny incoming >/dev/null 2>&1
        ufw default allow outgoing >/dev/null 2>&1
        for p in "${ports_array[@]}"; do ufw allow "$p" >/dev/null; done
        ufw --force enable >/dev/null 2>&1
        ufw status | grep ALLOW
    elif [ "$firewall" = "firewalld" ]; then
        firewall-cmd --permanent --remove-service=ssh >/dev/null 2>&1 || true
        for p in "${ports_array[@]}"; do
            firewall-cmd --permanent --add-port="$p"/tcp >/dev/null 2>&1
            firewall-cmd --permanent --add-port="$p"/udp >/dev/null 2>&1
        done
        firewall-cmd --reload >/dev/null 2>&1
        firewall-cmd --list-ports
    fi
}

# === 自动安全更新 ===
setup_auto_updates() {
    print_message "配置无人值守安全更新"
    apt-get update -y >/dev/null
    apt-get install -y unattended-upgrades apt-listchanges >/dev/null
    cat >/etc/apt/apt.conf.d/20auto-upgrades <<'EOF'
APT::Periodic::Update-Package-Lists "1";
APT::Periodic::Unattended-Upgrade "1";
EOF
    cat >/etc/apt/apt.conf.d/51unattended-upgrades-reboot.conf <<'EOF'
Unattended-Upgrade::Automatic-Reboot "true";
Unattended-Upgrade::Automatic-Reboot-Time "03:00";
EOF
    systemctl enable --now apt-daily.timer >/dev/null 2>&1
    systemctl enable --now apt-daily-upgrade.timer >/dev/null 2>&1
    echo "✅ 已启用每日安全补丁与自动重启 (03:00)"
}

# === 内存日志配置 ===
setup_memory_log() {
    print_message "启用内存日志 (journald volatile)"
    mkdir -p /etc/systemd/journald.conf.d
    cat >/etc/systemd/journald.conf.d/volatile.conf <<'EOF'
[Journal]
Storage=volatile
RuntimeMaxUse=10M
MaxRetentionSec=2day
Compress=yes
EOF
    systemctl restart systemd-journald
    echo "✅ 已启用内存日志 (防止写盘)。"
}

# === 主流程 ===
main() {
    local firewall_type
    firewall_type=$(detect_firewall)
    [ "$firewall_type" = "none" ] && firewall_type=$(setup_firewall)
    setup_fail2ban "$firewall_type"

    ssh_port=$(grep -i '^Port ' /etc/ssh/sshd_config | awk '{print $2}' | head -n1)
    [ -z "$ssh_port" ] && ssh_port=22
    all_ports="$ssh_port"

    if command -v xray &>/dev/null && pgrep -f "xray" &>/dev/null; then
        xray_ports=$(ss -tnlp | grep xray | awk '{print $4}' | awk -F: '{print $NF}' | sort -u)
        [ -n "$xray_ports" ] && all_ports="$all_ports $xray_ports"
    fi
    if pgrep -f "sing-box" &>/dev/null; then
        sb_ports=$(ss -tnlp | grep sing-box | awk '{print $4}' | awk -F: '{print $NF}' | sort -u)
        [ -n "$sb_ports" ] && all_ports="$all_ports $sb_ports"
    fi
    if pgrep -f "xpanel" >/dev/null || pgrep -f "x-ui" >/dev/null; then
        if [ -f /etc/x-ui/x-ui.db ]; then
            xp_ports=$(sqlite3 /etc/x-ui/x-ui.db "SELECT port FROM inbounds;" | grep -E '^[0-9]+$' | sort -u)
            [ -n "$xp_ports" ] && all_ports="$all_ports $xp_ports"
        fi
        all_ports="$all_ports 80"
    fi

    all_ports=$(echo "$all_ports" | tr ' ' '\n' | sort -u | tr '\n' ' ')
    remove_unused_rules "$all_ports" "$firewall_type"

    setup_auto_updates
    setup_memory_log

    hostname=$(hostname)
    msg="*VPS 安全配置完成*
> *服务器*: \`$hostname\`
> *防火墙*: \`$firewall_type\`
> *Fail2Ban*: \`启用\`
> *安全更新*: \`自动启用\`
> *日志模式*: \`内存 (volatile)\`
> *重启时间*: \`03:00\`
> *保留端口*: \`$all_ports\`"
    send_telegram "$msg"

    print_message "🎉 所有安全与维护配置已成功完成！"
}

main
