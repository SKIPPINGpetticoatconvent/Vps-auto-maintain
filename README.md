# VPS Auto Maintain

> 简体中文 | [English](#english)

一个强大的 VPS 自动化维护工具集，提供一键部署、定时维护和安全配置功能。

## 功能特性

- 🔄 **自动维护**: 系统更新、Xray/Sing-box 核心更新、定时重启
- 🤖 **Telegram 通知**: 实时状态监控、维护结果通知、封禁提醒
- 🛡️ **安全配置**: 防火墙自动配置、Fail2Ban SSH 防护、三档防护模式
- 📱 **Bot 管理**: 通过 Telegram Bot 交互式远程管理 VPS
- ⏰ **定时任务**: 智能调度、时区自适应、持久化存储
- 📊 **状态监控**: 实时查看系统状态、代理服务状态、定时任务状态
- 💾 **内存优化**: 系统日志内存化存储、内存使用优化
- 🔒 **安全更新**: 无人值守安全补丁、自动重启、多种部署模式

## 项目文件

### 脚本列表

- `deploy.sh` - VPS 自动维护一键部署脚本 (v4.4)
- `Telegram-Bot.sh` - Telegram Bot 管理系统部署脚本 (v5.3)
- `vps_secure_xpanel_fixed.sh` - VPS 终极安全与自动维护脚本 (V3.7.3)
- `Debian/security-auto-update.sh` - Debian 安全更新专用脚本 (v2.1)
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
- 定时任务管理（持久化存储）
- 系统日志查看

#### 3. 安全配置 (`vps_secure_xpanel_fixed.sh`)
- UFW/firewalld 防火墙配置
- Fail2Ban SSH 防护（三种模式）
- 端口自动检测和开放
- X-Panel/X-UI 兼容性支持
- Telegram 实时通知支持

#### 4. 安全更新 (`Debian/security-auto-update.sh`)
- 无人值守安全更新配置
- 内存日志存储优化
- 03:00 自动重启
- 轻量化环境专用

## 快速开始

### 环境要求
- Linux 操作系统 (Ubuntu/Debian/CentOS/Rocky/AlmaLinux)
- root 用户权限或 sudo 权限
- 网络连接
- 支持 systemd 的系统

### 安装部署

#### 选择适合您的部署方案

项目提供四种部署方案，请根据需求选择：

1. **完整维护方案** (`deploy.sh`) - 系统维护 + 定时任务 + Telegram 通知
2. **Bot 管理方案** (`Telegram-Bot.sh`) - 交互式 Bot 管理界面
3. **安全防护方案** (`vps_secure_xpanel_fixed.sh`) - 全面的安全配置和防护
4. **轻量更新方案** (`Debian/security-auto-update.sh`) - 仅安全更新，轻量部署

#### 方法一：在线运行（推荐，方便快捷）

```
# 基础维护部署
bash <(curl -sL https://raw.githubusercontent.com/FTDRTD/Vps-auto-maintain/main/deploy.sh)
```

```
# Telegram Bot 管理部署
bash <(curl -sL https://raw.githubusercontent.com/FTDRTD/Vps-auto-maintain/main/TG/Telegram-Bot.sh)
```

```
# 安全配置
bash <(curl -sL https://raw.githubusercontent.com/FTDRTD/Vps-auto-maintain/main/vps_secure_xpanel_fixed.sh)
```

```
# 安全更新专用
bash <(curl -sL https://raw.githubusercontent.com/FTDRTD/Vps-auto-maintain/main/Debian/security-auto-update.sh)
```


#### 方法二：克隆项目后运行

1. **克隆项目**
   ```bash
   git clone https://github.com/FTDRTD/Vps-auto-maintain.git
   cd Vps-auto-maintain
   ```

2. **运行部署脚本**
   ```bash
   # 基础维护部署
   chmod +x deploy.sh && ./deploy.sh

   # 或使用 Telegram Bot 管理
   chmod +x TG/Telegram-Bot.sh && ./TG/Telegram-Bot.sh

   # 安全配置
   chmod +x vps_secure_xpanel_fixed.sh && ./vps_secure_xpanel_fixed.sh

   # 安全更新专用
   chmod +x Debian/security-auto-update.sh && ./Debian/security-auto-update.sh
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
在使用相关脚本时需要提供：
- **Bot Token**: 从 @BotFather 获取
- **Chat ID**: 您的 Telegram 用户 ID

### 时区配置
所有脚本自动检测系统时区，并根据时区调整默认执行时间：
- **核心维护**: 默认东京时间 04:00
- **规则更新**: 默认北京时间 07:00（周日）
- **安全更新**: 默认 03:00 自动重启

### 端口检测与开放
自动检测并开放以下服务端口：
- **SSH 端口**: 自动检测当前 SSH 端口
- **Xray 端口**: 动态检测所有 Xray 入站端口
- **Sing-box 端口**: 动态检测所有 Sing-box 入站端口
- **X-Panel/X-UI 端口**: 从数据库动态检测并开放相关端口
- **80 端口**: 证书申请专用端口
- **443 端口**: HTTPS 流量端口

### Fail2Ban 防护模式
提供三种 SSH 防护强度：
- **普通模式**: 5次失败，封禁10分钟
- **激进模式**: 3次失败，封禁1小时（推荐）
- **偏执模式**: 2次失败，封禁12小时

### 脚本参数说明

#### deploy.sh 和 TG/Telegram-Bot.sh
- 智能检测 Xray/Sing-box 安装状态
- 按需创建维护脚本和定时任务
- 支持手动或自动时间配置
- 使用内存日志优化系统性能

#### vps_secure_xpanel_fixed.sh
- `--status`: 查看当前安全配置状态
- `--uninstall`: 尝试还原安全配置
- 支持 Telegram 实时通知
- 兼容 X-Panel/X-UI 面板管理

#### Debian/security-auto-update.sh
- 轻量化设计，适用于minimal环境
- 仅配置无人值守安全更新
- 支持卸载模式 (--uninstall/-u)
- 不包含复杂的交互功能
- 专为 Debian 系统优化

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
    ├── Fail2Ban
    └── 端口管理
```

## 更新日志

### v4.4 (deploy.sh)
- 智能检测 Xray/Sing-box 安装情况
- 按需配置维护任务
- 内存化日志存储优化
- Xray核心和Sing-box独立更新逻辑

### v5.3 (Telegram-Bot.sh)
- 持久化定时任务存储（SQLite）
- 兼容性修复和优化
- UV 包管理器集成
- 完整的交互式管理界面
- 错误处理和日志记录改进

### V3.7.3 (vps_secure_xpanel_fixed.sh)
- 终极安全配置脚本
- 三档 Fail2Ban 防护模式
- 智能端口检测和开放
- X-Panel/X-UI 兼容性支持
- Telegram 实时封禁通知
- 自动检测 Fail2Ban action 文件

### v2.1 (Debian/security-auto-update.sh)
- 轻量化安全更新方案
- 无人值守安全补丁
- 内存日志存储
- 03:00 自动重启
- 智能自检模块
- 添加卸载模式支持

## 注意事项

- ⚠️ 请在测试环境先验证脚本
- 🔐 妥善保管 Telegram Token 和 Chat ID
- ⏱️ 维护任务可能导致服务短暂中断
- 🛡️ 防火墙配置请谨慎操作
- 📊 建议定期检查 Bot 运行状态

## 许可证

本项目采用 MIT 许可证 - 详见 [LICENSE](LICENSE) 文件

## 贡献

欢迎提交 Issue 和 Pull Request！

---

# English

# VPS Auto Maintain

A powerful VPS automation maintenance toolkit that provides one-click deployment, scheduled maintenance, and security configuration.

## Features

- 🔄 **Auto Maintenance**: System updates, Xray/Sing-box core updates, scheduled reboots
- 🤖 **Telegram Notifications**: Real-time monitoring, maintenance notifications, ban alerts
- 🛡️ **Security Configuration**: Automatic firewall setup, Fail2Ban SSH protection, three protection modes
- 📱 **Bot Management**: Interactive remote VPS management via Telegram Bot
- ⏰ **Scheduled Tasks**: Smart scheduling, timezone adaptation, persistent storage
- 📊 **Status Monitoring**: Real-time system status, proxy service status, task status
- 💾 **Memory Optimization**: System logs in memory storage, memory usage optimization
- 🔒 **Security Updates**: Unattended security patches, automatic reboot, multiple deployment modes

## Project Files

### Script List

- `deploy.sh` - VPS auto maintenance one-click deployment script (v4.4)
- `Telegram-Bot.sh` - Telegram Bot management system deployment script (v5.3)
- `vps_secure_xpanel_fixed.sh` - Ultimate VPS security and auto maintenance script (V3.7.3)
- `Debian/security-auto-update.sh` - Debian security update dedicated script (v1.0)
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
- Scheduled task management (persistent storage)
- System log viewing

#### 3. Security Configuration (`vps_secure_xpanel_fixed.sh`)
- UFW/firewalld firewall configuration
- Fail2Ban SSH protection (three modes)
- Automatic port detection and opening
- X-Panel/X-UI panel compatibility
- Telegram real-time notifications

#### 4. Security Update (`Debian/security-auto-update.sh`)
- Unattended security updates
- Memory-based log storage optimization
- 03:00 automatic reboot
- Lightweight environment dedicated

## Quick Start

### Environment Requirements
- Linux OS (Ubuntu/Debian/CentOS/Rocky/AlmaLinux)
- Root user privileges or sudo access
- Network connection
- Systemd supported system

### Installation & Deployment

#### Choose Your Deployment Plan

The project provides four deployment options, please choose according to your needs:

1. **Complete Maintenance Plan** (`deploy.sh`) - System maintenance + scheduled tasks + Telegram notifications
2. **Bot Management Plan** (`Telegram-Bot.sh`) - Interactive Bot management interface
3. **Security Protection Plan** (`vps_secure_xpanel_fixed.sh`) - Comprehensive security configuration and protection
4. **Lightweight Update Plan** (`Debian/security-auto-update.sh`) - Security updates only, lightweight deployment

#### Method 1: Run Online (Recommended, Quick and Easy)

```bash
# Basic maintenance deployment
bash <(curl -sL https://raw.githubusercontent.com/FTDRTD/Vps-auto-maintain/main/deploy.sh)

# Telegram Bot management deployment
bash <(curl -sL https://raw.githubusercontent.com/FTDRTD/Vps-auto-maintain/main/TG/Telegram-Bot.sh)

# Security configuration
bash <(curl -sL https://raw.githubusercontent.com/FTDRTD/Vps-auto-maintain/main/vps_secure_xpanel_fixed.sh)

# Security update only
bash <(curl -sL https://raw.githubusercontent.com/FTDRTD/Vps-auto-maintain/main/Debian/security-auto-update.sh)
```

#### Method 2: Clone and Run

1. **Clone the project**
   ```bash
   git clone https://github.com/FTDRTD/Vps-auto-maintain.git
   cd Vps-auto-maintain
   ```

2. **Run deployment scripts**
   ```bash
   # Basic maintenance deployment
   chmod +x deploy.sh && ./deploy.sh

   # Or use Telegram Bot management
   chmod +x TG/Telegram-Bot.sh && ./TG/Telegram-Bot.sh

   # Security configuration
   chmod +x vps_secure_xpanel_fixed.sh && ./vps_secure_xpanel_fixed.sh

   # Security update only
   chmod +x Debian/security-auto-update.sh && ./Debian/security-auto-update.sh
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
Provide the following when using relevant scripts:
- **Bot Token**: Obtained from @BotFather
- **Chat ID**: Your Telegram user ID

### Timezone Configuration
All scripts automatically detect system timezone and adjust default execution times:
- **Core Maintenance**: Default Tokyo time 04:00
- **Rules Update**: Default Beijing time 07:00 (Sunday)
- **Security Updates**: Default 03:00 automatic reboot

### Port Detection & Opening
Automatically detects and opens ports for:
- **SSH Port**: Automatically detects current SSH port
- **Xray Ports**: Dynamically detects all Xray inbound ports
- **Sing-box Ports**: Dynamically detects all Sing-box inbound ports
- **X-Panel/X-UI Ports**: Dynamically detects and opens from database
- **Port 80**: Certificate application dedicated port
- **Port 443**: HTTPS traffic port

### Fail2Ban Protection Modes
Provides three SSH protection levels:
- **Normal Mode**: 5 failures, ban for 10 minutes
- **Aggressive Mode**: 3 failures, ban for 1 hour (recommended)
- **Paranoid Mode**: 2 failures, ban for 12 hours

### Script Parameter Guide

#### deploy.sh and TG/Telegram-Bot.sh
- Smart detection of Xray/Sing-box installation status
- Creates maintenance scripts and scheduled tasks as needed
- Supports manual or automatic time configuration
- Uses memory logging for system optimization

#### vps_secure_xpanel_fixed.sh
- `--status`: View current security configuration status
- `--uninstall`: Attempt to restore security configuration
- Supports Telegram real-time notifications
- Compatible with X-Panel/X-UI panel management

#### Debian/security-auto-update.sh
- Lightweight design, suitable for minimal environments
- Only configures unattended security updates
- Supports uninstall mode (--uninstall/-u)
- No complex interaction features included
- Optimized specifically for Debian systems

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
    ├── Fail2Ban
    └── Port Management
```

## Changelog

### v4.4 (deploy.sh)
- Intelligent detection of Xray/Sing-box installation
- Configure maintenance tasks as needed
- Memory-based log storage optimization
- Independent update logic for Xray core and Sing-box

### v5.3 (Telegram-Bot.sh)
- Persistent scheduled task storage (SQLite)
- Compatibility fixes and optimizations
- UV package manager integration
- Complete interactive management interface
- Improved error handling and logging

### V3.7.3 (vps_secure_xpanel_fixed.sh)
- Ultimate security configuration script
- Three-level Fail2Ban protection modes
- Smart port detection and opening
- X-Panel/X-UI panel compatibility support
- Telegram real-time ban notifications
- Automatic Fail2Ban action file detection

### v2.1 (Debian/security-auto-update.sh)
- Lightweight security update solution
- Unattended security patches
- Memory log storage
- 03:00 automatic reboot
- Intelligent self-check module
- Added uninstall mode support

## Important Notes

- ⚠️ Please test scripts in a test environment first
- 🔐 Safely store Telegram Token and Chat ID
- ⏱️ Maintenance tasks may cause brief service interruptions
- 🛡️ Be cautious with firewall configurations
- 📊 Regularly check Bot running status

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details

## Contributing

Issues and Pull Requests are welcome!