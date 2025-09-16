#!/bin/bash
# -----------------------------------------------------------------------------------------
# VPS 代理服务端口检测和防火墙配置脚本（最终安全锁定版）
#
# 功能：
# - 自动检测 Xray 和 Sing-box 的开放端口
# - 自动检测 SSH 端口并强制保留
# - 配置防火墙仅允许代理和 SSH 端口的流量
# - 主动移除防火墙中所有其他未知端口，实现安全锁定
# - 修复所有已知 bug 和兼容性问题
# - 支持 Telegram 通知
# -----------------------------------------------------------------------------------------

set -e

# --- 配置变量 ---
TG_TOKEN=""
TG_CHAT_ID=""
NOTIFY=true

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
        curl --connect-timeout 10 --retry 3 -s -X POST "https://api.telegram.org/bot$TG_TOKEN/sendMessage" \
            -d chat_id="$TG_CHAT_ID" \
            -d text="$message" \
            -d parse_mode="Markdown" > /dev/null
    fi
}

# 获取时区
get_timezone() {
    local tz
    if command -v timedatectl &> /dev/null; then
        tz=$(timedatectl | grep "Time zone" | awk '{print $3}')
    fi
    if [ -z "$tz" ] && [ -f /etc/timezone ]; then
        tz=$(cat /etc/timezone)
    fi
    if [ -z "$tz" ]; then
        tz="Etc/UTC"
    fi
    echo "$tz"
}

# 检测进程监听端口
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

# 从配置文件解析端口
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
            # 【修复】使用兼容性最强的 grep 方式，避免 "lookbehind assertion" 错误
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

# 检测防火墙类型
detect_firewall() {
    if systemctl is-active --quiet firewalld; then
        echo "firewalld"
    elif command -v ufw &> /dev/null && ufw status | grep -q "Status: active"; then
        echo "ufw"
    else
        echo "none"
    fi
}

# 添加防火墙规则
add_firewall_rule() {
    local port="$1"
    local protocol="$2"
    local firewall_type="$3"

    case "$firewall_type" in
        firewalld)
            set +e
            if ! sudo firewall-cmd --permanent --query-port="$port/$protocol" > /dev/null 2>&1; then
                sudo firewall-cmd --permanent --add-port="$port/$protocol" > /dev/null 2>&1
                # 标记有更改发生，以便最后统一重载
                FIREWALL_CHANGED=true
            fi
            set -e
            ;;
        ufw)
            # 对于 UFW，在清理阶段统一处理
            :
            ;;
    esac
}

# 移除未使用的防火墙规则
remove_unused_rules() {
    local ports_to_keep="$1"
    local firewall_type="$2"
    
    print_message "开始清理防火墙中未使用的端口"

    case "$firewall_type" in
        firewalld)
            echo "ℹ️ 正在检查 firewalld 永久规则..."
            local current_services
            local current_ports
            current_services=$(sudo firewall-cmd --permanent --list-services)
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
            echo "⚠️ UFW 将被重置，仅保留代理和SSH端口！"
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

# 主函数
main() {
    print_message "开始检测代理服务端口并配置防火墙"

    local timezone
    local time_now
    local firewall_type
    timezone=$(get_timezone)
    time_now=$(date '+%Y-%m-%d %H:%M:%S')
    firewall_type=$(detect_firewall)
    FIREWALL_CHANGED=false # 用于标记防火墙是否有变动

    echo "🔍 检测防火墙类型: $firewall_type"
    echo "🕒 系统时区: $timezone"
    echo "🕐 当前时间: $time_now"

    local ssh_port
    ssh_port=$(grep -i '^Port ' /etc/ssh/sshd_config | awk '{print $2}' | head -n1)
    [ -z "$ssh_port" ] && ssh_port=22
    echo "🛡️ 检测到 SSH 端口为: $ssh_port (此端口将被强制保留)"

    local xray_ports=""
    local sb_ports=""
    local all_ports=""
    
    if command -v xray &> /dev/null && pgrep -f "xray" > /dev/null; then
        xray_ports=$(get_process_ports "xray")
        [ -n "$xray_ports" ] && echo "✅ 检测到 Xray 运行端口: $xray_ports" && all_ports="$all_ports $xray_ports"
    fi

    if command -v sb &> /dev/null || command -v sing-box &> /dev/null; then
        if pgrep -f "sing-box" > /dev/null; then
            sb_ports=$(get_process_ports "sing-box")
            if [ -z "$sb_ports" ]; then
                local config_files=("/etc/sing-box/config.json" "/usr/local/etc/sing-box/config.json" "/opt/sing-box/config.json" /etc/sing-box/conf/*.json)
                local temp_sb_ports=""
                for config_file in "${config_files[@]}"; do
                    if [ -f "$config_file" ]; then
                        config_ports=$(parse_config_ports "$config_file")
                        [ -n "$config_ports" ] && temp_sb_ports="$temp_sb_ports $config_ports"
                    fi
                done
                sb_ports=$(echo "$temp_sb_ports" | tr ' ' '\n' | sort -u | tr '\n' ' ')
            fi
            [ -n "$sb_ports" ] && echo "✅ 检测到 Sing-box 运行端口:$sb_ports" && all_ports="$all_ports $sb_ports"
        fi
    fi

    local ports_to_keep
    ports_to_keep=$(echo "$all_ports $ssh_port" | tr ' ' '\n' | sort -u | tr '\n' ' ')

    if [ -z "$ports_to_keep" ]; then
        echo "ℹ️ 未检测到任何需要保留的端口，跳过防火墙配置。"
        exit 0
    fi
    
    echo "ℹ️ 将要确保以下端口开启:$ports_to_keep"
    
    if [ "$firewall_type" != "ufw" ]; then
        for port in $ports_to_keep; do
            add_firewall_rule "$port" "tcp" "$firewall_type"
            add_firewall_rule "$port" "udp" "$firewall_type"
        done
    fi
    
    remove_unused_rules "$ports_to_keep" "$firewall_type"

    local message="🔒 *防火墙安全锁定完成*
> *保留端口*: \`$ports_to_keep\`
> *防火墙类型*: \`$firewall_type\`"
    send_telegram "$message"
    print_message "防火墙配置完成，仅允许必需端口的流量"
}

# 参数处理
while [[ $# -gt 0 ]]; do
    case $1 in
        --no-notify) NOTIFY=false; shift ;;
        --token) TG_TOKEN="$2"; shift 2 ;;
        --chat-id) TG_CHAT_ID="$2"; shift 2 ;;
        *)
            echo "用法: $0 [--no-notify] [--token TOKEN] [--chat-id CHAT_ID]" >&2
            exit 1
            ;;
    esac
done

main