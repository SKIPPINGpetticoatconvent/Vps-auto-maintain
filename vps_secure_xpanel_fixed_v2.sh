#!/bin/bash
# =============================================================
# 🧩 VPS 一键安全防护脚本 vps_secure_xpanel_fixed_v2.sh
# 适配：Debian / Ubuntu / Rocky / AlmaLinux / xeefei X-Panel
# 作者：FTDRTD | 更新日期：2025-10-30
#
# 🧱 功能:
#   ✅ 自动检测 & 启用防火墙（UFW / Firewalld）
#   ✅ 自动安装并配置 Fail2Ban（三种封禁模式）
#   ✅ 自动检测 SSH / Xray / Sing-box / X-Panel 端口并放行
#   ✅ 自动修复 sshd jail 未加载问题
#   ✅ Telegram 通知可选
# =============================================================

set -Eeuo pipefail

C_GRN="\033[32m"; C_YLW="\033[33m"; C_RED="\033[31m"; C_CYA="\033[36m"; C_RST="\033[0m"
ok(){ echo -e "${C_GRN}[OK]${C_RST} $*"; }
warn(){ echo -e "${C_YLW}[WARN]${C_RST} $*"; }
err(){ echo -e "${C_RED}[ERR]${C_RST} $*"; }
info(){ echo -e "${C_CYA}[INFO]${C_RST} $*"; }

# --- Root 检查 ---
if [ "$(id -u)" -ne 0 ]; then err "请以 root 权限运行"; exit 1; fi

# --- Telegram 通知 ---
read -p "是否启用 Telegram 通知？(y/N): " enable_tg
if [[ "$enable_tg" =~ ^[Yy]$ ]]; then
  read -p "请输入 Telegram Bot Token: " TG_TOKEN
  read -p "请输入 Telegram Chat ID: " TG_CHAT_ID
  NOTIFY=true
else
  NOTIFY=false
fi

send_tg(){
  if [ "$NOTIFY" = true ] && [ -n "${TG_TOKEN:-}" ] && [ -n "${TG_CHAT_ID:-}" ]; then
    local msg="$1"
    curl -s -X POST "https://api.telegram.org/bot$TG_TOKEN/sendMessage" \
         -d chat_id="$TG_CHAT_ID" -d text="$msg" >/dev/null || true
  fi
}

# --- 安装依赖 ---
install_if_missing(){
  local pkg="$1"
  if ! command -v "$pkg" >/dev/null 2>&1; then
    apt-get update -y >/dev/null 2>&1
    apt-get install -y "$pkg" >/dev/null 2>&1 || yum install -y "$pkg" >/dev/null 2>&1 || true
  fi
}

install_if_missing curl
install_if_missing sqlite3

# --- 防火墙检测与自动启用 ---
detect_firewall(){
  if systemctl is-active --quiet firewalld 2>/dev/null; then
    echo "firewalld"
  elif command -v ufw &>/dev/null; then
    if ufw status 2>/dev/null | grep -q "Status: active"; then
      echo "ufw"
    else
      info "检测到 UFW 已安装但未启用，自动启用中..."
      ufw default deny incoming >/dev/null 2>&1
      ufw default allow outgoing >/dev/null 2>&1
      ufw allow 22 >/dev/null 2>&1
      yes | ufw enable >/dev/null 2>&1
      ok "UFW 已启用"
      echo "ufw"
    fi
  else
    echo "none"
  fi
}

setup_firewall(){
  local fw="$1"
  if [ "$fw" = "none" ]; then
    info "未检测到防火墙，自动安装并启用 UFW"
    apt install -y ufw >/dev/null 2>&1
    ufw default deny incoming >/dev/null 2>&1
    ufw default allow outgoing >/dev/null 2>&1
    ufw allow 22 >/dev/null 2>&1
    yes | ufw enable >/dev/null 2>&1
    ok "UFW 已安装并启用"
    echo "ufw"
  else
    echo "$fw"
  fi
}

# --- Fail2Ban 配置 ---
setup_fail2ban(){
  local fw="$1"
  install_if_missing fail2ban
  local banaction="iptables-allports"
  [[ "$fw" == "ufw" ]] && banaction="ufw-allports"
  [[ "$fw" == "firewalld" ]] && banaction="firewallcmd-ipset"

  echo -e "请选择 Fail2Ban SSH 防护模式:
  1) 普通模式: 5次失败封禁10分钟
  2) 激进模式: 3次失败封禁1小时（推荐）
  3) 偏执模式: 2次失败封禁12小时（屡教不改×3）"
  read -p "请输入选项 [1-3], 默认 2: " mode
  mode=${mode:-2}

  case "$mode" in
    1) bantime="10m"; maxretry="5"; findtime="10m"; ;;
    2) bantime="1h"; maxretry="3"; findtime="10m"; ;;
    3) bantime="12h"; maxretry="2"; findtime="10m"; ;;
    *) bantime="1h"; maxretry="3"; findtime="10m"; ;;
  esac

  cat >/etc/fail2ban/jail.local <<EOF
[DEFAULT]
banaction = $banaction
backend = systemd
bantime = $bantime
findtime = $findtime
maxretry = $maxretry

[sshd]
enabled = true
bantime.increment = true
bantime.factor = 2
bantime.max = 1w
EOF

  # 修复 sshd jail 缺失过滤器问题
  if [ ! -f /etc/fail2ban/filter.d/sshd.conf ]; then
    cat >/etc/fail2ban/filter.d/sshd.conf <<'EOF'
[Definition]
failregex = ^<HOST> .* sshd\[.*\]: (error: PAM: )?Authentication failure
            ^<HOST> .* sshd\[.*\]: Failed [a-zA-Z ]+ for .* from <HOST>
ignoreregex =
EOF
  fi

  systemctl enable --now fail2ban >/dev/null 2>&1
  systemctl restart fail2ban
  ok "Fail2Ban 已启用"
}

# --- Xray / Sing-box / X-Panel 端口检测 ---
detect_ports(){
  local all_ports="22"
  if command -v xray >/dev/null 2>&1 && pgrep -f "xray" >/dev/null 2>&1; then
    local xports=$(ss -tnlp | awk '/xray/ {print $4}' | awk -F: '{print $NF}' | sort -u)
    [ -n "$xports" ] && all_ports="$all_ports $xports"
  fi
  if pgrep -f "sing-box" >/dev/null 2>&1; then
    local sbox=$(ss -tnlp | awk '/sing-box/ {print $4}' | awk -F: '{print $NF}' | sort -u)
    [ -n "$sbox" ] && all_ports="$all_ports $sbox"
  fi
  for db in /etc/x-ui/x-ui.db /usr/local/x-ui/x-ui.db /etc/xpanel/x-ui.db; do
    [ -f "$db" ] && xp=$(sqlite3 "$db" "SELECT port FROM inbounds;" | grep -E '^[0-9]+$' | sort -u) && all_ports="$all_ports $xp"
  done
  all_ports="$all_ports 80"
  echo "$all_ports" | tr ' ' '\n' | sort -u | tr '\n' ' '
}

# --- 应用防火墙规则 ---
apply_rules(){
  local ports=($1) fw="$2"
  if [ "$fw" = "ufw" ]; then
    echo "y" | ufw reset >/dev/null 2>&1
    ufw default deny incoming >/dev/null 2>&1
    ufw default allow outgoing >/dev/null 2>&1
    for p in "${ports[@]}"; do ufw allow "$p" >/dev/null 2>&1; done
    yes | ufw enable >/dev/null 2>&1
    ok "UFW 已更新规则"
  elif [ "$fw" = "firewalld" ]; then
    for p in "${ports[@]}"; do
      firewall-cmd --permanent --add-port="${p}/tcp" >/dev/null 2>&1
      firewall-cmd --permanent --add-port="${p}/udp" >/dev/null 2>&1
    done
    firewall-cmd --reload >/dev/null 2>&1
    ok "Firewalld 已更新规则"
  else
    warn "未找到有效防火墙工具"
  fi
}

# --- 主流程 ---
main(){
  local fw=$(detect_firewall)
  fw=$(setup_firewall "$fw")
  setup_fail2ban "$fw"

  local ports=$(detect_ports)
  echo -e "\n🧩 最终保留端口:\n$ports"
  apply_rules "$ports" "$fw"

  ok "所有安全配置已应用完成"
  send_tg "✅ VPS 安全配置完成\n防火墙: $fw\n端口: $ports"
}

main
