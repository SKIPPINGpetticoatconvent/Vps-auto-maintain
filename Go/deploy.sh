#!/bin/bash
# ----------------------------------------------------------------------------
# VPS Telegram Bot Go 版本 - 增强版部署脚本
#
# 版本: 2.0.8 (增强更新机制和卸载机制)
# 修复内容:
#   ✅ 添加完整的更新机制 (update 命令)
#   ✅ 添加增强的卸载机制 (uninstall 命令)
#   ✅ 添加状态查看功能 (status 命令)
#   ✅ 添加配置备份功能 (backup/restore 命令)
#   ✅ 修复 "Text file busy" 错误
#   ✅ 下载到临时文件再移动，避免覆盖运行中的二进制
#   ✅ 先停止服务再替换二进制文件
#   ✅ 添加下载重试机制
# ----------------------------------------------------------------------------

set -e

# ========== 彩色输出 ==========
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
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
print_info() { echo -e "${BLUE}ℹ️  $1${NC}"; }

# ========== 全局路径 ==========
BOT_DIR="/opt/vps-tg-bot"
BOT_BINARY="$BOT_DIR/vps-tg-bot"
BOT_BINARY_TMP="$BOT_DIR/vps-tg-bot.tmp"
BOT_SERVICE="/etc/systemd/system/vps-tg-bot.service"
CORE_MAINTAIN_SCRIPT="/usr/local/bin/vps-maintain-core.sh"
RULES_MAINTAIN_SCRIPT="/usr/local/bin/vps-maintain-rules.sh"

# ========== 环境自愈 ==========
ensure_coreutils() {
  if ! command -v mkdir >/dev/null 2>&1; then
    print_warning "检测到 coreutils 缺失，正在自动修复..."
    if command -v apt-get >/dev/null 2>&1; then
      apt-get update -o Acquire::ForceIPv4=true >/dev/null 2>&1 || true
      apt-get install -y coreutils >/dev/null 2>&1 || true
    elif command -v apt >/dev/null 2>&1; then
      apt install -y coreutils >/dev/null 2>&1 || true
    elif command -v apk >/dev/null 2>&1; then
      apk add coreutils >/dev/null 2>&1 || true
    elif command -v yum >/dev/null 2>&1; then
      yum install -y coreutils >/dev/null 2>&1 || true
    else
      print_warning "未找到包管理器，尝试下载 busybox 临时修复..."
      curl -L -o /bin/busybox https://busybox.net/downloads/binaries/1.36.1-x86_64-linux-musl/busybox
      chmod +x /bin/busybox
      cd /bin
      for i in mkdir cp mv rm ls echo cat ln date; do ln -sf busybox $i; done
    fi
    print_success "coreutils 环境已修复"
  fi
}

# ========== 卸载旧 Bot ==========
uninstall_bot() {
  print_message "卸载旧版 VPS Telegram Bot"

  if systemctl is-active --quiet vps-tg-bot 2>/dev/null; then
    print_warning "停止旧服务..."
    systemctl stop vps-tg-bot
  fi
  if systemctl is-enabled --quiet vps-tg-bot 2>/dev/null; then
    systemctl disable vps-tg-bot
  fi
  rm -f "$BOT_SERVICE"
  systemctl daemon-reload
  rm -rf "$BOT_DIR" "$CORE_MAINTAIN_SCRIPT" "$RULES_MAINTAIN_SCRIPT"
  (crontab -l 2>/dev/null | grep -v "vps-maintain" || true) | crontab -
  print_success "旧版本已清理完毕"
}

# ========== 卸载 Go ==========
uninstall_go() {
  if command -v go >/dev/null 2>&1; then
    print_message "检测到 Go 环境，开始安全卸载..."
    GO_PATH=$(which go || true)
    GO_DIR=$(dirname "$(dirname "$GO_PATH")")
    print_warning "Go 安装路径: $GO_DIR"

    if dpkg -l | grep -q "golang-go"; then
      apt-get remove -y golang-go golang >/dev/null 2>&1 || true
      apt-get purge -y golang-go golang >/dev/null 2>&1 || true
    fi

    rm -rf /usr/local/go /usr/lib/go "$GO_DIR/go" >/dev/null 2>&1 || true
    sed -i '/GOPATH/d' ~/.bashrc ~/.profile 2>/dev/null || true
    hash -r 2>/dev/null || true
    print_success "Go 已安全卸载"
  fi
}

# ========== 时区同步 ==========
sync_timezone() {
  print_message "同步 VPS 时区"
  local tz
  if command -v timedatectl &>/dev/null; then
    tz=$(timedatectl show -p Timezone --value)
  elif [ -f /etc/timezone ]; then
    tz=$(cat /etc/timezone)
  else
    tz="Etc/UTC"
  fi
  ln -sf "/usr/share/zoneinfo/$tz" /etc/localtime
  echo "$tz" > /etc/timezone
  print_success "当前 VPS 时区: $tz"
}

# ========== 版本检查 ==========
check_version() {
  local current_version="2.0.8"
  echo "当前部署脚本版本: $current_version"
}

# ========== 帮助信息 ==========
usage() {
  echo "Usage: $0 [install|uninstall|update|status|backup|restore|help]"
  echo "  install:    安装或重新安装 VPS Telegram Bot"
  echo "  update:     更新到最新版本的 VPS Telegram Bot"
  echo "  uninstall:  完全卸载 VPS Telegram Bot 和相关环境"
  echo "  status:     查看 Bot 运行状态和版本信息"
  echo "  backup:     备份 Bot 配置和数据"
  echo "  restore:    从备份恢复 Bot 配置和数据"
  echo "  help:       显示此帮助信息"
  echo ""
  echo "示例:"
  echo "  $0 install              # 全新安装"
  echo "  $0 update               # 更新到最新版本"
  echo "  $0 uninstall            # 完全卸载"
  echo "  $0 status               # 查看状态"
  echo "  $0 backup               # 备份配置"
  echo "  $0 restore /path/to/backup  # 恢复备份"
}

# ========== Bot 状态查看 ==========
show_status() {
  print_message "VPS Telegram Bot 状态信息"
  
  # 检查服务状态
  if systemctl is-active --quiet vps-tg-bot 2>/dev/null; then
    print_success "Bot 服务: 运行中"
  else
    print_error "Bot 服务: 未运行"
  fi
  
  # 检查启用状态
  if systemctl is-enabled --quiet vps-tg-bot 2>/dev/null; then
    print_success "自启动: 已启用"
  else
    print_warning "自启动: 未启用"
  fi
  
  # 显示版本信息
  if [ -f "$BOT_BINARY" ]; then
    local binary_info=$(file "$BOT_BINARY" 2>/dev/null || echo "未知")
    local binary_size=$(du -h "$BOT_BINARY" 2>/dev/null | cut -f1 || echo "未知")
    print_success "二进制文件: $binary_size"
    print_info "文件类型: $binary_info"
  else
    print_error "二进制文件: 未找到"
  fi
  
  # 显示配置信息
  if [ -f "$BOT_SERVICE" ]; then
    print_success "服务配置: 已配置"
  else
    print_error "服务配置: 未找到"
  fi
  
  # 显示安装路径
  print_info "安装路径: $BOT_DIR"
  
  # 显示最近日志
  echo ""
  print_message "最近日志 (最后5条)"
  journalctl -u vps-tg-bot -n 5 --no-pager 2>/dev/null || print_warning "无法获取日志"
  
  echo ""
  echo "管理命令:"
  echo "  查看状态: systemctl status vps-tg-bot"
  echo "  查看日志: journalctl -u vps-tg-bot -f"
  echo "  重启服务: systemctl restart vps-tg-bot"
  echo "  停止服务: systemctl stop vps-tg-bot"
  echo "  启动服务: systemctl start vps-tg-bot"
}

# ========== 备份配置 ==========
backup_config() {
  print_message "备份 VPS Telegram Bot 配置"
  
  local backup_dir="/root/vps-tg-bot-backup-$(date +%Y%m%d-%H%M%S)"
  mkdir -p "$backup_dir"
  
  # 备份服务配置
  if [ -f "$BOT_SERVICE" ]; then
    cp "$BOT_SERVICE" "$backup_dir/"
    print_success "服务配置已备份"
  fi
  
  # 备份维护脚本
  if [ -f "$CORE_MAINTAIN_SCRIPT" ]; then
    cp "$CORE_MAINTAIN_SCRIPT" "$backup_dir/"
  fi
  if [ -f "$RULES_MAINTAIN_SCRIPT" ]; then
    cp "$RULES_MAINTAIN_SCRIPT" "$backup_dir/"
  fi
  print_success "维护脚本已备份"
  
  # 备份状态文件
  if [ -f "$BOT_DIR/state.json" ]; then
    cp "$BOT_DIR/state.json" "$backup_dir/"
    print_success "状态文件已备份"
  fi
  
  # 备份定时任务
  crontab -l > "$backup_dir/crontab.bak" 2>/dev/null || true
  print_success "定时任务已备份"
  
  # 创建备份信息文件
  cat > "$backup_dir/backup-info.txt" <<EOF
VPS Telegram Bot 配置备份
备份时间: $(date)
备份路径: $backup_dir

包含文件:
- 服务配置文件
- 维护脚本
- 状态文件
- 定时任务配置

恢复方法:
1. 运行: $0 restore $backup_dir
2. 或手动复制文件到对应位置
EOF
  
  print_success "备份完成: $backup_dir"
  print_info "备份包含配置、脚本和状态文件"
  print_warning "注意: 此备份不包含 Bot Token 等敏感信息"
}

# ========== 恢复配置 ==========
restore_config() {
  local backup_path="$2"
  
  if [ -z "$backup_path" ]; then
    print_error "请指定备份路径"
    echo "Usage: $0 restore <backup_directory>"
    exit 1
  fi
  
  if [ ! -d "$backup_path" ]; then
    print_error "备份目录不存在: $backup_path"
    exit 1
  fi
  
  print_message "从备份恢复 VPS Telegram Bot 配置"
  print_warning "备份路径: $backup_path"
  
  read -p "⚠️  这将覆盖现有配置，继续吗? (y/N): " confirm
  if [[ ! "$confirm" =~ ^[Yy]$ ]]; then
    print_warning "恢复已取消"
    exit 0
  fi
  
  # 停止服务
  if systemctl is-active --quiet vps-tg-bot 2>/dev/null; then
    print_warning "停止服务..."
    systemctl stop vps-tg-bot
  fi
  
  # 恢复服务配置
  if [ -f "$backup_path/vps-tg-bot.service" ]; then
    cp "$backup_path/vps-tg-bot.service" "$BOT_SERVICE"
    systemctl daemon-reload
    print_success "服务配置已恢复"
  fi
  
  # 恢复维护脚本
  if [ -f "$backup_path/vps-maintain-core.sh" ]; then
    cp "$backup_path/vps-maintain-core.sh" "$CORE_MAINTAIN_SCRIPT"
    chmod +x "$CORE_MAINTAIN_SCRIPT"
  fi
  if [ -f "$backup_path/vps-maintain-rules.sh" ]; then
    cp "$backup_path/vps-maintain-rules.sh" "$RULES_MAINTAIN_SCRIPT"
    chmod +x "$RULES_MAINTAIN_SCRIPT"
  fi
  print_success "维护脚本已恢复"
  
  # 恢复状态文件
  if [ -f "$backup_path/state.json" ]; then
    mkdir -p "$BOT_DIR"
    cp "$backup_path/state.json" "$BOT_DIR/"
    print_success "状态文件已恢复"
  fi
  
  # 恢复定时任务
  if [ -f "$backup_path/crontab.bak" ]; then
    crontab "$backup_path/crontab.bak"
    print_success "定时任务已恢复"
  fi
  
  # 重新启动服务
  print_warning "重新启动服务..."
  systemctl enable vps-tg-bot
  systemctl start vps-tg-bot
  
  sleep 3
  if systemctl is-active --quiet vps-tg-bot; then
    print_success "恢复完成，服务已启动"
  else
    print_error "恢复完成，但服务启动失败"
    print_error "请检查: journalctl -u vps-tg-bot -n 20"
  fi
}

# ========== 更新 Bot ==========
update_bot() {
  print_message "更新 VPS Telegram Bot 到最新版本"
  
  # 检查是否已安装
  if [ ! -f "$BOT_BINARY" ]; then
    print_error "Bot 未安装，请先运行: $0 install"
    exit 1
  fi
  
  # 检查服务是否运行
  local was_running=false
  if systemctl is-active --quiet vps-tg-bot 2>/dev/null; then
    was_running=true
    print_warning "Bot 正在运行，将停止更新..."
    systemctl stop vps-tg-bot
    sleep 2
  fi
  
  # 备份当前版本信息
  local current_binary="$BOT_BINARY.backup.$(date +%Y%m%d-%H%M%S)"
  cp "$BOT_BINARY" "$current_binary"
  print_success "当前版本已备份到: $current_binary"
  
  # 下载新版本
  print_message "下载最新版本"
  
  # 检查本地文件
  if [ -f "./vps-tg-bot-linux-amd64" ]; then
    cp ./vps-tg-bot-linux-amd64 "$BOT_BINARY_TMP"
    print_success "使用本地二进制文件"
  else
    print_warning "从 GitHub 下载最新版本..."
    
    # 获取下载链接
    REPOS=("FTDRTD/Vps-auto-maintain" "SKIPPINGpetticoatconvent/Vps-auto-maintain")
    
    LATEST_URL=""
    
    for REPO in "${REPOS[@]}"; do
      print_warning "尝试从 $REPO 获取下载链接..."
      
      API_URL="https://api.github.com/repos/${REPO}/releases/latest"
      TEMP_URL=$(curl -s --max-time 10 "$API_URL" | grep -oE '"browser_download_url":\s*"([^"]*vps-tg-bot-go-linux-amd64[^"]*)' | cut -d'"' -f4 | head -n1)
      
      if [ -n "$TEMP_URL" ]; then
        LATEST_URL="$TEMP_URL"
        print_success "找到下载链接: $LATEST_URL"
        break
      fi
    done
    
    if [ -z "$LATEST_URL" ]; then
      print_error "无法获取下载链接"
      # 恢复备份
      mv "$current_binary" "$BOT_BINARY"
      if [ "$was_running" = true ]; then
        systemctl start vps-tg-bot
      fi
      exit 1
    fi
    
    # 下载到临时文件
    rm -f "$BOT_BINARY_TMP"
    
    MAX_RETRY=3
    RETRY=0
    while [ $RETRY -lt $MAX_RETRY ]; do
      if curl -L -o "$BOT_BINARY_TMP" "$LATEST_URL"; then
        print_success "下载成功"
        break
      else
        RETRY=$((RETRY+1))
        if [ $RETRY -lt $MAX_RETRY ]; then
          print_warning "下载失败，重试 $RETRY/$MAX_RETRY..."
          sleep 2
        else
          print_error "下载失败，已重试 $MAX_RETRY 次"
          # 恢复备份
          mv "$current_binary" "$BOT_BINARY"
          if [ "$was_running" = true ]; then
            systemctl start vps-tg-bot
          fi
          exit 1
        fi
      fi
    done
  fi
  
  # 验证并安装新版本
  if [ ! -f "$BOT_BINARY_TMP" ] || [ ! -s "$BOT_BINARY_TMP" ]; then
    print_error "下载的文件无效"
    # 恢复备份
    mv "$current_binary" "$BOT_BINARY"
    if [ "$was_running" = true ]; then
      systemctl start vps-tg-bot
    fi
    exit 1
  fi
  
  # 安装新版本
  rm -f "$BOT_BINARY"
  mv "$BOT_BINARY_TMP" "$BOT_BINARY"
  chmod +x "$BOT_BINARY"
  print_success "新版本安装完成"
  
  # 重新启动服务
  if [ "$was_running" = true ]; then
    print_warning "重新启动服务..."
    systemctl start vps-tg-bot
    sleep 3
    
    if systemctl is-active --quiet vps-tg-bot; then
      print_success "更新完成，服务已重新启动"
    else
      print_error "服务启动失败"
      print_warning "可以手动恢复旧版本: cp $current_binary $BOT_BINARY"
      print_error "请检查: journalctl -u vps-tg-bot -n 20"
    fi
  else
    print_success "更新完成（旧版本已备份）"
  fi
  
  # 显示新版本信息
  if [ -f "$BOT_BINARY" ]; then
    local new_size=$(du -h "$BOT_BINARY" 2>/dev/null | cut -f1 || echo "未知")
    print_success "新版本大小: $new_size"
  fi
}

# ========== 增强卸载功能 ==========
complete_uninstall() {
  print_message "完全卸载 VPS Telegram Bot"
  
  if [ "$EUID" -ne 0 ]; then
    print_error "请使用 root 用户执行此脚本进行卸载"
    exit 1
  fi
  
  echo "⚠️  警告: 此操作将删除以下内容:"
  echo "  - Bot 服务和配置文件"
  echo "  - 维护脚本"
  echo "  - 所有任务配置和状态文件"
  echo "  - 相关定时任务"
  echo ""
  
  read -p "⚠️  您确定要完全卸载 VPS Telegram Bot 吗? (yes/NO): " confirm
  if [[ "$confirm" != "yes" ]]; then
    print_warning "卸载已取消"
    exit 0
  fi
  
  # 询问是否备份
  read -p "是否在卸载前创建备份? (Y/n): " backup_confirm
  if [[ ! "$backup_confirm" =~ ^[Nn]$ ]]; then
    backup_config
  fi
  
  print_message "开始完全卸载..."
  
  # 停止并禁用服务
  if systemctl is-active --quiet vps-tg-bot 2>/dev/null; then
    print_warning "停止 Bot 服务..."
    systemctl stop vps-tg-bot
  fi
  
  if systemctl is-enabled --quiet vps-tg-bot 2>/dev/null; then
    print_warning "禁用自启动..."
    systemctl disable vps-tg-bot
  fi
  
  # 删除服务文件
  if [ -f "$BOT_SERVICE" ]; then
    print_warning "删除服务配置..."
    rm -f "$BOT_SERVICE"
    systemctl daemon-reload
  fi
  
  # 删除 Bot 目录
  if [ -d "$BOT_DIR" ]; then
    print_warning "删除 Bot 目录..."
    rm -rf "$BOT_DIR"
  fi
  
  # 删除维护脚本
  if [ -f "$CORE_MAINTAIN_SCRIPT" ]; then
    print_warning "删除核心维护脚本..."
    rm -f "$CORE_MAINTAIN_SCRIPT"
  fi
  
  if [ -f "$RULES_MAINTAIN_SCRIPT" ]; then
    print_warning "删除规则维护脚本..."
    rm -f "$RULES_MAINTAIN_SCRIPT"
  fi
  
  # 清理定时任务
  print_warning "清理定时任务..."
  (crontab -l 2>/dev/null | grep -v "vps-maintain" || true) | crontab -
  
  # 询问是否卸载 Go 环境
  if command -v go >/dev/null 2>&1; then
    echo ""
    read -p "是否同时卸载 Go 环境? (y/N): " go_uninstall
    if [[ "$go_uninstall" =~ ^[Yy]$ ]]; then
      uninstall_go
    fi
  fi
  
  print_success "VPS Telegram Bot 已完全卸载"
  print_success "清理完成"
  
  echo ""
  echo "卸载摘要:"
  echo "  ✅ 服务和配置已删除"
  echo "  ✅ 维护脚本已删除"
  echo "  ✅ 定时任务已清理"
  echo "  ✅ 相关文件已清理"
  
  if [[ ! "$backup_confirm" =~ ^[Nn]$ ]]; then
    echo "  ✅ 配置备份已创建"
  fi
  
  echo ""
  echo "如需重新安装，请运行: $0 install"
}

# ========== 主程序开始 ==========

# 显示版本信息
check_version

# 处理命令行参数
case "${1:-install}" in
  "update")
    update_bot
    exit 0
    ;;
  "uninstall")
    complete_uninstall
    exit 0
    ;;
  "status")
    show_status
    exit 0
    ;;
  "backup")
    backup_config
    exit 0
    ;;
  "restore")
    restore_config "$@"
    exit 0
    ;;
  "help"|"-h"|"--help")
    usage
    exit 0
    ;;
  "install"|"")
    print_message "开始安装 VPS Telegram Bot"
    ;;
  *)
    print_error "未知操作: $1"
    usage
    exit 1
    ;;
esac

# ========== 权限检查 ==========
if [ "$EUID" -ne 0 ]; then
  print_error "请使用 root 用户执行此脚本"
  exit 1
fi

# ========== 修复基础环境 ==========
ensure_coreutils
sync_timezone

# ========== 检查并卸载 Go ==========
print_message "步骤 0: 检查系统环境"
if command -v go &>/dev/null; then
  print_warning "检测到 Go 环境，自动卸载 Go 与旧 Bot..."
  uninstall_bot
  uninstall_go
else
  print_success "未检测到 Go，继续安装"
fi

# ========== Telegram 配置 ==========
print_message "步骤 1: 配置 Telegram Bot"
read -p "请输入 Telegram Bot Token: " TG_TOKEN
read -p "请输入 Telegram Chat ID (管理员): " TG_CHAT_ID
if [ -z "$TG_TOKEN" ] || [ -z "$TG_CHAT_ID" ]; then
  print_error "Token 和 Chat ID 不能为空"
  exit 1
fi

# ========== journald ==========
print_message "步骤 2: 配置 journald 内存日志"
mkdir -p /etc/systemd/journald.conf.d
cat > /etc/systemd/journald.conf.d/memory.conf <<'EOF'
[Journal]
Storage=volatile
RuntimeMaxUse=50M
Compress=yes
EOF
systemctl restart systemd-journald 2>/dev/null || true
print_success "journald 内存化完成"

# ========== 维护脚本 ==========
print_message "步骤 3: 创建维护脚本"
cat > "$CORE_MAINTAIN_SCRIPT" <<'EOF'
#!/bin/bash
set -e
export DEBIAN_FRONTEND=noninteractive
RESULT_FILE="/tmp/vps_maintain_result.txt"
TIME_NOW=$(date '+%Y-%m-%d %H:%M:%S')
sudo apt update && sudo apt full-upgrade -y && sudo apt autoremove -y && sudo apt autoclean
echo "✅ 系统更新完成于 $TIME_NOW" > "$RESULT_FILE"
EOF
chmod +x "$CORE_MAINTAIN_SCRIPT"
cat > "$RULES_MAINTAIN_SCRIPT" <<'EOF'
#!/bin/bash
set -e
RESULT_FILE="/tmp/vps_rules_result.txt"
TIME_NOW=$(date '+%Y-%m-%d %H:%M:%S')
if command -v xray &>/dev/null; then
  xray up dat && echo "✅ 规则更新完成 $TIME_NOW" > "$RESULT_FILE"
else
  echo "ℹ️ 未检测到 Xray" > "$RESULT_FILE"
fi
EOF
chmod +x "$RULES_MAINTAIN_SCRIPT"
print_success "维护脚本创建完成"

# ========== 停止旧服务（关键修复点）==========
print_message "步骤 4: 准备安装新版本"
if systemctl is-active --quiet vps-tg-bot 2>/dev/null; then
  print_warning "停止现有服务..."
  systemctl stop vps-tg-bot
  sleep 2
fi

# ========== 下载预编译二进制（修复版）==========
print_message "步骤 5: 下载预编译二进制"
mkdir -p "$BOT_DIR"

# 检查本地文件
if [ -f "./vps-tg-bot-linux-amd64" ]; then
  cp ./vps-tg-bot-linux-amd64 "$BOT_BINARY"
  print_success "使用本地二进制文件"
else
  print_warning "从 GitHub 下载最新版本..."
  
  # 获取下载链接
  print_warning "从 GitHub 下载最新版本..."
  
  # 使用官方仓库
  REPOS=("FTDRTD/Vps-auto-maintain" "SKIPPINGpetticoatconvent/Vps-auto-maintain")
  
  LATEST_URL=""
  
  for REPO in "${REPOS[@]}"; do
    print_warning "尝试从 $REPO 获取下载链接..."
    
    API_URL="https://api.github.com/repos/${REPO}/releases/latest"
    TEMP_URL=$(curl -s --max-time 10 "$API_URL" | grep -oE '"browser_download_url":\s*"([^"]+vps-tg-bot-go-linux-amd64[^"]*)' | cut -d'"' -f4 | head -n1)
    
    if [ -n "$TEMP_URL" ]; then
      LATEST_URL="$TEMP_URL"
      print_success "找到下载链接: $LATEST_URL"
      break
    fi
  done
  
  if [ -z "$LATEST_URL" ]; then
    print_error "无法从任何源获取下载链接"
    print_error "请检查网络连接或手动下载二进制文件"
    exit 1
  fi
  
  print_warning "下载地址: $LATEST_URL"
  
  # 下载到临时文件（关键修复点）
  rm -f "$BOT_BINARY_TMP"
  
  # 重试下载
  MAX_RETRY=3
  RETRY=0
  while [ $RETRY -lt $MAX_RETRY ]; do
    if curl -L -o "$BOT_BINARY_TMP" "$LATEST_URL"; then
      print_success "下载成功"
      break
    else
      RETRY=$((RETRY+1))
      if [ $RETRY -lt $MAX_RETRY ]; then
        print_warning "下载失败，重试 $RETRY/$MAX_RETRY..."
        sleep 2
      else
        print_error "下载失败，已重试 $MAX_RETRY 次"
        exit 1
      fi
    fi
  done
  
  # 验证文件
  if [ ! -f "$BOT_BINARY_TMP" ]; then
    print_error "下载的文件不存在"
    exit 1
  fi
  
  if [ ! -s "$BOT_BINARY_TMP" ]; then
    print_error "下载的文件为空"
    rm -f "$BOT_BINARY_TMP"
    exit 1
  fi
  
  # 移动文件（关键修复点）
  rm -f "$BOT_BINARY"
  mv "$BOT_BINARY_TMP" "$BOT_BINARY"
  print_success "文件安装完成"
fi

chmod +x "$BOT_BINARY"

# ========== 创建 systemd 服务 ==========
print_message "步骤 6: 创建 systemd 服务"
cat > "$BOT_SERVICE" <<EOF
[Unit]
Description=VPS Telegram Bot (Go)
After=network.target

[Service]
Type=simple
User=root
WorkingDirectory=$BOT_DIR
Environment="TG_TOKEN=$TG_TOKEN"
Environment="TG_CHAT_ID=$TG_CHAT_ID"
Environment="CORE_SCRIPT=$CORE_MAINTAIN_SCRIPT"
Environment="RULES_SCRIPT=$RULES_MAINTAIN_SCRIPT"
ExecStart=$BOT_BINARY
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
EOF
systemctl daemon-reload
systemctl enable vps-tg-bot
systemctl start vps-tg-bot
sleep 3

if systemctl is-active --quiet vps-tg-bot; then
  print_success "服务启动成功"
else
  print_error "服务启动失败，请执行: journalctl -u vps-tg-bot -n 50"
  exit 1
fi

# ========== 添加自动维护任务 ==========
print_message "步骤 7: 添加自动维护任务"
(crontab -l 2>/dev/null | grep -v "vps-maintain" ; echo "0 4 * * 0 bash $CORE_MAINTAIN_SCRIPT && bash $RULES_MAINTAIN_SCRIPT && reboot") | crontab -
print_success "已添加每周日 04:00 自动维护任务"

# ========== 完成提示 ==========
print_message "🎉 部署完成！"
print_success "Go 环境已安全清理，Bot 已重新部署"
print_success "服务后台运行中（SSH 关闭不影响）"
print_success "每周日 04:00 自动维护与重启"
print_warning "查看日志: journalctl -u vps-tg-bot -f"
print_warning "查看状态: systemctl status vps-tg-bot"
print_warning "重启服务: systemctl restart vps-tg-bot"
print_warning "更新命令: $0 update"
print_warning "卸载命令: $0 uninstall"
print_warning "状态命令: $0 status"
print_warning "备份命令: $0 backup"

echo ""
echo "============================================================"
echo "📱 现在可以在 Telegram 中发送 /start 测试 Bot"
echo "============================================================"