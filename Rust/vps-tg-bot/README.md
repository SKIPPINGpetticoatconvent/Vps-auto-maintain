# VPS Telegram Bot (Rust Version)

一个基于 Rust 构建的 VPS 管理机器人，提供系统监控、维护、调度等功能的 Telegram 交互界面。

## 🚀 主要功能

### 系统监控
- 实时获取 CPU、内存、磁盘、网络使用情况
- 显示系统运行时间

### 系统维护
- **核心维护**: 系统包更新 (apt update/upgrade/autoremove)
- **规则维护**: Xray 规则数据库更新
- **特定软件更新**: Xray 和 Sing-box 独立更新
- **完整维护**: 包含以上所有操作

### 定时调度
- 支持 Cron 表达式的定时维护任务
- 调度计划持久化存储
- 运行时动态调整调度计划

### 日志管理
- 查看系统日志
- 支持自定义日志行数

### 交互界面
- 支持传统命令输入
- 提供直观的 Inline Keyboard 菜单
- 实时状态反馈

## 📋 支持的命令

### 基础命令
```
/start           # 启动机器人，显示菜单
/status          # 获取系统状态
/maintain        # 执行完整系统维护
/reboot          # 重启系统
```

### 维护命令
```
/update_xray     # 更新 Xray
/update_sb       # 更新 Sing-box
/maintain_core   # 执行核心系统维护
/maintain_rules  # 执行规则维护
```

### 管理命令
```
/logs            # 查看系统日志（默认20行）
/set_schedule <cron表达式>  # 设置定时维护计划
```

## 🔧 使用方法

### 1. 编译运行

```bash
# 进入项目目录
cd Rust/vps-tg-bot

# 编译项目
cargo build --release

# 运行机器人
./target/release/vps-tg-bot run
```

### 2. 安装为系统服务

```bash
# 安装服务
./target/release/vps-tg-bot install

# 卸载服务
./target/release/vps-tg-bot uninstall

# 直接运行（开发模式）
./target/release/vps-tg-bot run
```

### 3. 配置要求

首次运行时会创建配置文件，需要设置：
- Bot Token (从 @BotFather 获取)
- 管理员 Chat ID
- 调度计划 (Cron 表达式，默认: "0 0 4 * * Sun")

### 4. 使用示例

#### 设置每周日 4:00 自动维护
```
/set_schedule "0 0 4 * * Sun"
```

#### 查看系统状态
```
/status
```

#### 执行核心维护
```
/maintain_core
```

#### 查看最近30行日志
```
/logs 30
```

## 🏗️ 项目结构

```
src/
├── main.rs           # CLI 参数解析和主入口
├── config/           # 配置管理
├── bot/              # Bot 交互逻辑
│   ├── handlers.rs   # 命令处理器
│   └── keyboards.rs  # Inline Keyboard 定义
├── system/           # 系统操作
│   ├── info.rs       # 系统信息获取
│   └── ops.rs        # 系统维护操作
└── scheduler/        # 任务调度
    └── mod.rs        # 调度器实现
```

## 📊 新功能亮点

1. **Inline Keyboards**: 直观的按钮式交互，无需记忆命令
2. **细粒度维护**: 将维护任务细分，便于针对性操作
3. **调度持久化**: 调度计划自动保存，程序重启后保持有效
4. **实时反馈**: 所有操作都有详细的状态反馈
5. **类型安全**: 完整的 Rust 类型系统确保代码安全

## 🔒 安全特性

- 只有配置的管理员 Chat ID 可以执行敏感操作
- 重启命令需要确认（实际实现中可加强）
- 所有系统调用都有错误处理

## 📝 依赖项

主要依赖：
- `teloxide`: Telegram Bot 框架
- `tokio`: 异步运行时
- `sysinfo`: 系统信息获取
- `tokio-cron-scheduler`: 任务调度
- `serde`: 序列化/反序列化
- `clap`: 命令行参数解析

## 🚨 注意事项

1. 运行该机器人需要适当的系统权限（用于执行包管理和系统操作）
2. 首次使用前请确保已配置 Bot Token 和 Chat ID
3. 建议在测试环境中验证所有功能后再部署到生产环境
4. 定期备份重要数据和配置文件

## 📞 技术支持

如有问题，请检查：
1. 配置文件是否正确设置
2. 系统权限是否充足
3. 网络连接是否正常
4. Bot Token 是否有效

---
*构建于 2025 年，基于 Rust 语言和 Teloxide 框架*