# VPS Telegram Bot (Rust Version) Specification

## 1. 项目概述

本项目旨在将原有的 Shell + Python 混合架构的 VPS 维护 Bot 完全迁移到 Rust。目标是构建一个单一的、资源占用低、类型安全且易于部署的二进制文件，接管原脚本的所有运维和监控功能。

## 2. 功能需求列表

### 2.1 核心功能
| ID | 功能模块 | 原始逻辑 (Shell/Python) | Rust 实现方案 |
| :--- | :--- | :--- | :--- |
| F-01 | **Bot 交互** | `python-telegram-bot` 处理 `/start`, `/status` 等命令 | 使用 `teloxide` crate 实现异步 Bot 交互 |
| F-02 | **系统监控** | `uptime`, `date` 命令输出 | 使用 `sysinfo` crate 获取结构化的 CPU/内存/运行时间数据 |
| F-03 | **核心维护** | 调用 `apt update`, `xray up`, `sb up` | 使用 `std::process::Command` 封装系统包管理和特定软件更新命令 |
| F-04 | **规则更新** | 调用 `xray up dat` | 同上，封装为独立的维护任务 |
| F-05 | **定时任务** | `apscheduler` (每周日 04:00) | 使用 `tokio-cron-scheduler` 实现进程内调度，不再依赖外部 crontab |
| F-06 | **日志查看** | `journalctl` 查询 | 读取自身日志文件或通过 `systemd` API (或继续调用 `journalctl` 命令) |
| F-07 | **系统控制** | `reboot` 命令 | 调用系统重启命令，需处理权限检查 |

### 2.2 部署与配置 (CLI)
为了替代 Shell 安装脚本，Rust 二进制文件将包含自管理功能：
- **Install Mode**: `./vps-tg-bot install` - 交互式引导用户输入 Token/ChatID，生成配置文件，创建并启动 Systemd 服务。
- **Uninstall Mode**: `./vps-tg-bot uninstall` - 停止服务，删除文件和 Systemd 配置。
- **Run Mode**: `./vps-tg-bot run` - 守护进程模式（默认）。

## 3. 架构设计

### 3.1 目录结构
```text
Rust/
├── Cargo.toml
├── src/
│   ├── main.rs           # 入口：CLI 参数解析 (clap)，启动 Bot 或执行安装/卸载
│   ├── config.rs         # 配置管理：加载/保存 Token, ChatID (serde)
│   ├── bot/
│   │   ├── mod.rs        # Bot 初始化与 Dispatcher
│   │   ├── handlers.rs   # 命令处理器 (/status, /maintain 等)
│   │   └── keyboards.rs  # Inline Keyboard 菜单定义
│   ├── system/
│   │   ├── mod.rs
│   │   ├── info.rs       # 获取系统状态 (sysinfo)
│   │   ├── ops.rs        # 执行维护命令 (apt, xray, reboot)
│   │   └── service.rs    # Systemd 服务安装/卸载逻辑
│   └── scheduler/
│       └── mod.rs        # 定时任务管理器
└── tests/                # 集成测试
```

### 3.2 关键模块逻辑

#### A. 配置管理 (Config)
- 存储路径：`/etc/vps-tg-bot/config.toml` (或相对路径，视安装模式而定)。
- 字段：`bot_token`, `admin_chat_id`, `schedule_cron` (默认 "0 0 4 * * Sun")。

#### B. 系统操作 (System Ops)
- 封装 `Command::new("apt").arg("update")...` 等操作。
- **安全性**：确保只有在配置文件中指定的 `admin_chat_id` 才能触发敏感操作。
- **流式输出**：维护任务可能耗时较长，需考虑如何将进度反馈给用户（由于 TG 消息限制，建议仅发送“开始”和“结束+结果摘要”）。

#### C. 调度器 (Scheduler)
- 启动时初始化 `JobScheduler`。
- 注册每周维护任务：执行 `SystemOps::perform_maintenance()` -> 发送通知 -> 重启系统。

## 4. 依赖选择 (Crates)

| Crate | 用途 | 理由 |
| :--- | :--- | :--- |
| `teloxide` | Telegram Bot | 功能丰富，支持异步，类型安全，社区活跃 |
| `tokio` | Async Runtime | Rust 异步标准，teloxide 依赖 |
| `serde`, `serde_json`, `toml` | Serialization | 处理配置文件和 API 数据 |
| `clap` | CLI Parser | 解析 `install`, `uninstall`, `run` 子命令 |
| `sysinfo` | System Monitor | 跨平台获取系统资源使用情况 |
| `tokio-cron-scheduler` | Scheduler | 异步 Cron 调度 |
| `tracing`, `tracing-subscriber` | Logging | 结构化日志记录 |
| `anyhow` | Error Handling | 简化错误处理 |
| `reqwest` | HTTP Client | (Teloxide 内部使用) 用于网络请求 |
| `dotenvy` | Config | (可选) 开发环境配置加载 |

## 5. 伪代码与逻辑流

### 5.1 Main Entry (src/main.rs)
```rust
#[derive(Parser)]
enum Cli {
    Install,
    Uninstall,
    Run,
}

async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    match cli {
        Cli::Install => install_service().await?,
        Cli::Uninstall => uninstall_service().await?,
        Cli::Run => start_daemon().await?,
    }
    Ok(())
}

async fn start_daemon() -> Result<()> {
    let config = Config::load()?;
    
    // 1. 启动调度器
    let sched = JobScheduler::new().await?;
    sched.add(Job::new_async(config.schedule_cron, |_uuid, _l| {
        Box::pin(async {
            // 执行维护逻辑
            perform_weekly_maintenance().await;
        })
    })?).await?;
    sched.start().await?;

    // 2. 启动 Bot
    let bot = Bot::new(config.bot_token);
    Dispatcher::builder(bot, schema())
        .build()
        .dispatch()
        .await;
        
    Ok(())
}
```

### 5.2 维护任务逻辑 (src/system/ops.rs)
```rust
pub async fn perform_maintenance() -> String {
    let mut log = String::new();
    
    // 1. System Update
    log.push_str("🔄 Updating System...\n");
    match run_command("apt", &["update"]).await {
        Ok(_) => log.push_str("✅ Apt Update: Success\n"),
        Err(e) => log.push_str(&format!("❌ Apt Update: Failed ({})\n", e)),
    }
    // ... run apt full-upgrade, autoremove ...

    // 2. Xray/Sing-box Update
    if is_command_available("xray") {
        // run xray up
    }

    // 3. Return result
    log
}
```

### 5.3 Bot Handler (src/bot/handlers.rs)
```rust
pub async fn maintain_handler(bot: Bot, msg: Message, config: Config) -> ResponseResult<()> {
    // 鉴权
    if msg.chat.id != config.admin_chat_id {
        bot.send_message(msg.chat.id, "❌ Unauthorized").await?;
        return Ok(());
    }

    bot.send_message(msg.chat.id, "⏳ Starting maintenance...").await?;
    
    // 执行耗时任务
    let result = system::ops::perform_maintenance().await;
    
    bot.send_message(msg.chat.id, format!("✅ Maintenance Complete:\n```\n{}\n```", result))
        .parse_mode(ParseMode::MarkdownV2)
        .await?;
        
    // 重启提示
    bot.send_message(msg.chat.id, "⚠️ System will reboot in 5 seconds...").await?;
    tokio::time::sleep(Duration::from_secs(5)).await;
    system::ops::reboot()?;
    
    Ok(())
}
```

## 6. TDD 测试策略

由于涉及系统级操作，测试分为单元测试和集成测试（Mock）。

1.  **Config Test**:
    - 测试配置文件的序列化与反序列化。
    - 测试默认值的生成。

2.  **System Info Test (Mocked)**:
    - 编写 `SystemProvider` trait。
    - 在测试中 Mock `sysinfo` 的返回值，验证 Bot 状态消息的格式化逻辑。

3.  **Command Execution Test (Dry Run)**:
    - 为 `SystemOps` 增加 `dry_run` 模式。
    - 在测试模式下，不实际执行 `apt` 或 `reboot`，而是打印日志或返回模拟的 ExitStatus。
    - 验证命令构建的参数序列是否正确。

4.  **Scheduler Test**:
    - 测试 Cron 表达式解析。
    - 验证任务是否被正确添加到调度器。

## 7. 迁移步骤
1.  初始化 Rust 项目结构。
2.  实现 `Config` 和 `CLI` (Install/Uninstall) 基础骨架。
3.  实现 `SystemOps` 模块（核心维护逻辑）。
4.  集成 `teloxide` 实现 Bot 基本交互。
5.  连接 `Scheduler`。
6.  在测试 VPS 上编译并替换原 Shell 脚本进行验证。