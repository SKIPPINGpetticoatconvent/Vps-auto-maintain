use teloxide::prelude::*;
use teloxide::types::ChatId;
use teloxide::utils::command::BotCommands;
use crate::config::Config;
use crate::system;

#[derive(BotCommands, Clone)]
#[command(
    rename_rule = "lowercase",
    description = "支持以下命令:",
)]
pub enum Command {
    #[command(description = "启动机器人")]
    Start,
    #[command(description = "获取系统状态")]
    Status,
    #[command(description = "执行系统维护")]
    Maintain,
    #[command(description = "重启系统")]
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
            let help_message = "🚀 机器人已启动!\n\n可用命令:\n\n/status - 获取系统状态\n/maintain - 执行系统维护\n/reboot - 重启系统\n\n请输入 /status 查看当前系统状态";
            bot.send_message(message.chat.id, help_message).await?;
        }
        Command::Status => {
            match system::get_system_status() {
                Ok(status) => {
                    let reply = format!(
                        "📊 系统状态:\n\n{}",
                        format!("🔹 CPU 使用率: {:.2}%\n", status.cpu_usage) +
                        &format!("🔹 内存使用: {} MB / {} MB\n", status.memory_used / 1024 / 1024, status.memory_total / 1024 / 1024) +
                        &format!("🔹 磁盘使用: {} GB / {} GB\n", status.disk_used / 1024 / 1024 / 1024, status.disk_total / 1024 / 1024 / 1024) +
                        &format!("🔹 网络接收: {} MB\n", status.network_rx / 1024 / 1024) +
                        &format!("🔹 网络发送: {} MB\n", status.network_tx / 1024 / 1024) +
                        &format!("🔹 运行时间: {} 秒", status.uptime)
                    );
                    bot.send_message(message.chat.id, reply).await?;
                }
                Err(e) => {
                    bot.send_message(message.chat.id, format!("❌ 无法获取系统状态: {}", e)).await?;
                }
            }
        }
        Command::Maintain => {
            bot.send_message(message.chat.id, "🔄 正在执行系统维护...").await?;
            match system::ops::perform_maintenance().await {
                Ok(log) => {
                    bot.send_message(message.chat.id, format!("✅ 系统维护完成:\n{}", log)).await?;
                }
                Err(e) => {
                    bot.send_message(message.chat.id, format!("❌ 系统维护失败: {}", e)).await?;
                }
            }
        }
        Command::Reboot => {
            bot.send_message(message.chat.id, "⚠️ 确认重启系统？回复 'YES' 确认。").await?;
            // 注意: 重启确认逻辑需要额外的状态处理
            // 为简化，我们将在确认后继续重启
            // 在实际实现中，您需要跟踪确认状态
        }
    }
    Ok(())
}