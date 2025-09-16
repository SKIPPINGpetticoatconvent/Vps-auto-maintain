#!/bin/bash
# -----------------------------------------------------------------------------------------
# VPS 代理服务端口检测和防火墙配置脚本
#
# 功能：
# - 自动检测 Xray 和 Sing-box (sb) 的开放端口
# - 配置防火墙允许 UDP/TCP 流量通过这些端口
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
        # 使用 ss 命令检测监听端口（更可靠）
        if command -v ss &> /dev/null; then
            ports=$(ss -tlnp | grep "$process_name" | awk '{print $4}' | awk -F: '{print $NF}' | sort -u | tr '\n' ' ')
        elif command -v netstat &> /dev/null; then
            ports=$(netstat -tlnp | grep "$process_name" | awk '{print $4}' | awk -F: '{print $NF}' | sort -u | tr '\n' ' ')
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
            sudo firewall-cmd --permanent --add-port="$port/$protocol" > /dev/null 2>&1
            sudo firewall-cmd --reload > /dev/null 2>&1
            ;;
        ufw)
            sudo ufw allow "$port/$protocol" > /dev/null 2>&1
            ;;
        none)
            echo "⚠️ 未检测到活跃的防火墙，跳过规则添加"
            ;;
    esac
}

# 主函数
main() {
    print_message "开始检测代理服务端口并配置防火墙"

    local timezone=$(get_timezone)
    local time_now=$(date '+%Y-%m-%d %H:%M:%S')

    local xray_ports=""
    local sb_ports=""
    local firewall_type=$(detect_firewall)

    echo "🔍 检测防火墙类型: $firewall_type"
    echo "🕒 系统时区: $timezone"
    echo "🕐 当前时间: $time_now"

    # 检测 Xray 端口
    if command -v xray &> /dev/null && pgrep -f "xray" > /dev/null; then
        xray_ports=$(get_process_ports "xray")
        if [ -n "$xray_ports" ]; then
            echo "✅ 检测到 Xray 运行端口: $xray_ports"
            for port in $xray_ports; do
                add_firewall_rule "$port" "tcp" "$firewall_type"
                add_firewall_rule "$port" "udp" "$firewall_type"
            done
        else
            echo "⚠️ Xray 正在运行但未检测到监听端口"
        fi
    else
        echo "❌ Xray 未安装或未运行"
    fi

    # 检测 Sing-box 端口
    if command -v sb &> /dev/null; then
        # 检查是否有 sing-box 进程在运行
        if pgrep -f "sing-box" > /dev/null || pgrep -f "sb" > /dev/null; then
            # 优先检测 sing-box 进程端口，然后是 sb 管理脚本端口
            sb_ports=$(get_process_ports "sing-box")
            if [ -z "$sb_ports" ]; then
                sb_ports=$(get_process_ports "sb")
            fi

            if [ -n "$sb_ports" ]; then
                echo "✅ 检测到 Sing-box 运行端口: $sb_ports"
                for port in $sb_ports; do
                    add_firewall_rule "$port" "tcp" "$firewall_type"
                    add_firewall_rule "$port" "udp" "$firewall_type"
                done
            else
                echo "⚠️ Sing-box 正在运行但未检测到监听端口"
            fi
        else
            echo "ℹ️ Sing-box (sb) 已安装但未运行"
        fi
    else
        echo "❌ Sing-box (sb) 未安装"
    fi

    # 发送通知
    if [ -n "$xray_ports" ] || [ -n "$sb_ports" ]; then
        local message="🔧 *代理服务端口配置完成*
> *系统时区*: \`$timezone\`
> *当前时间*: \`$time_now\`
> *防火墙类型*: \`$firewall_type\`"

        if [ -n "$xray_ports" ]; then
            message="$message
> *Xray 端口*: \`$xray_ports\`"
        fi

        if [ -n "$sb_ports" ]; then
            message="$message
> *Sing-box 端口*: \`$sb_ports\`"
        fi

        send_telegram "$message"
        echo "✅ 防火墙规则配置完成，已允许相关端口的 UDP/TCP 流量"
    else
        echo "ℹ️ 未检测到运行中的代理服务，跳过防火墙配置"
    fi
}

# 参数处理
while [[ $# -gt 0 ]]; do
    case $1 in
        --no-notify)
            NOTIFY=false
            shift
            ;;
        --token)
            TG_TOKEN="$2"
            shift 2
            ;;
        --chat-id)
            TG_CHAT_ID="$2"
            shift 2
            ;;
        *)
            echo "用法: $0 [--no-notify] [--token TOKEN] [--chat-id CHAT_ID]"
            exit 1
            ;;
    esac
done

main