#!/bin/bash
# -----------------------------------------------------------------------------------------
# VPS 代理服务端口检测和防火墙配置脚本（终极安全交互版 v2.0）
#
# 功能：
# - 如果防火墙未启用，则自动安装并配置UFW或Firewalld
# - 自动检测 Xray 和 Sing-box 的开放端口
# - 自动检测 SSH 端口并强制保留
# - 主动移除防火墙中所有其他未知端口，实现安全锁定
# - 支持交互式输入Telegram配置，兼容所有执行方式
# - 修复所有已知 bug 和兼容性问题
# -----------------------------------------------------------------------------------------

set -e

# --- 变量初始化 ---
TG_TOKEN=""
TG_CHAT_ID=""
NOTIFY=false # 默认不通知，除非用户提供了配置

# --- 函数定义 ---
print_message() {
    echo ""
    echo "------------------------------------------------------------"
    echo "$1"
    echo "------------------------------------------------------------"
}

send_telegram() {
    if [ "$NOTIFY" = true ] && [ -n "$TG_TOKEN" ] && [ -n "$TG_CHAT_ID" ]; then
        local message="$1"
        # 使用 curl 发送消息，增加超时和重试
        curl --connect-timeout 10 --retry 3 -s -X POST "https://api.telegram.org/bot$TG_TOKEN/sendMessage" \
            -d chat_id="$TG_CHAT_ID" \
            -d text="$message" \
            -d parse_mode="Markdown" > /dev/null
    fi
}

get_timezone() {
    local tz
    tz=$(timedatectl | grep "Time zone" | awk '{print $3}') 2>/dev/null || tz=$(cat /etc/timezone) 2>/dev/null || tz="Etc/UTC"
    echo "$tz"
}

get_process_ports() {
    local process_name="$1"
    local ports=""
    if pgrep -f "$process_name" > /dev/null; then
        if command -v ss &> /dev/null; then
            ports=$(ss -tlnp | grep "$process_name" | awk '{print $4}' | awk -F: '{print $NF}' | sort -u | tr '\n' ' ')
        elif command -v netstat &> /dev/null; then
            ports=$(netstat -tlnp | grep "$process_name" | awk '{print $4}' | awk -F: '{print $NF}' | sort -u | tr '\n' ' ')
        fi
    fi
    echo "$ports"
}

parse_config_ports() {
    local config_file="$1"
    local ports=""
    if [ -f "$config_file" ]; then
        echo "📄 解析配置文件: $config_file" >&2
        if command -v jq &> /dev/null; then
            ports=$(jq -r '.inbounds[]?.listen_port // .inbounds[]?.port // empty' "$config_file" 2>/dev/null | grep -E '^[0-9]+$' | sort -u | tr '\n' ' ')
        fi
        if [ -z "$ports" ]; then
            echo "⚠️ jq 不可用，使用备用解析方法" >&2
            local found_ports
            found_ports=$(grep -o '"listen_port":[[:space:]]*[0-9]\+' "$config_file" | grep -o '[0-9]\+')
            if [ -z "$found_ports" ]; then
                found_ports=$(grep -o '"port":[[:space:]]*[0-9]\+' "$config_file" | grep -o '[0-9]\+')
            fi
            ports=$(echo "$found_ports" | sort -u | tr '\n' ' ')
        fi
        if [ -n "$ports" ]; then
            echo "📋 从配置文件读取到端口: $ports" >&2
        fi
    fi
    echo "$ports"
}

detect_firewall() {
    if systemctl is-active --quiet firewalld; then
        echo "firewalld"
    elif command -v ufw &> /dev/null && ufw status | grep -q "Status: active"; then
        echo "ufw"
    else
        echo "none"
    fi
}

setup_firewall() {
    print_message "未检测到活跃防火墙，将自动安装并配置"
    
    if [ -f /etc/os-release ]; then
        . /etc/os-release
        if [[ "$ID" == "ubuntu" || "$ID" == "debian" || "$ID_LIKE" == "debian" ]]; then
            echo "ℹ️ 检测到 Debian/Ubuntu 系统，将安装 UFW..."
            sudo apt-get update >/dev/null
            sudo apt-get install -y ufw >/dev/null
            echo "y" | sudo ufw reset >/dev/null
            sudo ufw default deny incoming >/dev/null
            sudo ufw default allow outgoing >/dev/null
            sudo ufw enable >/dev/null
            echo "✅ UFW 安装并启用成功。"
            echo "ufw"
        elif [[ "$ID" == "centos" || "$ID" == "rhel" || "$ID" == "fedora" || "$ID" == "almalinux" || "$ID_LIKE" == "rhel" ]]; then
            echo "ℹ️ 检测到 RHEL/CentOS 系列系统，将安装 firewalld..."
            if command -v dnf &>/dev/null; then
                sudo dnf install -y firewalld >/dev/null
            else
                sudo yum install -y firewalld >/dev/null
            fi
            sudo systemctl enable --now firewalld >/dev/null
            echo "✅ firewalld 安装并启用成功。"
            echo "firewalld"
        else
            echo "❌ 不支持的操作系统: $ID。请手动安装防火墙。" >&2; echo "none"
        fi
    else
        echo "❌ 无法确定操作系统类型。请手动安装防火墙。" >&2; echo "none"
    fi
}

add_firewall_rule() {
    local port="$1"
    local protocol="$2"
    local firewall_type="$3"
    case "$firewall_type" in
        firewalld)
            set +e
            if ! sudo firewall-cmd --permanent --query-port="$port/$protocol" > /dev/null 2>&1; then
                sudo firewall-cmd --permanent --add-port="$port/$protocol" > /dev/null 2>&1
                FIREWALL_CHANGED=true
            fi
            set -e
            ;;
        ufw)
            # UFW rules will be handled in the remove_unused_rules function
            ;;
    esac
}

remove_unused_rules() {
    local ports_to_keep="$1"
    local firewall_type="$2"
    print_message "开始清理防火墙中未使用的端口"
    case "$firewall_type" in
        firewalld)
            echo "ℹ️ 正在检查 firewalld 永久规则..."
            FIREWALL_CHANGED=false
            local current_services
            current_services=$(sudo firewall-cmd --permanent --list-services)
            local current_ports
            current_ports=$(sudo firewall-cmd --permanent --list-ports)
            
            for service in $current_services; do
                if [[ "$service" != "ssh" && "$service" != "dhcpv6-client" ]]; then
                    echo "➖ 正在移除服务: $service"
                    sudo firewall-cmd --permanent --remove-service="$service" > /dev/null 2>&1
                    FIREWALL_CHANGED=true
                fi
            done
            
            for port_rule in $current_ports; do
                local port_num
                port_num=$(echo "$port_rule" | cut -d'/' -f1)
                if ! echo " $ports_to_keep " | grep -q " $port_num "; then
                    echo "➖ 正在移除端口规则: $port_rule"
                    sudo firewall-cmd --permanent --remove-port="$port_rule" > /dev/null 2>&1
                    FIREWALL_CHANGED=true
                fi
            done
            
            if [ "$FIREWALL_CHANGED" = true ]; then
                echo "🔄 正在重载防火墙以应用更改..."
                sudo firewall-cmd --reload > /dev/null 2>&1
            else
                echo "✅ 无需清理，所有规则均为必需规则。"
            fi
            ;;
        ufw)
            echo "⚠️ UFW 将被重置，仅保留必需端口！"
            echo "   操作将在 5 秒后继续，按 Ctrl+C 取消。"
            sleep 5
            echo "🔄 正在重置 UFW..."
            echo "y" | sudo ufw reset > /dev/null 2>&1
            sudo ufw default deny incoming > /dev/null 2>&1
            sudo ufw default allow outgoing > /dev/null 2>&1
            
            echo "➕ 正在重新应用必要的规则..."
            for port in $ports_to_keep; do
                sudo ufw allow "$port" > /dev/null 2>&1
                echo "   允许端口: $port"
            done
            
            sudo ufw enable > /dev/null 2>&1
            echo "✅ UFW 已重置并配置完毕。"
            sudo ufw status
            ;;
    esac
}

main() {
    print_message "步骤 1: Telegram 通知配置 (可选)"
    read -p "是否要配置 Telegram 通知? [y/N]: " setup_notify
    if [[ "$setup_notify" =~ ^[Yy]$ ]]; then
        read -p "请输入你的 Telegram Bot Token: " input_token
        read -p "请输入你的 Telegram Chat ID: " input_chat_id
        
        if [ -n "$input_token" ] && [ -n "$input_chat_id" ]; then
            TG_TOKEN="$input_token"
            TG_CHAT_ID="$input_chat_id"
            NOTIFY=true
            echo "✅ Telegram 通知已配置。"
        else
            echo "⚠️ 输入不完整，将禁用 Telegram 通知。"
        fi
    else
        echo "ℹ️ 已跳过 Telegram 通知配置。"
    fi
    
    print_message "步骤 2: 开始一键式防火墙安全配置"
    
    local firewall_type
    firewall_type=$(detect_firewall)
    
    if [ "$firewall_type" = "none" ]; then
        firewall_type=$(setup_firewall)
        if [ "$firewall_type" = "none" ]; then
            exit 1
        fi
    fi

    echo "🔍 检测防火墙类型: $firewall_type"
    local timezone
    timezone=$(get_timezone)
    local time_now
    time_now=$(date '+%Y-%m-%d %H:%M:%S')
    echo "🕒 系统时区: $timezone"
    echo "🕐 当前时间: $time_now"

    local ssh_port
    ssh_port=$(grep -i '^Port ' /etc/ssh/sshd_config | awk '{print $2}' | head -n1)
    [ -z "$ssh_port" ] && ssh_port=22
    echo "🛡️ 检测到 SSH 端口为: $ssh_port (此端口将被强制保留)"

    local xray_ports=""
    local sb_ports=""
    local all_ports=""
    
    # 检测 Xray 端口
    if command -v xray &> /dev/null && pgrep -f "xray" > /dev/null; then
        xray_ports=$(get_process_ports "xray")
        if [ -n "$xray_ports" ]; then
            echo "✅ 检测到 Xray 运行端口: $xray_ports"
            all_ports="$all_ports $xray_ports"
        fi
    fi
    
    # 检测 Sing-box 端口
    if command -v sb &> /dev/null || command -v sing-box &> /dev/null; then
        if pgrep -f "sing-box" > /dev/null; then
            sb_ports=$(get_process_ports "sing-box")
            if [ -z "$sb_ports" ]; then
                local config_files=("/etc/sing-box/config.json" "/usr/local/etc/sing-box/config.json" "/etc/sing-box/conf/"*.json)
                local temp_sb_ports=""
                for config_file in "${config_files[@]}"; do
                    if [ -f "$config_file" ]; then
                        local config_ports
                        config_ports=$(parse_config_ports "$config_file")
                        if [ -n "$config_ports" ]; then
                            temp_sb_ports="$temp_sb_ports $config_ports"
                        fi
                    fi
                done
                sb_ports=$(echo "$temp_sb_ports" | tr ' ' '\n' | sort -u | tr '\n' ' ')
            fi
            if [ -n "$sb_ports" ]; then
                echo "✅ 检测到 Sing-box 运行端口: $sb_ports"
                all_ports="$all_ports $sb_ports"
            fi
        fi
    fi

    local ports_to_keep
    ports_to_keep=$(echo "$all_ports $ssh_port" | tr ' ' '\n' | sort -u | tr '\n' ' ')
    if [ -z "$(echo "$ports_to_keep" | xargs)" ]; 键，然后
        echo "ℹ️ 未检测到任何需要保留的端口，跳过防火墙配置。"
        exit 0
    fi
    
    echo "ℹ️ 将要确保以下端口开启: $ports_to_keep"
    
    if [ "$firewall_type" != "ufw" ]; then
        for port in $ports_to_keep; do
            add_firewall_rule "$port" "tcp" "$firewall_type"
            add_firewall_rule "$port" "udp" "$firewall_type"
        done
    fi
    
    remove_unused_rules "$ports_to_keep" "$firewall_type"

    local message="🔒 *防火墙安全锁定完成*
> *服务器*: \`$(hostname)\`
> *保留端口*: \`$ports_to_keep\`
> *防火墙类型*: \`$firewall_type\`"
    send_telegram "$message"
    print_message "防火墙配置完成，仅允许必需端口的流量"
}

main
