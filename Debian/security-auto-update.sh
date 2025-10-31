#!/bin/bash
# ============================================================
# Debian 无人值守安全更新 + 自动清理 + 内存日志 + 智能自检
# ============================================================

set -e

echo "🧩 正在配置无人值守安全更新环境..."

# 1️⃣ 安装必要组件
apt update -y
apt install -y unattended-upgrades apt-listchanges apt-utils

# 2️⃣ 配置仅启用安全更新源
cat >/etc/apt/apt.conf.d/50unattended-upgrades <<'EOF'
Unattended-Upgrade::Origins-Pattern {
  "origin=Debian,codename=${distro_codename},label=Debian-Security";
};
Unattended-Upgrade::Automatic-Reboot "true";
Unattended-Upgrade::Automatic-Reboot-Time "03:00";
Unattended-Upgrade::Remove-Unused-Dependencies "true";
Unattended-Upgrade::Remove-Unused-Kernel-Packages "true";
Unattended-Upgrade::Remove-Unused-Kernel-Packages-Immediately "true";
Unattended-Upgrade::Remove-New-Unused-Dependencies "true";
Unattended-Upgrade::Verbose "true";
Unattended-Upgrade::SyslogEnable "true";
Unattended-Upgrade::SyslogFacility "daemon";
EOF

# 3️⃣ 配置每日自动执行与清理
cat >/etc/apt/apt.conf.d/20auto-upgrades <<'EOF'
APT::Periodic::Update-Package-Lists "1";
APT::Periodic::Unattended-Upgrade "1";
APT::Periodic::AutocleanInterval "7";
APT::Periodic::Verbose "1";
EOF

# 4️⃣ 内存日志
mkdir -p /etc/systemd/journald.conf.d
cat >/etc/systemd/journald.conf.d/volatile.conf <<'EOF'
[Journal]
Storage=volatile
RuntimeMaxUse=10M
Compress=yes
EOF
systemctl restart systemd-journald

# 5️⃣ 启用定时任务
systemctl enable --now apt-daily.timer apt-daily-upgrade.timer >/dev/null 2>&1

# 6️⃣ 清理旧包
apt autoremove -y --purge
apt autoclean -y

# ============================================================
# 🔍 智能自检模块
# ============================================================
echo ""
echo "🧠 开始执行无人值守更新配置自检..."

# 定义一个检测函数
check_pattern() {
    local file="$1"
    local pattern="$2"
    local desc="$3"
    if grep -Eq "$pattern" "$file"; then
        echo "✅ [$desc] 已正确配置"
    else
        echo "⚠️ [$desc] 未检测到或配置错误 → 文件: $file"
    fi
}

# 1️⃣ 检查仅安全源更新
check_pattern "/etc/apt/apt.conf.d/50unattended-upgrades" \
'Debian-Security' \
"仅启用安全源 (Debian-Security)"

# 2️⃣ 检查自动重启
check_pattern "/etc/apt/apt.conf.d/50unattended-upgrades" \
'Automatic-Reboot[[:space:]]+"true"' \
"自动重启启用"

# 3️⃣ 检查删除旧内核
check_pattern "/etc/apt/apt.conf.d/50unattended-upgrades" \
'Remove-Unused-Kernel-Packages[[:space:]]+"true"' \
"旧内核自动清理"

# 4️⃣ 检查每日执行任务
check_pattern "/etc/apt/apt.conf.d/20auto-upgrades" \
'Unattended-Upgrade[[:space:]]+"1"' \
"每日无人值守更新任务"

# 5️⃣ 检查内存日志
check_pattern "/etc/systemd/journald.conf.d/volatile.conf" \
'Storage=volatile' \
"内存日志模式启用"

# 6️⃣ 检查APT定时器
if systemctl list-timers apt-* --no-pager | grep -q "apt-daily"; then
    echo "✅ [APT 定时器] 已启用"
else
    echo "⚠️ [APT 定时器] 未检测到启用"
fi

# 7️⃣ 试运行 dry-run 检查更新机制
echo ""
echo "🧪 测试无人值守升级 dry-run..."
unattended-upgrade --dry-run --debug | grep -E 'Checking|found that can be upgraded' || echo "（暂无可升级项）"

# 8️⃣ 输出总结
echo ""
echo "🎉 自检完成。若上方全部为 ✅，则系统无人值守更新配置完全正常！"
echo "系统将每日自动应用安全补丁，并在 03:00 自动重启（仅安全更新后）。"
