# VPS 自动维护脚本 (Vps-auto-maintain)

## 项目简介

这是一个用于 Linux VPS 的自动化维护脚本，能够自动更新系统包、检查和更新网络工具（如 Xray 和 Sing-box），并在维护完成后通过 Telegram Bot 发送通知。脚本支持自动重启，并在重启后再次发送通知，确保 VPS 的稳定运行和及时更新。

项目基于 Bash 脚本开发，兼容多种 Linux 发行版，无需 systemd 支持即可运行。

## 功能特性

- **自动化系统维护**：自动更新系统包并清理垃圾
- **网络工具更新**：支持检测和更新 Xray 和 Sing-box
- **端口检测和防火墙配置**：自动检测代理服务端口并配置防火墙规则，支持 Sing-box 和 Xray
- **Telegram 通知**：维护前、重启后分阶段发送通知，包括系统时区和当前时间
- **定时任务**：使用 Cron 设置每日自动执行（默认东京时区凌晨 4:00）
- **时区兼容**：智能检测系统时区，支持多种时区配置
- **容错机制**：内置重试机制和错误处理
- **兼容性**：适用于有无 systemd 的环境
- **一键部署**：`deploy.sh` 脚本实现快速安装和配置
- **内存日志记录**：支持将维护日志存储在内存中，提高性能并减少磁盘写入

## 系统要求

- Linux 操作系统（支持 Debian/Ubuntu/CentOS 等）
- 网络连接（用于 Telegram API 和软件包下载）
- Bash Shell
- curl 命令（通常预装）

## 安装和设置

### 步骤 1: 准备 Telegram Bot

1. 在 Telegram 中联系 @BotFather 创建新 Bot
2. 获取 Bot Token（例如：`1234567890:ABCdefGHijkLMNopQRstuVWxyz`）
3. 与您的 Bot 发送一条消息，然后访问 `https://api.telegram.org/bot<token>/getUpdates` 获取 Chat ID

## 快速一键安装（推荐）

如果您想跳过克隆仓库的步骤，可以直接在线执行部署脚本：

### 使用 wget（推荐）

```bash
bash <(wget -qO- -o- https://github.com/FTDRTD/Vps-auto-maintain/raw/main/deploy.sh)
```

### 使用 curl

```bash
bash <(curl -sL https://github.com/FTDRTD/Vps-auto-maintain/raw/main/deploy.sh)
```

### 执行步骤

1. 运行上述任意一条命令
2. 按提示输入您的 Telegram Bot Token 和 Chat ID
3. 脚本将自动完成部署并首次执行维护

> **注意**：确保您信任此代码源。一键命令会直接从互联网下载并执行脚本，请确定网络环境安全。

### 步骤 2: 部署脚本

1. 下载或克隆此仓库到您的 VPS：

```bash
git clone <repository-url>
cd <project-directory>
```

2. 运行部署脚本：

```bash
chmod +x deploy.sh
sudo ./deploy.sh
```

3. 输入您的 Telegram Bot Token 和 Chat ID
4. 脚本将自动：

   - 创建维护脚本 (`/usr/local/bin/vps-maintain.sh`)
   - 创建重启通知脚本 (`/usr/local/bin/vps-reboot-notify.sh`)
   - 设置每日定时任务
   - 执行首次维护和重启

## 使用方法

### 手动执行维护

```bash
sudo /usr/local/bin/vps-maintain.sh
```

### 查看定时任务

```bash
crontab -l
```

### 修改配置

如果需要修改 Telegram 配置，直接编辑脚本中的相应变量。

## 配置选项

### 定时任务设置

默认情况下，脚本设置为东京时区凌晨 4:00 执行维护。您可以在 `deploy.sh` 的步骤 4 中修改以下变量：

- `TOKYO_HOUR=4`：设置执行小时（东京时区）
- 修改为其他时区或其他时间

### Telegram 通知内容

通知信息包括：

- 系统时区
- 当前时间
- Xray 状态（最新/已更新/未安装）
- Sing-box 状态（最新/已更新/未安装）

## 端口检测和防火墙配置

项目包含端口检测脚本，用于自动检测代理服务端口并配置防火墙规则：

### 脚本文件详情

项目包含以下脚本文件：

#### 1. `deploy.sh` - VPS自动维护部署脚本 ⭐主脚本

- **版本**: 2.7 (最终版)
- **功能**: 一键部署VPS自动维护系统
- **特点**:
  - 自动创建维护和重启通知脚本
  - 支持自定义维护时间（默认东京时区凌晨4:00）
  - 配置内存日志存储以提升性能
  - 支持Telegram Bot通知
  - 兼容无systemd环境

#### 2. `detect_ports_fixed.sh` - 端口检测和防火墙配置脚本 ⭐推荐

- **版本**: 修复版
- **功能**: 自动检测代理服务端口并配置防火墙安全策略
- **特点**:
  - 自动检测Xray和Sing-box进程端口
  - 支持进程监听检测和配置文件解析
  - 智能防火墙配置（firewalld/ufw）
  - 主动移除未使用端口实现安全锁定
  - 修复所有已知bug和兼容性问题

#### 3. `detect_ports_ultimate.sh` - 一键式端口检测脚本 ⭐终极版

- **版本**: 终极版
- **功能**: 完全自动化的防火墙安全配置
- **特点**:
  - 继承所有 `detect_ports_fixed.sh`功能
  - **自动安装防火墙**: 如果系统未安装防火墙，会自动安装UFW或firewalld
  - 支持多种Linux发行版（Ubuntu/Debian/CentOS/RHEL等）
  - 一键式安全配置，无需手动干预

### 推荐使用方案

| 场景         | 推荐脚本                                     | 说明                             |
| ------------ | -------------------------------------------- | -------------------------------- |
| 首次使用     | `deploy.sh` + `detect_ports_ultimate.sh` | 先部署维护系统，再配置防火墙安全 |
| 已有维护系统 | `detect_ports_ultimate.sh`                 | 仅需配置防火墙安全策略           |
| 最小化配置   | `detect_ports_fixed.sh`                    | 适用于已有防火墙的环境           |
| 手动配置     | 逐个脚本单独运行                             | 根据需要选择性使用               |

### 使用方法

```bash
wget -qO- https://raw.githubusercontent.com/FTDRTD/Vps-auto-maintain/main/detect_ports_ultimate.sh | sudo bash
```

### 检测功能

- **进程端口检测**：检查 `sing-box` 进程正在监听的端口
- **配置文件解析**：从 `/etc/sing-box/config.json` 等配置文件读取端口
- **智能扫描**：扫描所有监听端口，精确识别代理服务进程
- **防火墙配置**：自动为检测到的端口配置 TCP/UDP 规则（支持 firewalld/ufw/iptables）

### 参数选项

- `--no-notify`: 禁用 Telegram 通知
- `--token TOKEN`: Telegram Bot Token
- `--chat-id ID`: Telegram Chat ID

### 系统要求

- `ss` 或 `netstat` 命令
- `jq` 命令（用于解析JSON配置）
- `curl` 命令（用于 Telegram 通知）

## 注意事项

⚠️ **重要警告**：

- 此脚本会在维护完成后自动重启服务器！
- 请确保所有重要数据已备份
- 首次运行后，服务器将立即重启
- 如果在生产环境中使用，确保不会影响关键服务

⚠️ **网络要求**：

- 脚本需要访问 Telegram API，某些网络环境可能需要代理
- 确保 VPS 能够访问软件包源和 Xray/Sing-box 更新源

⚠️ **权限要求**：

- 部署时需要 sudo 权限
- 确保用户有足够权限访问 `/usr/local/bin/` 和 Crontab

## 故障排除

### 常见问题

1. **Telegram 通知不发送**

   - 检查 Bot Token 和 Chat ID 是否正确
   - 确认网络能够访问 Telegram API
2. **脚本执行失败**

   - 检查是否所有依赖已安装
   - 查看系统日志：`journalctl -u cron` 或 `/var/log/cron`
3. **时区设置错误**

   - 脚本会自动检测时区，如有问题会在日志中显示
   - 手动检查：`date` 和 `/etc/timezone`
4. **Cron 任务不执行**

   - 确认 Cron 服务正在运行
   - 检查脚本权限是否正确

### 调试模式

如果需要调试，可以临时修改脚本，添加更多日志输出：

```bash
echo "Debug: $(date)" >> /var/log/vps-maintain.log
```

## 卸载说明

要卸载此脚本：

```bash
# 删除脚本文件
sudo rm /usr/local/bin/vps-maintain.sh /usr/local/bin/vps-reboot-notify.sh

# 删除定时任务
crontab -r  # 注意：这会删除所有定时任务，请慎用

# 或者手动编辑 crontab：
crontab -e
# 删除包含 vps-maintain.sh 的行
```

## 版本信息

当前版本：2.7

- 新增端口检测和防火墙配置功能
- 支持自动检测 Sing-box 和 Xray 端口
- 智能防火墙规则配置（firewalld/ufw/iptables）
- 兼容无 systemd 环境
- 修复时区获取问题
- 增强容错机制

## 许可证

此项目采用 MIT 许可证，详见 LICENSE 文件。

## 贡献

欢迎提交 Issue 和 Pull Request！

## 联系支持

如果您在使用过程中遇到问题，请：

1. 检查上述故障排除指南
2. 在 GitHub Issues 中详细描述问题
3. 提供相关日志信息（注意去除敏感信息）

---

**最后更新**: 2025-09-16
