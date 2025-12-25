use anyhow::Result;
use clap::Parser;
use teloxide::Bot;
use env_logger::Env;

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
async fn main() -> Result<()> {
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
            let config = config::Config::load()?;
            let bot_instance = Bot::new(config.bot_token.clone());
            let config_for_scheduler = config.clone();

            // 并行启动 Bot 和调度器
            let bot_task = tokio::spawn(async move {
                if let Err(e) = bot::run_bot(config.clone()).await {
                    eprintln!("Bot 错误: {}", e);
                }
            });

            let scheduler_task = tokio::spawn(async move {
                if let Err(e) = scheduler::start_scheduler(config_for_scheduler, bot_instance).await {
                    eprintln!("调度器错误: {}", e);
                }
            });

            // 等待两个任务完成
            tokio::try_join!(bot_task, scheduler_task)?;
        }
    }

    Ok(())
}