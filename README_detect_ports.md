# 代理服务端口检测和防火墙配置脚本

## 功能说明

`detect_ports.sh` 是一个自动检测 Xray 和 Sing-box (sb) 开放端口的脚本，能够：

- 自动检测正在运行的 Xray 和 Sing-box 进程
- 获取这些进程监听的端口号
- 检测系统防火墙类型（firewalld 或 ufw）
- 自动添加防火墙规则，允许检测到的端口通过 UDP 和 TCP 流量
- 支持 Telegram 通知功能

## 使用方法

### 基本使用

```bash
sudo ./detect_ports.sh
```

### 带参数使用

```bash
# 不发送 Telegram 通知
sudo ./detect_ports.sh --no-notify

# 指定 Telegram 配置
sudo ./detect_ports.sh --token YOUR_BOT_TOKEN --chat-id YOUR_CHAT_ID

# 组合使用
sudo ./detect_ports.sh --token YOUR_BOT_TOKEN --chat-id YOUR_CHAT_ID --no-notify
```

### 参数说明

- `--no-notify`: 禁用 Telegram 通知
- `--token TOKEN`: 指定 Telegram Bot Token
- `--chat-id ID`: 指定 Telegram Chat ID

## 工作原理

1. **进程检测**: 使用 `pgrep` 检查 Xray 和 Sing-box 是否正在运行
2. **端口检测**: 使用 `ss` 或 `netstat` 命令获取进程监听的端口
3. **防火墙检测**: 检查系统中活跃的防火墙类型
4. **规则添加**: 为检测到的端口添加 TCP 和 UDP 允许规则
5. **通知**: 可选发送 Telegram 消息报告配置结果

## 系统要求

- Linux 操作系统
- `ss` 或 `netstat` 命令（通常预装）
- `curl` 命令（用于 Telegram 通知）
- `sudo` 权限

## 支持的防火墙

- **firewalld**: CentOS/RHEL 系列
- **ufw**: Ubuntu/Debian 系列
- **无防火墙**: 仅显示警告，不进行配置

## 输出示例

```
------------------------------------------------------------
开始检测代理服务端口并配置防火墙
------------------------------------------------------------
🔍 检测防火墙类型: ufw
🕒 系统时区: Asia/Shanghai
🕐 当前时间: 2023-09-16 11:52:00
✅ 检测到 Xray 运行端口: 443 1080
✅ 检测到 Sing-box 运行端口: 2082 2083
✅ 防火墙规则配置完成，已允许相关端口的 UDP/TCP 流量
```

## 集成到现有维护脚本

可以将此脚本集成到 `vps-maintain.sh` 中，在系统更新后自动检测和配置端口：

```bash
# 在 vps-maintain.sh 的末尾添加
echo "🔧 检测代理服务端口..."
/path/to/detect_ports.sh --token YOUR_TOKEN --chat-id YOUR_ID
```

## 注意事项

- 脚本需要 `sudo` 权限来修改防火墙规则
- 如果没有检测到防火墙，脚本会显示警告但不会报错
- Telegram 通知需要有效的 Bot Token 和 Chat ID
- 脚本会自动跳过未运行的服务

## 故障排除

### 端口检测失败

```bash
# 检查进程是否真的在运行
ps aux | grep xray
ps aux | grep sing-box

# 检查端口监听
ss -tlnp | grep xray
ss -tlnp | grep sing-box
```

### 防火墙规则未生效

```bash
# 检查防火墙状态
sudo ufw status
sudo firewall-cmd --list-all

# 手动添加规则（示例）
sudo ufw allow 443/tcp
sudo ufw allow 443/udp
```

### Telegram 通知不发送

- 检查 Bot Token 和 Chat ID 是否正确
- 确认网络能访问 `api.telegram.org`
- 检查防火墙是否阻止了出站连接

## 安全注意事项

- 脚本只会为检测到的端口添加规则，不会开放所有端口
- 建议定期检查防火墙配置，确保没有不必要的端口开放
- 在生产环境中使用前，请仔细审核脚本逻辑