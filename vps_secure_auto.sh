#!/bin/bash
# =====================================================================
# 🧩 VPS 终极安全与自动维护脚本 (V4.1 完全版)
# 适配: Debian / Ubuntu / Rocky / AlmaLinux / xeefei X-Panel
# 作者: FTDRTD (融合/增强)
#
# 主要特性:
#   ✅ 防火墙(UFW/Firewalld)自动安装与配置 (含 IPv6)
#   ✅ Fail2Ban 自动动作检测 + 三档模式(普通/激进/偏执)
#   ✅ sshd jail 自修复 (过滤器/服务名/缺文件的一般性修复)
#   ✅ Xray / Sing-box / X-Panel 端口自动探测并放行
#   ✅ Telegram 完成通知 + 实时封禁/解封通知(可选)
#   ✅ 无人值守安全更新 + 03:00 自动重启
#   ✅ systemd 日志内存化 (Storage=volatile, 限额/保留期)
#   ✅ --status / --uninstall 辅助参数
# =====================================================================

set -euo pipefail

# --------------------- 配色/输出 ---------------------
C_RESET="\033[0m"; C_RED="\033[31m"; C_GRN="\033[32m"; C_YLW="\033[33m"; C_CYA="\033[36m"
info () { echo -e "${C_CYA}[INFO]${C_RESET} $*"; }
ok   () { echo -e "${C_GRN}[ OK ]${C_RESET} $*"; }
warn () { echo -e "${C_YLW}[WARN]${C_RESET} $*"; }
err  () { echo -e "${C_RED}[FAIL]${C_RESET} $*"; }

print_message () {
  echo -e "\n------------------------------------------------------------"
  echo -e "$1"
  echo -e "------------------------------------------------------------"
}

# --------------------- 变量/全局 ---------------------
NOTIFY=${NOTIFY:-false}
TG_TOKEN="${TG_TOKEN:-}"
TG_CHAT_ID="${TG_CHAT_ID:-}"
FAIL2BAN_MODE="未选择"
FIREWALL_TYPE="none"

# --------------------- Telegram ----------------------
send_telegram () {
  if [ "$NOTIFY" = true ] && [ -n "${TG_TOKEN:-}" ] && [ -n "${TG_CHAT_ID:-}" ]; then
    local message="$1"
    # 兼容 MarkdownV2 特殊字符
    message=$(echo "$message" | sed 's/\\/\\\\/g; s/\./\\./g; s/-/\\-/g; s/!/\\!/g; s/\(/\\(/g; s/\)/\\)/g; s/\[/\\[/g; s/\]/\\]/g; s/\{/\\{/g; s/\}/\\}/g; s/\*/\\*/g; s/_/\\_/g; s/`/\\`/g; s/</\\</g; s/>/\\>/g; s/#/\\#/g; s/\+/\\+/g; s/=/\\=/g; s/\|/\\|/g; s/\^/\\^/g; s/\$/\\$/g')
    curl --connect-timeout 10 --retry 3 -s -X POST \
      "https://api.telegram.org/bot$TG_TOKEN/sendMessage" \
      -d chat_id="$TG_CHAT_ID" -d text="$message" -d parse_mode="MarkdownV2" >/dev/null || true
  fi
}

# Fail2Ban action: Telegram 实时通知
install_f2b_tg_action () {
  cat >/etc/fail2ban/action.d/tg-notify.conf <<'EOF'
[Definition]
actionstart = 
actionstop  = 
actioncheck = 
actionban   = curl -s -X POST "https://api.telegram.org/bot<bot_token>/sendMessage" \
              -d chat_id="<chat_id>" \
              -d parse_mode="MarkdownV2" \
              --data-urlencode text="🚫 *Fail2Ban* 已封禁: *<name>*\nIP: `<ip>`\nJail: `<jail>`\nTime: <timestamp>"
actionunban = curl -s -X POST "https://api.telegram.org/bot<bot_token>/sendMessage" \
              -d chat_id="<chat_id>" \
              -d parse_mode="MarkdownV2" \
              --data-urlencode text="✅ *Fail2Ban* 已解封: *<name>*\nIP: `<ip>`\nJail: `<jail>`\nTime: <timestamp>"

[Init]
timestamp = %(now)s
# 运行时将以参数填充下列变量
bot_token = 
chat_id   = 
EOF
}

# --------------------- 前置检查 ----------------------
need_root () { [ "$(id -u)" -eq 0 ] || { err "请以 root 运行"; exit 1; }; }

install_pkg_if_missing () {
  local pkg="$1"
  if ! command -v "$pkg" >/dev/null 2>&1; then
    if command -v apt-get >/dev/null 2>&1; then
      apt-get update -y >/dev/null 2>&1 || true
      apt-get install -y "$pkg" >/dev/null 2>&1 || true
    elif command -v dnf >/dev/null 2>&1; then
      dnf install -y "$pkg" >/dev/null 2>&1 || true
    elif command -v yum >/dev/null 2>&1; then
      yum install -y "$pkg" >/dev/null 2>&1 || true
    fi
  fi
}

# --------------------- 防火墙 -------------------------
detect_firewall () {
  if systemctl is-active --quiet firewalld 2>/dev/null; then
    echo "firewalld"; return
  fi
  if command -v ufw >/dev/null 2>&1 && ufw status 2>/dev/null | grep -q "Status: active"; then
    echo "ufw"; return
  fi
  echo "none"
}

setup_firewall () {
  print_message "安装并启用防火墙"
  . /etc/os-release
  if [[ "${ID}" =~ (debian|ubuntu) || "${ID_LIKE:-}" =~ debian ]]; then
    install_pkg_if_missing ufw
    echo "y" | ufw reset >/dev/null 2>&1
    ufw default deny incoming >/dev/null 2>&1
    ufw default allow outgoing >/dev/null 2>&1
    ufw --force enable >/dev/null 2>&1
    FIREWALL_TYPE="ufw"
  else
    if ! command -v firewall-cmd >/dev/null 2>&1; then
      (dnf install -y firewalld || yum install -y firewalld) >/dev/null 2>&1 || true
    fi
    systemctl enable --now firewalld >/dev/null 2>&1 || true
    FIREWALL_TYPE="firewalld"
  fi
  ok "防火墙已启用: ${FIREWALL_TYPE}"
}

apply_firewall_rules () {
  local ports_to_keep="$1"
  print_message "应用新的防火墙规则"
  read -ra ports <<<"$ports_to_keep"

  if [ "$FIREWALL_TYPE" = "ufw" ]; then
    echo "y" | ufw reset >/dev/null 2>&1
    ufw default deny incoming >/dev/null 2>&1
    ufw default allow outgoing >/dev/null 2>&1
    for p in "${ports[@]}"; do ufw allow "$p" >/dev/null 2>&1; done  # 同步 v4/v6
    ufw --force enable >/dev/null 2>&1
    ufw status | grep ALLOW || true
  elif [ "$FIREWALL_TYPE" = "firewalld" ]; then
    # 清理旧端口并添加新端口 (TCP/UDP)
    local exist; exist=$(firewall-cmd --list-ports || true)
    for pp in $exist; do firewall-cmd --permanent --remove-port="$pp" >/dev/null 2>&1 || true; done
    for p in "${ports[@]}"; do
      firewall-cmd --permanent --add-port="${p}/tcp" >/dev/null 2>&1 || true
      firewall-cmd --permanent --add-port="${p}/udp" >/dev/null 2>&1 || true
    done
    firewall-cmd --reload >/dev/null 2>&1 || true
    firewall-cmd --list-ports || true
  else
    warn "未检测到有效防火墙，跳过规则应用。"
  fi
}

# --------------------- Fail2Ban -----------------------
detect_banaction () {
  local t="$1"
  if [ "$t" = "ufw" ]; then
    if   [ -f /etc/fail2ban/action.d/ufw-allports.conf ]; then echo "ufw-allports"
    elif [ -f /etc/fail2ban/action.d/ufw.conf ]; then echo "ufw"
    else echo "iptables-allports"; fi
  elif [ "$t" = "firewalld" ]; then
    if [ -f /etc/fail2ban/action.d/firewallcmd-ipset.conf ]; then echo "firewallcmd-ipset"
    else echo "iptables-allports"; fi
  else
    echo "iptables-allports"
  fi
}

selfheal_sshd_jail () {
  # 解决因缺过滤器/命名不一致导致的 "sshd does not exist"
  if [ ! -f /etc/fail2ban/filter.d/sshd.conf ]; then
    warn "缺少 sshd 过滤器，自动创建最小可用版本。"
    cat >/etc/fail2ban/filter.d/sshd.conf <<'EOF'
[Definition]
failregex = ^<HOST> .* sshd\[.*\]: (error: PAM: )?Authentication failure
            ^<HOST> .* sshd\[.*\]: Failed [a-zA-Z ]+ for .* from <HOST>
ignoreregex =
EOF
  fi
  # Debian 服务名为 ssh，但 jail 名仍为 sshd，确保可用
  systemctl status ssh >/dev/null 2>&1 || systemctl status sshd >/dev/null 2>&1 || true
}

setup_fail2ban () {
  print_message "配置 Fail2Ban (SSH 防护)"
  install_pkg_if_missing fail2ban

  local banaction; banaction=$(detect_banaction "$FIREWALL_TYPE")
  info "Fail2Ban 动作: ${banaction}"

  # 模式选择
  echo -e "请选择 Fail2Ban SSH 防护模式:
  1) 普通：5次失败封禁10分钟
  2) 激进：3次失败封禁1小时（递增）[推荐]
  3) 偏执：2次失败封禁12小时（递增×3）"
  read -rp "请输入选项 [1-3]，默认 2: " mode; mode=${mode:-2}

  local bantime="1h" maxretry="3" findtime="10m" inc_factor="2" inc_max="1w"
  case "$mode" in
    1) FAIL2BAN_MODE="普通";    bantime="10m"; maxretry="5"; findtime="10m"; inc_factor="2"; inc_max="1d" ;;
    2) FAIL2BAN_MODE="激进";    bantime="1h";  maxretry="3"; findtime="10m"; inc_factor="2"; inc_max="1w" ;;
    3) FAIL2BAN_MODE="偏执";    bantime="12h"; maxretry="2"; findtime="10m"; inc_factor="3"; inc_max="2w" ;;
    *) warn "无效输入，使用默认激进模式。"; FAIL2BAN_MODE="激进";;
  esac

  # 安装 Telegram action（若启用通知）
  if [ "$NOTIFY" = true ]; then install_f2b_tg_action; fi

  # 生成 jail.local
  {
    echo "[DEFAULT]"
    echo "banaction = ${banaction}"
    echo "backend = systemd"
    echo "bantime = ${bantime}"
    echo "findtime = ${findtime}"
    echo "maxretry = ${maxretry}"
    echo ""
    echo "[sshd]"
    echo "enabled = true"
    echo "bantime.increment = true"
    echo "bantime.factor = ${inc_factor}"
    echo "bantime.max = ${inc_max}"
    if [ "$NOTIFY" = true ] && [ -n "$TG_TOKEN" ] && [ -n "$TG_CHAT_ID" ]; then
      echo "action = %(action_)s"
      echo "         tg-notify[bot_token=${TG_TOKEN},chat_id=${TG_CHAT_ID}]"
    fi
  } >/etc/fail2ban/jail.local

  selfheal_sshd_jail

  systemctl enable --now fail2ban >/dev/null 2>&1 || true
  systemctl restart fail2ban || true

  # 快速健康检查
  if fail2ban-client status sshd >/dev/null 2>&1; then
    ok "Fail2Ban 已启动，jail: sshd (${FAIL2BAN_MODE})"
  else
    err "Fail2Ban sshd jail 未加载，请检查 /etc/fail2ban/jail.local 与日志。"
  fi
}

# --------------------- 端口探测 ----------------------
detect_ports () {
  local all_ports=""
  # SSH 端口
  local ssh_port; ssh_port=$(grep -iE '^\s*Port ' /etc/ssh/sshd_config | awk '{print $2}' | head -n1)
  [ -z "$ssh_port" ] && ssh_port=22
  info "检测到 SSH 端口: $ssh_port"
  all_ports="$all_ports $ssh_port"

  # Xray
  if command -v xray >/dev/null 2>&1 && pgrep -f "xray" >/dev/null 2>&1; then
    local xray_ports; xray_ports=$(ss -tnlp | awk '/xray/ {print $4}' | awk -F: '{print $NF}' | sort -u)
    [ -n "$xray_ports" ] && info "检测到 Xray 端口: $xray_ports" && all_ports="$all_ports $xray_ports"
  fi
  # Sing-box
  if pgrep -f "sing-box" >/dev/null 2>&1; then
    local sb_ports; sb_ports=$(ss -tnlp | awk '/sing-box/ {print $4}' | awk -F: '{print $NF}' | sort -u)
    [ -n "$sb_ports" ] && info "检测到 Sing-box 端口: $sb_ports" && all_ports="$all_ports $sb_ports"
  fi
  # X-Panel (多路径)
  if pgrep -f "xpanel|x-ui" >/dev/null 2>&1; then
    local db; for db in /etc/x-ui/x-ui.db /etc/xpanel/x-ui.db /usr/local/x-ui/x-ui.db; do
      if [ -f "$db" ]; then
        local xp; xp=$(sqlite3 "$db" "SELECT port FROM inbounds;" 2>/dev/null | grep -E '^[0-9]+$' | sort -u || true)
        [ -n "$xp" ] && info "检测到 X-Panel 入站端口: $xp" && all_ports="$all_ports $xp"
      fi
    done
    info "检测到面板进程，自动放行 80 (证书申请)"
    all_ports="$all_ports 80"
  fi

  # 去重/规整
  echo "$all_ports" | tr ' ' '\n' | grep -E '^[0-9]+$' | sort -u | tr '\n' ' '
}

# --------------------- 自动更新/日志 ------------------
setup_auto_updates () {
  print_message "配置无人值守安全更新"
  if command -v apt-get >/dev/null 2>&1; then
    apt-get update -y >/dev/null 2>&1 || true
    apt-get install -y unattended-upgrades apt-listchanges >/dev/null 2>&1 || true
    cat >/etc/apt/apt.conf.d/20auto-upgrades <<'EOF'
APT::Periodic::Update-Package-Lists "1";
APT::Periodic::Unattended-Upgrade "1";
EOF
    cat >/etc/apt/apt.conf.d/51unattended-upgrades-reboot.conf <<'EOF'
Unattended-Upgrade::Automatic-Reboot "true";
Unattended-Upgrade::Automatic-Reboot-Time "03:00";
EOF
    systemctl enable --now apt-daily.timer >/dev/null 2>&1 || true
    systemctl enable --now apt-daily-upgrade.timer >/devnull 2>&1 || true
    ok "已启用每日安全补丁与自动重启 (03:00)"
  else
    warn "非 Debian/Ubuntu 系，跳过 unattended-upgrades。"
  fi
}

setup_memory_log () {
  print_message "启用内存日志 (journald volatile)"
  mkdir -p /etc/systemd/journald.conf.d
  cat >/etc/systemd/journald.conf.d/volatile.conf <<'EOF'
[Journal]
Storage=volatile
RuntimeMaxUse=10M
MaxRetentionSec=2day
Compress=yes
EOF
  systemctl restart systemd-journald || true
  ok "日志内存化已启用。"
}

# --------------------- 状态/卸载 ---------------------
status_report () {
  print_message "当前安全状态"
  echo "主机名: $(hostname)"
  echo "防火墙: $(detect_firewall)"
  if command -v ufw >/dev/null 2>&1; then ufw status | sed 's/^/  /'; fi
  if command -v firewall-cmd >/dev/null 2>&1; then firewall-cmd --list-ports | sed 's/^/  /'; fi
  if command -v fail2ban-client >/dev/null 2>&1; then
    fail2ban-client status 2>/dev/null || true
    fail2ban-client status sshd 2>/dev/null || true
  fi
  echo "journald: $(grep -E '^(Storage|RuntimeMaxUse|MaxRetentionSec)=' -h /etc/systemd/journald.conf.d/*.conf 2>/dev/null | xargs -I{} echo "  {}")"
  if command -v systemctl >/dev/null 2>&1; then
    echo "APT 计时器:"; systemctl list-timers apt-* --no-pager 2>/dev/null || true
  fi
}

uninstall_all () {
  print_message "还原/卸载（尽力而为，不影响已装业务）"
  # Fail2Ban
  systemctl disable --now fail2ban >/dev/null 2>&1 || true
  rm -f /etc/fail2ban/jail.local /etc/fail2ban/action.d/tg-notify.conf || true
  # journald
  rm -f /etc/systemd/journald.conf.d/volatile.conf || true
  systemctl restart systemd-journald || true
  # unattended-upgrades (保留包，仅停计时器与配置)
  systemctl disable --now apt-daily.timer apt-daily-upgrade.timer >/dev/null 2>&1 || true
  rm -f /etc/apt/apt.conf.d/20auto-upgrades /etc/apt/apt.conf.d/51unattended-upgrades-reboot.conf || true
  ok "卸载/还原步骤完成。"
}

# --------------------- 主流程 ------------------------
main () {
  need_root

  # 参数处理
  case "${1:-}" in
    --status)    status_report; exit 0 ;;
    --uninstall) uninstall_all;  exit 0 ;;
  esac

  # Telegram 开关
  read -rp "是否启用 Telegram 通知？(y/N): " enable_tg
  if [[ "$enable_tg" =~ ^[Yy]$ ]]; then
    read -rp "请输入 Telegram Bot Token: " TG_TOKEN
    read -rp "请输入 Telegram Chat ID: " TG_CHAT_ID
    NOTIFY=true
  else
    NOTIFY=false
  fi

  # 依赖
  install_pkg_if_missing curl
  install_pkg_if_missing sqlite3

  # 防火墙
  FIREWALL_TYPE=$(detect_firewall)
  [ "$FIREWALL_TYPE" = "none" ] && setup_firewall || ok "检测到防火墙: $FIREWALL_TYPE"

  # Fail2Ban
  setup_fail2ban

  # 端口探测与放行
  local keep_ports; keep_ports=$(detect_ports)
  print_message "最终将保留的端口: $keep_ports"
  apply_firewall_rules "$keep_ports"

  # 自动更新 & 内存日志
  setup_auto_updates
  setup_memory_log

  # 最终通知
  local host; host=$(hostname)
  send_telegram "*VPS 安全配置完成*
> *服务器*: \`$host\`
> *防火墙*: \`$FIREWALL_TYPE\`
> *Fail2Ban模式*: \`$FAIL2BAN_MODE\`
> *端口保留*: \`$keep_ports\`
> *日志模式*: \`volatile\`
> *自动重启*: \`03:00\`"

  print_message "🎉 所有安全与维护配置已成功完成！"
  ok "可执行:  \`$(basename "$0") --status\` 查看状态，或 \`$(basename "$0") --uninstall\` 尝试还原"
}

main "$@"
