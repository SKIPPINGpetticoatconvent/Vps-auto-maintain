use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use log::{debug, error, info, warn};
use std::path::PathBuf;

mod bot;
mod config;
mod scheduler;
mod system;

use config::migration;
use config::Config;

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

    /// 初始化加密配置
    #[command(name = "init-config")]
    InitConfig {
        /// Bot Token
        #[arg(long)]
        token: String,
        /// Chat ID
        #[arg(long)]
        chat_id: i64,
        /// 输出加密配置文件路径
        #[arg(long, default_value = "/etc/vps-tg-bot-rust/config.enc")]
        output: PathBuf,
    },

    /// 迁移明文配置到加密存储
    #[command(name = "migrate-config")]
    MigrateConfig {
        /// 输入明文配置文件路径
        #[arg(long)]
        input: PathBuf,
        /// 输出加密配置文件路径（可选）
        #[arg(long)]
        output: Option<PathBuf>,
        /// 是否删除原明文文件
        #[arg(long, default_value = "false")]
        delete_legacy: bool,
    },

    /// 验证配置是否可用
    #[command(name = "verify-config")]
    VerifyConfig {
        /// 配置文件路径（可选，自动检测）
        #[arg(long)]
        path: Option<PathBuf>,
    },

    /// 导出解密配置（危险操作）
    #[command(name = "export-config")]
    ExportConfig {
        /// 输出明文配置文件路径
        #[arg(long)]
        output: PathBuf,
        /// 确认理解风险
        #[arg(long, help = "确认理解导出明文配置的风险")]
        confirm: bool,
    },

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
        Commands::InitConfig { token, chat_id, output } => {
            init_config(&token, chat_id, &output)?;
        }
        Commands::MigrateConfig {
            input,
            output,
            delete_legacy,
        } => {
            migrate_config(&input, output.as_ref(), delete_legacy)?;
        }
        Commands::VerifyConfig { path } => {
            verify_config(path.as_ref())?;
        }
        Commands::ExportConfig { output, confirm } => {
            export_config(&output, confirm)?;
        }
        Commands::CheckConfig => {
            check_config_status()?;
        }
    }

    Ok(())
}

/// 运行 Bot
async fn run_bot() -> Result<()> {
    info!("🚀 启动 VPS Telegram Bot...");

    let config = match config::Config::load() {
        Ok(cfg) => cfg,
        Err(e) => {
            error!("❌ 配置加载失败: {}", e);
            error!("💡 提示: 使用 'init-config' 命令初始化配置，或 'migrate-config' 迁移现有配置");
            return Err(anyhow::anyhow!("配置加载失败: {}", e));
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
    let scheduler_config = config.clone();
    let scheduler_bot = bot_instance.clone();
    tokio::spawn(async move {
        info!("🔄 启动调度器后台任务...");
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;
        }
    });

    // 等待调度器完全初始化
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // 然后启动 Bot
    info!("🤖 启动 Bot...");
    let bot_result = bot::run_bot(config).await;
    if let Err(e) = bot_result {
        error!("❌ Bot 启动失败: {}", e);
        return Err(anyhow::anyhow!("Bot 启动失败"));
    }

    Ok(())
}

/// 初始化加密配置
fn init_config(token: &str, chat_id: i64, output: &PathBuf) -> Result<()> {
    info!("🔧 初始化加密配置...");

    let config = migration::init_encrypted_config(
        token.to_string(),
        chat_id,
        output,
    ).context("初始化加密配置失败")?;

    info!("✅ 加密配置初始化成功");
    info!("📁 配置文件: {:?}", output);
    info!("🤖 Bot Token: {}...", &token[..20.min(token.len())]);
    info!("💬 Chat ID: {}", config.chat_id);
    info!("⏱️  检查间隔: {}秒", config.check_interval);

    println!("\n✅ 配置已成功初始化并加密保存！");
    println!("💡 提示: 现在可以使用 'run' 命令启动 Bot");

    Ok(())
}

/// 迁移明文配置到加密格式
fn migrate_config(input: &PathBuf, output: Option<&PathBuf>, delete_legacy: bool) -> Result<()> {
    info!("🔄 开始迁移明文配置到加密格式...");

    let output_path = output.cloned().unwrap_or_else(|| PathBuf::from("/etc/vps-tg-bot-rust/config.enc"));

    let result = migration::migrate_legacy_config(input, &output_path, delete_legacy);

    if result.success {
        info!("✅ 迁移成功完成");
        info!("📁 源文件: {:?}", result.source_path);
        info!("📁 目标文件: {:?}", result.destination_path);

        if result.deleted_legacy {
            info!("🗑️  已删除原明文配置文件");
        } else {
            warn!("⚠️  原明文配置文件仍保留，建议手动删除");
        }

        println!("\n✅ 配置迁移成功！");
        println!("💡 提示: 现在可以使用 'run' 命令启动 Bot");
    } else {
        error!("❌ 迁移失败: {}", result.message);
        return Err(anyhow::anyhow!("迁移失败: {}", result.message));
    }

    Ok(())
}

/// 验证配置是否可用
fn verify_config(path: Option<&PathBuf>) -> Result<()> {
    info!("🔍 验证配置...");

    let path_ref = path.as_ref().map(|p| p.as_path());
    let (valid, source, error) = migration::verify_config(path_ref);

    if valid {
        println!("✅ 配置有效");
        println!("📁 配置来源: {}", source);
    } else {
        println!("❌ 配置无效");
        println!("📁 配置来源: {}", source);
        if let Some(e) = error {
            println!("❌ 错误: {}", e);
        }
    }

    if !valid {
        return Err(anyhow::anyhow!("配置验证失败"));
    }

    Ok(())
}

/// 导出解密配置（危险操作）
fn export_config(output: &PathBuf, confirm: bool) -> Result<()> {
    warn!("⚠️  警告：此操作将导出解密配置到明文文件！");

    // 检测加密配置路径
    let encrypted_paths = migration::detect_encrypted_configs();
    if encrypted_paths.is_empty() {
        error!("❌ 未找到加密配置文件");
        return Err(anyhow::anyhow!("未找到加密配置文件"));
    }

    let encrypted_path = &encrypted_paths[0];
    info!("📁 检测到加密配置文件: {:?}", encrypted_path);

    migration::export_decrypted_config(encrypted_path, output, confirm)
        .context("导出配置失败")?;

    println!("✅ 配置已导出到明文文件: {:?}", output);
    println!("⚠️  警告: 请尽快删除此明文文件并使用加密配置！");

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

    // 检查加密配置
    println!("\n📋 加密配置文件:");
    let encrypted_configs = migration::detect_encrypted_configs();
    if !encrypted_configs.is_empty() {
        for path in &encrypted_configs {
            println!("  ✅ 发现加密配置: {:?}", path);
        }
    } else {
        println!("  ❌ 未找到加密配置文件");
    }

    // 检查明文配置
    println!("\n📋 明文配置文件:");
    let legacy_configs = migration::detect_legacy_configs();
    if !legacy_configs.is_empty() {
        for path in &legacy_configs {
            println!("  ⚠️  发现明文配置: {:?}", path);
            println!("     建议使用 'migrate-config' 迁移到加密格式");
        }
    } else {
        println!("  ℹ️  未找到明文配置文件");
    }

    // 检查是否需要迁移
    println!("\n📋 迁移建议:");
    let (needed, legacy_path, _) = migration::check_migration_needed();
    if needed {
        println!("  ⚠️  检测到明文配置，建议迁移到加密格式");
        if let Some(path) = legacy_path {
            println!("  💡 运行: cargo run -- migrate-config --input {:?}", path);
        }
    } else if encrypted_configs.is_empty() && legacy_configs.is_empty() {
        println!("  ℹ️  未找到任何配置");
        println!("  💡 运行: cargo run -- init-config --token <TOKEN> --chat-id <ID>");
    } else {
        println!("  ✅ 配置状态正常");
    }

    println!();

    Ok(())
}
