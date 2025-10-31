#!/bin/bash
# ============================================================
# Debian 无人值守安全更新 + 自动清理 + 内存日志
# 适用于 VPS / 无盘 / 长期运行场景
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

Unattended-Upgrade::Package-Blacklist {
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

# 3️⃣ 启用自动更新周期（每日）
cat >/etc/apt/apt.conf.d/20auto-upgrades <<'EOF'
APT::Periodic::Update-Package-Lists "1";
APT::Periodic::Unattended-Upgrade "1";
APT::Periodic::AutocleanInterval "7";
APT::Periodic::Verbose "1";
EOF

# 4️⃣ 内存日志（防止写盘）
mkdir -p /etc/systemd/journald.conf.d
cat >/etc/systemd/journald.conf.d/volatile.conf <<'EOF'
[Journal]
Storage=volatile
RuntimeMaxUse=10M
Compress=yes
EOF
systemctl restart systemd-journald

# 5️⃣ 启用定时任务
systemctl enable --now apt-daily.timer
systemctl enable --now apt-daily-upgrade.timer

# 6️⃣ 立即执行一次清理（可选）
apt autoremove -y --purge
apt autoclean -y

# 7️⃣ 验证状态
echo ""
echo "🕐 当前 APT 定时任务:"
systemctl list-timers apt-* --no-pager

echo ""
echo "🔍 测试无人值守升级 dry-run:"
unattended-upgrade --dry-run --debug | grep -E 'Checking|found that can be upgraded' || true

echo ""
echo "✅ 自动安全更新已启用，系统将每日应用安全补丁并在 03:00 自动重启（仅安全更新后）。"
