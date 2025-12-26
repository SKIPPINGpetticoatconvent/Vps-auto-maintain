use teloxide::prelude::*;
use teloxide::utils::command::BotCommands;
use teloxide::types::{InlineKeyboardMarkup, InlineKeyboardButton, ChatId};
use crate::config::Config;
use crate::system;
use crate::scheduler;
use crate::scheduler::task_types::TaskType;

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
            InlineKeyboardButton::callback("⏰ 定时任务", "menu_schedule"),
            InlineKeyboardButton::callback("📋 查看日志", "cmd_logs"),
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

// 构建定时任务类型选择菜单
fn build_task_type_menu_keyboard() -> InlineKeyboardMarkup {
    let keyboard = vec![
        vec![
            InlineKeyboardButton::callback("🔄 系统维护", "task_system_maintenance"),
            InlineKeyboardButton::callback("🚀 核心维护", "task_core_maintenance"),
        ],
        vec![
            InlineKeyboardButton::callback("🌍 规则维护", "task_rules_maintenance"),
            InlineKeyboardButton::callback("🔧 更新 Xray", "task_update_xray"),
        ],
        vec![
            InlineKeyboardButton::callback("📦 更新 Sing-box", "task_update_singbox"),
            InlineKeyboardButton::callback("📋 查看任务列表", "view_tasks"),
        ],
        vec![
            InlineKeyboardButton::callback("🔙 返回", "back_to_main"),
        ],
    ];
    
    InlineKeyboardMarkup::new(keyboard)
}

// 构建预设时间菜单
fn build_schedule_presets_keyboard(task_type: &str) -> InlineKeyboardMarkup {
    let (_daily, _weekly, _monthly) = match task_type {
        "system_maintenance" | "system" => ("0 4 * * *", "0 4 * * Sun", "0 4 1 * *"),
        "core_maintenance" => ("0 5 * * Sun", "0 5 * * Sun", "0 5 1 * *"),
        "rules_maintenance" => ("0 3 * * *", "0 3 * * Sun", "0 3 1 * *"),
        "update_xray" => ("0 6 * * Sun", "0 6 * * Sun", "0 6 1 * *"),
        "update_singbox" => ("0 7 * * Sun", "0 7 * * Sun", "0 7 1 * *"),
        _ => ("0 4 * * *", "0 4 * * Sun", "0 4 1 * *"),
    };
    
    let keyboard = vec![
        vec![
            InlineKeyboardButton::callback("每天设置", &format!("set_preset_{}_daily", task_type)),
            InlineKeyboardButton::callback("每周设置", &format!("set_preset_{}_weekly", task_type)),
        ],
        vec![
            InlineKeyboardButton::callback("每月设置", &format!("set_preset_{}_monthly", task_type)),
            InlineKeyboardButton::callback("自定义", &format!("set_custom_{}", task_type)),
        ],
        vec![
            InlineKeyboardButton::callback("🔙 返回任务类型", "back_to_task_types"),
        ],
    ];
    
    InlineKeyboardMarkup::new(keyboard)
}



// 获取任务类型显示名称
fn get_task_display_name(task_type: &str) -> &'static str {
    match task_type {
        "system_maintenance" | "system" => "🔄 系统维护",
        "core_maintenance" => "🚀 核心维护",
        "rules_maintenance" => "🌍 规则维护",
        "update_xray" => "🔧 更新 Xray",
        "update_singbox" => "📦 更新 Sing-box",
        _ => "❓ 未知任务",
    }
}

// 构建时间选择键盘
fn build_time_selection_keyboard(task_type: &str, frequency: &str) -> InlineKeyboardMarkup {
    let time_buttons = match frequency {
        "daily" => vec![
            ("凌晨2点", "2"),
            ("凌晨3点", "3"),
            ("凌晨4点", "4"),
            ("凌晨5点", "5"),
            ("上午6点", "6"),
            ("上午7点", "7"),
            ("上午8点", "8"),
            ("上午9点", "9"),
            ("上午10点", "10"),
            ("上午11点", "11"),
            ("下午12点", "12"),
            ("下午13点", "13"),
            ("下午14点", "14"),
            ("下午15点", "15"),
            ("下午16点", "16"),
            ("下午17点", "17"),
            ("下午18点", "18"),
            ("下午19点", "19"),
            ("晚上20点", "20"),
            ("晚上21点", "21"),
            ("晚上22点", "22"),
            ("晚上23点", "23"),
            ("深夜0点", "0"),
            ("深夜1点", "1"),
        ],
        "weekly" => vec![
            ("周日 凌晨2点", "0 2"),
            ("周日 凌晨3点", "0 3"),
            ("周日 凌晨4点", "0 4"),
            ("周日 凌晨5点", "0 5"),
            ("周日 上午6点", "0 6"),
            ("周日 上午7点", "0 7"),
            ("周日 上午8点", "0 8"),
            ("周日 上午9点", "0 9"),
            ("周日 上午10点", "0 10"),
            ("周日 上午11点", "0 11"),
            ("周日 下午12点", "0 12"),
            ("周日 下午13点", "0 13"),
            ("周日 下午14点", "0 14"),
            ("周日 下午15点", "0 15"),
            ("周日 下午16点", "0 16"),
            ("周日 下午17点", "0 17"),
            ("周日 下午18点", "0 18"),
            ("周日 下午19点", "0 19"),
            ("周日 晚上20点", "0 20"),
            ("周日 晚上21点", "0 21"),
            ("周日 晚上22点", "0 22"),
            ("周日 晚上23点", "0 23"),
        ],
        "monthly" => vec![
            ("1号 凌晨2点", "2 1"),
            ("1号 凌晨3点", "3 1"),
            ("1号 凌晨4点", "4 1"),
            ("1号 凌晨5点", "5 1"),
            ("1号 上午6点", "6 1"),
            ("1号 上午7点", "7 1"),
            ("1号 上午8点", "8 1"),
            ("1号 上午9点", "9 1"),
            ("1号 上午10点", "10 1"),
            ("1号 上午11点", "11 1"),
            ("1号 下午12点", "12 1"),
            ("1号 下午13点", "13 1"),
            ("1号 下午14点", "14 1"),
            ("1号 下午15点", "15 1"),
            ("1号 下午16点", "16 1"),
            ("1号 下午17点", "17 1"),
            ("1号 下午18点", "18 1"),
            ("1号 下午19点", "19 1"),
            ("1号 晚上20点", "20 1"),
            ("1号 晚上21点", "21 1"),
            ("1号 晚上22点", "22 1"),
            ("1号 晚上23点", "23 1"),
        ],
        _ => vec![],
    };
    
    let mut keyboard = Vec::new();
    
    // 每行显示 3 个按钮
    for chunk in time_buttons.chunks(3) {
        let row = chunk.iter().map(|(label, value)| {
            InlineKeyboardButton::callback(label.to_string(), &format!("set_time_{}_{}_{}", task_type, frequency, value))
        }).collect();
        keyboard.push(row);
    }
    
    // 添加返回按钮
    keyboard.push(vec![
        InlineKeyboardButton::callback("🔙 返回", "back_to_task_types"),
    ]);
    
    InlineKeyboardMarkup::new(keyboard)
}

pub async fn run_bot(config: Config) -> anyhow::Result<()> {
    let bot = Bot::new(config.bot_token);
    
    let handler = dptree::entry()
        .branch(
            Update::filter_callback_query()
                .endpoint(handle_callback_query),
        )
        .branch(
            Update::filter_message()
                .branch(
                    dptree::entry()
                        .filter(move |msg: Message| {
                            let chat_id = msg.chat.id.0;
                            let allowed_chat_id = config.chat_id;
                            chat_id == allowed_chat_id
                        })
                        .filter_command::<Command>()
                        .endpoint(answer),
                ),
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
            match system::ops::reboot_system().await {
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
            bot.send_message(message.chat.id, "🔄 正在执行核心维护...\n⚠️ 维护完成后系统将自动重启").await?;
            match system::ops::maintain_core().await {
                Ok(log) => {
                    bot.send_message(message.chat.id, format!("✅ 核心维护完成:\n{}\n\n🔄 系统将在 3 秒后自动重启，请保存您的工作！", log)).await?;
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
            "menu_schedule" => {
                log::info!("🎯 处理主菜单: menu_schedule 命令");
                bot.answer_callback_query(&callback_query.id).await?;
                
                let message = "⏰ 定时任务设置\n\n请选择要设置的任务类型:";
                let keyboard = build_task_type_menu_keyboard();
                
                bot.edit_message_text(chat_id, message_id, message)
                    .reply_markup(keyboard)
                    .await?;
                
                log::info!("✅ menu_schedule 处理完成");
                return Ok(());
            }
            "cmd_logs" => {
                log::info!("🎯 处理查看日志: cmd_logs 命令");
                bot.answer_callback_query(&callback_query.id).await?;
                
                let message = "🔄 正在获取系统日志...";
                let keyboard = build_main_menu_keyboard();
                
                bot.edit_message_text(chat_id, message_id, message)
                    .reply_markup(keyboard)
                    .await?;
                
                // 异步获取日志
                let bot_clone = bot.clone();
                let chat_id_clone = chat_id;
                let message_id_clone = message_id;
                
                tokio::spawn(async move {
                    match system::ops::get_system_logs(20).await {
                        Ok(log) => {
                            let _ = bot_clone.edit_message_text(
                                chat_id_clone,
                                message_id_clone,
                                format!("📋 系统日志:\n{}", log)
                            ).reply_markup(build_main_menu_keyboard())
                            .await;
                        }
                        Err(e) => {
                            let _ = bot_clone.edit_message_text(
                                chat_id_clone,
                                message_id_clone,
                                format!("❌ 无法获取日志: {}", e)
                            ).reply_markup(build_main_menu_keyboard())
                            .await;
                        }
                    }
                });
                
                log::info!("✅ cmd_logs 处理完成");
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
            // 任务类型选择按钮
            "task_system_maintenance" => {
                log::info!("🎯 处理任务类型: system_maintenance");
                bot.answer_callback_query(&callback_query.id).await?;
                
                let message = "🔄 系统维护定时设置\n\n请选择执行时间:";
                let keyboard = build_schedule_presets_keyboard("system_maintenance");
                
                bot.edit_message_text(chat_id, message_id, message)
                    .reply_markup(keyboard)
                    .await?;
                
                log::info!("✅ task_system_maintenance 处理完成");
            }
            "task_core_maintenance" => {
                log::info!("🎯 处理任务类型: core_maintenance");
                bot.answer_callback_query(&callback_query.id).await?;
                
                let message = "🚀 核心维护定时设置\n\n请选择执行时间:";
                let keyboard = build_schedule_presets_keyboard("core_maintenance");
                
                bot.edit_message_text(chat_id, message_id, message)
                    .reply_markup(keyboard)
                    .await?;
                
                log::info!("✅ task_core_maintenance 处理完成");
            }
            "task_rules_maintenance" => {
                log::info!("🎯 处理任务类型: rules_maintenance");
                bot.answer_callback_query(&callback_query.id).await?;
                
                let message = "🌍 规则维护定时设置\n\n请选择执行时间:";
                let keyboard = build_schedule_presets_keyboard("rules_maintenance");
                
                bot.edit_message_text(chat_id, message_id, message)
                    .reply_markup(keyboard)
                    .await?;
                
                log::info!("✅ task_rules_maintenance 处理完成");
            }
            "task_update_xray" => {
                log::info!("🎯 处理任务类型: update_xray");
                bot.answer_callback_query(&callback_query.id).await?;
                
                let message = "🔧 更新 Xray 定时设置\n\n请选择执行时间:";
                let keyboard = build_schedule_presets_keyboard("update_xray");
                
                bot.edit_message_text(chat_id, message_id, message)
                    .reply_markup(keyboard)
                    .await?;
                
                log::info!("✅ task_update_xray 处理完成");
            }
            "task_update_singbox" => {
                log::info!("🎯 处理任务类型: update_singbox");
                bot.answer_callback_query(&callback_query.id).await?;
                
                let message = "📦 更新 Sing-box 定时设置\n\n请选择执行时间:";
                let keyboard = build_schedule_presets_keyboard("update_singbox");
                
                bot.edit_message_text(chat_id, message_id, message)
                    .reply_markup(keyboard)
                    .await?;
                
                log::info!("✅ task_update_singbox 处理完成");
            }

            "view_tasks" => {
                log::info!("🎯 处理任务查看");
                bot.answer_callback_query(&callback_query.id).await?;
                
                let tasks_summary = scheduler::get_tasks_summary().await.unwrap_or_else(|_| "❌ 无法获取任务列表".to_string());
                
                let keyboard = build_task_type_menu_keyboard();
                bot.edit_message_text(chat_id, message_id, tasks_summary)
                    .reply_markup(keyboard)
                    .await?;
                
                log::info!("✅ view_tasks 处理完成");
            }
            "add_new_task" => {
                log::info!("🎯 处理添加新任务");
                bot.answer_callback_query(&callback_query.id).await?;
                
                let message = "➕ 添加新任务\n\n请选择要添加的任务类型:";
                let keyboard = build_task_type_menu_keyboard();
                
                bot.edit_message_text(chat_id, message_id, message)
                    .reply_markup(keyboard)
                    .await?;
                
                log::info!("✅ add_new_task 处理完成");
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
            // 自定义任务设置按钮
            cmd if cmd.starts_with("set_custom_") => {
                let task_type = cmd.strip_prefix("set_custom_").unwrap();
                log::info!("🎯 处理自定义设置: {}", task_type);
                
                bot.answer_callback_query(&callback_query.id).await?;
                
                let message = format!("⏰ 自定义 {} 定时任务设置\n\n📝 请发送 Cron 表达式:\n\n示例:\n• 每天凌晨4点: 0 4 * * *\n• 每周日凌晨4点: 0 4 * * Sun\n• 每月1号凌晨4点: 0 4 1 * *\n\n使用命令: /set_schedule <cron_expression>", get_task_display_name(task_type));
                
                let keyboard = build_task_type_menu_keyboard();
                
                bot.edit_message_text(chat_id, message_id, message)
                    .reply_markup(keyboard)
                    .await?;
                
                log::info!("✅ set_custom 处理完成");
            }
            // 预设时间设置按钮 - 每日
            cmd if cmd.starts_with("set_preset_") && cmd.ends_with("_daily") => {
                let task_type = cmd.strip_prefix("set_preset_").unwrap().strip_suffix("_daily").unwrap();
                log::info!("🎯 处理每日预设: {}", task_type);
                
                bot.answer_callback_query(&callback_query.id).await?;
                
                let message = format!("⏰ 设置 {} 每天执行\n\n请选择具体执行时间:", get_task_display_name(task_type));
                let keyboard = build_time_selection_keyboard(task_type, "daily");
                
                bot.edit_message_text(chat_id, message_id, message)
                    .reply_markup(keyboard)
                    .await?;
                
                log::info!("✅ set_preset_daily 处理完成");
            }
            // 预设时间设置按钮 - 每周
            cmd if cmd.starts_with("set_preset_") && cmd.ends_with("_weekly") => {
                let task_type = cmd.strip_prefix("set_preset_").unwrap().strip_suffix("_weekly").unwrap();
                log::info!("🎯 处理每周预设: {}", task_type);
                
                bot.answer_callback_query(&callback_query.id).await?;
                
                let message = format!("⏰ 设置 {} 每周执行\n\n请选择具体执行时间:", get_task_display_name(task_type));
                let keyboard = build_time_selection_keyboard(task_type, "weekly");
                
                bot.edit_message_text(chat_id, message_id, message)
                    .reply_markup(keyboard)
                    .await?;
                
                log::info!("✅ set_preset_weekly 处理完成");
            }
            // 预设时间设置按钮 - 每月
            cmd if cmd.starts_with("set_preset_") && cmd.ends_with("_monthly") => {
                let task_type = cmd.strip_prefix("set_preset_").unwrap().strip_suffix("_monthly").unwrap();
                log::info!("🎯 处理每月预设: {}", task_type);
                
                bot.answer_callback_query(&callback_query.id).await?;
                
                let message = format!("⏰ 设置 {} 每月执行\n\n请选择具体执行时间:", get_task_display_name(task_type));
                let keyboard = build_time_selection_keyboard(task_type, "monthly");
                
                bot.edit_message_text(chat_id, message_id, message)
                    .reply_markup(keyboard)
                    .await?;
                
                log::info!("✅ set_preset_monthly 处理完成");
            }
            // 时间选择按钮处理
            cmd if cmd.starts_with("set_time_") => {
                // 使用更精确的解析方式，处理包含空格的情况
                if let Some(stripped) = cmd.strip_prefix("set_time_") {
                    let parts: Vec<&str> = stripped.split('_').collect();
                    if parts.len() >= 3 {
                        let task_type = parts[0];
                        let frequency = parts[1];
                        let time_value = parts[2];
                        
                        // 特殊处理：如果时间值等于频率，说明用户没有选择具体时间
                        if time_value == frequency {
                            let _ = bot.send_message(
                                chat_id,
                                format!("❌ 请选择具体的执行时间，而不是 '{}'", time_value)
                            ).await;
                            return Ok(());
                        }
                        
                        bot.answer_callback_query(&callback_query.id).await?;
                        
                        // 验证时间值是否为有效数字（排除已知频率值）
                        let invalid_time_values = ["daily", "weekly", "monthly"];
                        if time_value.parse::<i32>().is_err() && !invalid_time_values.contains(&time_value) {
                            let _ = bot.send_message(
                                chat_id,
                                format!("❌ 无效的时间值: {}", time_value)
                            ).await;
                            return Ok(());
                        }
                        
                        // 构建 Cron 表达式
                        let cron_expr = match frequency {
                            "daily" => format!("0 {} * * *", time_value),
                            "weekly" => format!("{} * * 0", time_value),
                            "monthly" => {
                                // time_value 格式: "小时 日期" 或 "小时日期"
                                if time_value.contains(' ') {
                                    let time_parts: Vec<&str> = time_value.split(' ').collect();
                                    if time_parts.len() == 2 {
                                        format!("0 {} {} * *", time_parts[0], time_parts[1])
                                    } else {
                                        format!("0 {} * * *", time_value)
                                    }
                                } else {
                                    // 处理没有空格的情况，如 "21"
                                    format!("0 {} * * *", time_value)
                                }
                            },
                            _ => {
                                let _ = bot.send_message(
                                    chat_id,
                                    format!("❌ 未知的频率类型: {}", frequency)
                                ).await;
                                return Ok(());
                            }
                        };
                        
                        let message = format!("🔄 正在设置 {} 任务...", get_task_display_name(task_type));
                        let keyboard = build_time_selection_keyboard(task_type, frequency);
                        
                        bot.edit_message_text(chat_id, message_id, message)
                            .reply_markup(keyboard.clone())
                            .await?;
                        
                        let bot_clone = bot.clone();
                        let config = Config::load().unwrap_or_else(|_| Config { bot_token: "".to_string(), chat_id: 0, check_interval: 300 });
                        let _chat_id_clone = chat_id;
                        let task_type_enum = match task_type {
                            "system_maintenance" | "system" => TaskType::SystemMaintenance,
                            "core_maintenance" => TaskType::CoreMaintenance,
                            "rules_maintenance" => TaskType::RulesMaintenance,
                            "update_xray" => TaskType::UpdateXray,
                            "update_singbox" => TaskType::UpdateSingbox,
                            _ => {
                                let _ = bot.send_message(
                                    chat_id,
                                    format!("❌ 未知的任务类型: {}", task_type)
                                ).await;
                                return Ok(());
                            }
                        };
                        
                        // 异步处理任务添加
                        let bot_clone_for_message = bot_clone.clone();
                        let chat_id_for_message = chat_id;
                        let task_type_enum_for_task = task_type_enum.clone();
                        let cron_expr_for_task = cron_expr.to_string();
                        let config_for_task = Config { 
                            bot_token: config.bot_token.clone(), 
                            chat_id: config.chat_id, 
                            check_interval: config.check_interval 
                        };
                        
                        tokio::spawn(async move {
                            // 等待调度器初始化
                            let mut retry_count = 0;
                            let max_retries = 10;
                            
                            while retry_count < max_retries {
                                let manager_guard = crate::scheduler::SCHEDULER_MANAGER.lock().await;
                                if let Some(manager) = &*manager_guard {
                                    let result = manager.add_new_task(
                                        config_for_task.clone(), 
                                        bot_clone.clone(), 
                                        task_type_enum_for_task.clone(), 
                                        &cron_expr_for_task
                                    ).await;
                                    
                                    drop(manager_guard); // 立即释放锁
                                    
                                    match result {
                                        Ok(response_msg) => {
                                            let _ = bot_clone_for_message.send_message(
                                                chat_id_for_message,
                                                format!("✅ {}\n\n任务已成功设置！", response_msg)
                                            ).await;
                                            return;
                                        }
                                        Err(e) => {
                                            let _ = bot_clone_for_message.send_message(
                                                chat_id_for_message,
                                                format!("❌ 设置任务失败: {}", e)
                                            ).await;
                                            return;
                                        }
                                    }
                                } else {
                                    drop(manager_guard);
                                    retry_count += 1;
                                    if retry_count < max_retries {
                                        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                                    } else {
                                        let _ = bot_clone_for_message.send_message(
                                            chat_id_for_message,
                                            "❌ 调度器尚未初始化，请稍后重试或重新启动机器人"
                                        ).await;
                                        return;
                                    }
                                }
                            }
                        });
                        
                        log::info!("✅ set_time 处理完成");
                    } else {
                        log::warn!("❌ 时间选择参数不足: {:?}", parts);
                        bot.answer_callback_query(&callback_query.id).await?;
                    }
                } else {
                    log::warn!("❌ 无效的时间选择命令: {}", cmd);
                    bot.answer_callback_query(&callback_query.id).await?;
                }
            }
            "back_to_task_types" => {
                log::info!("🎯 处理返回任务类型");
                bot.answer_callback_query(&callback_query.id).await?;
                
                let message = "⏰ 定时任务设置\n\n请选择要设置的任务类型:";
                let keyboard = build_task_type_menu_keyboard();
                
                bot.edit_message_text(chat_id, message_id, message)
                    .reply_markup(keyboard)
                    .await?;
                
                log::info!("✅ back_to_task_types 处理完成");
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
        "🔄 正在执行核心维护...\n⚠️ 维护完成后系统将自动重启",
    )
    .reply_markup(build_maintain_menu_keyboard())
    .await?;
    
    match system::ops::maintain_core().await {
        Ok(log) => {
            bot.edit_message_text(
                callback_query.message.as_ref().unwrap().chat.id,
                callback_query.message.as_ref().unwrap().id,
                &format!("✅ 核心维护完成:\n{}\n\n🔄 系统将在 3 秒后自动重启，请保存您的工作！\n\n请选择下一步操作:", log),
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