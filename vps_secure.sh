#!/bin/bash
# -----------------------------------------------------------------------------------------
# VPS 代理服务端口检测与防火墙配置脚本（V3.8.12 全能修正版）
# 兼容 mack-a v2ray-agent / X-UI / Sing-box / 233boy
#
# 🩵 更新日志:
# V3.8.12-FULL
#   ✅ [回归] 修复 V3.8.11 中遗漏的 X-UI/X-Panel 检测模块
#   ✅ [X-UI] 检测到面板时，自动读取数据库端口并强制放行 80 (用于证书申请)
#   ✅ [继承] 保留 V3.8.11 的双重扫描、透明日志、防冲突所有特性
# -----------------------------------------------------------------------------------------

set -e
start_time=$(date +%s)

if [ "$(id -u)" -ne 0 ]; then
    echo "❌ 请以 root 权限运行本脚本。"
    exit 1
fi

FAIL2BAN_MODE="未选择"

# === 用户交互 ===
read -p "是否启用 Telegram 通知？(y/N): " enable_tg
if [[ "$enable_tg" =~ ^[Yy]$ ]]; then
    read -p "请输入 Telegram Bot Token: " TG_TOKEN
    read -p "请输入 Telegram Chat ID: " TG_CHAT_ID
    NOTIFY=true
else
    NOTIFY=false
fi

# --- 基础函数 ---
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
            -d chat_id="$TG_CHAT_ID" -d text="$message" -d parse_mode="MarkdownV2" >/dev/null 2>&1
    fi
}

install_dependency() {
    local pkg="$1"
    if ! command -v "$pkg" &>/dev/null; then
        echo "ℹ️ 未检测到 $pkg，正在安装..."
        if [ -f /etc/debian_version ]; then
            apt-get update -y >/dev/null 2>&1
            apt-get install -y "$pkg" >/dev/null 2>&1
        elif [ -f /etc/redhat-release ]; then
            yum install -y "$pkg" >/dev/null 2>&1 || dnf install -y "$pkg" >/dev/null 2>&1
        fi
        echo "✅ $pkg 安装完成。"
    fi
}

install_dependency "sqlite3"
install_dependency "jq"

# --- 端口提取函数 ---
extract_public_ports() {
    local file="$1"
    local key_port="$2"     # "port" or "listen_port"
    local key_listen="$3"   # "listen"
    local ports=""

    # 提取逻辑：过滤掉明确绑定到 127.0.0.1 或 localhost 的端口
    local jq_ports
    jq_ports=$(sed 's://.*$::g' "$file" | jq -r ".inbounds[] | select((.$key_listen == null) or (.$key_listen != \"127.0.0.1\" and .$key_listen != \"localhost\")) | .$key_port" 2>/dev/null | grep -E '^[0-9]+$' | sort -u)
    
    if [ -n "$jq_ports" ]; then
        ports="$ports $jq_ports"
    else
        # 兜底：grep 暴力匹配
        if ! grep -q "\"$key_listen\"\s*:\s*\"127.0.0.1\"" "$file"; then
             local grep_ports
             grep_ports=$(grep -oE "\"$key_port\"\s*:\s*[0-9]+" "$file" | grep -oE '[0-9]+' | sort -u)
             ports="$ports $grep_ports"
        fi
    fi
    echo "$ports" | tr ' ' '\n' | sort -u | tr '\n' ' '
}

get_ssh_port() {
    local port
    port=$(grep -iE '^\s*Port\s+[0-9]+' /etc/ssh/sshd_config 2>/dev/null | \
           grep -v '^\s*#' | \
           awk '{print $2}' | \
           grep -E '^[0-9]+$' | \
           head -n1)
    echo "${port:-22}"
}

detect_firewall() {
    if systemctl is-active --quiet firewalld 2>/dev/null; then
        echo "firewalld"
    elif command -v ufw &>/dev/null && LC_ALL=C ufw status 2>/dev/null | grep -qE "^Status:[[:space:]]+active"; then
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
            echo "✅ UFW 安装并启用成功。"
        else
            yum install -y firewalld >/dev/null 2>&1 || dnf install -y firewalld >/dev/null 2>&1
            systemctl enable --now firewalld >/dev/null 2>&1
            echo "✅ Firewalld 安装并启用成功。"
        fi
    else
        echo "❌ 无法识别的操作系统，请手动安装防火墙。"
    fi
}

detect_banaction() {
    local firewall_type="$1"
    if [ "$firewall_type" = "ufw" ]; then
        if [ -f "/etc/fail2ban/action.d/ufw-allports.conf" ]; then echo "ufw-allports"; 
        elif [ -f "/etc/fail2ban/action.d/ufw.conf" ]; then echo "ufw"; 
        else echo "iptables-allports"; fi
    elif [ "$firewall_type" = "firewalld" ]; then
        if [ -f "/etc/fail2ban/action.d/firewallcmd-ipset.conf" ]; then echo "firewallcmd-ipset"; 
        else echo "iptables-allports"; fi
    else
        echo "iptables-allports"
    fi
}

setup_fail2ban() {
    local firewall_type="$1"
    print_message "配置 Fail2Ban (SSH 防护)"
    if ! command -v fail2ban-client &>/dev/null; then
        echo "ℹ️ 正在安装 Fail2Ban..."
        install_dependency "fail2ban"
    fi
    
    systemctl stop fail2ban >/dev/null 2>&1
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
    *) FAIL2BAN_MODE="激进 (Aggressive)"; bantime="1h"; maxretry="3"; findtime="10m" ;;
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
port = ssh
logpath = /var/log/auth.log
bantime.increment = true
bantime.factor = 2
bantime.max = 1w
EOF
    systemctl enable --now fail2ban >/dev/null 2>&1
    systemctl restart fail2ban
    echo "✅ Fail2Ban 已配置为 [$FAIL2BAN_MODE] 并启动。"
}

remove_unused_rules() {
    local ports_to_keep="$1"
    local firewall="$2"
    local safe_ssh_port="$3"
    [ -z "$safe_ssh_port" ] && safe_ssh_port=22

    print_message "清理并应用新的防火墙规则"
    
    if systemctl is-active --quiet fail2ban; then
        echo "⏸️  临时暂停 Fail2Ban 以避免冲突..."
        systemctl stop fail2ban
    fi

    local ports_array=($ports_to_keep)

    if [ "$firewall" = "ufw" ]; then
        echo "y" | ufw reset >/dev/null 2>&1
        ufw default deny incoming >/dev/null 2>&1
        ufw default allow outgoing >/dev/null 2>&1
        
        echo "🔒 优先强制放行 SSH 端口: $safe_ssh_port"
        ufw allow "${safe_ssh_port}/tcp" >/dev/null

        for p in "${ports_array[@]}"; do 
            if [ "$p" != "$safe_ssh_port" ]; then
                echo "🌐 放行端口: $p"
                ufw allow "$p" >/dev/null
            fi
        done
        ufw --force enable >/dev/null 2>&1
        echo "✅ UFW 规则已更新"

    elif [ "$firewall" = "firewalld" ]; then
        local existing_ports
        existing_ports=$(firewall-cmd --list-ports 2>/dev/null)
        for p in $existing_ports; do
            firewall-cmd --permanent --remove-port="$p" >/dev/null 2>&1
        done

        echo "🔒 优先强制放行 SSH 端口: $safe_ssh_port"
        firewall-cmd --permanent --add-port="$safe_ssh_port"/tcp >/dev/null 2>&1
        
        for p in "${ports_array[@]}"; do
             if [ "$p" != "$safe_ssh_port" ]; then
                echo "🌐 放行端口: $p"
                firewall-cmd --permanent --add-port="$p"/tcp >/dev/null 2>&1
                firewall-cmd --permanent --add-port="$p"/udp >/dev/null 2>&1
            fi
        done
        firewall-cmd --reload >/dev/null 2>&1
        echo "✅ Firewalld 规则已更新"
    else
        echo "⚠️ 错误：未找到防火墙工具！"
    fi
}

self_check() {
    print_message "🔍 正在自检..."
    sleep 3
    local all_ok=true
    local issues=()

    if ! systemctl is-active --quiet fail2ban; then
        issues+=("Fail2Ban未运行")
        all_ok=false
    fi

    local ssh_port
    ssh_port=$(get_ssh_port)
    local fw
    fw=$(detect_firewall)

    if [ "$fw" = "ufw" ]; then
        if ! LC_ALL=C ufw status 2>/dev/null | grep -qE "(^|[[:space:]])${ssh_port}(/tcp)?.*(ALLOW|allow)"; then
            echo "⚠️ SSH 端口 $ssh_port 未放行！"
            issues+=("SSH未放行")
            all_ok=false
        fi
    fi

    echo "------------------------------------------------------------"
    if [ "$all_ok" = true ]; then
        echo "🎉 自检通过"
    else
        echo "⚠️ 自检发现问题: ${issues[*]}"
    fi
    echo "------------------------------------------------------------"
}

# --- 主程序 ---
main() {
    local firewall_type
    firewall_type=$(detect_firewall)
    if [ "$firewall_type" = "none" ]; then
        setup_firewall
        firewall_type=$(detect_firewall)
    fi
    [ "$firewall_type" = "none" ] && { echo "❌ 防火墙错误"; exit 1; }

    echo "✅ 防火墙: $firewall_type"

    local ssh_port
    ssh_port=$(get_ssh_port)
    echo "🛡️ SSH 端口: $ssh_port"
    local all_ports="$ssh_port"

    # === 智能 Web 端口检测 ===
    if pgrep -x "nginx" >/dev/null || pgrep -x "apache2" >/dev/null; then
        echo "🌐 检测到 Web 服务器，放行 80/443"
        all_ports="$all_ports 80 443"
    else
        echo "ℹ️ 未检测到 Web 服务器 (Nginx)，跳过 80/443"
    fi

    # === Xray 端口深度检测 (双重扫描模式) ===
    xray_ports=""
    
    # 1. 扫描配置目录 (Config Scan)
    if [ -d "/etc/v2ray-agent" ] || command -v xray &>/dev/null; then
        xray_config_dirs=("/etc/xray/conf" "/etc/v2ray-agent/xray/conf" "/usr/local/etc/xray")
        
        for config_dir in "${xray_config_dirs[@]}"; do
            if [ -d "$config_dir" ]; then
                echo "📂 扫描目录: $config_dir"
                for config_file in "$config_dir"/*.json; do
                    [ -f "$config_file" ] || continue
                    config_ports=$(extract_public_ports "$config_file" "port" "listen")
                    if [ -n "$config_ports" ]; then
                        echo "   📄 文件 $(basename "$config_file") -> 发现端口: $config_ports"
                        xray_ports="$xray_ports $config_ports"
                    fi
                done
            fi
        done
    fi

    # 2. 扫描运行进程 (Process Scan - 强制执行)
    echo "🕵️ 正在执行系统网络扫描 (ss/netstat)..."
    sys_ports=$(ss -tnlp 2>/dev/null | grep -E "xray|v2ray" | grep -v "127.0.0.1" | grep -v "\[::1\]" | awk '{print $4}' | grep -oE '[0-9]+$' | sort -u)
    
    if [ -n "$sys_ports" ]; then
         echo "   ⚙️ 进程扫描发现端口: $sys_ports"
         xray_ports="$xray_ports $sys_ports"
    fi

    # 合并结果
    xray_ports=$(echo "$xray_ports" | tr ' ' '\n' | sort -u | tr '\n' ' ')
    if [ -n "$xray_ports" ]; then
        echo "🛡️ 检测到 Xray 公网端口: $xray_ports"
        all_ports="$all_ports $xray_ports"
    fi

    # === Sing-box 端口检测 ===
    sb_ports=""
    sb_config_dirs=("/etc/sing-box/conf" "/etc/v2ray-agent/sing-box/conf/config")
    for config_dir in "${sb_config_dirs[@]}"; do
        if [ -d "$config_dir" ]; then
            for config_file in "$config_dir"/*.json; do
                [ -f "$config_file" ] || continue
                config_ports=$(extract_public_ports "$config_file" "listen_port" "listen")
                [ -n "$config_ports" ] && sb_ports="$sb_ports $config_ports"
            done
        fi
    done
    # Sing-box 进程扫描
    sys_sb_ports=$(ss -tnlp 2>/dev/null | grep -w "sing-box" | grep -v "127.0.0.1" | grep -v "\[::1\]" | awk '{print $4}' | grep -oE '[0-9]+$' | sort -u)
    [ -n "$sys_sb_ports" ] && sb_ports="$sb_ports $sys_sb_ports"

    sb_ports=$(echo "$sb_ports" | tr ' ' '\n' | sort -u | tr '\n' ' ')
    [ -n "$sb_ports" ] && echo "🛡️ 检测到 Sing-box 端口: $sb_ports" && all_ports="$all_ports $sb_ports"

    # === X-Panel / X-UI 端口检测 (回归!) ===
    if pgrep -f "xpanel" >/dev/null || pgrep -f "x-ui" >/dev/null; then
        echo "🌐 检测到 X-UI/X-Panel 进程"
        if [ -f /etc/x-ui/x-ui.db ]; then
            xpanel_ports=$(sqlite3 /etc/x-ui/x-ui.db "SELECT port FROM inbounds WHERE port IS NOT NULL AND port != '';" 2>/dev/null | grep -E '^[0-9]+$' | sort -u)
            if [ -n "$xpanel_ports" ]; then
                echo "   📊 面板入站端口: $xpanel_ports"
                all_ports="$all_ports $xpanel_ports"
            fi
        fi
        echo "   🔓 自动放行 80 端口 (用于证书申请)"
        all_ports="$all_ports 80"
    fi

    all_ports=$(echo "$all_ports" | tr ' ' '\n' | sort -u | tr '\n' ' ')
    print_message "最终放行端口: $all_ports"
    
    remove_unused_rules "$all_ports" "$firewall_type" "$ssh_port"
    setup_fail2ban "$firewall_type"

    print_message "✅ 所有安全配置已成功应用！"
}

main
self_check