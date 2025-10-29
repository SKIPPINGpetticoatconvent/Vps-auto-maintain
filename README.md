# VPS Auto Maintain

> 简体中文 | [English](#english)

一个强大的 VPS 自动化维护工具集，提供一键部署、定时维护和安全配置功能。

## 功能特性

- 🔄 **自动维护**: 系统更新、代理核心更新、定时重启
- 🤖 **Telegram 通知**: 实时状态监控和维护结果通知
- 🛡️ **安全配置**: 防火墙自动配置、Fail2Ban SSH 防护
- 📱 **Bot 管理**: 通过 Telegram Bot 远程管理 VPS
- ⏰ **定时任务**: 灵活的定时维护调度
- 📊 **状态监控**: 实时查看系统和代理服务状态

## 项目文件

### 脚本列表

- `deploy.sh` - VPS 自动维护一键部署脚本
- `Telegram-Bot.sh` - Telegram Bot 管理系统部署脚本
- `vps_secure_xpanel_fixed.sh` - VPS 安全配置脚本
- `LICENSE` - MIT 许可证
- `.gitignore` - Git 忽略文件

### 主要功能模块

#### 1. 系统维护 (`deploy.sh`)
- 系统软件更新和升级
- Xray/Sing-box 核心更新
- 定时任务配置
- Telegram 通知集成

#### 2. Telegram Bot 管理 (`Telegram-Bot.sh`)
- 交互式 VPS 管理界面
- 即时维护命令执行
- 定时任务管理
- 系统日志查看

#### 3. 安全配置 (`vps_secure_xpanel_fixed.sh`)
- UFW/firewalld 防火墙配置
- Fail2Ban SSH 防护（三种模式）
- 端口自动检测和开放
- X-Panel 兼容性支持

## 快速开始

### 环境要求
- Linux 操作系统 (Ubuntu/Debian/CentOS)
- root 用户权限
- 网络连接

### 安装部署

1. **克隆项目**
   ```bash
   git clone https://github.com/your-repo/Vps-auto-maintain.git
   cd Vps-auto-maintain
   ```

2. **运行部署脚本**
   ```bash
   # 基础维护部署
   chmod +x deploy.sh
   ./deploy.sh

   # 或使用 Telegram Bot 管理
   chmod +x Telegram-Bot.sh
   ./Telegram-Bot.sh
   ```

3. **安全配置**
   ```bash
   chmod +x vps_secure_xpanel_fixed.sh
   ./vps_secure_xpanel_fixed.sh
   ```

## 使用说明

### Telegram Bot 管理

部署完成后，在 Telegram 中发送 `/start` 打开管理面板：

- 📊 **系统状态**: 查看 VPS 状态和时间
- 🔧 **立即维护**: 执行系统更新和重启
- 📜 **规则更新**: 更新代理规则文件
- ⚙️ **定时设置**: 配置自动维护任务
- 📋 **查看日志**: 检查系统日志
- 🔄 **重启 VPS**: 远程重启服务器

### 维护任务

脚本会自动创建以下定时任务：
- **核心维护**: 每日凌晨 4:00 (东京时间)
- **规则更新**: 每周日早上 7:00 (北京时间)

### 安全模式

Fail2Ban 提供三种防护模式：
1. **普通模式**: 5 次失败，封禁 10 分钟
2. **激进模式**: 3 次失败，封禁 1 小时（推荐）
3. **偏执模式**: 2 次失败，封禁 12 小时

## 配置说明

### Telegram 配置
在使用脚本时需要提供：
- Bot Token: 从 @BotFather 获取
- Chat ID: 您的 Telegram 用户 ID

### 时区配置
脚本自动检测系统时区，并根据时区调整默认执行时间。

### 端口检测
自动检测并开放以下服务端口：
- SSH 端口
- Xray 端口
- Sing-box 端口
- X-Panel 管理端口
- 80 端口（证书申请用）

## 架构设计

```
VPS Auto Maintain
├── 系统层
│   ├── 定时任务 (cron)
│   ├── 系统服务 (systemd)
│   └── 日志系统 (journald/rsyslog)
├── 维护层
│   ├── 核心维护脚本
│   ├── 规则更新脚本
│   └── 重启通知脚本
├── 管理层
│   ├── Telegram Bot
│   └── Web 界面 (可选)
└── 安全层
    ├── 防火墙 (UFW/firewalld)
    └── Fail2Ban
```

## 更新日志

### v4.4 (deploy.sh)
- 智能检测 Xray/Sing-box 安装情况
- 按需配置维护任务
- 内存化日志存储优化

### v5.3 (Telegram-Bot.sh)
- 持久化定时任务存储
- 兼容性修复
- UV 包管理器集成

### v3.7.1 (vps_secure_xpanel_fixed.sh)
- BUG 修复：移除无用规则函数语法错误
- 全端口封禁模式优化

## 注意事项

- ⚠️ 请在测试环境先验证脚本
- 🔐 妥善保管 Telegram Token 和 Chat ID
- ⏱️ 维护任务可能导致服务短暂中断
- 🛡️ 防火墙配置请谨慎操作

## 许可证

本项目采用 MIT 许可证 - 详见 [LICENSE](LICENSE) 文件

## 贡献

欢迎提交 Issue 和 Pull Request！

---

# English

# VPS Auto Maintain

A powerful VPS automation maintenance toolkit that provides one-click deployment, scheduled maintenance, and security configuration.

## Features

- 🔄 **Auto Maintenance**: System updates, proxy core updates, scheduled reboots
- 🤖 **Telegram Notifications**: Real-time status monitoring and maintenance notifications
- 🛡️ **Security Configuration**: Automatic firewall configuration, Fail2Ban SSH protection
- 📱 **Bot Management**: Remote VPS management via Telegram Bot
- ⏰ **Scheduled Tasks**: Flexible maintenance scheduling
- 📊 **Status Monitoring**: Real-time system and proxy service monitoring

## Project Files

### Script List

- `deploy.sh` - VPS auto maintenance one-click deployment script
- `Telegram-Bot.sh` - Telegram Bot management system deployment script
- `vps_secure_xpanel_fixed.sh` - VPS security configuration script
- `LICENSE` - MIT License
- `.gitignore` - Git ignore file

### Main Function Modules

#### 1. System Maintenance (`deploy.sh`)
- System software updates and upgrades
- Xray/Sing-box core updates
- Cron job configuration
- Telegram notification integration

#### 2. Telegram Bot Management (`Telegram-Bot.sh`)
- Interactive VPS management interface
- Instant maintenance command execution
- Scheduled task management
- System log viewing

#### 3. Security Configuration (`vps_secure_xpanel_fixed.sh`)
- UFW/firewalld firewall configuration
- Fail2Ban SSH protection (three modes)
- Automatic port detection and opening
- X-Panel compatibility support

## Quick Start

### Environment Requirements
- Linux OS (Ubuntu/Debian/CentOS)
- Root user privileges
- Network connection

### Installation & Deployment

1. **Clone the project**
   ```bash
   git clone https://github.com/your-repo/Vps-auto-maintain.git
   cd Vps-auto-maintain
   ```

2. **Run deployment scripts**
   ```bash
   # Basic maintenance deployment
   chmod +x deploy.sh
   ./deploy.sh

   # Or use Telegram Bot management
   chmod +x Telegram-Bot.sh
   ./Telegram-Bot.sh
   ```

3. **Security configuration**
   ```bash
   chmod +x vps_secure_xpanel_fixed.sh
   ./vps_secure_xpanel_fixed.sh
   ```

## Usage Guide

### Telegram Bot Management

After deployment, send `/start` in Telegram to open the management panel:

- 📊 **System Status**: View VPS status and time
- 🔧 **Immediate Maintenance**: Execute system updates and reboot
- 📜 **Rules Update**: Update proxy rule files
- ⚙️ **Schedule Settings**: Configure automatic maintenance tasks
- 📋 **View Logs**: Check system logs
- 🔄 **Reboot VPS**: Remote server reboot

### Maintenance Tasks

The script automatically creates the following scheduled tasks:
- **Core Maintenance**: Daily at 4:00 AM (Tokyo time)
- **Rules Update**: Every Sunday at 7:00 AM (Beijing time)

### Security Modes

Fail2Ban provides three protection modes:
1. **Normal Mode**: 5 failures, ban for 10 minutes
2. **Aggressive Mode**: 3 failures, ban for 1 hour (recommended)
3. **Paranoid Mode**: 2 failures, ban for 12 hours

## Configuration Guide

### Telegram Configuration
Provide the following when using scripts:
- Bot Token: Obtained from @BotFather
- Chat ID: Your Telegram user ID

### Timezone Configuration
Scripts automatically detect system timezone and adjust default execution times accordingly.

### Port Detection
Automatically detects and opens ports for:
- SSH port
- Xray ports
- Sing-box ports
- X-Panel management ports
- Port 80 (for certificate application)

## Architecture Design

```
VPS Auto Maintain
├── System Layer
│   ├── Scheduled Tasks (cron)
│   ├── System Services (systemd)
│   └── Logging System (journald/rsyslog)
├── Maintenance Layer
│   ├── Core Maintenance Script
│   ├── Rules Update Script
│   └── Reboot Notification Script
├── Management Layer
│   ├── Telegram Bot
│   └── Web Interface (optional)
└── Security Layer
    ├── Firewall (UFW/firewalld)
    └── Fail2Ban
```

## Changelog

### v4.4 (deploy.sh)
- Intelligent detection of Xray/Sing-box installation
- Configure maintenance tasks as needed
- Memory-based log storage optimization

### v5.3 (Telegram-Bot.sh)
- Persistent scheduled task storage
- Compatibility fixes
- UV package manager integration

### v3.7.1 (vps_secure_xpanel_fixed.sh)
- BUG Fix: Syntax error in remove unused rules function
- All ports ban mode optimization

## Important Notes

- ⚠️ Please test scripts in a test environment first
- 🔐 Safely store Telegram Token and Chat ID
- ⏱️ Maintenance tasks may cause brief service interruptions
- 🛡️ Be cautious with firewall configurations

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details

## Contributing

Issues and Pull Requests are welcome!