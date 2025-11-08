#!/bin/bash
# -----------------------------------------------------------------------------------------
# VPS 代理服务端口检测与防火墙配置脚本（V3.8.2 完整修复版）
# 兼容 xeefei X-Panel / X-UI / Xray / Sing-box
#
# 更新日志:
# V3.8.2 - [完整修复版]
#   修复 Fail2Ban 日志路径：自动检测系统类型
#   修复偏执模式 bantime：12小时而非1小时
#   删除重复的 Sing-box 端口检测逻辑
#   增强端口去重：过滤非数字和空值
#   添加防火墙重置警告提示
#   改进配置文件遍历：使用 nullglob
#   增强错误处理和用户提示
# -----------------------------------------------------------------------------------------

set -e
shopt -s nullglob
start_time=$(date +%s)

if [ "$(id -u)" -ne 0 ]; then
    echo "请以 root 权限运行本脚本。"
    exit 1
fi

FAIL2BAN_MODE="未选择"

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
            -d chat_id="$TG_CHAT_ID" -d text="$message" -d parse_mode="MarkdownV2" >/dev/null 2>&1
    fi
}

install_dependencies() {
    local pkg_manager=""
    local update_cmd=""
    local install_cmd=""

    if [ -f /etc/debian_version ]; then
        pkg_manager="apt"
        update_cmd="apt-get update -y"
        install_cmd="apt-get install -y"
    elif [ -f /etc/redhat-release ]; then
        if command -v dnf &>/dev/null; then
            pkg_manager="dnf"
            install_cmd="dnf install -y"
        else
            pkg_manager="yum"
            install_cmd="yum install -y"
        fi
    else
        echo "无法识别系统类型，跳过依赖安装"
        return
    fi

    if ! command -v sqlite3 &>/dev/null; then
        echo "未检测到 sqlite3，正在安装..."
        [ -n "$update_cmd" ] && $update_cmd >/dev/null 2>&1
        $install_cmd sqlite3 >/dev/null 2>&1 || $install_cmd sqlite >/dev/null 2>&1
        if command -v sqlite3 &>/dev/null; then
            echo "sqlite3 安装完成。"
        else
            echo "sqlite3 安装失败，数据库功能可能不可用"
        fi
    fi

    if ! command -v jq &>/dev/null; then
        echo "未检测到 jq，正在安装..."
        [ -n "$update_cmd" ] && $update_cmd >/dev/null 2>&1
        $install_cmd jq >/dev/null 2>&1
        if command -v jq &>/dev/null; then
            echo "jq 安装完成。"
        else
            echo "jq 安装失败，JSON配置解析功能可能不可用"
        fi
    fi
}

install_dependencies

detect_firewall() {
    if systemctl is-active --quiet firewalld 2>/dev/null; then
        echo "firewalld"
    elif command -v ufw &>/dev/null && ufw status 2>/dev/null | grep -qE "^Status:\s+active"; then
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
            echo "警告：即将重置防火墙规则，可能导致短暂连接中断..."
            sleep 2
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

detect_logpath() {
    if [ -f /etc/debian_version ]; then
        echo "/var/log/auth.log"
    elif [ -f /etc/redhat-release ]; then
        echo "/var/log/secure"
    else
        if [ -f /var/log/auth.log ]; then
            echo "/var/log/auth.log"
        elif [ -f /var/log/secure ]; then
            echo "/var/log/secure"
        else
            echo "/var/log/auth.log"
        fi
    fi
}

setup_fail2ban() {
    local firewall_type="$1"
    print_message "配置 Fail2Ban (SSH 防护)"

    if ! command -v fail2ban-client &>/dev/null; then
        echo "正在安装 Fail2Ban..."
        apt-get install -y fail2ban >/dev/null 2>&1 || yum install -y fail2ban >/dev/null 2>&1 || dnf install -y fail2ban >/dev/null 2>&1
        if command -v fail2ban-client &>/dev/null; then
            echo "Fail2Ban 安装完成。"
        else
            echo "Fail2Ban 安装失败，请手动安装"
            return 1
        fi
    fi

    rm -f /etc/fail2ban/filter.d/sshd-ddos.conf
    local banaction=$(detect_banaction "$firewall_type")
    local logpath=$(detect_logpath)
    echo "Fail2Ban 将使用动作: $banaction"
    echo "Fail2Ban 将监控日志: $logpath"

    echo "请选择 Fail2Ban SSH 防护模式:"
    echo "  1) 普通模式: 5次失败封禁10分钟"
    echo "  2) 激进模式: 推荐！3次失败封禁1小时，屡教不改翻倍"
    echo "  3) 偏执模式: 2次失败封禁12小时，屡教不改×3"
    read -p "请输入选项 [1-3], 默认 2: " mode
    mode=${mode:-2}

    case $mode in
    1) FAIL2BAN_MODE="普通 (Normal)"; bantime="10m"; maxretry="5"; findtime="10m" ;;
    2) FAIL2BAN_MODE="激进 (Aggressive)"; bantime="1h"; maxretry="3"; findtime="10m" ;;
    3) FAIL2BAN_MODE="偏执 (Paranoid)"; bantime="12h"; maxretry="2"; findtime="10m" ;;
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
logpath = ${logpath}
bantime.increment = true
bantime.factor = 2
bantime.max = 1w
EOF

    systemctl enable --now fail2ban >/dev/null 2>&1
    systemctl restart fail2ban
    echo "Fail2Ban 已配置为 [$FAIL2BAN_MODE] 并启动。"
}

remove_unused_rules() {
    local ports_to_keep="$1"
    local firewall="$2"
    print_message "清理并应用新的防火墙规则"
    
    local ports_array=($(echo "$ports_to_keep" | tr ' ' '\n' | grep -E '^[0-9]+$' | sort -un))
    
    if [ ${#ports_array[@]} -eq 0 ]; then
        echo "警告：未检测到任何有效端口，跳过防火墙配置"
        return
    fi
    
    echo "即将配置的端口: ${ports_array[*]}"

    if [ "$firewall" = "ufw" ]; then
        echo "警告：即将重置防火墙规则，可能导致短暂连接中断..."
        sleep 2
        echo "y" | ufw reset >/dev/null 2>&1
        ufw default deny incoming >/dev/null 2>&1
        ufw default allow outgoing >/dev/null 2>&1
        for p in "${ports_array[@]}"; do 
            ufw allow "$p" >/dev/null 2>&1
        done
        ufw --force enable >/dev/null 2>&1
        echo "UFW 规则已更新。"
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
        echo "firewalld 规则已更新。"
    else
        echo "未找到有效防火墙工具。"
    fi
}

self_check() {
    print_message "正在进行配置自检（增强版）..."
    sleep 5
    local all_ok=true
    local issues=()

    if systemctl is-active --quiet fail2ban 2>/dev/null; then
        echo "Fail2Ban 服务正在运行。"
    else
        echo "Fail2Ban 未运行！"
        issues+=("Fail2Ban服务未运行")
        all_ok=false
    fi

    if ! fail2ban-client status sshd >/dev/null 2>&1; then
        echo "SSH Jail 初次检测未加载，等待 5 秒后重试..."
        sleep 5
        systemctl reload fail2ban >/dev/null 2>&1
        if fail2ban-client status sshd >/dev/null 2>&1; then
            echo "SSH Jail 已在重试后加载成功。"
        else
            echo "SSH Jail 加载失败，请检查配置。"
            issues+=("SSH-Jail未加载")
            all_ok=false
        fi
    else
        echo "SSH Jail 已正确加载。"
    fi

    local fw
    fw=$(detect_firewall)
    if [ "$fw" = "ufw" ]; then
        if ufw status 2>/dev/null | grep -qE "^Status:\s+active"; then
            echo "UFW 已启用。"
        else
            echo "UFW 未启用。"
            issues+=("UFW未激活")
            all_ok=false
        fi
    elif [ "$fw" = "firewalld" ]; then
        if firewall-cmd --state 2>/dev/null | grep -qE "^running$"; then
            echo "Firewalld 已启用。"
        else
            echo "Firewalld 未启用。"
            issues+=("Firewalld未运行")
            all_ok=false
        fi
    else
        echo "防火墙未启用。"
        issues+=("无防火墙")
        all_ok=false
    fi

    local ssh_port
    ssh_port=$(grep -iE '^\s*Port\s+[0-9]+' /etc/ssh/sshd_config 2>/dev/null | \
               grep -v '^\s*#' | \
               awk '{print $2}' | \
               grep -E '^[0-9]+$' | \
               head -n1)
    [ -z "$ssh_port" ] && ssh_port=22
    
    if ss -tln 2>/dev/null | grep -qE "[^0-9]${ssh_port}(\s|$)"; then
        echo "SSH 端口 $ssh_port 监听正常。"
    else
        echo "SSH 端口 $ssh_port 未监听！"
        issues+=("SSH端口${ssh_port}未监听")
        all_ok=false
    fi

    if [ "$fw" = "ufw" ]; then
        if ! ufw status 2>/dev/null | grep -qE "^${ssh_port}(/tcp)?\s+(ALLOW|allow)"; then
            echo "SSH 端口 $ssh_port 未在 UFW 规则中！"
            issues+=("SSH端口未放行")
            all_ok=false
        fi
    elif [ "$fw" = "firewalld" ]; then
        if ! firewall-cmd --list-ports 2>/dev/null | grep -qE "${ssh_port}/(tcp|udp)"; then
            echo "SSH 端口 $ssh_port 未在 Firewalld 规则中！"
            issues+=("SSH端口未放行")
            all_ok=false
        fi
    fi

    echo "------------------------------------------------------------"
    local hostname=$(hostname)
    local duration=$(( $(date +%s) - start_time ))

    if [ "$all_ok" = true ]; then
        echo "自检通过：所有关键安全配置均正常工作。"
        result="自检通过"
        issue_summary=""
    else
        echo "自检发现问题，请手动检查。"
        result="发现问题"
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

main() {
    local firewall_type
    firewall_type=$(detect_firewall)
    [ "$firewall_type" = "none" ] && firewall_type=$(setup_firewall)

    setup_fail2ban "$firewall_type"

    local ssh_port
    ssh_port=$(grep -iE '^\s*Port\s+[0-9]+' /etc/ssh/sshd_config 2>/dev/null | \
               grep -v '^\s*#' | \
               awk '{print $2}' | \
               grep -E '^[0-9]+$' | \
               head -n1)
    [ -z "$ssh_port" ] && ssh_port=22
    echo "检测到 SSH 端口: $ssh_port"

    local all_ports="$ssh_port"

    if command -v xray &>/dev/null && pgrep -x "xray" &>/dev/null; then
        xray_ports=$(ss -tnlp 2>/dev/null | grep -w xray | awk '{print $4}' | grep -oE '[0-9]+$' | sort -u | tr '\n' ' ')
        if [ -n "$xray_ports" ]; then
            echo "检测到 Xray 端口: $xray_ports"
            all_ports="$all_ports $xray_ports"
        fi
    fi

    if [ -d "/etc/xray/conf" ]; then
        xray_config_ports=""
        for config_file in /etc/xray/conf/*.json; do
            [ -f "$config_file" ] || continue
            if command -v jq &>/dev/null; then
                config_ports=$(jq -r '.inbounds[]?.port // empty' "$config_file" 2>/dev/null | grep -E '^[0-9]+$' | sort -u | tr '\n' ' ')
                if [ -n "$config_ports" ]; then
                    xray_config_ports="$xray_config_ports $config_ports"
                fi
            fi
        done
        if [ -n "$xray_config_ports" ]; then
            xray_config_ports=$(echo "$xray_config_ports" | tr ' ' '\n' | grep -E '^[0-9]+$' | sort -u | tr '\n' ' ')
            echo "检测到 233boy Xray 配置端口: $xray_config_ports"
            all_ports="$all_ports $xray_config_ports"
        fi
    fi

    if pgrep -x "sing-box" &>/dev/null; then
        sb_ports=""
        
        if command -v jq &>/dev/null && [ -d "/etc/sing-box/conf" ]; then
            for config_file in /etc/sing-box/conf/*.json; do
                [ -f "$config_file" ] || continue
                config_ports=$(jq -r '.inbounds[]?.listen_port // empty' "$config_file" 2>/dev/null | grep -E '^[0-9]+$' | sort -u | tr '\n' ' ')
                if [ -n "$config_ports" ]; then
                    sb_ports="$sb_ports $config_ports"
                fi
            done
        fi
        
        if [ -z "$sb_ports" ]; then
            sb_ports=$(ss -tnlp 2>/dev/null | grep -w "sing-box" | awk '{print $4}' | grep -oE '[0-9]+$' | sort -u | tr '\n' ' ')
        fi
        
        sb_ports=$(echo "$sb_ports" | tr ' ' '\n' | grep -E '^[0-9]+$' | sort -u | tr '\n' ' ')
        if [ -n "$sb_ports" ]; then
            echo "检测到 Sing-box 端口: $sb_ports"
            all_ports="$all_ports $sb_ports"
        fi
    fi

    if pgrep -f "xpanel" >/dev/null || pgrep -f "x-ui" >/dev/null; then
        if [ -f /etc/x-ui/x-ui.db ] && command -v sqlite3 &>/dev/null; then
            xpanel_ports=$(sqlite3 /etc/x-ui/x-ui.db \
                "SELECT port FROM inbounds WHERE port IS NOT NULL AND port != '';" 2>/dev/null | \
                grep -E '^[0-9]+$' | sort -u | tr '\n' ' ')
            if [ -n "$xpanel_ports" ]; then
                echo "检测到 X-Panel 入站端口: $xpanel_ports"
                all_ports="$all_ports $xpanel_ports"
            fi
        fi
        echo "检测到面板进程，自动放行 80 端口（用于证书申请）。"
        all_ports="$all_ports 80"
    fi

    all_ports=$(echo "$all_ports" | tr ' ' '\n' | grep -E '^[0-9]+$' | sort -un | tr '\n' ' ' | sed 's/ $//')
    
    if [ -z "$all_ports" ]; then
        echo "错误：未检测到任何有效端口！"
        exit 1
    fi
    
    print_message "最终将保留的端口: $all_ports"
    remove_unused_rules "$all_ports" "$firewall_type"

    print_message "所有安全配置已成功应用！"
}

main
self_check