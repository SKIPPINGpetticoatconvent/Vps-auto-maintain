# 代理服务端口检测和防火墙配置脚本

## 文件说明

项目包含两个版本的脚本：

### 1. `detect_ports.sh` - 原始版本
- 基础的端口检测功能
- 支持手动指定端口（已移除）
- 简单的检测逻辑

### 2. `detect_ports_clean.sh` - 增强版本 ⭐推荐
- 完整的配置文件解析
- 智能进程检测
- 详细的调试信息
- 更好的错误处理

## 使用方法

### 推荐使用增强版本：

```bash
# 下载脚本
wget https://github.com/FTDRTD/Vps-auto-maintain/raw/main/detect_ports_clean.sh

# 运行脚本
sudo bash detect_ports_clean.sh --token YOUR_TOKEN --chat-id YOUR_ID
```

### 或者使用原始版本：

```bash
# 下载脚本
wget https://github.com/FTDRTD/Vps-auto-maintain/raw/main/detect_ports.sh

# 运行脚本
sudo bash detect_ports.sh --token YOUR_TOKEN --chat-id YOUR_ID
```

## 检测逻辑

增强版本的检测顺序：

1. **进程端口检测**：检查 `sing-box` 进程正在监听的端口
2. **配置文件解析**：从 `/etc/sing-box/config.json` 等配置文件读取端口
3. **智能扫描**：扫描所有监听端口，识别代理相关进程
4. **防火墙配置**：自动为检测到的端口配置 TCP/UDP 规则

## 支持的配置文件路径

- `/etc/sing-box/config.json` (主配置文件)
- `/etc/sing-box/conf/Hysteria2-36479.json` (Hysteria 配置)
- `/etc/sing-box/conf/TUIC-46500.json` (TUIC 配置)
- `/usr/local/etc/sing-box/config.json`
- `/opt/sing-box/config.json`

## 系统要求

- Linux 操作系统
- `ss` 或 `netstat` 命令
- `jq` 命令（推荐，用于解析JSON配置）
- `curl` 命令（用于 Telegram 通知）
- `sudo` 权限

## 输出示例

```
------------------------------------------------------------
开始检测代理服务端口并配置防火墙
------------------------------------------------------------
🔍 检测防火墙类型: firewalld
🕒 系统时区: Etc/UTC
🕐 当前时间: 2025-09-16 12:00:37
✅ 检测到 Xray 运行端口: 11910 12544 36892 42722 50406 58403
🔍 正在检测 Sing-box 监听端口...
📄 解析配置文件: /etc/sing-box/config.json
📋 从配置文件读取到端口: 36479 46500
✅ 检测到 Sing-box 运行端口: 36479 46500
✅ 防火墙规则配置完成，已允许相关端口的 UDP/TCP 流量
```

## 参数选项

- `--no-notify`: 禁用 Telegram 通知
- `--token TOKEN`: Telegram Bot Token
- `--chat-id ID`: Telegram Chat ID

## 故障排除

### Sing-box 端口未检测到

1. **检查进程状态**：
   ```bash
   ps aux | grep sing-box
   ```

2. **检查端口监听**：
   ```bash
   ss -tlnp | grep -i sing
   ```

3. **检查配置文件**：
   ```bash
   cat /etc/sing-box/config.json | jq '.inbounds[]?.port'
   ```

4. **查看 Sing-box 日志**：
   ```bash
   journalctl -u sing-box -f
   ```

### 配置文件解析失败

如果没有 `jq` 命令，脚本会使用备用的 `grep` 方法解析配置：

```bash
# 安装 jq（推荐）
apt update && apt install -y jq
```

### 防火墙配置失败

检查防火墙状态：
```bash
# firewalld
firewall-cmd --list-all

# ufw
ufw status
```

## 安全注意事项

- 脚本只会为检测到的端口添加规则
- 建议定期检查防火墙配置
- 在生产环境中使用前请审核脚本逻辑

## 更新日志

### v2.0 - 增强版本
- ✅ 添加配置文件解析功能
- ✅ 改进进程检测逻辑
- ✅ 增强调试信息输出
- ✅ 更好的错误处理

### v1.0 - 原始版本
- ✅ 基础端口检测功能
- ✅ 防火墙规则配置
- ✅ Telegram 通知支持