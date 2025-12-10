use std::sync::Arc;
use std::sync::mpsc;
use std::thread;
use log::{info, error};
use env_logger::Env;

use vps_tg_bot::{
    config::Config,
    system::RealSystem,
    scheduler::Scheduler,
    bot::Bot,
};

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    
    info!("Starting VPS TG Bot...");

    // 1. Load Config
    let config = match Config::from_env() {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to load config: {}", e);
            std::process::exit(1);
        }
    };

    // 2. Initialize System Ops
    let system = Arc::new(RealSystem::new());

    // 3. Initialize Scheduler
    let mut scheduler = match Scheduler::new(&config, system.clone()) {
        Ok(s) => s,
        Err(e) => {
            error!("Failed to initialize scheduler: {}", e);
            std::process::exit(1);
        }
    };

    // 4. Initialize Bot
    let bot = match Bot::new(&config, system.clone()) {
        Ok(b) => b,
        Err(e) => {
            error!("Failed to initialize bot: {}", e);
            std::process::exit(1);
        }
    };

    // 5. Create communication channel
    let (tx, rx) = mpsc::channel();

    // 6. Spawn Scheduler in a separate thread
    thread::spawn(move || {
        scheduler.run(rx);
    });

    // 7. Start Bot Polling (in main thread/task)
    info!("Bot started polling...");
    if let Err(e) = bot.start_polling(tx).await {
        error!("Bot polling error: {}", e);
        std::process::exit(1);
    }
}
