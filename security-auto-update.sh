#!/bin/bash
# ============================================================
# Debian 一键启用自动安全更新 + 内存日志
# 适用于 VPS / 最简轻量环境
# ============================================================

set -e

echo "🧩 开始配置无人值守安全更新环境..."

# 1️⃣ 安装必要组件
apt update -y
apt install -y unattended-upgrades apt-listchanges

# 2️⃣ 启用自动安全更新
dpkg-reconfigure -plow unattended-upgrades

# 3️⃣ 开启自动重启（仅当有安全更新）
cat >/etc/apt/apt.conf.d/51unattended-upgrades-reboot.conf <<'EOF'
Unattended-Upgrade::Automatic-Reboot "true";
Unattended-Upgrade::Automatic-Reboot-Time "03:00";
EOF

# 4️⃣ 将日志写入内存（防止写盘）
mkdir -p /etc/systemd/journald.conf.d
cat >/etc/systemd/journald.conf.d/volatile.conf <<'EOF'
[Journal]
Storage=volatile
RuntimeMaxUse=10M
Compress=yes
EOF
systemctl restart systemd-journald

# 5️⃣ 启用 systemd 定时任务
systemctl enable --now apt-daily.timer
systemctl enable --now apt-daily-upgrade.timer

echo "✅ 已启用自动安全更新和定时任务"

# 6️⃣ 验证状态
echo ""
echo "🕐 当前 APT 定时任务:"
systemctl list-timers apt-* --no-pager

echo ""
echo "🔍 验证无人值守升级是否正常:"
unattended-upgrade --dry-run --debug | grep -E 'Checking|found that can be upgraded' || true

echo ""
echo "🎉 配置完成！系统将每日自动应用安全补丁，如需重启将于 03:00 自动执行。"
