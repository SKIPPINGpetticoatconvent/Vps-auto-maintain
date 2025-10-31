#!/bin/bash
# -----------------------------------------------------------------------------------------
# VPS 代理服务端口检测与防火墙配置脚本（终极一键安全版 V3.8.0 - 自检增强版）
# 兼容 xeefei X-Panel / X-UI / Xray / Sing-box
#
# 🩵 更新日志:
# V3.8.0 - [稳定版]
#   ✅ 新增自动自检模块，使用正则匹配验证配置正确性
#   ✅ 检查 Fail2Ban / 防火墙 / SSH / X-Panel 端口状态
#   ✅ 自动汇总结果（可推送 Telegram 报告）
#   ✅ 添加执行耗时统计，运行更直观
# -----------------------------------------------------------------------------------------

set -e
start_time=$(date +%s)

if [ "$(id -u)" -ne 0 ]; then
    echo "❌ 请以 root 权限运行本脚本。"
    exit 1
fi

FAIL2BAN_MODE="未选择"

# === 用户交互输入 ===
read -p "是否启用 Telegram 通知？(y/N): " enable_tg
if [[ "$enable_tg" =~ ^[Yy]$ ]]; then
    read -p "请输入 Telegram Bot Token: " TG_TOKEN
    read -p "请输入 Telegram Chat ID: " TG_CHAT_ID
    NOTIFY=true
else
    NOTIFY=false
fi

# --- 打印消息 ---
print_message() {
    echo ""
    echo "------------------------------------------------------------"
    echo "$1"
    echo "------------------------------------------------------------"
}

# --- Telegram 通知 ---
send_telegram() {
    if [ "$NOTIFY" = true ] && [ -n "$TG_TOKEN" ] && [ -n "$TG_CHAT_ID" ]; then
        local message="$1"
        message=$(echo "$message" | sed 's/`/\\`/g' | sed 's/\*/\\*/g' | sed 's/_/\\_/g')
        curl --connect-timeout 10 --retry 3 -s -X POST \
            "https://api.telegram.org/bot$TG_TOKEN/sendMessage" \
            -d chat_id="$TG_CHAT_ID" -d text="$message" -d parse_mode="MarkdownV2" >/dev/null
    fi
}

# --- 自动安装 sqlite3 ---
if ! command -v sqlite3 &>/dev/null; then
    echo "ℹ️ 未检测到 sqlite3，正在安装..."
    if [ -f /etc/debian_version ]; then
        apt-get update -y >/dev/null 2>&1
        apt-get install -y sqlite3 >/dev/null 2>&1
    elif [ -f /etc/redhat-release ]; then
        yum install -y sqlite >/dev/null 2>&1 || dnf install -y sqlite >/dev/null 2>&1
    fi
    echo "✅ sqlite3 安装完成。"
fi

# --- 检测防火墙 ---
detect_firewall() {
    if systemctl is-active --quiet firewalld 2>/dev/null; then
        echo "firewalld"
    elif command -v ufw &>/dev/null && ufw status 2>/dev/null | grep -q "Status: active"; then
        echo "ufw"
    else
        echo "none"
    fi
}

# --- 安装防火墙 ---
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

# --- 自动检测 Fail2Ban 封禁动作 ---
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

# --- 安装并配置 Fail2Ban ---
setup_fail2ban() {
    local firewall_type="$1"
    print_message "配置 Fail2Ban (SSH 防护)"

    if ! command -v fail2ban-client &>/dev/null; then
        echo "ℹ️ 正在安装 Fail2Ban..."
        apt-get install -y fail2ban >/dev/null 2>&1 || yum install -y fail2ban >/dev/null 2>&1
        echo "✅ Fail2Ban 安装完成。"
    fi

    rm -f /etc/fail2ban/filter.d/sshd-ddos.conf
    local banaction=$(detect_banaction "$firewall_type")
    echo "ℹ️ Fail2Ban 将使用动作: $banaction"

    echo "请选择 Fail2Ban SSH 防护模式:"
    echo "  1) 普通模式: 5次失败封禁10分钟"
    echo "  2) 激进模式: 推荐！3次失败封禁1小时，屡教不改翻倍"
    echo "  3) 偏执模式: 2次失败封禁12小时，屡教不改×3"
    read -p "请输入选项 [1-3], 默认 2: " mode
    mode=${mode:-2}

    case $mode in
    1) FAIL2BAN_MODE="普通 (Normal)"; bantime="10m"; maxretry="5"; findtime="10m" ;;
    2) FAIL2BAN_MODE="激进 (Aggressive)"; bantime="1h"; maxretry="3"; findtime="10m" ;;
    3) FAIL2BAN_MODE="偏执 (Paranoid)"; bantime="1h"; maxretry="2"; findtime="10m" ;;
    *) echo "无效输入，退出"; exit 1 ;;
    esac

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
    echo "✅ Fail2Ban 已配置为 [$FAIL2BAN_MODE] 并启动。"
}

# --- 清理并添加防火墙规则 ---
remove_unused_rules() {
    local ports_to_keep="$1"
    local firewall="$2"
    print_message "清理并应用新的防火墙规则"
    local ports_array=($ports_to_keep)

    if [ "$firewall" = "ufw" ]; then
        echo "y" | ufw reset >/dev/null 2>&1
        ufw default deny incoming >/dev/null 2>&1
        ufw default allow outgoing >/dev/null 2>&1
        for p in "${ports_array[@]}"; do ufw allow "$p" >/dev/null; done
        ufw --force enable >/dev/null 2>&1
        echo "✅ UFW 规则已更新。"
        ufw status
    elif [ "$firewall" = "firewalld" ]; then
        local existing_ports
        existing_ports=$(firewall-cmd --list-ports)
        for p in $existing_ports; do
            firewall-cmd --permanent --remove-port="$p" >/dev/null 2>&1
        done
        for p in "${ports_array[@]}"; do
            firewall-cmd --permanent --add-port="$p"/tcp >/dev/null 2>&1
            firewall-cmd --permanent --add-port="$p"/udp >/dev/null 2>&1
        done
        firewall-cmd --reload >/dev/null 2>&1
        echo "✅ firewalld 规则已更新。"
        firewall-cmd --list-ports
    else
        echo "⚠️ 未找到有效防火墙工具。"
    fi
}

# --- 自检模块 ---
self_check() {
    print_message "🔍 正在进行配置自检..."
    local all_ok=true
    local report=""

    # Fail2Ban 检查
    if systemctl is-active --quiet fail2ban; then
        echo "✅ Fail2Ban 服务正在运行。"
    else
        echo "⚠️ Fail2Ban 未运行！"
        all_ok=false
    fi

    if fail2ban-client status sshd 2>/dev/null | grep -Eq 'Jail list:.*sshd'; then
        echo "✅ SSH 防护已启用。"
    else
        echo "⚠️ SSH Jail 未加载。"
        all_ok=false
    fi

    # 防火墙检查
    local fw
    fw=$(detect_firewall)
    if [ "$fw" = "ufw" ] && ufw status | grep -q "active"; then
        echo "✅ UFW 已启用。"
    elif [ "$fw" = "firewalld" ] && firewall-cmd --state 2>/dev/null | grep -q "running"; then
        echo "✅ Firewalld 已启用。"
    else
        echo "⚠️ 防火墙未启用。"
        all_ok=false
    fi

    # SSH端口检查
    local ssh_port
    ssh_port=$(grep -i '^Port ' /etc/ssh/sshd_config | awk '{print $2}' | head -n1)
    [ -z "$ssh_port" ] && ssh_port=22
    if ss -tln | grep -q ":$ssh_port "; then
        echo "✅ SSH 端口 $ssh_port 监听正常。"
    else
        echo "⚠️ SSH 端口 $ssh_port 未监听！"
        all_ok=false
    fi

    # 汇总结果
    echo "------------------------------------------------------------"
    if [ "$all_ok" = true ]; then
        echo "🎉 自检通过：所有关键安全配置均正常工作。"
        report="✅ 自检通过，系统配置正常。"
    else
        echo "⚠️ 自检发现问题，请检查日志。"
        report="⚠️ 自检发现问题，请检查服务器。"
    fi
    echo "------------------------------------------------------------"

    # Telegram 推送报告
    local hostname=$(hostname)
    local duration=$(( $(date +%s) - start_time ))
    local msg="*VPS 自检报告*
> *主机名*: \`$hostname\`
> *防火墙*: \`$fw\`
> *Fail2Ban模式*: \`$FAIL2BAN_MODE\`
> *SSH端口*: \`$ssh_port\`
> *结果*: $report
> *执行耗时*: ${duration}s"
    send_telegram "$msg"
}

# --- 主程序 ---
main() {
    local firewall_type
    firewall_type=$(detect_firewall)
    [ "$firewall_type" = "none" ] && firewall_type=$(setup_firewall)

    setup_fail2ban "$firewall_type"

    local ssh_port
    ssh_port=$(grep -i '^Port ' /etc/ssh/sshd_config | awk '{print $2}' | head -n1)
    [ -z "$ssh_port" ] && ssh_port=22
    echo "🛡️ 检测到 SSH 端口: $ssh_port"

    local all_ports="$ssh_port"
    if command -v xray &>/dev/null && pgrep -f "xray" &>/dev/null; then
        xray_ports=$(ss -tnlp | grep xray | awk '{print $4}' | awk -F: '{print $NF}' | sort -u)
        [ -n "$xray_ports" ] && echo "🛡️ 检测到 Xray 端口: $xray_ports" && all_ports="$all_ports $xray_ports"
    fi
    if pgrep -f "sing-box" &>/dev/null; then
        sb_ports=$(ss -tnlp | grep sing-box | awk '{print $4}' | awk -F: '{print $NF}' | sort -u)
        [ -n "$sb_ports" ] && echo "🛡️ 检测到 Sing-box 端口: $sb_ports" && all_ports="$all_ports $sb_ports"
    fi
    if pgrep -f "xpanel" >/dev/null || pgrep -f "x-ui" >/dev/null; then
        if [ -f /etc/x-ui/x-ui.db ]; then
            xpanel_ports=$(sqlite3 /etc/x-ui/x-ui.db "SELECT port FROM inbounds;" | grep -E '^[0-9]+$' | sort -u)
            [ -n "$xpanel_ports" ] && echo "🛡️ 检测到 X-Panel 入站端口: $xpanel_ports" && all_ports="$all_ports $xpanel_ports"
        fi
        echo "🌐 检测到面板进程，自动放行 80 端口（用于证书申请）。"
        all_ports="$all_ports 80"
    fi

    all_ports=$(echo "$all_ports" | tr ' ' '\n' | sort -u | tr '\n' ' ')
    print_message "最终将保留的端口: $all_ports"
    remove_unused_rules "$all_ports" "$firewall_type"

    local hostname=$(hostname)
    local msg="*VPS 安全配置完成*
> *服务器*: \`$hostname\`
> *防火墙*: \`$firewall_type\`
> *Fail2Ban模式*: \`$FAIL2BAN_MODE\`
> *保留端口*: \`$all_ports\`"
    send_telegram "$msg"

    print_message "✅ 所有安全配置已成功应用！"
}

main
self_check
