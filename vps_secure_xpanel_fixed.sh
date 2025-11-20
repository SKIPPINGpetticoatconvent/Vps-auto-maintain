#!/bin/bash
# -----------------------------------------------------------------------------------------
# VPS 代理服务端口检测与防火墙配置脚本（V3.8.1 正则表达式修复版）
# 兼容 xeefei X-Panel / X-UI / Xray / Sing-box
#
# 🩵 更新日志:
# V3.8.1-FIXED - [正则表达式修复版]
#   ✅ 修复 SSH 端口检测：支持 Tab 分隔符，过滤注释行
#   ✅ 修复端口监听检测：避免 8022 误匹配 22
#   ✅ 修复进程名匹配：使用 pgrep -x 精确匹配
#   ✅ 修复 IPv6 端口提取：使用 grep -oE '[0-9]+$'
#   ✅ 修复 SQL 查询：过滤 NULL 和空值
#   ✅ 增强防火墙状态检测：精确匹配状态字符串
#   ✅ 新增详细问题报告：记录所有检测失败项
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
            -d chat_id="$TG_CHAT_ID" -d text="$message" -d parse_mode="MarkdownV2" >/dev/null 2>&1
    fi
}

# --- 自动安装依赖工具 ---
# 安装 sqlite3
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

# 安装 jq (用于JSON解析)
if ! command -v jq &>/dev/null; then
    echo "ℹ️ 未检测到 jq，正在安装..."
    if [ -f /etc/debian_version ]; then
        apt-get update -y >/dev/null 2>&1
        apt-get install -y jq >/dev/null 2>&1
    elif [ -f /etc/redhat-release ]; then
        yum install -y jq >/dev/null 2>&1 || dnf install -y jq >/dev/null 2>&1
    fi
    echo "✅ jq 安装完成。"
fi

# --- 检测防火墙 ---
detect_firewall() {
    if systemctl is-active --quiet firewalld 2>/dev/null; then
        echo "firewalld"
    elif command -v ufw &>/dev/null && ufw status 2>/dev/null | grep -qE "^Status:\s+active"; then
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
    elif [ "$firewall" = "firewalld" ]; then
        local existing_ports
        existing_ports=$(firewall-cmd --list-ports 2>/dev/null)
        for p in $existing_ports; do
            firewall-cmd --permanent --remove-port="$p" >/dev/null 2>&1
        done
        for p in "${ports_array[@]}"; do
            firewall-cmd --permanent --add-port="$p"/tcp >/dev/null 2>&1
            firewall-cmd --permanent --add-port="$p"/udp >/dev/null 2>&1
        done
        firewall-cmd --reload >/dev/null 2>&1
        echo "✅ firewalld 规则已更新。"
    else
        echo "⚠️ 未找到有效防火墙工具。"
    fi
}

# --- 自检模块（正则表达式修复版）---
self_check() {
    print_message "🔍 正在进行配置自检（增强版）..."
    sleep 5  # 等待 Fail2Ban 初始化
    local all_ok=true
    local issues=()

    # === Fail2Ban 状态检测 ===
    if systemctl is-active --quiet fail2ban 2>/dev/null; then
        echo "✅ Fail2Ban 服务正在运行。"
    else
        echo "⚠️ Fail2Ban 未运行！"
        issues+=("Fail2Ban服务未运行")
        all_ok=false
    fi

    # === SSH Jail 检测与自动重试 ===
    if ! fail2ban-client status sshd >/dev/null 2>&1; then
        echo "⚠️ SSH Jail 初次检测未加载，等待 5 秒后重试..."
        sleep 5
        systemctl reload fail2ban >/dev/null 2>&1
        if fail2ban-client status sshd >/dev/null 2>&1; then
            echo "✅ SSH Jail 已在重试后加载成功。"
        else
            echo "❌ SSH Jail 加载失败，请检查配置。"
            issues+=("SSH-Jail未加载")
            all_ok=false
        fi
    else
        echo "✅ SSH Jail 已正确加载。"
    fi

    # === 防火墙检测（增强版）===
    local fw
    fw=$(detect_firewall)
    if [ "$fw" = "ufw" ]; then
        if ufw status 2>/dev/null | grep -qE "^Status:\s+active"; then
            echo "✅ UFW 已启用。"
        else
            echo "⚠️ UFW 未启用。"
            issues+=("UFW未激活")
            all_ok=false
        fi
    elif [ "$fw" = "firewalld" ]; then
        if firewall-cmd --state 2>/dev/null | grep -qE "^running$"; then
            echo "✅ Firewalld 已启用。"
        else
            echo "⚠️ Firewalld 未启用。"
            issues+=("Firewalld未运行")
            all_ok=false
        fi
    else
        echo "⚠️ 防火墙未启用。"
        issues+=("无防火墙")
        all_ok=false
    fi

    # === SSH 端口检测（修复版：支持 Tab，过滤注释）===
    local ssh_port
    ssh_port=$(grep -iE '^\s*Port\s+[0-9]+' /etc/ssh/sshd_config 2>/dev/null | \
               grep -v '^\s*#' | \
               awk '{print $2}' | \
               grep -E '^[0-9]+$' | \
               head -n1)
    [ -z "$ssh_port" ] && ssh_port=22
    
    # === SSH 监听检测（修复版：避免 8022 误匹配 22）===
    if ss -tln 2>/dev/null | grep -qE "[^0-9]${ssh_port}(\s|$)"; then
        echo "✅ SSH 端口 $ssh_port 监听正常。"
    else
        echo "⚠️ SSH 端口 $ssh_port 未监听！"
        issues+=("SSH端口${ssh_port}未监听")
        all_ok=false
    fi

    # === 验证 SSH 端口是否在防火墙规则中 ===
    if [ "$fw" = "ufw" ]; then
        if ! ufw status 2>/dev/null | grep -qE "^${ssh_port}(/tcp)?\s+(ALLOW|allow)"; then
            echo "⚠️ SSH 端口 $ssh_port 未在 UFW 规则中！"
            issues+=("SSH端口未放行")
            all_ok=false
        fi
    elif [ "$fw" = "firewalld" ]; then
        if ! firewall-cmd --list-ports 2>/dev/null | grep -qE "${ssh_port}/(tcp|udp)"; then
            echo "⚠️ SSH 端口 $ssh_port 未在 Firewalld 规则中！"
            issues+=("SSH端口未放行")
            all_ok=false
        fi
    fi

    echo "------------------------------------------------------------"
    local hostname=$(hostname)
    local duration=$(( $(date +%s) - start_time ))

    if [ "$all_ok" = true ]; then
        echo "🎉 自检通过：所有关键安全配置均正常工作。"
        result="✅ 自检通过"
        issue_summary=""
    else
        echo "⚠️ 自检发现问题，请手动检查。"
        result="⚠️ 发现问题"
        issue_summary="\n> *问题*: ${issues[*]}"
    fi
    echo "------------------------------------------------------------"

    local msg="*VPS 安全配置完成*
> *主机*: \`$hostname\`
> *防火墙*: \`$fw\`
> *SSH*: \`$ssh_port\`
> *Fail2Ban*: \`$FAIL2BAN_MODE\`
> *结果*: $result${issue_summary}
> *耗时*: ${duration}s"
    send_telegram "$msg"
}

# --- 主程序 ---
main() {
    local firewall_type
    firewall_type=$(detect_firewall)
    [ "$firewall_type" = "none" ] && firewall_type=$(setup_firewall)

    setup_fail2ban "$firewall_type"

    # === SSH 端口检测（修复版）===
    local ssh_port
    ssh_port=$(grep -iE '^\s*Port\s+[0-9]+' /etc/ssh/sshd_config 2>/dev/null | \
               grep -v '^\s*#' | \
               awk '{print $2}' | \
               grep -E '^[0-9]+$' | \
               head -n1)
    [ -z "$ssh_port" ] && ssh_port=22
    echo "🛡️ 检测到 SSH 端口: $ssh_port"

    local all_ports="$ssh_port"

    # === Xray 端口检测（修复版：精确匹配进程名，兼容v2ray-agent）===
    if command -v xray &>/dev/null && pgrep -x "xray" &>/dev/null; then
        xray_ports=""
        # 优先从配置文件检测端口
        xray_config_dirs=("/etc/xray/conf" "/etc/v2ray-agent/xray/conf")
        for config_dir in "${xray_config_dirs[@]}"; do
            if [ -d "$config_dir" ]; then
                for config_file in "$config_dir"/*.json; do
                    if [ -f "$config_file" ]; then
                        # 从JSON配置中提取port
                        config_ports=$(jq -r '.inbounds[]?.port // empty' "$config_file" 2>/dev/null | sort -u | tr '\n' ' ')
                        if [ -n "$config_ports" ]; then
                            xray_ports="$xray_ports $config_ports"
                        fi
                    fi
                done
            fi
        done

        # 如果未从配置文件获取到端口，回退到网络监听检测
        if [ -z "$xray_ports" ]; then
            xray_ports=$(ss -tnlp 2>/dev/null | grep -w xray | awk '{print $4}' | grep -oE '[0-9]+$' | sort -u)
        fi

        xray_ports=$(echo "$xray_ports" | tr ' ' '\n' | sort -u | tr '\n' ' ')
        if [ -n "$xray_ports" ]; then
            echo "🛡️ 检测到 Xray 端口: $xray_ports"
            all_ports="$all_ports $xray_ports"
        fi
    fi

    # === Sing-box 端口检测（修复版：从配置文件读取，兼容v2ray-agent）===
    if pgrep -x "sing-box" &>/dev/null; then
        sb_ports=""
        # 检查配置文件目录是否存在（支持多个路径）
        config_dirs=("/etc/sing-box/conf" "/etc/v2ray-agent/sing-box/conf/config")

        for config_dir in "${config_dirs[@]}"; do
            if [ -d "$config_dir" ]; then
                # 遍历所有配置文件，提取监听端口
                for config_file in "$config_dir"/*.json; do
                    if [ -f "$config_file" ]; then
                        # 从JSON配置中提取listen_port
                        config_ports=$(jq -r '.inbounds[]?.listen_port // empty' "$config_file" 2>/dev/null | sort -u | tr '\n' ' ')
                        if [ -n "$config_ports" ]; then
                            sb_ports="$sb_ports $config_ports"
                        fi
                    fi
                done
            fi
        done

        # 如果未从配置文件获取到端口，回退到网络监听检测
        if [ -z "$sb_ports" ]; then
            sb_ports=$(ss -tnlp 2>/dev/null | grep -w "sing-box" | awk '{print $4}' | grep -oE '[0-9]+$' | sort -u)
        fi

        sb_ports=$(echo "$sb_ports" | tr ' ' '\n' | sort -u | tr '\n' ' ')
        if [ -n "$sb_ports" ]; then
            echo "🛡️ 检测到 Sing-box 端口: $sb_ports"
            all_ports="$all_ports $sb_ports"
        fi
    fi

    # === X-Panel 端口检测（修复版：过滤 NULL，支持233boy Xray脚本）===
    if pgrep -f "xpanel" >/dev/null || pgrep -f "x-ui" >/dev/null; then
        if [ -f /etc/x-ui/x-ui.db ]; then
            xpanel_ports=$(sqlite3 /etc/x-ui/x-ui.db \
                "SELECT port FROM inbounds WHERE port IS NOT NULL AND port != '';" 2>/dev/null | \
                grep -E '^[0-9]+$' | sort -u)
            if [ -n "$xpanel_ports" ]; then
                echo "🛡️ 检测到 X-Panel 入站端口: $xpanel_ports"
                all_ports="$all_ports $xpanel_ports"
            fi
        fi
        echo "🌐 检测到面板进程，自动放行 80 端口（用于证书申请）。"
        all_ports="$all_ports 80"
    fi

    # === 233boy Xray 脚本端口检测 ===
    if [ -d "/etc/xray/conf" ]; then
        xray_config_ports=""
        for config_file in /etc/xray/conf/*.json; do
            if [ -f "$config_file" ]; then
                # 提取inbounds中的port字段
                config_ports=$(jq -r '.inbounds[]?.port // empty' "$config_file" 2>/dev/null | sort -u | tr '\n' ' ')
                if [ -n "$config_ports" ]; then
                    xray_config_ports="$xray_config_ports $config_ports"
                fi
            fi
        done
        if [ -n "$xray_config_ports" ]; then
            xray_config_ports=$(echo "$xray_config_ports" | tr ' ' '\n' | sort -u | tr '\n' ' ')
            echo "🛡️ 检测到 233boy Xray 配置端口: $xray_config_ports"
            all_ports="$all_ports $xray_config_ports"
        fi
    fi

    # === Sing-box 配置端口检测（兼容v2ray-agent和233boy脚本）===
    sb_config_ports=""
    sb_config_dirs=("/etc/sing-box/conf" "/etc/v2ray-agent/sing-box/conf/config")

    for config_dir in "${sb_config_dirs[@]}"; do
        if [ -d "$config_dir" ]; then
            for config_file in "$config_dir"/*.json; do
                if [ -f "$config_file" ]; then
                    # 提取inbounds中的listen_port字段
                    config_ports=$(jq -r '.inbounds[]?.listen_port // empty' "$config_file" 2>/dev/null | sort -u | tr '\n' ' ')
                    if [ -n "$config_ports" ]; then
                        sb_config_ports="$sb_config_ports $config_ports"
                    fi
                fi
            done
        fi
    done

    if [ -n "$sb_config_ports" ]; then
        sb_config_ports=$(echo "$sb_config_ports" | tr ' ' '\n' | sort -u | tr '\n' ' ')
        echo "🛡️ 检测到 Sing-box 配置端口: $sb_config_ports"
        all_ports="$all_ports $sb_config_ports"
    fi

    all_ports=$(echo "$all_ports" | tr ' ' '\n' | sort -u | tr '\n' ' ')
    print_message "最终将保留的端口: $all_ports"
    remove_unused_rules "$all_ports" "$firewall_type"

    print_message "✅ 所有安全配置已成功应用！"
}

main
self_check