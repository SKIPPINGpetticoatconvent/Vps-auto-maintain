#!/bin/bash
# -----------------------------------------------------------------------------------------
# VPS 防火墙自动锁定脚本 (版本 5.0 - 交互前置重构版)
#
# 作者: FTDRTD
# 仓库: https://github.com/FTDRTD/Vps-auto-maintain
#
# 借鉴了 vps-auto-maintain 的设计哲学，将用户交互全部前置，
# 使得脚本结构更清晰，执行更可靠。
# -----------------------------------------------------------------------------------------

set -e

# --- 全局变量定义 ---
TG_TOKEN=""
TG_CHAT_ID=""
NOTIFY=false
FW_TYPE=""

# --- 函数定义 ---
print_message() {
    echo ""
    echo "------------------------------------------------------------"
    echo "$1"
    echo "------------------------------------------------------------"
}

# (这里放置所有不需要与用户交互的函数)
get_timezone() {
    local tz
    if command -v timedatectl &> /dev/null; then tz=$(timedatectl | grep "Time zone" | awk '{print $3}'); fi
    if [ -z "$tz" ] && [ -f /etc/timezone ]; then tz=$(cat /etc/timezone); fi
    if [ -z "$tz" ]; then tz="Etc/UTC"; fi
    echo "$tz"
}

send_telegram() {
    if [ "$NOTIFY" = true ] && [ -n "$TG_TOKEN" ] && [ -n "$TG_CHAT_ID" ]; then
        local message="$1"
        curl --connect-timeout 10 --retry 3 -s -X POST "https://api.telegram.org/bot$TG_TOKEN/sendMessage" \
            -d chat_id="$TG_CHAT_ID" -d text="$message" -d parse_mode="Markdown" > /dev/null || true
    fi
}

detect_firewall() {
    if systemctl is-active --quiet firewalld; then echo "firewalld";
    elif command -v ufw &> /dev/null && ufw status | grep -q "Status: active"; then echo "ufw";
    else echo "none"; fi
}

setup_firewall() {
    print_message "步骤 0: 未检测到活跃防火墙，将自动安装并配置"
    if [ -f /etc/os-release ]; then . /etc/os-release; fi
    
    if [[ "$ID" == "ubuntu" || "$ID" == "debian" || "$ID_LIKE" == "debian" ]]; then
        echo "--> 检测到 Debian/Ubuntu 系统，正在安装 UFW..."
        sudo apt-get update >/dev/null && sudo apt-get install -y ufw >/dev/null
        echo "y" | sudo ufw reset >/dev/null
        sudo ufw default deny incoming >/dev/null && sudo ufw default allow outgoing >/dev/null
        sudo ufw enable >/dev/null
        echo "--> ✅ UFW 安装并启用成功。"
        FW_TYPE="ufw"
    elif [[ "$ID" == "centos" || "$ID" == "rhel" || "$ID" == "fedora" || "$ID" == "almalinux" || "$ID_LIKE" == "rhel" ]]; then
        echo "--> 检测到 RHEL/CentOS 系列系统，正在安装 firewalld..."
        if command -v dnf &>/dev/null; then sudo dnf install -y firewalld >/dev/null; else sudo yum install -y firewalld >/dev/null; fi
        sudo systemctl enable --now firewalld >/dev/null
        echo "--> ✅ firewalld 安装并启用成功。"
        FW_TYPE="firewalld"
    else
        echo "--> ❌ 错误：不支持的操作系统: $ID。请手动安装防火墙。" >&2; exit 1
    fi
}

# --- 主程序开始 ---

# --- 阶段一: 信息收集 (所有交互在此完成) ---
print_message "阶段一: 信息收集"
read -p "是否要配置 Telegram 通知? [y/N]: " setup_notify
if [[ "$setup_notify" =~ ^[Yy]$ ]]; then
    read -p "请输入你的 Telegram Bot Token: " input_token
    read -p "请输入你的 Telegram Chat ID: " input_chat_id
    if [ -n "$input_token" ] && [ -n "$input_chat_id" ]; then
        TG_TOKEN="$input_token"
        TG_CHAT_ID="$input_chat_id"
        NOTIFY=true
        echo "--> ✅ Telegram 通知已配置。"
    else
        echo "--> ⚠️ 警告：输入不完整，将禁用 Telegram 通知。"
    fi
else
    echo "--> ℹ️ 已跳过 Telegram 通知配置。"
fi

read -p "所有信息已收集完毕。按 Enter 键开始自动化执行，或按 Ctrl+C 取消..."

# --- 阶段二: 自动化执行 (不再有任何交互) ---
print_message "阶段二: 开始自动化执行"

# 步骤 2.1: 系统和防火墙检测
echo "--> 正在检测系统和运行的服务..."
FW_TYPE=$(detect_firewall)
if [ "$FW_TYPE" = "none" ]; then
    setup_firewall
fi
echo "--> 🔍 检测到防火墙类型: $FW_TYPE"

local ssh_port; ssh_port=$(grep -i '^Port ' /etc/ssh/sshd_config | awk '{print $2}' | head -n1); [ -z "$ssh_port" ] && ssh_port=22
echo "--> 🛡️  检测到 SSH 端口为: $ssh_port (此端口将被强制保留)"

# 步骤 2.2: 服务端口检测
local xray_ports sb_ports all_ports
if command -v xray &>/dev/null && pgrep -f xray >/dev/null; then
    xray_ports=$(ss -tlnp 2>/dev/null | grep xray | awk '{print $4}' | awk -F: '{print $NF}' | sort -u | tr '\n' ' ')
    if [ -n "$xray_ports" ]; then echo "--> ✅ 检测到 Xray 运行端口: $xray_ports"; fi
fi
if (command -v sb &>/dev/null || command -v sing-box &>/dev/null) && pgrep -f sing-box >/dev/null; then
    sb_ports=$(ss -tlnp 2>/dev/null | grep sing-box | awk '{print $4}' | awk -F: '{print $NF}' | sort -u | tr '\n' ' ')
    if [ -n "$sb_ports" ]; then echo "--> ✅ 检测到 Sing-box 运行端口: $sb_ports"; fi
fi

local ports_to_keep; ports_to_keep=$(echo "$ssh_port $xray_ports $sb_ports" | tr ' ' '\n' | sort -un | tr '\n' ' ')
if [ -z "$(echo "$ports_to_keep" | xargs)" ]; then
    echo "--> ℹ️ 未检测到任何需要保留的端口 (除了SSH)，跳过防火墙配置。"
    exit 0
fi
echo "--> ℹ️ 将要确保以下端口开启: $ports_to_keep"

# 步骤 2.3: 应用防火墙规则
print_message "正在应用防火墙规则..."
if [ "$FW_TYPE" = "firewalld" ]; then
    echo "--> 正在配置 firewalld..."
    FIREWALL_CHANGED=false
    for port in $ports_to_keep; do
        if ! sudo firewall-cmd --permanent --query-port="$port/tcp" >/dev/null 2>&1; then sudo firewall-cmd --permanent --add-port="$port/tcp" >/dev/null; FIREWALL_CHANGED=true; fi
        if ! sudo firewall-cmd --permanent --query-port="$port/udp" >/dev/null 2>&1; then sudo firewall-cmd --permanent --add-port="$port/udp" >/dev/null; FIREWALL_CHANGED=true; fi
    done
    local current_ports; current_ports=$(sudo firewall-cmd --permanent --list-ports)
    for port_rule in $current_ports; do
        local port_num; port_num=$(echo "$port_rule" | cut -d'/' -f1)
        if ! echo " $ports_to_keep " | grep -q " $port_num "; then echo "--> ➖ 正在移除未使用的端口规则: $port_rule"; sudo firewall-cmd --permanent --remove-port="$port_rule" >/dev/null; FIREWALL_CHANGED=true; fi
    done
    if [ "$FIREWALL_CHANGED" = true ]; then echo "--> 🔄 正在重载防火墙以应用更改..."; sudo firewall-cmd --reload >/dev/null; else echo "--> ✅ 无需更改，firewalld 规则已是最新。"; fi
elif [ "$FW_TYPE" = "ufw" ]; 键，然后
    echo "--> ⚠️  警告: UFW 将被重置以锁定端口！"
    echo "    操作将在 5 秒后继续，按 Ctrl+C 取消..."
    sleep 5
    echo "--> 🔄 正在重置 UFW..."; echo "y" | sudo ufw reset >/dev/null
    sudo ufw default deny incoming >/dev/null && sudo ufw default allow outgoing >/dev/null
    for port 在 $ports_to_keep; do sudo ufw allow "$port" >/dev/null; echo "--> ➕ 允许端口: $port"; done
    sudo ufw enable >/dev/null
    echo "--> ✅ UFW 已重置并配置完毕。"; sudo ufw status
fi
echo "--> 👍 防火墙锁定完成。"

# 步骤 2.4: 发送最终通知
local timezone; timezone=$(get_timezone)
local time_now; time_now=$(date '+%Y-%m-%d %H:%M:%S')
local message="🔒 *防火墙安全锁定完成*
> *服务器*: \`$(hostname)\`
> *保留端口*: \`$ports_to_keep\`
> *防火墙类型*: \`$FW_TYPE\`
> *执行时间*: \`$time_now ($timezone)\`"
send_telegram "$message"
print_message "所有操作完成。您的服务器现已得到防火墙保护。"
