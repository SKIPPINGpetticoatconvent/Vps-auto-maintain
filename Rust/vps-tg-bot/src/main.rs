use anyhow::Result;
use clap::Parser;
use teloxide::Bot;

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

            // 并行启动 Bot 和 Scheduler
            let bot_task = tokio::spawn(async move {
                if let Err(e) = bot::run_bot(config.clone()).await {
                    eprintln!("Bot error: {}", e);
                }
            });

            let scheduler_task = tokio::spawn(async move {
                if let Err(e) = scheduler::start_scheduler(config_for_scheduler, bot_instance).await {
                    eprintln!("Scheduler error: {}", e);
                }
            });

            // 等待两个任务完成
            tokio::try_join!(bot_task, scheduler_task)?;
        }
    }

    Ok(())
}