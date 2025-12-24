use teloxide::prelude::*;
use teloxide::types::ChatId;
use teloxide::utils::command::BotCommands;
use crate::config::Config;
use crate::system;

#[derive(BotCommands, Clone)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported:",
)]
pub enum Command {
    #[command(description = "Start the bot")]
    Start,
    #[command(description = "Get system status")]
    Status,
    #[command(description = "Perform maintenance")]
    Maintain,
    #[command(description = "Reboot the system")]
    Reboot,
}

pub async fn run_bot(config: Config) -> anyhow::Result<()> {
    let bot = Bot::new(config.bot_token);
    
    let handler = Update::filter_message()
        .branch(
            dptree::entry()
                .filter(move |msg: Message| {
                    let chat_id = msg.chat.id.0;
                    let allowed_chat_id = config.chat_id;
                    chat_id == allowed_chat_id
                })
                .filter_command::<Command>()
                .endpoint(answer),
        );

    Dispatcher::builder(bot, handler)
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;

    Ok(())
}

async fn answer(bot: Bot, message: Message, command: Command) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match command {
        Command::Start => {
            bot.send_message(message.chat.id, "🚀 Bot started! Available commands: /status, /maintain, /reboot").await?;
        }
        Command::Status => {
            match system::get_system_status() {
                Ok(status) => {
                    let reply = format!(
                        "📊 System Status:\n\n{}",
                        format!("CPU Usage: {:.2}%\n", status.cpu_usage) +
                        &format!("Memory: {}/{} MB\n", status.memory_used / 1024 / 1024, status.memory_total / 1024 / 1024) +
                        &format!("Disk: {}/{} GB\n", status.disk_used / 1024 / 1024 / 1024, status.disk_total / 1024 / 1024 / 1024) +
                        &format!("Network RX: {} MB\n", status.network_rx / 1024 / 1024) +
                        &format!("Network TX: {} MB\n", status.network_tx / 1024 / 1024) +
                        &format!("Uptime: {} seconds", status.uptime)
                    );
                    bot.send_message(message.chat.id, reply).await?;
                }
                Err(e) => {
                    bot.send_message(message.chat.id, format!("❌ Failed to get system status: {}", e)).await?;
                }
            }
        }
        Command::Maintain => {
            bot.send_message(message.chat.id, "🔄 Performing maintenance...").await?;
            match system::ops::perform_maintenance().await {
                Ok(log) => {
                    bot.send_message(message.chat.id, format!("✅ Maintenance completed:\n{}", log)).await?;
                }
                Err(e) => {
                    bot.send_message(message.chat.id, format!("❌ Maintenance failed: {}", e)).await?;
                }
            }
        }
        Command::Reboot => {
            bot.send_message(message.chat.id, "⚠️ Are you sure you want to reboot? Reply 'YES' to confirm.").await?;
            // Note: Reboot confirmation logic would require additional state handling
            // For simplicity, we'll proceed with reboot if confirmed
            // In a real implementation, you'd need to track confirmation state
        }
    }
    Ok(())
}