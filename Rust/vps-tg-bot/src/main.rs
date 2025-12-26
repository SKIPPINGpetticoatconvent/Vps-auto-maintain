use clap::Parser;
use teloxide::Bot;
use env_logger::Env;
use tokio_cron_scheduler::JobSchedulerError;

mod bot;
mod config;
mod scheduler;
mod system;

#[derive(Parser, Debug)]
enum Cli {
    Install,
    Uninstall,
    Run,
}

#[tokio::main]
async fn main() {
    // 初始化日志记录器
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    
    let cli = Cli::parse();

    match cli {
        Cli::Install => {
            println!("Install mode");
        }
        Cli::Uninstall => {
            println!("Uninstall mode");
        }
        Cli::Run => {
            let config = match config::Config::load() {
                Ok(cfg) => cfg,
                Err(e) => {
                    log::error!("❌ 配置加载失败: {}", e);
                    return;
                }
            };
            let bot_instance = Bot::new(config.bot_token.clone());
            let config_for_scheduler = config.clone();

            log::info!("🚀 启动 VPS Telegram Bot...");
            
            // 首先启动调度器
            log::info!("⏰ 初始化调度器...");
            let scheduler_result = scheduler::start_scheduler(config_for_scheduler.clone(), bot_instance.clone()).await;
            if let Err(e) = scheduler_result {
                log::error!("❌ 调度器初始化失败: {:?}", e);
                return;
            }
            log::info!("✅ 调度器初始化成功");
            
            // 启动后台任务保持调度器运行
            let scheduler_config = config.clone();
            let scheduler_bot = bot_instance.clone();
            tokio::spawn(async move {
                log::info!("🔄 启动调度器后台任务...");
                loop {
                    tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;
                }
            });
            
            // 等待调度器完全初始化
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            
            // 然后启动 Bot
            log::info!("🤖 启动 Bot...");
            let bot_result = bot::run_bot(config).await;
            if let Err(e) = bot_result {
                log::error!("❌ Bot 启动失败: {}", e);
            }
        }
    }
}