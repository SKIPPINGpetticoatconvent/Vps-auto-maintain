#!/bin/bash
# ----------------------------------------------------------------------------
# VPS Telegram Bot 完整卸载脚本
#
# 版本: 1.0.0
# 功能: 完全清理 Bot 及相关组件
# ----------------------------------------------------------------------------

set -e

# ========== 彩色输出 ==========
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

print_message() {
  echo ""
  echo "============================================================"
  echo "$1"
  echo "============================================================"
}
print_success() { echo -e "${GREEN}✅ $1${NC}"; }
print_error()   { echo -e "${RED}❌ $1${NC}"; }
print_warning() { echo -e "${YELLOW}⚠️  $1${NC}"; }

# ========== 全局路径 ==========
BOT_DIR="/opt/vps-tg-bot"
BOT_SERVICE="/etc/systemd/system/vps-tg-bot.service"
CORE_MAINTAIN_SCRIPT="/usr/local/bin/vps-maintain-core.sh"
RULES_MAINTAIN_SCRIPT="/usr/local/bin/vps-maintain-rules.sh"
JOURNALD_CONFIG="/etc/systemd/journald.conf.d/memory.conf"

# ========== 权限检查 ==========
if [ "$EUID" -ne 0 ]; then
  print_error "请使用 root 用户执行此脚本"
  exit 1
fi

# ========== 确认卸载 ==========
print_message "⚠️  VPS Telegram Bot 卸载程序"
echo -e "${RED}此操作将完全删除:${NC}"
echo "  • Bot 服务和二进制文件"
echo "  • 维护脚本"
echo "  • 定时任务"
echo "  • journald 配置"
echo ""
read -p "确认卸载？(输入 YES 继续): " CONFIRM

if [ "$CONFIRM" != "YES" ]; then
  print_warning "卸载已取消"
  exit 0
fi

# ========== 停止并删除服务 ==========
print_message "步骤 1: 停止并删除服务"
if systemctl is-active --quiet vps-tg-bot 2>/dev/null; then
  print_warning "停止服务..."
  systemctl stop vps-tg-bot
  print_success "服务已停止"
fi

if systemctl is-enabled --quiet vps-tg-bot 2>/dev/null; then
  print_warning "禁用服务..."
  systemctl disable vps-tg-bot
  print_success "服务已禁用"
fi

if [ -f "$BOT_SERVICE" ]; then
  rm -f "$BOT_SERVICE"
  systemctl daemon-reload
  print_success "服务文件已删除"
else
  print_warning "服务文件不存在，跳过"
fi

# ========== 删除 Bot 目录 ==========
print_message "步骤 2: 删除 Bot 程序"
if [ -d "$BOT_DIR" ]; then
  rm -rf "$BOT_DIR"
  print_success "Bot 目录已删除: $BOT_DIR"
else
  print_warning "Bot 目录不存在，跳过"
fi

# ========== 删除维护脚本 ==========
print_message "步骤 3: 删除维护脚本"
DELETED=0
if [ -f "$CORE_MAINTAIN_SCRIPT" ]; then
  rm -f "$CORE_MAINTAIN_SCRIPT"
  print_success "系统维护脚本已删除"
  DELETED=1
fi

if [ -f "$RULES_MAINTAIN_SCRIPT" ]; then
  rm -f "$RULES_MAINTAIN_SCRIPT"
  print_success "规则维护脚本已删除"
  DELETED=1
fi

if [ $DELETED -eq 0 ]; then
  print_warning "维护脚本不存在，跳过"
fi

# ========== 删除定时任务 ==========
print_message "步骤 4: 删除定时任务"
CURRENT_CRON=$(crontab -l 2>/dev/null || true)
if echo "$CURRENT_CRON" | grep -q "vps-maintain"; then
  (crontab -l 2>/dev/null | grep -v "vps-maintain" || true) | crontab -
  print_success "定时任务已删除"
else
  print_warning "未找到相关定时任务，跳过"
fi

# ========== 恢复 journald 配置 ==========
print_message "步骤 5: 恢复 journald 配置"
if [ -f "$JOURNALD_CONFIG" ]; then
  read -p "是否恢复 journald 默认配置？(y/n): " RESTORE_JOURNALD
  if [ "$RESTORE_JOURNALD" = "y" ] || [ "$RESTORE_JOURNALD" = "Y" ]; then
    rm -f "$JOURNALD_CONFIG"
    systemctl restart systemd-journald 2>/dev/null || true
    print_success "journald 配置已恢复"
  else
    print_warning "保留 journald 自定义配置"
  fi
else
  print_warning "未找到 journald 自定义配置，跳过"
fi

# ========== 清理临时文件 ==========
print_message "步骤 6: 清理临时文件"
rm -f /tmp/vps_maintain_result.txt /tmp/vps_rules_result.txt
print_success "临时文件已清理"

# ========== 完成提示 ==========
print_message "🎉 卸载完成！"
print_success "VPS Telegram Bot 已完全移除"
echo ""
echo "已删除的内容:"
echo "  ✓ 服务: $BOT_SERVICE"
echo "  ✓ 程序: $BOT_DIR"
echo "  ✓ 脚本: $CORE_MAINTAIN_SCRIPT"
echo "  ✓ 脚本: $RULES_MAINTAIN_SCRIPT"
echo "  ✓ 定时任务 (crontab)"
echo ""
print_warning "如需重新安装，请运行原部署脚本"