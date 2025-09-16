#!/bin/bash
# -----------------------------------------------------------------------------------------
# VPS 代理服务端口检测和防火墙配置脚本（安全锁定版）
#
# 功能：
# - 自动检测 Xray 和 Sing-box (sb) 的开放端口
# - 自动检测 SSH 端口并加入白名单
# - 配置防火墙允许代理和 SSH 端口的流量
# - 【新】移除防火墙中所有其他未被使用的端口，实现安全锁定
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
            ports=$(grep -oP '(?<="listen_port":\s*)\d+' "$config_file" | sort -u | tr '\n' ' ')
            if [ -z "$ports" ]; then
                ports=$(grep -oP '(?<="port":\s*)\d+' "$config_file" | sort -u | tr '\n' ' ')
            fi
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
                sudo firewall-cmd --reload > /dev/null 2>&1
            fi
            set -e
            ;;
        ufw)
            sudo ufw allow "$port/$protocol" > /dev/null 2>&1
            ;;
    esac
}

# 【新功能】移除未使用的防火墙规则
remove_unused_rules() {
    local ports_to_keep="$1"
    local firewall_type="$2"
    
    print_message "开始清理防火墙中未使用的端口"

    case "$firewall_type" in
        firewalld)
            echo "ℹ️ 正在检查 firewalld 永久规则..."
            local changes_made=false
            # 获取当前永久规则中的服务和端口
            local current_services=$(sudo firewall-cmd --permanent --list-services)
            local current_ports=$(sudo firewall-cmd --permanent --list-ports)

            # 清理服务 (只保留 ssh 和 dhcpv6-client)
            for service in $current_services; do
                if [[ "$service" != "ssh" && "$service" != "dhcpv6-client" ]]; then
                    echo "➖ 正在移除服务: $service"
                    sudo firewall-cmd --permanent --remove-service="$service" > /dev/null 2>&1
                    changes_made=true
                fi
            done

            # 清理端口
            for port_rule in $current_ports; do
                local port_num=$(echo "$port_rule" | cut -d'/' -f1)
                # 检查当前端口是否在需要保留的列表中
                if ! echo " $ports_to_keep " | grep -q " $port_num "; then
                    echo "➖ 正在移除端口规则: $port_rule"
                    sudo firewall-cmd --permanent --remove-port="$port_rule" > /dev/null 2>&1
                    changes_made=true
                fi
            done

            if [ "$changes_made" = true ]; then
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
        none)
            echo "⚠️ 未检测到活跃的防火墙，跳过清理操作。"
            ;;
    esac
}


# 主函数
main() {
    print_message "开始检测代理服务端口并配置防火墙"

    local timezone=$(get_timezone)
    local time_now=$(date '+%Y-%m-%d %H:%M:%S')
    local firewall_type=$(detect_firewall)

    echo "🔍 检测防火墙类型: $firewall_type"
    echo "🕒 系统时区: $timezone"
    echo "🕐 当前时间: $time_now"

    # 【新】自动检测SSH端口
    local ssh_port=$(grep -i '^Port ' /etc/ssh/sshd_config | awk '{print $2}' | head -n1)
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
            if [ -n "$sb_ports" ]; then
                echo "✅ 检测到 Sing-box 运行端口:$sb_ports"
                all_ports="$all_ports $sb_ports"
            fi
        fi
    fi

    # 统一处理所有需要保留的端口
    local ports_to_keep=$(echo "$all_ports $ssh_port" | tr ' ' '\n' | sort -u | tr '\n' ' ')

    if [ -n "$ports_to_keep" ]; then
        echo "ℹ️ 将要确保以下端口开启: $ports_to_keep"
        for port in $ports_to_keep; do
            if [[ "$port" =~ ^[0-9]+$ ]]; then
                add_firewall_rule "$port" "tcp" "$firewall_type"
                add_firewall_rule "$port" "udp" "$firewall_type"
            fi
        done
        
        # 【新】调用清理函数
        remove_unused_rules "$ports_to_keep" "$firewall_type"

        local message="🔒 *防火墙安全锁定完成*
> *保留端口*: \`$ports_to_keep\`
> *防火墙类型*: \`$firewall_type\`"
        send_telegram "$message"
        print_message "防火墙配置完成，仅允许必需端口的流量"
    else
        echo "ℹ️ 未检测到运行中的代理服务，跳过防火墙配置"
    fi
}

# 参数处理...
# （此处代码与原版相同，为简洁省略）
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