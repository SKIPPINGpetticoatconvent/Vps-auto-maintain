use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use log::{debug, error, info, warn};
use std::path::{PathBuf, Path};
use std::io::{self, Write, IsTerminal};

mod bot;
mod config;
mod scheduler;
mod system;

use config::migration;

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
            migrate_config(&input, output, delete_legacy)?;
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

/// 等待并重新加载配置（用于 systemd 环境）
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
    
    // 检查配置文件状态
    check_config_file_status().await;
    
    // 如果是 systemd 环境，尝试等待和重试
    if is_systemd {
        warn!("⚠️  检测到 systemd 环境，尝试等待配置初始化...");
        
        match wait_and_reload_config().await {
            Ok(config) => {
                info!("✅ 在 systemd 环境中成功加载配置");
                return Ok(config);
            }
            Err(e) => {
                error!("❌ systemd 环境配置重试失败: {}", e);
            }
        }
    }
    
    // 提供恢复建议
    provide_recovery_suggestions(is_systemd, is_container).await;
    
    Err(anyhow::anyhow!("非交互式环境配置加载失败: {}", original_error))
}

/// 检查配置文件状态
async fn check_config_file_status() {
    use crate::config::migration;
    
    let encrypted_configs = migration::detect_encrypted_configs();
    let legacy_configs = migration::detect_legacy_configs();
    
    if !encrypted_configs.is_empty() {
        error!("📁 发现加密配置文件:");
        for path in &encrypted_configs {
            if let Ok(metadata) = std::fs::metadata(path) {
                let size = metadata.len();
                let modified = metadata.modified()
                    .map(|t| format!("{:?}", t))
                    .unwrap_or_else(|_| "unknown".to_string());
                error!("    {:?} (大小: {} 字节, 修改时间: {})", path, size, modified);
            } else {
                error!("    {:?} (无法读取元数据)", path);
            }
        }
    }
    
    if !legacy_configs.is_empty() {
        error!("⚠️  发现明文配置文件（建议迁移到加密格式）:");
        for path in &legacy_configs {
            error!("    {:?}", path);
        }
    }
    
    if encrypted_configs.is_empty() && legacy_configs.is_empty() {
        error!("📁 未找到任何配置文件");
    }
}

/// 提供恢复建议
async fn provide_recovery_suggestions(is_systemd: bool, is_container: bool) {
    error!("💡 恢复建议:");
    
    if is_systemd {
        error!("  🔧 systemd 环境:");
        error!("    1. 检查安装脚本是否正确执行");
        error!("    2. 手动初始化配置: vps-tg-bot-rust init-config --token <TOKEN> --chat-id <ID>");
        error!("    3. 验证配置: vps-tg-bot-rust verify-config");
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
        error!("    1. 初始化配置: vps-tg-bot-rust init-config --token <TOKEN> --chat-id <ID>");
        error!("    2. 或设置环境变量: export BOT_TOKEN=<TOKEN> && export CHAT_ID=<ID>");
        error!("    3. 验证配置: vps-tg-bot-rust verify-config");
    }
    
    error!("  📋 通用建议:");
    error!("    • 检查 BOT_TOKEN 是否有效");
    error!("    • 检查 CHAT_ID 是否正确");
    error!("    • 确保有写入配置目录的权限");
    error!("    • 查看详细错误日志");
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
            if std::io::stdin().is_terminal() {
                println!("\nℹ️  检测到首次运行或配置丢失。");
                println!("🛠️  进入交互式配置模式...\n");
                
                let token = loop {
                    match prompt_input("请输入 BOT_TOKEN: ") {
                        Ok(t) if !t.is_empty() => break t,
                        _ => println!("❌ Token 不能为空，请重新输入"),
                    }
                };
                
                let chat_id = loop {
                    match prompt_input("请输入 CHAT_ID: ") {
                        Ok(s) => match s.parse::<i64>() {
                            Ok(id) => break id,
                            Err(_) => println!("❌ 无效的 Chat ID (应为数字)，请重新输入"),
                        },
                        Err(_) => println!("❌ 输入错误，请重新输入"),
                    }
                };

                // 确定配置文件路径
                let default_path = PathBuf::from("/etc/vps-tg-bot-rust/config.enc");
                let local_path = PathBuf::from("config.enc");
                
                // 尝试使用默认路径，如果目录不可写则使用当前目录
                let output_path = if let Some(parent) = default_path.parent() {
                    if parent.exists() {
                         match std::fs::metadata(parent) {
                            Ok(meta) if !meta.permissions().readonly() => default_path,
                            _ => local_path,
                         }
                    } else {
                        // 尝试创建目录
                        match std::fs::create_dir_all(parent) {
                            Ok(_) => default_path,
                            Err(_) => local_path,
                        }
                    }
                } else {
                    local_path
                };

                // 初始化配置
                match init_config(&token, chat_id, &output_path) {
                    Ok(_) => {
                        info!("✅ 配置初始化完成，重新加载配置...");
                        config::Config::load()?
                    },
                    Err(err) => {
                        error!("❌ 配置初始化失败: {}", err);
                        return Err(err);
                    }
                }
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

    // 然后启动 Bot
    info!("🤖 启动 Bot...");
    let bot_result = bot::run_bot(config).await;
    if let Err(e) = bot_result {
        error!("❌ Bot 启动失败: {}", e);
        return Err(anyhow::anyhow!("Bot 启动失败"));
    }

    Ok(())
}

/// 提示用户输入
fn prompt_input(prompt: &str) -> Result<String> {
    print!("{}", prompt);
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_string())
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
fn migrate_config(input: &Path, output: Option<PathBuf>, delete_legacy: bool) -> Result<()> {
    info!("🔄 开始迁移明文配置到加密格式...");

    let output_path = output.unwrap_or_else(|| PathBuf::from("/etc/vps-tg-bot-rust/config.enc"));

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
