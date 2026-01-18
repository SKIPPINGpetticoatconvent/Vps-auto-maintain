use anyhow::Result;
use clap::{Parser, Subcommand};
use is_terminal::IsTerminal;
use log::{debug, error, info, warn};
use std::io;

mod bot;
mod config;
mod scheduler;
mod system;

/// VPS Telegram Bot - Rust 版本
#[derive(Parser, Debug)]
#[command(name = "vps-tg-bot-rust")]
#[command(about = "VPS Telegram Bot - Rust 版本", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// 运行 Bot（默认命令）
    Run,

    /// 检查配置状态
    #[command(name = "check-config")]
    CheckConfig,
}

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志记录器
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let cli = Cli::parse();

    // 如果没有子命令，默认执行 run
    let command = cli.command.unwrap_or(Commands::Run);

    match command {
        Commands::Run => {
            run_bot().await?;
        }
        Commands::CheckConfig => {
            check_config_status()?;
        }
    }

    Ok(())
}

/// 等待并重新加载配置（用于 systemd 环境）
#[allow(dead_code)]
async fn wait_and_reload_config() -> Result<config::Config> {
    info!("⏳ 等待配置初始化（最多等待 60 秒）...");
    
    let max_attempts = 12; // 12 * 5 = 60 秒
    let delay_duration = std::time::Duration::from_secs(5);
    
    for attempt in 1..=max_attempts {
        info!("🔄 尝试加载配置 (第 {} 次，共 {} 次)", attempt, max_attempts);
        
        match config::Config::load() {
            Ok(config) => {
                info!("✅ 配置加载成功");
                return Ok(config);
            }
            Err(e) => {
                warn!("⚠️  第 {} 次配置加载失败: {}", attempt, e);
                
                if attempt < max_attempts {
                    info!("⏱️  等待 {} 秒后重试...", delay_duration.as_secs());
                    tokio::time::sleep(delay_duration).await;
                } else {
                    error!("❌ 达到最大重试次数，配置加载失败");
                    return Err(anyhow::anyhow!("配置加载最终失败: {}", e));
                }
            }
        }
    }
    
    Err(anyhow::anyhow!("配置重试超时"))
}

/// 处理非交互式环境的配置加载失败
async fn handle_non_interactive_config_failure(original_error: &anyhow::Error) -> Result<config::Config> {
    error!("❌ 非交互式环境配置加载失败");
    
    // 检测运行环境
    let is_systemd = std::env::var("SYSTEMD_EXEC_PID").is_ok() || 
                     std::env::var("INVOCATION_ID").is_ok() ||
                     std::path::Path::new("/run/systemd/system").exists();
    
    let is_container = std::env::var("container").is_ok() ||
                      std::path::Path::new("/.dockerenv").exists() ||
                      std::path::Path::new("/run/.containerenv").exists();
    
    // 提供详细的诊断信息
    error!("🔍 诊断信息:");
    error!("  运行环境: {}", if is_systemd { "systemd" } else if is_container { "container" } else { "unknown" });
    error!("  错误类型: {}", original_error);
    
    // 提供恢复建议
    provide_recovery_suggestions(is_systemd, is_container).await;
    
    Err(anyhow::anyhow!("非交互式环境配置加载失败: {}", original_error))
}

/// 提供恢复建议
async fn provide_recovery_suggestions(is_systemd: bool, is_container: bool) {
    error!("💡 恢复建议:");
    
    if is_systemd {
        error!("  🔧 systemd 环境:");
        error!("    1. 检查安装脚本是否正确执行");
        error!("    2. 确保环境变量文件存在: /etc/vps-tg-bot-rust/env");
        error!("    3. 验证环境变量文件权限: ls -la /etc/vps-tg-bot-rust/env");
        error!("    4. 重启服务: systemctl restart vps-tg-bot-rust");
        error!("    5. 检查服务状态: systemctl status vps-tg-bot-rust");
        error!("    6. 查看详细日志: journalctl -u vps-tg-bot-rust -f");
    } else if is_container {
        error!("  🐳 容器环境:");
        error!("    1. 确保容器有足够的权限访问文件系统");
        error!("    2. 检查容器是否以 root 权限运行");
        error!("    3. 挂载必要的卷: -v /etc/vps-tg-bot-rust:/etc/vps-tg-bot-rust");
        error!("    4. 设置环境变量: BOT_TOKEN, CHAT_ID");
    } else {
        error!("  🖥️  普通环境:");
        error!("    1. 设置环境变量: export BOT_TOKEN=<TOKEN> && export CHAT_ID=<ID>");
        error!("    2. 或使用环境文件: source /path/to/env");
    }
    
    error!("  📋 通用建议:");
    error!("    • 检查 BOT_TOKEN 是否有效");
    error!("    • 检查 CHAT_ID 是否正确");
    error!("    • 确保有读取环境变量文件的权限");
    error!("    • 查看详细错误日志");
}

/// 发送启动通知
async fn send_startup_notification(bot: &teloxide::Bot, chat_id: i64) {
    use teloxide::prelude::Requester;
    use teloxide::types::ChatId;
    use teloxide::payloads::SendMessageSetters;
    
    // 获取系统运行时间
    let uptime = match tokio::process::Command::new("uptime")
        .arg("-p")
        .output()
        .await
    {
        Ok(output) => String::from_utf8_lossy(&output.stdout).trim().to_string(),
        Err(_) => "未知".to_string(),
    };
    
    // 获取系统启动时间
    let boot_time = match tokio::process::Command::new("uptime")
        .arg("-s")
        .output()
        .await
    {
        Ok(output) => String::from_utf8_lossy(&output.stdout).trim().to_string(),
        Err(_) => "未知".to_string(),
    };
    
    let message = format!(
        "✅ *VPS Bot 已上线*\n\n\
        🖥️ 系统启动时间: {}\n\
        ⏱️ 运行时长: {}\n\n\
        📋 Bot 已就绪，可以接收命令。",
        boot_time, uptime
    );
    
    if let Err(_) = bot.send_message(ChatId(chat_id), message)
        .parse_mode(teloxide::types::ParseMode::MarkdownV2)
        .await
    {
        // 如果 MarkdownV2 失败，尝试纯文本
        let plain_message = format!(
            "✅ VPS Bot 已上线\n\n\
            🖥️ 系统启动时间: {}\n\
            ⏱️ 运行时长: {}\n\n\
            📋 Bot 已就绪，可以接收命令。",
            boot_time, uptime
        );
        if let Err(e) = bot.send_message(ChatId(chat_id), plain_message).await {
            warn!("发送启动通知失败: {}", e);
        }
    }
}

/// 运行 Bot
async fn run_bot() -> Result<()> {
    info!("🚀 启动 VPS Telegram Bot...");

    let config = match config::Config::load() {
        Ok(cfg) => {
            info!("✅ 配置加载成功");
            cfg
        },
        Err(e) => {
            warn!("⚠️  配置加载失败: {}", e);
            
            // 检测是否为交互式终端
            if io::stdin().is_terminal() {
                println!("\nℹ️  检测到首次运行或配置丢失。");
                println!("🛠️  请设置以下环境变量:");
                println!("  export BOT_TOKEN=\"你的Bot Token\"");
                println!("  export CHAT_ID=\"你的Chat ID\"");
                println!("  或创建环境文件: /etc/vps-tg-bot-rust/env");
                println!("  格式:");
                println!("  BOT_TOKEN=你的Bot Token");
                println!("  CHAT_ID=你的Chat ID");
                println!("\n✅ 设置完成后重新启动 Bot");
                
                return Err(anyhow::anyhow!("请先配置环境变量"));
            } else {
                // 非交互式环境，使用增强的错误处理
                match handle_non_interactive_config_failure(&e).await {
                    Ok(config) => config,
                    Err(_) => return Err(anyhow::anyhow!("配置加载失败: {}", e)),
                }
            }
        }
    };

    info!("✅ 配置加载成功");
    debug!("Chat ID: {}", config.chat_id);
    debug!("Check Interval: {}秒", config.check_interval);

    let bot_instance = teloxide::Bot::new(config.bot_token.clone());
    let config_for_scheduler = config.clone();

    // 首先启动调度器
    info!("⏰ 初始化调度器...");
    let scheduler_result = scheduler::start_scheduler(config_for_scheduler.clone(), bot_instance.clone()).await;
    if let Err(e) = scheduler_result {
        error!("❌ 调度器初始化失败: {:?}", e);
        return Err(anyhow::anyhow!("调度器初始化失败"));
    }
    info!("✅ 调度器初始化成功");

    // 初始化维护历史管理器
    info!("📜 初始化维护历史管理器...");
    let history_result = scheduler::maintenance_history::init_maintenance_history().await;
    if let Err(e) = history_result {
        error!("❌ 维护历史管理器初始化失败: {:?}", e);
        return Err(anyhow::anyhow!("维护历史管理器初始化失败"));
    }
    info!("✅ 维护历史管理器初始化成功");

    // 启动后台任务保持调度器运行
    let _scheduler_config = config.clone();
    let _scheduler_bot = bot_instance.clone();
    tokio::spawn(async move {
        info!("🔄 启动调度器后台任务...");
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;
        }
    });

    // 等待调度器完全初始化
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // 发送启动通知
    info!("📢 发送启动通知...");
    send_startup_notification(&bot_instance, config.chat_id).await;

    // 然后启动 Bot
    info!("🤖 启动 Bot...");
    let bot_result = bot::run_bot(config).await;
    if let Err(e) = bot_result {
        error!("❌ Bot 启动失败: {}", e);
        return Err(anyhow::anyhow!("Bot 启动失败"));
    }

    Ok(())
}

/// 检查配置状态
fn check_config_status() -> Result<()> {
    println!("\n🔍 检查配置状态...\n");

    // 检查环境变量
    println!("📋 环境变量:");
    let bot_token = std::env::var("BOT_TOKEN").ok();
    let chat_id = std::env::var("CHAT_ID").ok();
    let check_interval = std::env::var("CHECK_INTERVAL").ok();

    if bot_token.is_some() {
        println!("  ✅ BOT_TOKEN 已设置");
    } else {
        println!("  ❌ BOT_TOKEN 未设置");
    }
    if chat_id.is_some() {
        println!("  ✅ CHAT_ID 已设置");
    } else {
        println!("  ❌ CHAT_ID 未设置");
    }
    if let Some(interval) = &check_interval {
        println!("  ℹ️  CHECK_INTERVAL: {}", interval);
    }

    // 检查环境文件
    println!("\n📋 环境文件:");
    let env_file_path = "/etc/vps-tg-bot-rust/env";
    if std::path::Path::new(env_file_path).exists() {
        println!("  ✅ 环境文件存在: {}", env_file_path);
        
        // 检查文件权限
        if let Ok(metadata) = std::fs::metadata(env_file_path) {
            let permissions = metadata.permissions();
            println!("  📁 文件权限: {:?}", permissions);
        }
    } else {
        println!("  ❌ 环境文件不存在: {}", env_file_path);
    }

    println!("\n📋 配置建议:");
    if bot_token.is_none() && chat_id.is_none() {
        println!("  ⚠️  未设置必需的环境变量");
        println!("  💡 创建环境文件: {}", env_file_path);
        println!("  💡 格式:");
        println!("     BOT_TOKEN=你的Bot Token");
        println!("     CHAT_ID=你的Chat ID");
    } else if bot_token.is_none() || chat_id.is_none() {
        println!("  ⚠️  部分环境变量未设置");
        println!("  💡 请确保 BOT_TOKEN 和 CHAT_ID 都已设置");
    } else {
        println!("  ✅ 环境变量配置完整");
    }

    println!();

    Ok(())
}