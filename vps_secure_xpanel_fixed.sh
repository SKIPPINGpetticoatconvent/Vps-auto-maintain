#!/bin/bash
# -----------------------------------------------------------------------------------------
# VPS 代理服务端口检测和防火墙配置脚本（终极一键安全版 V3.2 - 兼容 xeefei X-Panel）
#
# 功能：
# - 自动安装防火墙（UFW/firewalld）并启用
# - [新增] 提供三种可选的 Fail2Ban 安全模式（普通/激进/偏执）
# - 自动安装 Fail2Ban 并根据选择的模式强化 SSH 防护
# - 自动检测 SSH、Xray、Sing-box、X-Panel（x-ui/xpanel）端口
# - 若检测到 x-ui 进程则自动开放 80 端口（证书申请）
# - 清理无用防火墙端口
# - 可选 Telegram 通知（运行时输入 Token/Chat ID）
# -----------------------------------------------------------------------------------------

set -e

# 全局变量
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

# --- Telegram 消息发送 ---
send_telegram() {
    if [ "$NOTIFY" = true ] && [ -n "$TG_TOKEN" ] && [ -n "$TG_CHAT_ID" ]; then
        local message="$1"
        # 确保消息中的特殊字符被正确处理
        message=$(echo "$message" | sed 's/`/\`/g' | sed 's/\*/\\\*/g' | sed 's/_/\\_/g')
        curl --connect-timeout 10 --retry 3 -s -X POST "https://api.telegram.org/bot$TG_TOKEN/sendMessage" \
            -d chat_id="$TG_CHAT_ID" \
            -d text="$message" \
            -d parse_mode="MarkdownV2" >/dev/null
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

# --- [新增] 创建 sshd-ddos 过滤器 ---
create_sshd_ddos_filter() {
    print_message "创建 Fail2Ban 高级过滤器 (sshd-ddos)"
    cat >/etc/fail2ban/filter.d/sshd-ddos.conf <<EOF
# Fail2Ban filter for sshd-ddos
# 捕捉高频次的SSH连接请求，即使没有登录尝试
[Definition]
failregex = ^%(__prefix_line)sDid not receive identification string from <HOST>
            ^%(__prefix_line)sConnection reset by <HOST>
            ^%(__prefix_line)sConnection closed by <HOST>
            ^%(__prefix_line)sSSH: Server;Ltype: Kex;Remote: <HOST>
ignoreregex =
EOF
    echo "✅ sshd-ddos 过滤器已创建。"
}

# --- [重构] 安装并配置 Fail2Ban (带模式选择) ---
setup_fail2ban() {
    print_message "配置 Fail2Ban (SSH 防护)"

    # 1. 安装 Fail2Ban
    if ! command -v fail2ban-client &>/dev/null; then
        echo "ℹ️ 正在安装 Fail2Ban..."
        apt-get install -y fail2ban >/dev/null 2>&1 || yum install -y fail2ban >/dev/null 2>&1
        echo "✅ Fail2Ban 安装完成。"
    fi

    # 2. 用户选择安全模式
    echo "请为 Fail2Ban 选择一个 SSH 防护模式:"
    echo "  1) 普通模式 (Normal): 5次失败 -> 封禁10分钟。适合普通用户。"
    echo "  2) 激进模式 (Aggressive): 推荐！多层防御，自动加重惩罚，有效抵御持续攻击。"
    echo "  3) 偏执模式 (Paranoid): 极其严格，惯犯将被永久封禁。请确保您有其他登录方式！"
    read -p "请输入选项 [1-3], (默认: 2): " mode
    mode=${mode:-2}

    # 3. 根据选择应用配置
    case $mode in
    1)
        FAIL2BAN_MODE="普通 (Normal)"
        print_message "应用 Fail2Ban [普通模式]"
        cat >/etc/fail2ban/jail.local <<EOF
[DEFAULT]
bantime  = 10m
findtime = 10m
maxretry = 5

[sshd]
enabled = true
EOF
        ;;
    2)
        FAIL2BAN_MODE="激进 (Aggressive)"
        print_message "应用 Fail2Ban [激进模式]"
        create_sshd_ddos_filter
        cat >/etc/fail2ban/jail.local <<EOF
[DEFAULT]
# 默认的封禁时间，可以设置得短一些
bantime  = 1h
findtime = 10m
maxretry = 3

[sshd]
# 针对标准登录失败的基础防护
enabled  = true
maxretry = 3
findtime = 10m
bantime  = 1h

[sshd-aggressive]
# 针对一天内多次触发封禁的“惯犯”
enabled  = true
filter   = sshd
logpath  = %(sshd_log)s
backend  = %(sshd_backend)s
maxretry = 5
findtime = 1d
bantime  = 1w

[sshd-ddos]
# 针对高频连接扫描
enabled  = true
filter   = sshd-ddos
logpath  = %(sshd_log)s
backend  = %(sshd_backend)s
maxretry = 5
findtime = 1m
bantime  = 1d
EOF
        ;;
    3)
        FAIL2BAN_MODE="偏执 (Paranoid)"
        print_message "应用 Fail2Ban [偏执模式]"
        create_sshd_ddos_filter
        cat >/etc/fail2ban/jail.local <<EOF
[DEFAULT]
bantime  = 1h
findtime = 10m
maxretry = 3

[sshd]
enabled  = true
maxretry = 2     # 2次失败就封
findtime = 10m
bantime  = 12h   # 首次封禁12小时

[sshd-aggressive]
enabled  = true
filter   = sshd
logpath  = %(sshd_log)s
backend  = %(sshd_backend)s
maxretry = 3     # 一天内被封禁3次
findtime = 1d
bantime  = -1    # 永久封禁！

[sshd-ddos]
enabled  = true
filter   = sshd-ddos
logpath  = %(sshd_log)s
backend  = %(sshd_backend)s
maxretry = 3
findtime = 1m
bantime  = 1w    # 高频扫描直接封一周
EOF
        ;;
    *)
        echo "无效输入，已退出。"
        exit 1
        ;;
    esac

    systemctl enable --now fail2ban >/dev/null 2>&1
    systemctl restart fail2ban
    echo "✅ Fail2Ban 已配置为 [$FAIL2BAN_MODE] 并启动。"
}

# --- 清理并添加防火墙规则 ---
remove_unused_rules() {
    local ports_to_keep="$1"
    local firewall="$2"
    print_message "清理并应用新的防火墙规则"
    
    # 将空格分隔的端口列表转换为数组
    local ports_array=($ports_to_keep)

    if [ "$firewall" = "ufw" ]; {
        echo "y" | ufw reset >/dev/null 2>&1
        ufw default deny incoming >/dev/null 2>&1
        ufw default allow outgoing >/dev/null 2>&1
        for p in "${ports_array[@]}"; do ufw allow "$p" >/dev/null; done
        ufw --force enable >/dev/null 2>&1
        echo "✅ UFW 规则已更新。"
        ufw status
    } elif [ "$firewall" = "firewalld" ]; {
        # 移除所有现有规则
        local existing_ports=$(firewall-cmd --list-ports)
        for p in $existing_ports; do
            firewall-cmd --permanent --remove-port=$p >/dev/null 2>&1
        done
        # 添加需要保留的规则
        for p in "${ports_array[@]}"; do
            firewall-cmd --permanent --add-port="$p"/tcp >/dev/null 2>&1
            firewall-cmd --permanent --add-port="$p"/udp >/dev/null 2>&1
        done
        firewall-cmd --reload >/dev/null 2>&1
        echo "✅ firewalld 规则已更新。"
        firewall-cmd --list-ports
    } else {
        echo "⚠️ 未找到有效的防火墙工具 (ufw/firewalld)。"
    }
}


# --- 主程序 ---
main() {
    setup_fail2ban

    local firewall_type
    firewall_type=$(detect_firewall)
    [ "$firewall_type" = "none" ] && firewall_type=$(setup_firewall)

    local ssh_port
    ssh_port=$(grep -i '^Port ' /etc/ssh/sshd_config | awk '{print $2}' | head -n1)
    [ -z "$ssh_port" ] && ssh_port=22
    echo "🛡️  检测到 SSH 端口: $ssh_port"

    local all_ports="$ssh_port"

    # Xray
    if command -v xray &>/dev/null && pgrep -f "xray" &>/dev/null; then
        xray_ports=$(ss -tnlp | grep xray | awk '{print $4}' | awk -F: '{print $NF}' | sort -u | tr '\n' ' ')
        if [ -n "$xray_ports" ]; then
            echo "🛡️  检测到 Xray 端口: $xray_ports"
            all_ports="$all_ports $xray_ports"
        fi
    fi

    # Sing-box
    if pgrep -f "sing-box" &>/dev/null; then
        sb_ports=$(ss -tnlp | grep sing-box | awk '{print $4}' | awk -F: '{print $NF}' | sort -u | tr '\n' ' ')
        if [ -n "$sb_ports" ]; then
            echo "🛡️  检测到 Sing-box 端口: $sb_ports"
            all_ports="$all_ports $sb_ports"
        fi
    fi

    # X-Panel / x-ui / xpanel
    if pgrep -f "xpanel" >/dev/null || pgrep -f "x-ui" >/dev/null; then
        if [ -f /etc/x-ui/x-ui.db ]; then
            # 兼容不同版本的sqlite3输出
            xpanel_ports=$(sqlite3 /etc/x-ui/x-ui.db "SELECT port FROM inbounds;" | grep -E '^[0-9]+$' | sort -u | tr '\n' ' ')
            if [ -n "$xpanel_ports" ]; 键，然后
                echo "🛡️  检测到 X-Panel 入站端口: $xpanel_ports"
                all_ports="$all_ports $xpanel_ports"
            fi
        fi

        # 检测到 x-ui 时自动加入 80 端口
        if pgrep -f "x-ui" >/dev/null || pgrep -f "xpanel" >/dev/null; then
            echo "🌐 检测到面板进程，自动放行 80 端口（用于证书申请）。"
            all_ports="$all_ports 80"
        fi
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
