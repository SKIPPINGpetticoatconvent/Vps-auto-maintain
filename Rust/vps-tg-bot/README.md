# VPS Telegram Bot

基于 Rust 开发的 VPS 自动化管理系统，通过 Telegram Bot 实现远程监控和维护功能。

## 项目简介

VPS Telegram Bot 是一个轻量级的系统管理工具，专为 VPS 管理员设计。它提供了一个友好的 Telegram 界面，让您可以通过手机或电脑远程监控和管理您的 VPS 系统。

项目采用 Rust 语言开发，具有高性能、安全性和可靠性的特点。Bot 支持定时任务调度、实时系统监控、日志查看等核心功能，帮助您高效管理 VPS。

## 主要功能

### 📊 系统监控
- **实时系统状态**：监控运行时间、内存使用情况、系统负载
- **资源使用统计**：显示已用内存和总内存容量
- **负载平均值**：展示 1分钟、5分钟、15分钟的系统负载

### 🔧 自动化维护
- **核心维护任务**：执行系统核心维护脚本（`/usr/local/bin/vps-maintain-core.sh`）
- **规则更新任务**：执行规则更新脚本（`/usr/local/bin/vps-maintain-rules.sh`）
- **定时调度**：支持 Cron 表达式定时执行维护任务
- **手动触发**：支持立即执行维护任务

### 📋 日志管理
- **服务日志查看**：通过 `journalctl` 查看系统服务日志
- **日志行数控制**：可指定查看的日志行数（默认 20 行）
- **实时更新**：获取最新的系统日志信息

### 🛡️ 安全特性
- **权限验证**：只允许授权的 Chat ID 使用
- **Root 权限要求**：需要 Root 权限执行系统操作
- **脚本白名单**：只允许执行预定义的安全脚本
- **路径验证**：严格的脚本路径和权限检查

## 安装和配置

### 系统要求

- **操作系统**：Linux（推荐 Debian/Ubuntu）
- **Rust 版本**：1.70 或更高版本
- **权限**：需要 Root 权限运行
- **依赖服务**：
  - systemd（用于日志管理）
  - systemctl（用于服务管理）

### 1. 安装 Rust 工具链

```bash
# 安装 Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# 验证安装
rustc --version
cargo --version
```

### 2. 构建项目

```bash
# 进入项目目录
cd Rust/vps-tg-bot

# 编译项目
cargo build --release

# 可执行文件位置
# target/release/vps-tg-bot
```

### 3. 环境变量配置

创建环境变量文件或设置系统环境变量：

#### 必需的环境变量

```bash
# Telegram Bot Token（从 @BotFather 获取）
export TG_TOKEN="your_bot_token_here"

# 授权的 Telegram Chat ID
export TG_CHAT_ID="your_chat_id_here"
```

#### 可选的环境变量

```bash
# 状态文件存储路径（默认：/var/lib/vps-tg-bot）
export STATE_PATH="/var/lib/vps-tg-bot"

# 脚本存储路径（默认：/usr/local/bin/vps-tg-bot/scripts）
export SCRIPTS_PATH="/usr/local/bin/vps-tg-bot/scripts"

# 日志服务名称（默认：vps-tg-bot）
export LOGS_SERVICE="vps-tg-bot"
```

### 4. 目录权限设置

```bash
# 创建必要的目录
sudo mkdir -p /var/lib/vps-tg-bot
sudo mkdir -p /usr/local/bin/vps-tg-bot/scripts

# 设置权限（需要根据实际用户调整）
sudo chown -R root:root /var/lib/vps-tg-bot
sudo chown -R root:root /usr/local/bin/vps-tg-bot
sudo chmod 755 /var/lib/vps-tg-bot
sudo chmod 755 /usr/local/bin/vps-tg-bot/scripts
```

### 5. 创建维护脚本

创建核心维护脚本 `/usr/local/bin/vps-maintain-core.sh`：

```bash
#!/bin/bash
# VPS 核心维护脚本
set -e

echo "开始执行核心维护任务..."

# 更新系统包
apt update && apt upgrade -y

# 清理临时文件
apt autoremove -y
apt autoclean

# 更新系统时间
ntpdate -s time.nist.gov || true

# 清理日志
journalctl --vacuum-time=7d

echo "核心维护任务完成"
```

创建规则更新脚本 `/usr/local/bin/vps-maintain-rules.sh`：

```bash
#!/bin/bash
# VPS 规则更新脚本
set -e

echo "开始执行规则更新任务..."

# 更新防火墙规则（如使用 iptables）
# iptables-restore < /etc/iptables/rules.v4

# 更新系统安全配置
# 这里添加您的自定义规则更新逻辑

echo "规则更新任务完成"
```

设置脚本权限：

```bash
sudo chmod +x /usr/local/bin/vps-maintain-core.sh
sudo chmod +x /usr/local/bin/vps-maintain-rules.sh
```

### 6. 创建 Systemd 服务

创建服务文件 `/etc/systemd/system/vps-tg-bot.service`：

```ini
[Unit]
Description=VPS Telegram Bot
After=network.target
Wants=network.target

[Service]
Type=simple
User=root
Group=root
Environment=TG_TOKEN=your_bot_token
Environment=TG_CHAT_ID=your_chat_id
Environment=STATE_PATH=/var/lib/vps-tg-bot
Environment=SCRIPTS_PATH=/usr/local/bin/vps-tg-bot/scripts
Environment=LOGS_SERVICE=vps-tg-bot
ExecStart=/path/to/vps-tg-bot/target/release/vps-tg-bot
Restart=always
RestartSec=10
StandardOutput=journal
StandardError=journal
SyslogIdentifier=vps-tg-bot

[Install]
WantedBy=multi-user.target
```

启动服务：

```bash
# 重新加载 systemd 配置
sudo systemctl daemon-reload

# 启用服务开机自启
sudo systemctl enable vps-tg-bot

# 启动服务
sudo systemctl start vps-tg-bot

# 检查服务状态
sudo systemctl status vps-tg-bot
```

## 使用说明

### Bot 命令

VPS Telegram Bot 支持以下命令：

#### 基本命令

- `/start` - 开始与 Bot 交互，显示主菜单
- `/menu` - 显示主菜单

#### 交互式菜单

主菜单提供以下功能按钮：

##### 📊 系统状态
- **功能**：获取实时系统监控信息
- **显示内容**：
  - 系统运行时间（秒）
  - 内存使用情况（已用/总量，MB）
  - 系统负载平均值（1分钟、5分钟、15分钟）

##### 🔧 立即维护
执行手动维护操作，包含子菜单：

- **🔧 核心维护**：立即执行核心维护脚本
- **📜 规则更新**：立即执行规则更新脚本

##### 📋 查看日志
- **功能**：获取 Bot 服务日志
- **显示内容**：最近 20 行系统服务日志

### 使用流程

1. **启动 Bot**
   ```
   /start
   ```

2. **查看系统状态**
   ```
   📊 系统状态
   ```

3. **执行维护任务**
   ```
   🔧 立即维护 → 🔧 核心维护
   ```

4. **查看日志**
   ```
   📋 查看日志
   ```

### 定时任务

Bot 支持自动定时执行维护任务：

- **核心维护**：默认每天 4:00 AM 执行
- **规则更新**：根据需要配置时间

定时任务配置保存在 `/var/lib/vps-tg-bot/jobs.json`：

```json
{
  "CoreMaintain": {
    "job_type": "CoreMaintain",
    "schedule": "0 0 4 * * * *",
    "enabled": true,
    "last_run": "2024-01-01T04:00:00Z"
  }
}
```

## 开发和测试

### 开发环境设置

1. **克隆项目**
   ```bash
   git clone <repository-url>
   cd vps-tg-bot/Rust/vps-tg-bot
   ```

2. **安装依赖**
   ```bash
   cargo fetch
   ```

3. **开发模式运行**
   ```bash
   # 设置环境变量
   export TG_TOKEN="your_test_token"
   export TG_CHAT_ID="your_test_chat_id"

   # 运行开发版本
   cargo run
   ```

### 测试

#### 单元测试

```bash
# 运行所有测试
cargo test

# 运行特定模块测试
cargo test --lib

# 运行测试并显示详细输出
cargo test -- --nocapture
```

#### 测试覆盖

项目包含以下测试：

- **配置测试** (`config.rs`)：
  - 环境变量加载测试
  - 配置验证测试
  - 错误处理测试

- **调度器测试** (`scheduler.rs`)：
  - 任务添加/删除测试
  - 任务调度测试
  - 状态持久化测试

- **Bot 测试** (`bot.rs`)：
  - 授权验证测试
  - 命令处理测试

- **系统操作测试** (`system.rs`)：
  - Mock 对象测试
  - 脚本执行测试
  - 系统信息获取测试

### 调试

#### 日志配置

Bot 使用 `env_logger` 进行日志管理：

```bash
# 设置日志级别
export RUST_LOG=info
export RUST_LOG=vps_tg_bot=debug

# 或在代码中设置
env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
```

#### 常见调试场景

1. **Bot 无响应**
   ```bash
   # 检查 Bot 状态
   sudo systemctl status vps-tg-bot
   
   # 查看日志
   sudo journalctl -u vps-tg-bot -f
   ```

2. **权限错误**
   ```bash
   # 检查是否以 root 权限运行
   sudo systemctl status vps-tg-bot
   
   # 验证脚本权限
   ls -la /usr/local/bin/vps-maintain-*.sh
   ```

3. **配置问题**
   ```bash
   # 验证环境变量
   sudo systemctl show vps-tg-bot | grep Environment
   
   # 测试配置加载
   TG_TOKEN=test TG_CHAT_ID=123 cargo run
   ```

### 代码结构

```
src/
├── main.rs          # 程序入口点
├── bot.rs           # Telegram Bot 逻辑
├── config.rs        # 配置管理
├── scheduler.rs     # 任务调度器
├── system.rs        # 系统操作接口
├── error.rs         # 错误类型定义
├── types.rs         # 共享类型定义
├── utils.rs         # 工具函数
└── lib.rs           # 库入口
```

### 贡献指南

1. **代码规范**
   - 遵循 Rust 官方代码规范
   - 使用 `rustfmt` 格式化代码
   - 使用 `clippy` 进行代码检查

2. **提交规范**
   ```bash
   # 格式化代码
   cargo fmt
   
   # 代码检查
   cargo clippy
   
   # 运行测试
   cargo test
   ```

3. **错误处理**
   - 使用自定义错误类型
   - 提供详细的错误信息
   - 适当的错误恢复机制

## 故障排除

### 常见问题

#### 1. Bot 无法启动
**症状**：服务启动失败
**解决方案**：
```bash
# 检查配置文件
sudo systemctl status vps-tg-bot

# 检查环境变量
sudo journalctl -u vps-tg-bot --no-pager | tail -20

# 验证权限
sudo -u root /path/to/vps-tg-bot
```

#### 2. 权限不足错误
**症状**：`Root privileges required`
**解决方案**：
```bash
# 确保以 root 权限运行
sudo systemctl edit vps-tg-bot
# 在 [Service] 部分添加：
# User=root
# Group=root
```

#### 3. 脚本执行失败
**症状**：`Script file not found` 或权限错误
**解决方案**：
```bash
# 检查脚本存在性
ls -la /usr/local/bin/vps-maintain-*.sh

# 检查脚本权限
sudo chmod +x /usr/local/bin/vps-maintain-*.sh

# 手动测试脚本
sudo /usr/local/bin/vps-maintain-core.sh
```

#### 4. Telegram API 错误
**症状**：Bot 无法发送消息
**解决方案**：
```bash
# 验证 Token
curl -X GET "https://api.telegram.org/bot<YOUR_TOKEN>/getMe"

# 检查网络连接
ping api.telegram.org

# 验证 Chat ID
curl -X GET "https://api.telegram.org/bot<YOUR_TOKEN>/getUpdates"
```

### 日志分析

#### 关键日志条目

1. **正常启动**
   ```
   INFO vps_tg_bot: Starting VPS TG Bot...
   INFO vps_tg_bot: Bot started polling...
   ```

2. **维护任务执行**
   ```
   INFO vps_tg_bot: Executing scheduled job: CoreMaintain
   INFO vps_tg_bot: Job CoreMaintain finished: success=true
   ```

3. **错误情况**
   ```
   ERROR vps_tg_bot: Failed to load config: Missing TG_TOKEN
   ERROR vps_tg_bot: Job RulesUpdate failed: Script file not found
   ```

### 性能优化

1. **内存使用**
   - 监控内存使用情况
   - 避免内存泄漏
   - 及时释放不需要的资源

2. **CPU 优化**
   - 调度器检查间隔：10 秒
   - 避免频繁的系统调用
   - 合理设置脚本执行超时时间

## 许可证

本项目采用 MIT 许可证。详见 [LICENSE](../../LICENSE) 文件。

## 支持

如果您在使用过程中遇到问题，请：

1. 检查本文档的故障排除部分
2. 查看项目的 GitHub Issues
3. 提交新的 Issue 描述问题

## 更新日志

### v1.0.0
- 初始版本发布
- 基础系统监控功能
- 定时任务调度
- Telegram Bot 交互界面
- 安全权限控制

---

**注意**：本工具需要 Root 权限运行，请谨慎使用并确保在安全的环境中进行部署。