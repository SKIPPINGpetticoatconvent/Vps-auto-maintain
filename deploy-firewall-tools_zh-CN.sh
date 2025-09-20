#!/bin/bash
# -----------------------------------------------------------------------------------------
# 防火墙管理工具部署器 (v2.0 - 集成首次安装)
#
# 作者: FTDRTD
#
# 此脚本将为您的系统部署两个强大的中文防火墙管理工具，
# 并在首次运行时自动安装和配置防火墙（如果需要）。
# -----------------------------------------------------------------------------------------

set -e

# --- 变量定义 ---
HARDEN_SCRIPT="/usr/local/bin/harden-firewall"
LOCKDOWN_SCRIPT="/usr/local/bin/lockdown-firewall"

# --- 函数定义 ---
print_message() {
    echo ""
    echo "------------------------------------------------------------"
    echo "$1"
    echo "------------------------------------------------------------"
}

# 仅在部署器中使用的防火墙安装函数
setup_initial_firewall() {
    print_message "步骤 1.5: 防火墙初始化检查"
    if systemctl is-active --quiet firewalld || (command -v ufw &>/dev/null && ufw status | grep -q "Status: active"); then
        echo "--> ✅ 已检测到活跃的防火墙，无需安装。"
        return
    fi
    
    echo "--> ⚠️ 未检测到活跃防火墙，将为您自动安装一个。"
    if [ -f /etc/os-release ]; then . /etc/os-release; fi
    
    if [[ "$ID" == "ubuntu" || "$ID" == "debian" || "$ID_LIKE" == "debian" ]]; then
        echo "--> 检测到 Debian/Ubuntu 系统，正在安装 UFW..."
        sudo apt-get update >/dev/null && sudo DEBIAN_FRONTEND=noninteractive apt-get install -y ufw >/dev/null
        # 预先允许默认SSH端口，以防万一
        sudo ufw allow 22/tcp >/dev/null
        # 使用 echo "y" 自动应答SSH连接警告
        echo "y" | sudo ufw enable >/dev/null
        echo "--> ✅ UFW 安装并启用成功。"
    elif [[ "$ID" == "centos" || "$ID" == "rhel" || "$ID" == "fedora" || "$ID" == "almalinux" || "$ID_LIKE" == "rhel" ]]; then
        echo "--> 检测到 RHEL/CentOS 系列系统，正在安装 firewalld..."
        if command -v dnf &>/dev/null; then sudo dnf install -y firewalld >/dev/null; else sudo yum install -y firewalld >/dev/null; fi
        sudo systemctl enable --now firewalld >/dev/null
        echo "--> ✅ firewalld 安装并启用成功。"
    else
        echo "--> ❌ 错误：不支持的操作系统: $ID。无法自动安装防火墙。" >&2; exit 1
    fi
}


# --- 主程序开始 ---

# --- 步骤 0: 清理旧版本 ---
print_message "步骤 0: 清理旧版本（如果存在）..."
sudo rm -f "$HARDEN_SCRIPT" "$LOCKDOWN_SCRIPT"
echo "--> ✅ 旧版本清理完成。"

# --- 步骤 1: 用户输入 (可选的TG配置) ---
print_message "步骤 1: 配置 Telegram 通知 (可选)"
echo "此配置将会被嵌入到生成的两个工具脚本中。"
read -p "是否要配置 Telegram 通知? [y/N]: " setup_notify
TG_TOKEN=""
TG_CHAT_ID=""
NOTIFY=false
if [[ "$setup_notify" =~ ^[Yy]$ ]]; then
    read -p "请输入你的 Telegram Bot Token: " TG_TOKEN
    read -p "请输入你的 Telegram Chat ID: " TG_CHAT_ID
    if [ -n "$TG_TOKEN" ] && [ -n "$TG_CHAT_ID" ]; then
        NOTIFY=true
        echo "--> ✅ Telegram 通知已配置。"
    else
        echo "--> ⚠️  警告：输入不完整，生成的脚本中将禁用通知功能。"
    fi
fi

# --- 步骤 1.5: 防火墙初始化检查 (核心修复) ---
setup_initial_firewall

# --- 步骤 2: 创建 'harden-firewall' (安全加固) 脚本 ---
print_message "步骤 2: 正在创建 'harden-firewall' (安全加固) 工具..."
sudo tee "$HARDEN_SCRIPT" > /dev/null <<'EOF'
#!/bin/bash
# Firewall Hardening Script (Add-Only Mode) - 由部署器生成
set -e
TG_TOKEN="__TG_TOKEN__"
TG_CHAT_ID="__TG_CHAT_ID__"
NOTIFY=__NOTIFY__
send_telegram() { if [ "$NOTIFY" = true ] && [ -n "$TG_TOKEN" ] && [ -n "$TG_CHAT_ID" ]; then curl -s -X POST "https://api.telegram.org/bot$TG_TOKEN/sendMessage" -d chat_id="$TG_CHAT_ID" -d text="$1" -d parse_mode="Markdown" >/dev/null || true; fi; }
echo "--- 正在开始防火墙安全加固检查 (仅添加模式) ---"
FW_TYPE=$(if systemctl is-active --quiet firewalld; then echo "firewalld"; elif command -v ufw &>/dev/null && ufw status | grep -q "Status: active"; then echo "ufw"; else echo "none"; fi)
if [ "$FW_TYPE" = "none" ]; then echo "❌ 错误：未检测到活跃的防火墙 (UFW 或 Firewalld)。" >&2; exit 1; fi
echo "--> 检测到防火墙: $FW_TYPE"
ssh_port=$(grep -i '^Port ' /etc/ssh/sshd_config | awk '{print $2}' | head -n1); [ -z "$ssh_port" ] && ssh_port=22
xray_ports=""; if command -v xray &>/dev/null && pgrep -f xray >/dev/null; then xray_ports=$(ss -tlnp 2>/dev/null | grep xray | awk '{print $4}' | awk -F: '{print $NF}' | sort -u | tr '\n' ' '); fi
sb_ports=""; if (command -v sb &>/dev/null || command -v sing-box &>/dev/null) && pgrep -f sing-box >/dev/null; then sb_ports=$(ss -tlnp 2>/dev/null | grep sing-box | awk '{print $4}' | awk -F: '{print $NF}' | sort -u | tr '\n' ' '); fi
ports_to_add=$(echo "$ssh_port $xray_ports $sb_ports" | tr ' ' '\n' | sort -un | tr '\n' ' ')
echo "--> 检测到需要放行的端口: $ports_to_add"
newly_added_ports=""
if [ "$FW_TYPE" = "firewalld" ]; then
    for port in $ports_to_add; do
        if ! sudo firewall-cmd --permanent --query-port="$port/tcp" >/dev/null 2>&1; then sudo firewall-cmd --permanent --add-port="$port/tcp" >/dev/null; newly_added_ports="$newly_added_ports $port/tcp"; fi
        if ! sudo firewall-cmd --permanent --query-port="$port/udp" >/dev/null 2>&1; then sudo firewall-cmd --permanent --add-port="$port/udp" >/dev/null; newly_added_ports="$newly_added_ports $port/udp"; fi
    done
    if [ -n "$newly_added_ports" ]; then echo "--> 正在重载 firewalld..."; sudo firewall-cmd --reload >/dev/null; fi
elif [ "$FW_TYPE" = "ufw" ]; then
    for port in $ports_to_add; do
        if ! sudo ufw status | grep -q "^\s*$port\b.*ALLOW"; then sudo ufw allow "$port" >/dev/null; newly_added_ports="$newly_added_ports $port"; fi
    done
fi
if [ -n "$newly_added_ports" ]; then
    echo "--> ✅ 成功：已将新端口添加到防火墙: $newly_added_ports"
    send_telegram "✅ *防火墙加固：已添加新端口*
> *服务器*: \`$(hostname)\`
> *新增端口*: \`$newly_added_ports\`"
else
    echo "--> ℹ️ 信息：所有必需端口均已放行，未做任何更改。"
fi
echo "--- 防火墙安全加固检查完成 ---"
EOF

# --- 步骤 3: 创建 'lockdown-firewall' (安全锁定) 脚本 ---
# (此部分与上一版本完全相同，无需修改)
print_message "步骤 3: 正在创建 'lockdown-firewall' (安全锁定) 工具..."
sudo tee "$LOCKDOWN_SCRIPT" > /dev/null <<'EOF'
#!/bin/bash
# Firewall Lockdown Script (Remove-Unknown Mode) - 由部署器生成
set -e
TG_TOKEN="__TG_TOKEN__"
TG_CHAT_ID="__TG_CHAT_ID__"
NOTIFY=__NOTIFY__
print_message() { echo ""; echo "------------------------------------------------------------"; echo "$1"; echo "------------------------------------------------------------"; }
send_telegram() { if [ "$NOTIFY" = true ] && [ -n "$TG_TOKEN" ] && [ -n "$TG_CHAT_ID" ]; then curl -s -X POST "https://api.telegram.org/bot$TG_TOKEN/sendMessage" -d chat_id="$TG_CHAT_ID" -d text="$1" -d parse_mode="Markdown" >/dev/null || true; fi; }
print_message "防火墙安全锁定初始化 (移除未知端口模式)"
FW_TYPE=$(if systemctl is-active --quiet firewalld; then echo "firewalld"; elif command -v ufw &>/dev/null && ufw status | grep -q "Status: active"; then echo "ufw"; else echo "none"; fi)
if [ "$FW_TYPE" = "none" ]; then echo "❌ 错误：未检测到活跃的防火墙。" >&2; exit 1; fi
echo "--> 检测到防火墙: $FW_TYPE"
ssh_port=$(grep -i '^Port ' /etc/ssh/sshd_config | awk '{print $2}' | head -n1); [ -z "$ssh_port" ] && ssh_port=22
xray_ports=""; if command -v xray &>/dev/null && pgrep -f xray >/dev/null; then xray_ports=$(ss -tlnp 2>/dev/null | grep xray | awk '{print $4}' | awk -F: '{print $NF}' | sort -u | tr '\n' ' '); fi
sb_ports=""; if (command -v sb &>/dev/null || command -v sing-box &>/dev/null) && pgrep -f sing-box >/dev/null; then sb_ports=$(ss -tlnp 2>/dev/null | grep sing-box | awk '{print $4}' | awk -F: '{print $NF}' | sort -u | tr '\n' ' '); fi
ports_to_keep=$(echo "$ssh_port $xray_ports $sb_ports" | tr ' ' '\n' | sort -un | tr '\n' ' ')
echo "--> 将要保留的必需端口: $ports_to_keep"
print_message "⚠️ 警告：此操作将移除所有非必需的端口规则！"
read -p "您确定要继续吗? [y/N]: " confirmation
if [[ ! "$confirmation" =~ ^[Yy]$ ]]; then echo "--> 用户取消了操作。"; exit 0; fi
if [ "$FW_TYPE" = "firewalld" ]; then
    echo "--> 正在锁定 firewalld..."
    FIREWALL_CHANGED=false
    current_ports=$(sudo firewall-cmd --permanent --list-ports)
    for port_rule in $current_ports; do
        port_num=$(echo "$port_rule" | cut -d'/' -f1)
        if ! echo " $ports_to_keep " | grep -q " $port_num "; then echo "--> 正在移除未知端口规则: $port_rule"; sudo firewall-cmd --permanent --remove-port="$port_rule" >/dev/null; FIREWALL_CHANGED=true; fi
    done
    if [ "$FIREWALL_CHANGED" = true ]; then echo "--> 正在重载 firewalld..."; sudo firewall-cmd --reload >/dev/null; else echo "--> 未发现可移除的未知端口。"; fi
elif [ "$FW_TYPE" = "ufw" ]; then
    echo "--> 正在通过重置来锁定 UFW...";
    echo "y" | sudo ufw reset >/dev/null
    sudo ufw default deny incoming >/dev/null && sudo ufw default allow outgoing >/dev/null
    for port in $ports_to_keep; do sudo ufw allow "$port" >/dev/null; echo "--> 已允许必需端口: $port"; done
    echo "y" | sudo ufw enable >/dev/null
fi
final_message="🔒 *防火墙安全锁定完成*
> *服务器*: \`$(hostname)\`
> *保留端口*: \`$ports_to_keep\`"
send_telegram "$final_message"
print_message "防火墙安全锁定完成。仅保留必需端口。"
if [ "$FW_TYPE" = "ufw" ]; then sudo ufw status; fi
EOF


# --- 步骤 4: 替换变量并设置权限 ---
print_message "步骤 4: 正在完成脚本配置..."
sudo sed -i "s|__TG_TOKEN__|$TG_TOKEN|g" "$HARDEN_SCRIPT" "$LOCKDOWN_SCRIPT"
sudo sed -i "s|__TG_CHAT_ID__|$TG_CHAT_ID|g" "$HARDEN_SCRIPT" "$LOCKDOWN_SCRIPT"
sudo sed -i "s|__NOTIFY__|$NOTIFY|g" "$HARDEN_SCRIPT" "$LOCKDOWN_SCRIPT"
sudo chmod +x "$HARDEN_SCRIPT" "$LOCKDOWN_SCRIPT"
echo "--> ✅ 脚本权限设置完成。"

# --- 步骤 5: 完成 ---
# (此部分与上一版本完全相同，无需修改)
print_message "部署完成！"
echo "您的系统上现在有两个新的命令可用："
echo ""
echo "  - 安全地添加新服务端口 (可用于定时任务):"
echo "    sudo harden-firewall"
echo ""
echo "  - 移除所有未知端口 (请谨慎手动运行):"
echo "    sudo lockdown-firewall"
echo ""
echo "您可以将 'harden-firewall' 添加到定时任务中，例如每天凌晨执行一次："
echo "  (crontab -l 2>/dev/null; echo '0 4 * * * /usr/local/bin/harden-firewall') | crontab -"
echo ""
