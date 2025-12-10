use std::sync::Arc;
use std::sync::mpsc;
use teloxide::prelude::*;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup, ParseMode};
use teloxide::utils::command::BotCommands; 
use crate::system::SystemOps;
use crate::config::Config;
use crate::scheduler::{SchedulerCommand, JobType};
use crate::error::BotError;

pub struct Bot {
    bot: teloxide::Bot,
    system: Arc<dyn SystemOps>,
    config: Config,
}

impl Bot {
    pub fn new(config: &Config, system: Arc<dyn SystemOps>) -> Result<Self, BotError> {
        let bot = teloxide::Bot::new(&config.tg_token);
        Ok(Self {
            bot,
            system,
            config: config.clone(),
        })
    }

    pub async fn start_polling(&self, tx: mpsc::Sender<SchedulerCommand>) -> Result<(), BotError> {
        let bot = self.bot.clone();
        let config = self.config.clone();
        let system = self.system.clone();
        let tx_clone = tx;

        let handler = Update::filter_message()
            .branch(
                dptree::filter(move |msg: Message| msg.chat.id.0 == config.tg_chat_id)
                    .filter_command::<Command>()
                    .endpoint(answer)
            );
            
        let callback_handler = Update::filter_callback_query()
            .branch(
                dptree::filter(move |q: CallbackQuery| q.message.map(|m| m.chat.id.0 == config.tg_chat_id).unwrap_or(false))
                    .endpoint(handle_callback)
            );

        let system_for_handler = system.clone();
        let config_for_handler = self.config.clone();
        
        // Removed enable_ctrlc_handler() since it's not available in this version of teloxide or DispatcherBuilder
        // We will rely on manual termination or the orchestrator's signal handling if needed,
        // or just let it run until process kill.
        Dispatcher::builder(bot, 
            dptree::entry()
                .branch(handler)
                .branch(callback_handler)
        )
        .dependencies(dptree::deps![system_for_handler, config_for_handler, tx_clone])
        .build()
        .dispatch()
        .await;

        Ok(())
    }

    pub fn is_authorized(&self, chat_id: i64) -> bool {
        chat_id == self.config.tg_chat_id
    }
}

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "These commands are supported:")]
enum Command {
    #[command(description = "Start interaction with bot")]
    Start,
    #[command(description = "Show menu")]
    Menu,
}

async fn answer(
    bot: teloxide::Bot,
    msg: Message,
    cmd: Command,
    _system: Arc<dyn SystemOps>,
    _config: Config,
    _tx: mpsc::Sender<SchedulerCommand>,
) -> ResponseResult<()> {
    match cmd {
        Command::Start | Command::Menu => {
            let keyboard = make_main_keyboard();
            bot.send_message(msg.chat.id, "🤖 *VPS 管理 Bot*\n\n欢迎使用 VPS 自动化管理系统\n请选择操作：")
                .parse_mode(ParseMode::MarkdownV2) 
                .reply_markup(keyboard)
                .await?;
        }
    };
    Ok(())
}

async fn handle_callback(
    bot: teloxide::Bot,
    q: CallbackQuery,
    system: Arc<dyn SystemOps>,
    _config: Config,
    tx: mpsc::Sender<SchedulerCommand>,
) -> ResponseResult<()> {
    if let Some(data) = q.data {
        let chat_id = q.message.as_ref().map(|m| m.chat.id).unwrap();
        
        match data.as_str() {
            "status" => {
                match system.get_system_info() {
                    Ok(info) => {
                        let text = format!(
                            "📊 *系统状态*\n\n⏱ 运行时间: {}s\n🧠 内存: {}/{} MB\n⚖️ 负载: {:.2}, {:.2}, {:.2}",
                            info.uptime,
                            info.memory_used / 1024 / 1024,
                            info.memory_total / 1024 / 1024,
                            info.load_avg[0], info.load_avg[1], info.load_avg[2]
                        );
                        let escaped_text = text.replace(".", "\\.").replace("-", "\\-");
                        bot.send_message(chat_id, escaped_text).parse_mode(ParseMode::MarkdownV2).await?;
                    },
                    Err(e) => {
                        bot.send_message(chat_id, format!("❌ 获取状态失败: {}", e)).await?;
                    }
                }
            },
            "maintain_now" => {
                 let keyboard = InlineKeyboardMarkup::new(vec![
                    vec![InlineKeyboardButton::callback("🔧 核心维护", "maintain_core")],
                    vec![InlineKeyboardButton::callback("📜 规则更新", "maintain_rules")],
                ]);
                bot.send_message(chat_id, "🔧 *维护操作*\n请选择：")
                    .parse_mode(ParseMode::MarkdownV2)
                    .reply_markup(keyboard)
                    .await?;
            },
            "maintain_core" => {
                bot.send_message(chat_id, "⏳ 正在执行核心维护...").await?;
                let _ = tx.send(SchedulerCommand::ForceRun(JobType::CoreMaintain));
            },
            "maintain_rules" => {
                bot.send_message(chat_id, "⏳ 正在执行规则更新...").await?;
                 let _ = tx.send(SchedulerCommand::ForceRun(JobType::RulesUpdate));
            },
            "view_logs" => {
                 match system.get_service_logs("vps-tg-bot", 20) {
                     Ok(logs) => {
                         let escaped_logs = logs.replace("`", "\\`");
                         bot.send_message(chat_id, format!("📋 *日志*\n```\n{}\n```", escaped_logs))
                            .parse_mode(ParseMode::MarkdownV2)
                            .await?;
                     },
                     Err(e) => {
                         bot.send_message(chat_id, format!("❌ 获取日志失败: {}", e)).await?;
                     }
                 }
            },
            _ => {
                 bot.send_message(chat_id, "❓ 未知命令").await?;
            }
        }
    }
    Ok(())
}

fn make_main_keyboard() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![
        vec![InlineKeyboardButton::callback("📊 系统状态", "status")],
        vec![InlineKeyboardButton::callback("🔧 立即维护", "maintain_now")],
        vec![InlineKeyboardButton::callback("📋 查看日志", "view_logs")],
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::system::MockSystemOps;
    use std::path::PathBuf;

    fn create_test_config(chat_id: i64) -> Config {
        Config {
            tg_token: "token".to_string(),
            tg_chat_id: chat_id,
            state_path: PathBuf::from("/tmp"),
            scripts_path: PathBuf::from("/tmp"),
            logs_service: "service".to_string(),
        }
    }

    #[test]
    fn test_is_authorized() {
        let system = Arc::new(MockSystemOps::new());
        let config = create_test_config(123456789);
        
        let bot = Bot::new(&config, system).unwrap();

        assert!(bot.is_authorized(123456789));
        assert!(!bot.is_authorized(987654321));
    }
}
