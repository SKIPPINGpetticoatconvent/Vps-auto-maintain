#!/bin/bash
# -----------------------------------------------------------------------------------------
# VPS 代理服务端口检测和防火墙配置脚本（修复版）
#
# 功能：
# - 自动检测 Xray 和 Sing-box (sb) 的开放端口
# - 从配置文件解析端口信息
# - 配置防火墙允许 UDP/TCP 流量通过这些端口
# - 修复端口误匹配和函数返回值污染问题
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

# 从配置文件解析端口
parse_config_ports() {
    local config_file="$1"
    local ports=""

    if [ -f "$config_file" ]; then
        # 【修复】将日志信息输出到 stderr (>&2)，以避免污染函数的 stdout 返回值
        echo "📄 解析配置文件: $config_file" >&2

        # 方法1: 使用 jq 解析 JSON（推荐）
        if command -v jq &> /dev/null; then
            ports=$(jq -r '.inbounds[]?.listen_port // .inbounds[]?.port // empty' "$config_file" 2>/dev/null | grep -E '^[0-9]+$' | sort -u | tr '\n' ' ')
        fi

        # 方法2: 如果 jq 不可用，使用 grep 解析
        if [ -z "$ports" ]; then
            # 【修复】将日志信息输出到 stderr (>&2)
            echo "⚠️ jq 不可用，使用备用解析方法" >&2
            # 查找 listen_port 或 port 字段后的数字
            ports=$(grep -o '"listen_port":[[:space:]]*[0-9]\+' "$config_file" | grep -o '[0-9]\+' | sort -u | tr '\n' ' ')
            if [ -z "$ports" ]; then
                ports=$(grep -o '"port":[[:space:]]*[0-9]\+' "$config_file" | grep -o '[0-9]\+' | sort -u | tr '\n' ' ')
            fi
        fi

        if [ -n "$ports" ]; then
            # 【修复】将日志信息输出到 stderr (>&2)
            echo "📋 从配置文件读取到端口: $ports" >&2
        fi
    fi

    # 仅将最终的端口号输出到 stdout，作为函数的返回值
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
            # 临时禁用 set -e，以防止 firewall-cmd 的“已存在”警告导致脚本退出
            set +e
            # 检查端口是否已在永久规则中
            if ! sudo firewall-cmd --permanent --query-port="$port/$protocol" > /dev/null 2>&1; then
                # echo "ℹ️ Port $port/$protocol not found in permanent firewall rules. Adding..." >&2
                sudo firewall-cmd --permanent --add-port="$port/$protocol" > /dev/null 2>&1
                # 仅在添加了新规则时才重载防火墙，提高效率
                sudo firewall-cmd --reload > /dev/null 2>&1
            # else
            #    echo "✅ Port $port/$protocol is already configured in firewall. No changes needed." >&2
            fi
            # 重新启用 set -e
            set -e
            ;;
        ufw)
            sudo ufw allow "$port/$protocol" > /dev/null 2>&1
            ;;
        none)
            echo "⚠️ 未检测到活跃的防火墙，跳过规则添加" >&2
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
    local all_ports=""
    local unique_ports=""
    local firewall_type=$(detect_firewall)

    echo "🔍 检测防火墙类型: $firewall_type"
    echo "🕒 系统时区: $timezone"
    echo "🕐 当前时间: $time_now"

    # 检测 Xray 端口
    if command -v xray &> /dev/null && pgrep -f "xray" > /dev/null; then
        xray_ports=$(get_process_ports "xray")
        if [ -n "$xray_ports" ]; then
            echo "✅ 检测到 Xray 运行端口: $xray_ports"
            all_ports="$all_ports $xray_ports"
        else
            echo "⚠️ Xray 正在运行但未检测到监听端口"
        fi
    else
        echo "❌ Xray 未安装或未运行"
    fi

    # 检测 Sing-box 端口
    if command -v sb &> /dev/null || command -v sing-box &> /dev/null; then
        if pgrep -f "sing-box" > /dev/null; then
            echo "🔍 正在检测 Sing-box 监听端口..."

            # 方法1: 检测 sing-box 进程端口
            sb_ports=$(get_process_ports "sing-box")
            [ -n "$sb_ports" ] && echo "📡 检测到 sing-box 进程端口: $sb_ports"

            # 方法2: 从配置文件解析端口
            if [ -z "$sb_ports" ]; then
                echo "🔍 尝试从 Sing-box 配置文件读取端口..." >&2
                local config_files=(
                    "/etc/sing-box/config.json"
                    "/etc/sing-box/conf/Hysteria2-36479.json"
                    "/etc/sing-box/conf/TUIC-46500.json"
                    "/usr/local/etc/sing-box/config.json"
                    "/opt/sing-box/config.json"
                )
                local temp_sb_ports=""
                for config_file in "${config_files[@]}"; do
                    config_ports=$(parse_config_ports "$config_file")
                    if [ -n "$config_ports" ]; then
                        temp_sb_ports="$temp_sb_ports $config_ports"
                    fi
                done
                sb_ports=$(echo "$temp_sb_ports" | tr ' ' '\n' | sort -u | tr '\n' ' ')
            fi

            if [ -n "$sb_ports" ]; then
                echo "✅ 检测到 Sing-box 运行端口:$sb_ports"
                all_ports="$all_ports $sb_ports"
            else
                echo "⚠️ Sing-box 正在运行但未检测到监听端口"
            fi
        else
            echo "ℹ️ Sing-box 已安装但未运行"
        fi
    else
        echo "❌ Sing-box 未安装"
    fi

    # 统一处理所有端口，去重并添加防火墙规则
    if [ -n "$all_ports" ]; then
        unique_ports=$(echo "$all_ports" | tr ' ' '\n' | sort -u | tr '\n' ' ')
        
        for port in $unique_ports; do
            if [[ "$port" =~ ^[0-9]+$ ]]; then
                add_firewall_rule "$port" "tcp" "$firewall_type"
                add_firewall_rule "$port" "udp" "$firewall_type"
            fi
        done
        
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
            echo "用法: $0 [--no-notify] [--token TOKEN] [--chat-id CHAT_ID]" >&2
            echo "示例:" >&2
            echo "  $0 --token YOUR_TOKEN --chat-id YOUR_ID" >&2
            exit 1
            ;;
    esac
done

main