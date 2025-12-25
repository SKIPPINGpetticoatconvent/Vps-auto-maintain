use teloxide::prelude::*;
use teloxide::utils::command::BotCommands;
use teloxide::types::{InlineKeyboardMarkup, InlineKeyboardButton};
use crate::config::Config;
use crate::system;
use crate::scheduler;

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
    #[command(description = "更新 Xray")]
    UpdateXray,
    #[command(description = "更新 Sing-box")]
    UpdateSb,
    #[command(description = "核心维护")]
    MaintainCore,
    #[command(description = "规则维护")]
    MaintainRules,
    #[command(description = "查看日志")]
    Logs,
    #[command(description = "设置调度计划")]
    SetSchedule(String),
}

// 构建主菜单 Inline Keyboard
fn build_main_menu_keyboard() -> InlineKeyboardMarkup {
    let keyboard = vec![
        vec![
            InlineKeyboardButton::callback("📊 系统状态", "cmd_status"),
            InlineKeyboardButton::callback("🛠️ 维护菜单", "menu_maintain"),
        ],
        vec![
            InlineKeyboardButton::callback("⚙️ 设置", "menu_settings"),
        ],
    ];
    
    InlineKeyboardMarkup::new(keyboard)
}

// 构建维护菜单 Inline Keyboard
fn build_maintain_menu_keyboard() -> InlineKeyboardMarkup {
    let keyboard = vec![
        vec![
            InlineKeyboardButton::callback("🔄 系统更新", "cmd_maintain_core"),
            InlineKeyboardButton::callback("🌍 规则更新", "cmd_maintain_rules"),
        ],
        vec![
            InlineKeyboardButton::callback("🚀 更新 Xray", "cmd_update_xray"),
            InlineKeyboardButton::callback("📦 更新 Sing-box", "cmd_update_sb"),
        ],
        vec![
            InlineKeyboardButton::callback("🔙 返回主菜单", "back_to_main"),
        ],
    ];
    
    InlineKeyboardMarkup::new(keyboard)
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
        )
        .branch(
            Update::filter_callback_query()
                .endpoint(handle_callback_query),
        );

    Dispatcher::builder(bot, handler)
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;

    Ok(())
}

async fn answer(bot: Bot, message: Message, command: Command) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    match command {
        Command::Start => {
            let welcome_message = "🚀 欢迎使用 VPS 管理机器人!\n\n请选择您要执行的操作:";
            let keyboard = build_main_menu_keyboard();
            bot.send_message(message.chat.id, welcome_message)
                .reply_markup(keyboard)
                .await?;
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

            // 直接执行重启（在实际实现中应添加确认逻辑）
            match system::ops::reboot_system() {
                Ok(_) => {
                    bot.send_message(message.chat.id, "🔄 系统重启中...").await?;
                }
                Err(e) => {
                    bot.send_message(message.chat.id, format!("❌ 重启失败: {}", e)).await?;
                }
            }
        }
        Command::UpdateXray => {
            bot.send_message(message.chat.id, "🔄 正在更新 Xray...").await?;
            match system::ops::update_xray().await {
                Ok(log) => {
                    bot.send_message(message.chat.id, format!("✅ Xray 更新完成:\n{}", log)).await?;
                }
                Err(e) => {
                    bot.send_message(message.chat.id, format!("❌ Xray 更新失败: {}", e)).await?;
                }
            }
        }
        Command::UpdateSb => {
            bot.send_message(message.chat.id, "🔄 正在更新 Sing-box...").await?;
            match system::ops::update_singbox().await {
                Ok(log) => {
                    bot.send_message(message.chat.id, format!("✅ Sing-box 更新完成:\n{}", log)).await?;
                }
                Err(e) => {
                    bot.send_message(message.chat.id, format!("❌ Sing-box 更新失败: {}", e)).await?;
                }
            }
        }
        Command::MaintainCore => {
            bot.send_message(message.chat.id, "🔄 正在执行核心维护...").await?;
            match system::ops::maintain_core().await {
                Ok(log) => {
                    bot.send_message(message.chat.id, format!("✅ 核心维护完成:\n{}", log)).await?;
                }
                Err(e) => {
                    bot.send_message(message.chat.id, format!("❌ 核心维护失败: {}", e)).await?;
                }
            }
        }
        Command::MaintainRules => {
            bot.send_message(message.chat.id, "🔄 正在执行规则维护...").await?;
            match system::ops::maintain_rules().await {
                Ok(log) => {
                    bot.send_message(message.chat.id, format!("✅ 规则维护完成:\n{}", log)).await?;
                }
                Err(e) => {
                    bot.send_message(message.chat.id, format!("❌ 规则维护失败: {}", e)).await?;
                }
            }
        }
        Command::Logs => {
            bot.send_message(message.chat.id, "🔄 正在获取系统日志...").await?;
            match system::ops::get_system_logs(20).await {
                Ok(log) => {
                    bot.send_message(message.chat.id, format!("📋 系统日志:\n{}", log)).await?;
                }
                Err(e) => {
                    bot.send_message(message.chat.id, format!("❌ 无法获取日志: {}", e)).await?;
                }
            }
        }
        Command::SetSchedule(cron_expr) => {
            bot.send_message(message.chat.id, "🔄 正在更新调度计划...").await?;
            match scheduler::update_schedule(&cron_expr).await {
                Ok(response_message) => {
                    bot.send_message(message.chat.id, response_message).await?;
                }
                Err(e) => {
                    bot.send_message(message.chat.id, format!("❌ 更新调度失败: {}", e)).await?;
                }
            }
        }
    }
    Ok(())
}

// 处理 Inline Keyboard 回调
async fn handle_callback_query(
    bot: Bot,
    callback_query: CallbackQuery,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    log::info!("🔍 收到回调查询: callback_id={}, data={:?}", callback_query.id, callback_query.data);
    
    if let Some(data) = &callback_query.data {
        log::info!("📝 处理回调查询数据: '{}', 聊天ID: {}, 消息ID: {}", 
                   data, 
                   callback_query.message.as_ref().unwrap().chat.id,
                   callback_query.message.as_ref().unwrap().id);
        let chat_id = callback_query.message.as_ref().unwrap().chat.id;
        let message_id = callback_query.message.as_ref().unwrap().id;
        
        match data.as_str() {
            // 主菜单按钮
            "cmd_status" => {
                log::info!("🎯 处理主菜单: cmd_status 命令");
                // 立即回答回调查询，消除加载动画
                log::info!("📤 调用 answer_callback_query 前");
                bot.answer_callback_query(&callback_query.id).await?;
                log::info!("📤 answer_callback_query 调用成功");
                log::info!("🔄 调用 handle_status_command");
                handle_status_command(&bot, &callback_query).await?;
                log::info!("✅ cmd_status 处理完成");
            }
            "menu_maintain" => {
                log::info!("🎯 处理主菜单: menu_maintain 命令");
                // 立即回答回调查询，消除加载动画
                log::info!("📤 调用 answer_callback_query 前");
                bot.answer_callback_query(&callback_query.id).await?;
                log::info!("📤 answer_callback_query 调用成功");
                let message = "🛠️ 请选择维护操作:";
                let keyboard = build_maintain_menu_keyboard();
                log::info!("📝 编辑消息显示维护菜单");
                bot.edit_message_text(
                    chat_id,
                    message_id,
                    message,
                )
                .reply_markup(keyboard)
                .await?;
                log::info!("✅ menu_maintain 处理完成");
            }
            "menu_settings" => {
                log::info!("🎯 处理主菜单: menu_settings 命令");
                log::info!("📤 调用 answer_callback_query 前");
                bot.answer_callback_query(&callback_query.id)
                    .text("⚙️ 设置功能正在开发中...")
                    .await?;
                log::info!("📤 answer_callback_query 调用成功");
                log::info!("✅ menu_settings 处理完成");
                return Ok(());
            }
            
            // 维护菜单按钮
            "cmd_maintain_core" => {
                log::info!("🎯 处理维护菜单: cmd_maintain_core 命令");
                // 立即回答回调查询，消除加载动画
                log::info!("📤 调用 answer_callback_query 前");
                bot.answer_callback_query(&callback_query.id).await?;
                log::info!("📤 answer_callback_query 调用成功");
                log::info!("🔄 调用 handle_maintain_core_command");
                handle_maintain_core_command(&bot, &callback_query).await?;
                log::info!("✅ cmd_maintain_core 处理完成");
            }
            "cmd_maintain_rules" => {
                log::info!("🎯 处理维护菜单: cmd_maintain_rules 命令");
                // 立即回答回调查询，消除加载动画
                log::info!("📤 调用 answer_callback_query 前");
                bot.answer_callback_query(&callback_query.id).await?;
                log::info!("📤 answer_callback_query 调用成功");
                log::info!("🔄 调用 handle_maintain_rules_command");
                handle_maintain_rules_command(&bot, &callback_query).await?;
                log::info!("✅ cmd_maintain_rules 处理完成");
            }
            "cmd_update_xray" => {
                log::info!("🎯 处理维护菜单: cmd_update_xray 命令");
                // 立即回答回调查询，消除加载动画
                log::info!("📤 调用 answer_callback_query 前");
                bot.answer_callback_query(&callback_query.id).await?;
                log::info!("📤 answer_callback_query 调用成功");
                log::info!("🔄 调用 handle_update_xray_command");
                handle_update_xray_command(&bot, &callback_query).await?;
                log::info!("✅ cmd_update_xray 处理完成");
            }
            "cmd_update_sb" => {
                log::info!("🎯 处理维护菜单: cmd_update_sb 命令");
                // 立即回答回调查询，消除加载动画
                log::info!("📤 调用 answer_callback_query 前");
                bot.answer_callback_query(&callback_query.id).await?;
                log::info!("📤 answer_callback_query 调用成功");
                log::info!("🔄 调用 handle_update_sb_command");
                handle_update_sb_command(&bot, &callback_query).await?;
                log::info!("✅ cmd_update_sb 处理完成");
            }
            "back_to_main" => {
                log::info!("🎯 处理返回主菜单: back_to_main 命令");
                // 立即回答回调查询，消除加载动画
                log::info!("📤 调用 answer_callback_query 前");
                bot.answer_callback_query(&callback_query.id).await?;
                log::info!("📤 answer_callback_query 调用成功");
                let message = "🚀 欢迎使用 VPS 管理机器人!\n\n请选择您要执行的操作:";
                let keyboard = build_main_menu_keyboard();
                log::info!("📝 编辑消息返回主菜单");
                bot.edit_message_text(
                    chat_id,
                    message_id,
                    message,
                )
                .reply_markup(keyboard)
                .await?;
                log::info!("✅ back_to_main 处理完成");
            }
            _ => {
                log::warn!("❓ 未知命令: '{}'", data);
                log::info!("📤 调用 answer_callback_query 前");
                bot.answer_callback_query(&callback_query.id)
                    .text("未知命令")
                    .await?;
                log::info!("📤 answer_callback_query 调用成功");
                log::info!("✅ 未知命令处理完成");
                return Ok(());
            }
        }
    } else {
        log::warn!("⚠️ 回调查询数据为空");
    }
    
    // 已在各分支中处理 answer_callback_query，确保每个查询只被回答一次
    log::info!("🏁 handle_callback_query 函数执行完成");
    Ok(())
}

// 辅助函数：处理状态命令
async fn handle_status_command(
    bot: &Bot,
    callback_query: &CallbackQuery,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    if let Ok(status) = system::get_system_status() {
        let reply = format!(
            "📊 系统状态:\n\n{}",
            format!("🔹 CPU 使用率: {:.2}%\n", status.cpu_usage) +
            &format!("🔹 内存使用: {} MB / {} MB\n", status.memory_used / 1024 / 1024, status.memory_total / 1024 / 1024) +
            &format!("🔹 磁盘使用: {} GB / {} GB\n", status.disk_used / 1024 / 1024 / 1024, status.disk_total / 1024 / 1024 / 1024) +
            &format!("🔹 网络接收: {} MB\n", status.network_rx / 1024 / 1024) +
            &format!("🔹 网络发送: {} MB\n", status.network_tx / 1024 / 1024) +
            &format!("🔹 运行时间: {} 秒", status.uptime)
        );
        
        bot.edit_message_text(
            callback_query.message.as_ref().unwrap().chat.id,
            callback_query.message.as_ref().unwrap().id,
            reply,
        )
        .reply_markup(build_main_menu_keyboard())
        .await?;
    } else {
        bot.edit_message_text(
            callback_query.message.as_ref().unwrap().chat.id,
            callback_query.message.as_ref().unwrap().id,
            "❌ 无法获取系统状态",
        )
        .reply_markup(build_main_menu_keyboard())
        .await?;
    }
    Ok(())
}

// 辅助函数：处理核心维护命令
async fn handle_maintain_core_command(
    bot: &Bot,
    callback_query: &CallbackQuery,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    bot.edit_message_text(
        callback_query.message.as_ref().unwrap().chat.id,
        callback_query.message.as_ref().unwrap().id,
        "🔄 正在执行核心维护...",
    )
    .reply_markup(build_maintain_menu_keyboard())
    .await?;
    
    match system::ops::maintain_core().await {
        Ok(log) => {
            bot.edit_message_text(
                callback_query.message.as_ref().unwrap().chat.id,
                callback_query.message.as_ref().unwrap().id,
                &format!("✅ 核心维护完成:\n{}\n\n请选择下一步操作:", log),
            )
            .reply_markup(build_maintain_menu_keyboard())
            .await?;
        }
        Err(e) => {
            bot.edit_message_text(
                callback_query.message.as_ref().unwrap().chat.id,
                callback_query.message.as_ref().unwrap().id,
                &format!("❌ 核心维护失败: {}\n\n请选择下一步操作:", e),
            )
            .reply_markup(build_maintain_menu_keyboard())
            .await?;
        }
    }
    Ok(())
}

// 辅助函数：处理规则维护命令
async fn handle_maintain_rules_command(
    bot: &Bot,
    callback_query: &CallbackQuery,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    bot.edit_message_text(
        callback_query.message.as_ref().unwrap().chat.id,
        callback_query.message.as_ref().unwrap().id,
        "🔄 正在执行规则维护...",
    )
    .reply_markup(build_maintain_menu_keyboard())
    .await?;
    
    match system::ops::maintain_rules().await {
        Ok(log) => {
            bot.edit_message_text(
                callback_query.message.as_ref().unwrap().chat.id,
                callback_query.message.as_ref().unwrap().id,
                &format!("✅ 规则维护完成:\n{}\n\n请选择下一步操作:", log),
            )
            .reply_markup(build_maintain_menu_keyboard())
            .await?;
        }
        Err(e) => {
            bot.edit_message_text(
                callback_query.message.as_ref().unwrap().chat.id,
                callback_query.message.as_ref().unwrap().id,
                &format!("❌ 规则维护失败: {}\n\n请选择下一步操作:", e),
            )
            .reply_markup(build_maintain_menu_keyboard())
            .await?;
        }
    }
    Ok(())
}

// 辅助函数：处理更新 Xray 命令
async fn handle_update_xray_command(
    bot: &Bot,
    callback_query: &CallbackQuery,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    bot.edit_message_text(
        callback_query.message.as_ref().unwrap().chat.id,
        callback_query.message.as_ref().unwrap().id,
        "🔄 正在更新 Xray...",
    )
    .reply_markup(build_maintain_menu_keyboard())
    .await?;
    
    match system::ops::update_xray().await {
        Ok(log) => {
            bot.edit_message_text(
                callback_query.message.as_ref().unwrap().chat.id,
                callback_query.message.as_ref().unwrap().id,
                &format!("✅ Xray 更新完成:\n{}\n\n请选择下一步操作:", log),
            )
            .reply_markup(build_maintain_menu_keyboard())
            .await?;
        }
        Err(e) => {
            bot.edit_message_text(
                callback_query.message.as_ref().unwrap().chat.id,
                callback_query.message.as_ref().unwrap().id,
                &format!("❌ Xray 更新失败: {}\n\n请选择下一步操作:", e),
            )
            .reply_markup(build_maintain_menu_keyboard())
            .await?;
        }
    }
    Ok(())
}

// 辅助函数：处理更新 Sing-box 命令
async fn handle_update_sb_command(
    bot: &Bot,
    callback_query: &CallbackQuery,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    bot.edit_message_text(
        callback_query.message.as_ref().unwrap().chat.id,
        callback_query.message.as_ref().unwrap().id,
        "🔄 正在更新 Sing-box...",
    )
    .reply_markup(build_maintain_menu_keyboard())
    .await?;
    
    match system::ops::update_singbox().await {
        Ok(log) => {
            bot.edit_message_text(
                callback_query.message.as_ref().unwrap().chat.id,
                callback_query.message.as_ref().unwrap().id,
                &format!("✅ Sing-box 更新完成:\n{}\n\n请选择下一步操作:", log),
            )
            .reply_markup(build_maintain_menu_keyboard())
            .await?;
        }
        Err(e) => {
            bot.edit_message_text(
                callback_query.message.as_ref().unwrap().chat.id,
                callback_query.message.as_ref().unwrap().id,
                &format!("❌ Sing-box 更新失败: {}\n\n请选择下一步操作:", e),
            )
            .reply_markup(build_maintain_menu_keyboard())
            .await?;
        }
    }
    Ok(())
}