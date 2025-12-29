# VPS Telegram Bot LoadCredential 安全凭证升级报告

## 概述

本次升级将 VPS Telegram Bot 从环境变量文件存储方式迁移到更安全的 systemd `LoadCredential` 凭证管理方案，提升了敏感凭证的安全性。

## 升级内容

### 1. 安装脚本升级 (`Rust/install.sh`)

#### 新增变量定义
- 添加了 `CREDSTORE_DIR="/etc/credstore"` 凭证存储目录
- 添加了 `BOT_TOKEN_CRED` 和 `CHAT_ID_CRED` 凭证文件路径

#### 新增函数
- `detect_existing_credentials()`: 检测现有凭证文件
- `read_existing_credentials()`: 从现有凭证文件读取配置
- `setup_credential_config()`: 创建凭证文件配置（替代原来的环境变量配置）

#### 凭证文件管理
- 凭证文件存储在 `/etc/credstore/` 目录
- BOT_TOKEN 凭证文件: `/etc/credstore/vps-tg-bot-rust.bot-token`
- CHAT_ID 凭证文件: `/etc/credstore/vps-tg-bot-rust.chat-id`
- 文件权限设置为 400（仅 root 可读）

#### systemd 服务配置
```ini
[Unit]
Description=VPS Telegram Bot (Rust)
After=network.target

[Service]
Type=simple
User=root
Group=root
WorkingDirectory=/etc/vps-tg-bot-rust
ExecStart=/usr/local/bin/vps-tg-bot-rust run
Restart=on-failure
RestartSec=10
StandardOutput=journal
StandardError=journal

# 使用 LoadCredential 加载敏感凭证
LoadCredential=bot-token:/etc/credstore/vps-tg-bot-rust.bot-token
LoadCredential=chat-id:/etc/credstore/vps-tg-bot-rust.chat-id

[Install]
WantedBy=multi-user.target
```

#### 卸载功能增强
- 新增删除凭证文件的步骤
- 在卸载过程中清理所有凭证相关文件

### 2. Rust 配置加载器升级

#### 环境变量加载器增强 (`Rust/vps-tg-bot/src/config/loader/env.rs`)

##### 新增功能
- `load_from_credentials()`: 从 systemd 凭证文件加载配置
- 支持从 `/run/credentials/vps-tg-bot-rust.service/` 目录读取凭证
- 配置来源跟踪功能

##### 加载优先级
1. **环境变量** (`BOT_TOKEN`, `CHAT_ID`) - 优先级最高（用于开发/测试）
2. **systemd 凭证文件** - 生产环境使用

##### 配置来源枚举扩展
```rust
pub enum ConfigSource {
    /// 环境变量配置
    Environment,
    /// systemd 凭证文件配置
    CredentialFile,
}
```

#### 配置加载模块升级 (`Rust/vps-tg-bot/src/config/loader/mod.rs`)
- 更新文档说明支持多种配置源
- 改进日志输出，显示实际使用的配置源
- 保持向后兼容性

#### 配置类型扩展 (`Rust/vps-tg-bot/src/config/types.rs`)
- 扩展 `ConfigSource` 枚举以支持凭证文件
- 保持原有验证逻辑不变

## 安全性改进

### 1. 凭证存储安全
- **之前**: 凭证存储在 `/etc/vps-tg-bot-rust/env` 文件中，权限 600
- **现在**: 凭证存储在 `/etc/credstore/` 目录，权限 400，更严格的访问控制

### 2. 运行时隔离
- 凭证文件在服务运行时通过 systemd 自动挂载到 `/run/credentials/`
- 服务停止后凭证自动清理
- 凭证文件在运行时具有临时性质

### 3. 权限控制
- 凭证目录权限: 755 (root 可访问)
- 凭证文件权限: 400 (仅 root 可读)
- systemd 自动管理凭证文件生命周期

## 向后兼容性

### 1. 环境变量支持
- 仍然支持通过环境变量 `BOT_TOKEN` 和 `CHAT_ID` 配置
- 环境变量优先级高于凭证文件
- 适用于开发和测试环境

### 2. 配置验证
- 保持原有的配置验证逻辑
- 相同的格式要求和错误处理

### 3. API 兼容
- 配置加载 API 保持不变
- 现有的配置相关代码无需修改

## 部署和迁移

### 新安装
1. 运行 `./install.sh` 脚本
2. 输入 BOT_TOKEN 和 CHAT_ID
3. 脚本自动创建凭证文件并配置 systemd 服务

### 现有安装升级
1. 脚本自动检测现有配置
2. 如有环境文件，自动迁移到凭证文件
3. 保留现有配置，无需重新输入

### 凭证文件验证
- 部署后可通过 `journalctl -u vps-tg-bot-rust` 查看启动日志
- 确认配置来源显示为 "systemd 凭证文件"

## 验收标准完成情况

✅ **安装脚本正确创建凭证文件到 `/etc/credstore/`**
- 凭证文件路径: `/etc/credstore/vps-tg-bot-rust.bot-token`
- 凭证文件路径: `/etc/credstore/vps-tg-bot-rust.chat-id`

✅ **systemd 服务配置使用 `LoadCredential` 指令**
- 服务文件包含 `LoadCredential=bot-token:` 和 `LoadCredential=chat-id:` 指令

✅ **Rust 程序能从 `/run/credentials/` 读取凭证**
- 配置加载器支持从 `/run/credentials/vps-tg-bot-rust.service/` 读取凭证

✅ **保持向后兼容：仍支持环境变量方式**
- 环境变量 `BOT_TOKEN` 和 `CHAT_ID` 优先级更高
- 适用于开发和测试环境

✅ **更新时保留现有凭证**
- 升级脚本自动检测并保留现有配置
- 如有环境文件，自动迁移到凭证文件

✅ **代码能够编译通过**
- 所有 Rust 代码编译无错误
- 仅有一个无害的 dead_code 警告

## 使用指南

### 管理命令
```bash
# 查看服务状态
systemctl status vps-tg-bot-rust

# 查看服务日志
journalctl -u vps-tg-bot-rust -f

# 停止服务
systemctl stop vps-tg-bot-rust

# 启动服务
systemctl start vps-tg-bot-rust

# 重启服务
systemctl restart vps-tg-bot-rust
```

### 凭证文件位置
- BOT_TOKEN: `/etc/credstore/vps-tg-bot-rust.bot-token`
- CHAT_ID: `/etc/credstore/vps-tg-bot-rust.chat-id`

### 开发环境配置
对于开发和测试，可以使用环境变量：
```bash
export BOT_TOKEN="your_bot_token"
export CHAT_ID="your_chat_id"
export CHECK_INTERVAL="300"
```

## 总结

本次升级成功实现了从环境变量到 systemd LoadCredential 的安全凭证管理方案迁移，在保持向后兼容性的同时，大幅提升了凭证存储的安全性。升级后的系统符合现代 Linux 安全最佳实践，为生产环境提供了更可靠的凭证保护机制。