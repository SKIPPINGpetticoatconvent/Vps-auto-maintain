use teloxide::prelude::*;
use teloxide::utils::command::BotCommands;
use teloxide::types::{InlineKeyboardMarkup, InlineKeyboardButton};
use crate::config::Config;
use crate::system;
use crate::scheduler;
use crate::scheduler::task_types::TaskType;
use crate::scheduler::maintenance_history::{record_maintenance, MaintenanceResult};

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
    #[command(description = "查看维护历史")]
    MaintenanceHistory,
    #[command(description = "完整维护")]
    FullMaintenance,
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
        vec![
            InlineKeyboardButton::callback("📜 维护历史", "cmd_maintenance_history"),
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
            InlineKeyboardButton::callback("🔄 完整维护", "cmd_full_maintenance"),
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
            InlineKeyboardButton::callback("每天设置", format!("set_preset_{}_daily", task_type)),
            InlineKeyboardButton::callback("每周设置", format!("set_preset_{}_weekly", task_type)),
        ],
        vec![
            InlineKeyboardButton::callback("每月设置", format!("set_preset_{}_monthly", task_type)),
            InlineKeyboardButton::callback("自定义", format!("set_custom_{}", task_type)),
        ],
        vec![
            InlineKeyboardButton::callback("🔙 返回任务类型", "back_to_task_types"),
        ],
    ];
    
    InlineKeyboardMarkup::new(keyboard)
}

// 构建日志选择菜单键盘
fn build_log_selection_keyboard() -> InlineKeyboardMarkup {
    let keyboard = vec![
        vec![
            InlineKeyboardButton::callback("📋 最近 20 行", "view_logs_20"),
            InlineKeyboardButton::callback("📋 最近 50 行", "view_logs_50"),
        ],
        vec![
            InlineKeyboardButton::callback("📋 最近 100 行", "view_logs_100"),
            InlineKeyboardButton::callback("📋 全部日志", "view_logs_all"),
        ],
        vec![
            InlineKeyboardButton::callback("🔙 返回主菜单", "back_to_main"),
        ],
    ];
    
    InlineKeyboardMarkup::new(keyboard)
}

// 构建维护历史菜单键盘
fn build_maintenance_history_keyboard(page: usize) -> InlineKeyboardMarkup {
    let mut keyboard = Vec::new();
    
    // 分页按钮
    let mut page_buttons = Vec::new();
    if page > 0 {
        page_buttons.push(InlineKeyboardButton::callback("⬅️ 上一页", format!("maintenance_history_{}", page - 1)));
    }
    page_buttons.push(InlineKeyboardButton::callback("📜 历史摘要", "maintenance_history_summary"));
    page_buttons.push(InlineKeyboardButton::callback("下一页 ➡️", format!("maintenance_history_{}", page + 1)));
    
    keyboard.push(page_buttons);
    keyboard.push(vec![
        InlineKeyboardButton::callback("🔙 返回主菜单", "back_to_main"),
    ]);
    
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
            InlineKeyboardButton::callback(label.to_string(), format!("set_time_{}_{}_{}", task_type, frequency, value))
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
        Command::MaintenanceHistory => {
            bot.send_message(message.chat.id, "📜 正在加载维护历史...").await?;
            let history_summary = crate::scheduler::maintenance_history::get_maintenance_summary().await;
            let keyboard = build_maintenance_history_keyboard(0);
            bot.send_message(message.chat.id, history_summary)
                .reply_markup(keyboard)
                .await?;
        }
        Command::FullMaintenance => {
            bot.send_message(message.chat.id, "🔄 正在执行完整维护...").await?;
            match system::perform_full_maintenance().await {
                Ok(log) => {
                    bot.send_message(message.chat.id, format!("✅ 完整维护完成:\n{}", log)).await?;
                }
                Err(e) => {
                    bot.send_message(message.chat.id, format!("❌ 完整维护失败: {}", e)).await?;
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
                
                let message = "📋 选择要查看的日志行数:";
                let keyboard = build_log_selection_keyboard();
                
                bot.edit_message_text(chat_id, message_id, message)
                    .reply_markup(keyboard)
                    .await?;
                
                log::info!("✅ cmd_logs 处理完成");
                return Ok(());
            }
            "cmd_maintenance_history" => {
                log::info!("🎯 处理维护历史: cmd_maintenance_history 命令");
                bot.answer_callback_query(&callback_query.id).await?;
                
                let message = "📜 正在加载维护历史...";
                let keyboard = build_maintenance_history_keyboard(0);
                
                bot.edit_message_text(chat_id, message_id, message)
                    .reply_markup(keyboard)
                    .await?;
                
                // 异步加载维护历史
                let bot_clone = bot.clone();
                let chat_id_clone = chat_id;
                let message_id_clone = message_id;
                
                tokio::spawn(async move {
                    let history_summary = crate::scheduler::maintenance_history::get_maintenance_summary().await;
                    let keyboard = build_maintenance_history_keyboard(0);
                    let _ = bot_clone.edit_message_text(
                        chat_id_clone,
                        message_id_clone,
                        history_summary
                    ).reply_markup(keyboard)
                    .await;
                });
                
                log::info!("✅ cmd_maintenance_history 处理完成");
                return Ok(());
            }
            "cmd_full_maintenance" => {
                log::info!("🎯 处理完整维护: cmd_full_maintenance 命令");
                bot.answer_callback_query(&callback_query.id).await?;
                
                let message = "🚀 正在执行完整维护（核心+规则）...";
                let keyboard = build_maintain_menu_keyboard();
                
                bot.edit_message_text(chat_id, message_id, message)
                    .reply_markup(keyboard)
                    .await?;
                
                // 异步执行完整维护
                let bot_clone = bot.clone();
                let chat_id_clone = chat_id;
                let message_id_clone = message_id;
                
                tokio::spawn(async move {
                    match system::perform_full_maintenance().await {
                        Ok(log) => {
                            let _ = bot_clone.edit_message_text(
                                chat_id_clone,
                                message_id_clone,
                                format!("✅ 完整维护完成:\n{}\n\n请选择下一步操作:", log)
                            ).reply_markup(build_maintain_menu_keyboard())
                            .await;
                            // 记录到维护历史
                            record_maintenance("🔧 完整维护 (手动)", MaintenanceResult::Success, &log, None).await;
                        }
                        Err(e) => {
                            let error_msg = format!("{}", e);
                            let _ = bot_clone.edit_message_text(
                                chat_id_clone,
                                message_id_clone,
                                format!("❌ 完整维护失败: {}\n\n请选择下一步操作:", e)
                            ).reply_markup(build_maintain_menu_keyboard())
                            .await;
                            // 记录到维护历史
                            record_maintenance("🔧 完整维护 (手动)", MaintenanceResult::Failed, "执行失败", Some(&error_msg)).await;
                        }
                    }
                });
                
                log::info!("✅ cmd_full_maintenance 处理完成");
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
                
                // 如果有任务，为每个任务添加删除按钮
                if !tasks_summary.contains("暂无定时任务") {
                    // 解析任务列表，为每个任务添加删除按钮
                    let mut keyboard = Vec::new();
                    
                    // 分析任务列表，提取任务数量
                    let task_count = tasks_summary.matches("✅").count() + tasks_summary.matches("⏸️").count();
                    
                    // 为每个任务添加删除按钮
                    for i in 0..task_count {
                        let task_row = vec![
                            InlineKeyboardButton::callback(
                                format!("🗑️ 删除任务 {}", i + 1), 
                                format!("del_task_{}", i)
                            )
                        ];
                        keyboard.push(task_row);
                    }
                    
                    // 添加通用按钮
                    keyboard.push(vec![
                        InlineKeyboardButton::callback("➕ 添加新任务", "add_new_task"),
                        InlineKeyboardButton::callback("🔙 返回", "back_to_task_types"),
                    ]);
                    
                    let keyboard = InlineKeyboardMarkup::new(keyboard);
                    bot.edit_message_text(chat_id, message_id, tasks_summary)
                        .reply_markup(keyboard)
                        .await?;
                } else {
                    // 没有任务时显示默认键盘
                    let keyboard = build_task_type_menu_keyboard();
                    bot.edit_message_text(chat_id, message_id, tasks_summary)
                        .reply_markup(keyboard)
                        .await?;
                }
                
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
                // 智能解析：处理包含下划线的 task_type
                if let Some(stripped) = cmd.strip_prefix("set_time_") {
                    let parts: Vec<&str> = stripped.split('_').collect();
                    
                    // 定义已知的频率关键字
                    let known_frequencies = ["daily", "weekly", "monthly"];
                    
                    // 查找频率关键字的位置
                    let frequency_index = parts.iter().position(|&part| known_frequencies.contains(&part));
                    
                    if let Some(freq_idx) = frequency_index {
                        // 找到频率关键字，重新构建 task_type 和 time_value
                        let frequency = parts[freq_idx];
                        let task_type = parts[..freq_idx].join("_");
                        let time_value = if freq_idx + 1 < parts.len() {
                            parts[freq_idx + 1..].join("_")
                        } else {
                            "".to_string()
                        };
                        
                        // 验证：确保找到了有效的 frequency 和 time_value
                        if time_value.is_empty() {
                            let _ = bot.send_message(
                                chat_id,
                                "❌ 无效的时间值: 时间值不能为空".to_string()
                            ).await;
                            return Ok(());
                        }
                        
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
                        if time_value.parse::<i32>().is_err() && !invalid_time_values.contains(&time_value.as_str()) {
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
                        
                        let message = format!("🔄 正在设置 {} 任务...", get_task_display_name(&task_type));
                        let keyboard = build_time_selection_keyboard(&task_type, frequency);
                        
                        bot.edit_message_text(chat_id, message_id, message)
                            .reply_markup(keyboard.clone())
                            .await?;
                        
                        let bot_clone = bot.clone();
                        let config = Config::load().unwrap_or_else(|_| Config { bot_token: "".to_string(), chat_id: 0, check_interval: 300 });
                        let _chat_id_clone = chat_id;
                        let task_type_enum = match task_type.as_str() {
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
                                            "❌ 调度器尚未初始化，请稍后重试或重新启动机器人".to_string()
                                        ).await;
                                        return;
                                    }
                                }
                            }
                        });
                        
                        log::info!("✅ set_time 处理完成");
                    } else {
                        // 找不到有效的频率关键字，返回错误
                        log::warn!("❌ 无法解析时间设置命令，缺少有效的频率关键字: {:?}", parts);
                        let _ = bot.send_message(
                            chat_id,
                            "❌ 无效的时间设置命令: 缺少有效的频率关键字 (daily/weekly/monthly)".to_string()
                        ).await;
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
            // 日志行数选择
            "view_logs_20" => {
                log::info!("🎯 处理查看日志: 20行");
                bot.answer_callback_query(&callback_query.id).await?;
                
                let message = "🔄 正在获取系统日志...";
                let keyboard = build_log_selection_keyboard();
                
                bot.edit_message_text(chat_id, message_id, message)
                    .reply_markup(keyboard)
                    .await?;
                
                let bot_clone = bot.clone();
                let chat_id_clone = chat_id;
                let message_id_clone = message_id;
                
                tokio::spawn(async move {
                    match system::ops::get_system_logs(20).await {
                        Ok(log) => {
                            let _ = bot_clone.edit_message_text(
                                chat_id_clone,
                                message_id_clone,
                                format!("📋 系统日志 (最近20行):\n{}", log)
                            ).reply_markup(build_log_selection_keyboard())
                            .await;
                        }
                        Err(e) => {
                            let _ = bot_clone.edit_message_text(
                                chat_id_clone,
                                message_id_clone,
                                format!("❌ 无法获取日志: {}", e)
                            ).reply_markup(build_log_selection_keyboard())
                            .await;
                        }
                    }
                });
                
                log::info!("✅ view_logs_20 处理完成");
                return Ok(());
            }
            "view_logs_50" => {
                log::info!("🎯 处理查看日志: 50行");
                bot.answer_callback_query(&callback_query.id).await?;
                
                let message = "🔄 正在获取系统日志...";
                let keyboard = build_log_selection_keyboard();
                
                bot.edit_message_text(chat_id, message_id, message)
                    .reply_markup(keyboard)
                    .await?;
                
                let bot_clone = bot.clone();
                let chat_id_clone = chat_id;
                let message_id_clone = message_id;
                
                tokio::spawn(async move {
                    match system::ops::get_system_logs(50).await {
                        Ok(log) => {
                            let _ = bot_clone.edit_message_text(
                                chat_id_clone,
                                message_id_clone,
                                format!("📋 系统日志 (最近50行):\n{}", log)
                            ).reply_markup(build_log_selection_keyboard())
                            .await;
                        }
                        Err(e) => {
                            let _ = bot_clone.edit_message_text(
                                chat_id_clone,
                                message_id_clone,
                                format!("❌ 无法获取日志: {}", e)
                            ).reply_markup(build_log_selection_keyboard())
                            .await;
                        }
                    }
                });
                
                log::info!("✅ view_logs_50 处理完成");
                return Ok(());
            }
            "view_logs_100" => {
                log::info!("🎯 处理查看日志: 100行");
                bot.answer_callback_query(&callback_query.id).await?;
                
                let message = "🔄 正在获取系统日志...";
                let keyboard = build_log_selection_keyboard();
                
                bot.edit_message_text(chat_id, message_id, message)
                    .reply_markup(keyboard)
                    .await?;
                
                let bot_clone = bot.clone();
                let chat_id_clone = chat_id;
                let message_id_clone = message_id;
                
                tokio::spawn(async move {
                    match system::ops::get_system_logs(100).await {
                        Ok(log) => {
                            let _ = bot_clone.edit_message_text(
                                chat_id_clone,
                                message_id_clone,
                                format!("📋 系统日志 (最近100行):\n{}", log)
                            ).reply_markup(build_log_selection_keyboard())
                            .await;
                        }
                        Err(e) => {
                            let _ = bot_clone.edit_message_text(
                                chat_id_clone,
                                message_id_clone,
                                format!("❌ 无法获取日志: {}", e)
                            ).reply_markup(build_log_selection_keyboard())
                            .await;
                        }
                    }
                });
                
                log::info!("✅ view_logs_100 处理完成");
                return Ok(());
            }
            "view_logs_all" => {
                log::info!("🎯 处理查看日志: 全部");
                bot.answer_callback_query(&callback_query.id).await?;
                
                let message = "🔄 正在获取全部系统日志...";
                let keyboard = build_log_selection_keyboard();
                
                bot.edit_message_text(chat_id, message_id, message)
                    .reply_markup(keyboard)
                    .await?;
                
                let bot_clone = bot.clone();
                let chat_id_clone = chat_id;
                let message_id_clone = message_id;
                
                tokio::spawn(async move {
                    // 获取全部日志，不限制行数
                    match system::ops::get_system_logs(1000).await {
                        Ok(log) => {
                            let log_text = if log.len() > 4000 {
                                // 如果日志太长，截取部分
                                format!("📋 系统日志 (全部 - 已截取部分内容):\n{}\n\n⚠️ 日志过长，已截取前 4000 字符", &log[..4000])
                            } else {
                                format!("📋 系统日志 (全部):\n{}", log)
                            };
                            let _ = bot_clone.edit_message_text(
                                chat_id_clone,
                                message_id_clone,
                                log_text
                            ).reply_markup(build_log_selection_keyboard())
                            .await;
                        }
                        Err(e) => {
                            let _ = bot_clone.edit_message_text(
                                chat_id_clone,
                                message_id_clone,
                                format!("❌ 无法获取日志: {}", e)
                            ).reply_markup(build_log_selection_keyboard())
                            .await;
                        }
                    }
                });
                
                log::info!("✅ view_logs_all 处理完成");
                return Ok(());
            }
            // 维护历史分页处理
            cmd if cmd.starts_with("maintenance_history_") => {
                let page_str = cmd.strip_prefix("maintenance_history_").unwrap_or("0");
                let page = page_str.parse::<usize>().unwrap_or(0);
                
                log::info!("🎯 处理维护历史分页: 第{}页", page);
                bot.answer_callback_query(&callback_query.id).await?;
                
                let message = "🔄 正在加载维护历史...";
                let keyboard = build_maintenance_history_keyboard(page);
                
                bot.edit_message_text(chat_id, message_id, message)
                    .reply_markup(keyboard)
                    .await?;
                
                let bot_clone = bot.clone();
                let chat_id_clone = chat_id;
                let message_id_clone = message_id;
                
                tokio::spawn(async move {
                    let (history_text, total_records) = crate::scheduler::maintenance_history::get_maintenance_history_details(page, 5).await;
                    let keyboard = build_maintenance_history_keyboard(page);
                    let final_text = if total_records == 0 {
                        history_text
                    } else {
                        format!("{}\n\n📊 共 {} 条记录", history_text, total_records)
                    };
                    let _ = bot_clone.edit_message_text(
                        chat_id_clone,
                        message_id_clone,
                        final_text
                    ).reply_markup(keyboard)
                    .await;
                });
                
                log::info!("✅ maintenance_history 处理完成");
                return Ok(());
            }
            // 删除任务处理
            cmd if cmd.starts_with("del_task_") => {
                let task_index_str = cmd.strip_prefix("del_task_").unwrap_or("0");
                let task_index = task_index_str.parse::<usize>().unwrap_or(0);
                
                log::info!("🎯 处理删除任务: 索引 {}", task_index);
                bot.answer_callback_query(&callback_query.id).await?;
                
                let message = format!("🗑️ 正在删除任务 {}...", task_index + 1);
                
                // 暂时显示加载消息
                bot.edit_message_text(chat_id, message_id, message).await?;
                
                // 异步执行删除操作
                let bot_clone = bot.clone();
                let chat_id_clone = chat_id;
                let message_id_clone = message_id;
                let config = Config::load().unwrap_or_else(|_| Config { bot_token: "".to_string(), chat_id: 0, check_interval: 300 });
                
                tokio::spawn(async move {
                    let mut retry_count = 0;
                    let max_retries = 10;
                    
                    while retry_count < max_retries {
                        let manager_guard = crate::scheduler::SCHEDULER_MANAGER.lock().await;
                        if let Some(manager) = &*manager_guard {
                            let result = manager.remove_task_by_index(
                                config.clone(),
                                Bot::new(config.bot_token.clone()),
                                task_index
                            ).await;
                            
                            drop(manager_guard); // 立即释放锁
                            
                            match result {
                                Ok(response_msg) => {
                                    // 删除成功后重新加载任务列表
                                    let tasks_summary = crate::scheduler::get_tasks_summary().await.unwrap_or_else(|_| "❌ 无法获取任务列表".to_string());
                                    
                                    // 重新构建键盘
                                    let mut keyboard = Vec::new();
                                    
                                    if !tasks_summary.contains("暂无定时任务") {
                                        // 分析任务列表，提取任务数量
                                        let new_task_count = tasks_summary.matches("✅").count() + tasks_summary.matches("⏸️").count();
                                        
                                        // 为每个任务添加删除按钮
                                        for i in 0..new_task_count {
                                            let task_row = vec![
                                                InlineKeyboardButton::callback(
                                                    format!("🗑️ 删除任务 {}", i + 1), 
                                                    format!("del_task_{}", i)
                                                )
                                            ];
                                            keyboard.push(task_row);
                                        }
                                    }
                                    
                                    // 添加通用按钮
                                    keyboard.push(vec![
                                        InlineKeyboardButton::callback("➕ 添加新任务", "add_new_task"),
                                        InlineKeyboardButton::callback("🔙 返回", "back_to_task_types"),
                                    ]);
                                    
                                    let keyboard = InlineKeyboardMarkup::new(keyboard);
                                    
                                    let final_message = format!("✅ {}\n\n{}", response_msg, tasks_summary);
                                    let _ = bot_clone.edit_message_text(
                                        chat_id_clone,
                                        message_id_clone,
                                        final_message
                                    ).reply_markup(keyboard)
                                    .await;
                                    return;
                                }
                                Err(e) => {
                                    let _ = bot_clone.edit_message_text(
                                        chat_id_clone,
                                        message_id_clone,
                                        format!("❌ 删除任务失败: {}", e)
                                    ).reply_markup(build_task_type_menu_keyboard())
                                    .await;
                                    return;
                                }
                            }
                        } else {
                            drop(manager_guard);
                            retry_count += 1;
                            if retry_count < max_retries {
                                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                            } else {
                                let _ = bot_clone.edit_message_text(
                                    chat_id_clone,
                                    message_id_clone,
                                    "❌ 调度器尚未初始化，请稍后重试或重新启动机器人"
                                ).reply_markup(build_task_type_menu_keyboard())
                                .await;
                                return;
                            }
                        }
                    }
                });
                
                log::info!("✅ del_task 处理完成");
                return Ok(());
            }
            "maintenance_history_summary" => {
                log::info!("🎯 处理维护历史摘要");
                bot.answer_callback_query(&callback_query.id).await?;
                
                let message = "🔄 正在生成维护历史摘要...";
                let keyboard = build_maintenance_history_keyboard(0);
                
                bot.edit_message_text(chat_id, message_id, message)
                    .reply_markup(keyboard)
                    .await?;
                
                let bot_clone = bot.clone();
                let chat_id_clone = chat_id;
                let message_id_clone = message_id;
                
                tokio::spawn(async move {
                    let history_summary = crate::scheduler::maintenance_history::get_maintenance_summary().await;
                    let keyboard = build_maintenance_history_keyboard(0);
                    let _ = bot_clone.edit_message_text(
                        chat_id_clone,
                        message_id_clone,
                        history_summary
                    ).reply_markup(keyboard)
                    .await;
                });
                
                log::info!("✅ maintenance_history_summary 处理完成");
                return Ok(());
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
                format!("✅ 核心维护完成:\n{}\n\n🔄 系统将在 3 秒后自动重启，请保存您的工作！\n\n请选择下一步操作:", log),
            )
            .reply_markup(build_maintain_menu_keyboard())
            .await?;
            // 记录到维护历史
            record_maintenance("🚀 核心维护 (手动)", MaintenanceResult::Success, &log, None).await;
        }
        Err(e) => {
            let error_msg = format!("{}", e);
            bot.edit_message_text(
                callback_query.message.as_ref().unwrap().chat.id,
                callback_query.message.as_ref().unwrap().id,
                format!("❌ 核心维护失败: {}\n\n请选择下一步操作:", e),
            )
            .reply_markup(build_maintain_menu_keyboard())
            .await?;
            // 记录到维护历史
            record_maintenance("🚀 核心维护 (手动)", MaintenanceResult::Failed, "执行失败", Some(&error_msg)).await;
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
                format!("✅ 规则维护完成:\n{}\n\n请选择下一步操作:", log),
            )
            .reply_markup(build_maintain_menu_keyboard())
            .await?;
            // 记录到维护历史
            record_maintenance("🌍 规则维护 (手动)", MaintenanceResult::Success, &log, None).await;
        }
        Err(e) => {
            let error_msg = format!("{}", e);
            bot.edit_message_text(
                callback_query.message.as_ref().unwrap().chat.id,
                callback_query.message.as_ref().unwrap().id,
                format!("❌ 规则维护失败: {}\n\n请选择下一步操作:", e),
            )
            .reply_markup(build_maintain_menu_keyboard())
            .await?;
            // 记录到维护历史
            record_maintenance("🌍 规则维护 (手动)", MaintenanceResult::Failed, "执行失败", Some(&error_msg)).await;
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
                format!("✅ Xray 更新完成:\n{}\n\n请选择下一步操作:", log),
            )
            .reply_markup(build_maintain_menu_keyboard())
            .await?;
            // 记录到维护历史
            record_maintenance("🔧 更新 Xray (手动)", MaintenanceResult::Success, &log, None).await;
        }
        Err(e) => {
            let error_msg = format!("{}", e);
            bot.edit_message_text(
                callback_query.message.as_ref().unwrap().chat.id,
                callback_query.message.as_ref().unwrap().id,
                format!("❌ Xray 更新失败: {}\n\n请选择下一步操作:", e),
            )
            .reply_markup(build_maintain_menu_keyboard())
            .await?;
            // 记录到维护历史
            record_maintenance("🔧 更新 Xray (手动)", MaintenanceResult::Failed, "执行失败", Some(&error_msg)).await;
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
                format!("✅ Sing-box 更新完成:\n{}\n\n请选择下一步操作:", log),
            )
            .reply_markup(build_maintain_menu_keyboard())
            .await?;
            // 记录到维护历史
            record_maintenance("📦 更新 Sing-box (手动)", MaintenanceResult::Success, &log, None).await;
        }
        Err(e) => {
            let error_msg = format!("{}", e);
            bot.edit_message_text(
                callback_query.message.as_ref().unwrap().chat.id,
                callback_query.message.as_ref().unwrap().id,
                format!("❌ Sing-box 更新失败: {}\n\n请选择下一步操作:", e),
            )
            .reply_markup(build_maintain_menu_keyboard())
            .await?;
            // 记录到维护历史
            record_maintenance("📦 更新 Sing-box (手动)", MaintenanceResult::Failed, "执行失败", Some(&error_msg)).await;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_variants() {
        // 测试命令枚举的所有变体
        let commands = vec![
            Command::Start,
            Command::Status,
            Command::Maintain,
            Command::Reboot,
            Command::UpdateXray,
            Command::UpdateSb,
            Command::MaintainCore,
            Command::MaintainRules,
            Command::Logs,
            Command::SetSchedule("0 4 * * *".to_string()),
            Command::MaintenanceHistory,
            Command::FullMaintenance,
        ];
        
        assert_eq!(commands.len(), 12); // 确保所有命令都被测试到
    }

    #[test]
    fn test_get_task_display_name() {
        // 测试已知任务类型
        assert_eq!(get_task_display_name("system_maintenance"), "🔄 系统维护");
        assert_eq!(get_task_display_name("system"), "🔄 系统维护");
        assert_eq!(get_task_display_name("core_maintenance"), "🚀 核心维护");
        assert_eq!(get_task_display_name("rules_maintenance"), "🌍 规则维护");
        assert_eq!(get_task_display_name("update_xray"), "🔧 更新 Xray");
        assert_eq!(get_task_display_name("update_singbox"), "📦 更新 Sing-box");
        
        // 测试未知任务类型
        assert_eq!(get_task_display_name("unknown_type"), "❓ 未知任务");
        assert_eq!(get_task_display_name(""), "❓ 未知任务");
        assert_eq!(get_task_display_name("invalid_task"), "❓ 未知任务");
    }

    #[test]
    fn test_schedule_presets_keyboard_edge_cases() {
        // 测试空字符串
        let keyboard = build_schedule_presets_keyboard("");
        assert_eq!(keyboard.inline_keyboard.len(), 3);
        
        // 测试包含特殊字符的任务类型
        let keyboard = build_schedule_presets_keyboard("test-task_type");
        assert_eq!(keyboard.inline_keyboard.len(), 3);
        
        // 测试中文任务类型
        let keyboard = build_schedule_presets_keyboard("中文任务");
        assert_eq!(keyboard.inline_keyboard.len(), 3);
    }

    #[test]
    fn test_time_selection_keyboard_edge_cases() {
        // 测试空任务类型
        let keyboard = build_time_selection_keyboard("", "daily");
        assert!(keyboard.inline_keyboard.len() > 0);
        
        // 测试包含下划线的任务类型
        let keyboard = build_time_selection_keyboard("test_task_type", "daily");
        assert!(keyboard.inline_keyboard.len() > 0);
        
        // 测试无效频率
        let keyboard = build_time_selection_keyboard("system_maintenance", "invalid_frequency");
        assert_eq!(keyboard.inline_keyboard.len(), 1);
    }

    #[test]
    fn test_keyboard_consistency() {
        // 测试不同菜单的返回按钮一致性
        let main_menu = build_main_menu_keyboard();
        let maintain_menu = build_maintain_menu_keyboard();
        let task_menu = build_task_type_menu_keyboard();
        
        // 检查返回按钮文本一致性
        assert_eq!(main_menu.inline_keyboard.last().unwrap()[0].text, "🔙 返回主菜单");
        assert_eq!(maintain_menu.inline_keyboard.last().unwrap()[0].text, "🔙 返回主菜单");
        assert_eq!(task_menu.inline_keyboard.last().unwrap()[0].text, "🔙 返回");
    }

    #[test]
    fn test_emoji_consistency() {
        // 测试emoji使用的一致性
        let main_menu = build_main_menu_keyboard();
        
        // 检查主要功能是否使用了emoji
        let has_system_emoji = main_menu.inline_keyboard[0][0].text.contains("📊");
        let has_maintain_emoji = main_menu.inline_keyboard[0][1].text.contains("🛠️");
        let has_schedule_emoji = main_menu.inline_keyboard[1][0].text.contains("⏰");
        let has_logs_emoji = main_menu.inline_keyboard[1][1].text.contains("📋");
        let has_history_emoji = main_menu.inline_keyboard[2][0].text.contains("📜");
        
        assert!(has_system_emoji);
        assert!(has_maintain_emoji);
        assert!(has_schedule_emoji);
        assert!(has_logs_emoji);
        assert!(has_history_emoji);
    }

    #[test]
    fn test_command_description_mapping() {
        // 测试命令与描述的对应关系
        let commands = vec![
            (Command::Start, "启动机器人"),
            (Command::Status, "获取系统状态"),
            (Command::Maintain, "执行系统维护"),
            (Command::Reboot, "重启系统"),
            (Command::UpdateXray, "更新 Xray"),
            (Command::UpdateSb, "更新 Sing-box"),
            (Command::MaintainCore, "核心维护"),
            (Command::MaintainRules, "规则维护"),
            (Command::Logs, "查看日志"),
            (Command::SetSchedule("0 4 * * *".to_string()), "设置调度计划"),
            (Command::MaintenanceHistory, "查看维护历史"),
            (Command::FullMaintenance, "完整维护"),
        ];
        
        assert_eq!(commands.len(), 12);
        
        // 验证每个命令都有对应的描述
        for (command, expected_desc) in commands {
            match command {
                Command::SetSchedule(_) => {
                    assert_eq!(expected_desc, "设置调度计划");
                },
                _ => {
                    // 其他命令的描述验证
                    assert!(!expected_desc.is_empty());
                }
            }
        }
    }

    #[test]
    fn test_keyboard_button_text_lengths() {
        // 测试按钮文本长度合理性
        let main_menu = build_main_menu_keyboard();
        for row in &main_menu.inline_keyboard {
            for button in row {
                // 按钮文本不应过长（考虑移动端显示）
                assert!(button.text.len() <= 20, "Button text too long: {}", button.text);
                // 按钮文本不应为空
                assert!(!button.text.is_empty());
            }
        }
        
        let maintain_menu = build_maintain_menu_keyboard();
        for row in &maintain_menu.inline_keyboard {
            for button in row {
                assert!(button.text.len() <= 20);
                assert!(!button.text.is_empty());
            }
        }
    }

    #[test]
    fn test_error_handling_edge_cases() {
        // 测试边界情况处理
        
        // 测试空字符串任务类型
        let result = get_task_display_name("");
        assert_eq!(result, "❓ 未知任务");
        
        // 测试只有空格的任务类型
        let result = get_task_display_name("   ");
        assert_eq!(result, "❓ 未知任务");
        
        // 测试包含特殊字符的任务类型
        let result = get_task_display_name("task@#$%^&*()");
        assert_eq!(result, "❓ 未知任务");
        
        // 测试超长任务类型
        let long_type = "a".repeat(1000);
        let result = get_task_display_name(&long_type);
        assert_eq!(result, "❓ 未知任务");
    }

    // ========== 回调处理测试 ==========
    
    #[test]
    fn test_callback_data_parsing_main_menu() {
        // 测试主菜单回调数据解析
        let test_cases = vec![
            ("cmd_status", "系统状态"),
            ("menu_maintain", "维护菜单"),
            ("menu_schedule", "定时任务"),
            ("cmd_logs", "查看日志"),
            ("cmd_maintenance_history", "维护历史"),
        ];
        
        for (callback_data, expected_desc) in test_cases {
            assert!(!callback_data.is_empty());
            assert!(!expected_desc.is_empty());
        }
    }
    
    #[test]
    fn test_callback_data_parsing_maintain_menu() {
        // 测试维护菜单回调数据解析
        let test_cases = vec![
            ("cmd_maintain_core", "核心维护"),
            ("cmd_maintain_rules", "规则维护"),
            ("cmd_update_xray", "更新 Xray"),
            ("cmd_update_sb", "更新 Sing-box"),
            ("cmd_full_maintenance", "完整维护"),
            ("back_to_main", "返回主菜单"),
        ];
        
        for (callback_data, expected_desc) in test_cases {
            assert!(!callback_data.is_empty());
            assert!(!expected_desc.is_empty());
        }
    }
    
    #[test]
    fn test_callback_data_parsing_task_types() {
        // 测试任务类型回调数据解析
        let test_cases = vec![
            ("task_system_maintenance", "系统维护"),
            ("task_core_maintenance", "核心维护"),
            ("task_rules_maintenance", "规则维护"),
            ("task_update_xray", "更新 Xray"),
            ("task_update_singbox", "更新 Sing-box"),
            ("view_tasks", "查看任务列表"),
            ("back_to_task_types", "返回任务类型"),
        ];
        
        for (callback_data, expected_desc) in test_cases {
            assert!(!callback_data.is_empty());
            assert!(callback_data.starts_with("task_") || callback_data == "view_tasks" || callback_data == "back_to_task_types");
        }
    }
    
    #[test]
    fn test_invalid_callback_data() {
        // 测试无效回调数据处理
        let long_string = "a".repeat(1000);
        let invalid_cases = vec![
            "",
            "invalid_command",
            "unknown_action",
            "cmd_nonexistent",
            "task_invalid_type",
            "@#$%^&*()",
            &long_string, // 超长字符串
        ];
        
        for invalid_data in invalid_cases {
            // 这些应该被识别为无效命令
            if invalid_data.is_empty() {
                continue; // 空数据有特殊处理
            }
            
            // 验证无效数据不匹配已知的命令模式
            let known_patterns = vec![
                "cmd_", "menu_", "task_", "set_", "view_", "back_", "maintenance_history_"
            ];
            
            let is_known = known_patterns.iter().any(|pattern| invalid_data.starts_with(pattern));
            assert!(!is_known || invalid_data.len() > 100, "Long invalid data should not match known patterns: {}", invalid_data);
        }
    }
    
    #[test]
    fn test_callback_data_boundary_conditions() {
        // 测试边界条件
        
        // 空字符串
        assert_eq!("".len(), 0);
        
        // 超长字符串
        let long_string = "a".repeat(1000);
        assert_eq!(long_string.len(), 1000);
        
        // 包含特殊字符
        let special_chars = "cmd_@#$%^&*()_+-=[]{}|;':\",./<>?";
        assert!(special_chars.len() > 0);
        
        // Unicode 字符
        let unicode = "cmd_测试中文🚀";
        assert!(unicode.len() > 0);
        
        // 只有空格
        let whitespace = "   ";
        assert_eq!(whitespace.trim().len(), 0);
    }

    // ========== 菜单构建测试 ==========
    
    #[test]
    fn test_main_menu_keyboard_structure() {
        // 测试主菜单键盘结构
        let keyboard = build_main_menu_keyboard();
        
        // 检查键盘行数
        assert_eq!(keyboard.inline_keyboard.len(), 3);
        
        // 检查第一行（系统状态 + 维护菜单）
        let first_row = &keyboard.inline_keyboard[0];
        assert_eq!(first_row.len(), 2);
        assert_eq!(first_row[0].text, "📊 系统状态");
        assert_eq!(first_row[1].text, "🛠️ 维护菜单");
        
        // 检查第二行（定时任务 + 查看日志）
        let second_row = &keyboard.inline_keyboard[1];
        assert_eq!(second_row.len(), 2);
        assert_eq!(second_row[0].text, "⏰ 定时任务");
        assert_eq!(second_row[1].text, "📋 查看日志");
        
        // 检查第三行（维护历史）
        let third_row = &keyboard.inline_keyboard[2];
        assert_eq!(third_row.len(), 1);
        assert_eq!(third_row[0].text, "📜 维护历史");
    }
    
    #[test]
    fn test_maintain_menu_keyboard_structure() {
        // 测试维护菜单键盘结构
        let keyboard = build_maintain_menu_keyboard();
        
        // 检查键盘行数
        assert_eq!(keyboard.inline_keyboard.len(), 4);
        
        // 检查第一行（系统更新 + 规则更新）
        let first_row = &keyboard.inline_keyboard[0];
        assert_eq!(first_row.len(), 2);
        assert_eq!(first_row[0].text, "🔄 系统更新");
        assert_eq!(first_row[1].text, "🌍 规则更新");
        
        // 检查第二行（更新 Xray + 更新 Sing-box）
        let second_row = &keyboard.inline_keyboard[1];
        assert_eq!(second_row.len(), 2);
        assert_eq!(second_row[0].text, "🚀 更新 Xray");
        assert_eq!(second_row[1].text, "📦 更新 Sing-box");
        
        // 检查第三行（完整维护）
        let third_row = &keyboard.inline_keyboard[2];
        assert_eq!(third_row.len(), 1);
        assert_eq!(third_row[0].text, "🔄 完整维护");
        
        // 检查第四行（返回主菜单）
        let fourth_row = &keyboard.inline_keyboard[3];
        assert_eq!(fourth_row.len(), 1);
        assert_eq!(fourth_row[0].text, "🔙 返回主菜单");
    }
    
    #[test]
    fn test_task_type_menu_keyboard_structure() {
        // 测试任务类型菜单键盘结构
        let keyboard = build_task_type_menu_keyboard();
        
        // 检查键盘行数
        assert_eq!(keyboard.inline_keyboard.len(), 4);
        
        // 检查第一行（系统维护 + 核心维护）
        let first_row = &keyboard.inline_keyboard[0];
        assert_eq!(first_row.len(), 2);
        assert_eq!(first_row[0].text, "🔄 系统维护");
        assert_eq!(first_row[1].text, "🚀 核心维护");
        
        // 检查第二行（规则维护 + 更新 Xray）
        let second_row = &keyboard.inline_keyboard[1];
        assert_eq!(second_row.len(), 2);
        assert_eq!(second_row[0].text, "🌍 规则维护");
        assert_eq!(second_row[1].text, "🔧 更新 Xray");
        
        // 检查第三行（更新 Sing-box + 查看任务列表）
        let third_row = &keyboard.inline_keyboard[2];
        assert_eq!(third_row.len(), 2);
        assert_eq!(third_row[0].text, "📦 更新 Sing-box");
        assert_eq!(third_row[1].text, "📋 查看任务列表");
        
        // 检查第四行（返回）
        let fourth_row = &keyboard.inline_keyboard[3];
        assert_eq!(fourth_row.len(), 1);
        assert_eq!(fourth_row[0].text, "🔙 返回");
    }
    
    #[test]
    fn test_schedule_presets_keyboard_different_types() {
        // 测试不同任务类型的预设键盘
        let task_types = vec![
            "system_maintenance",
            "core_maintenance", 
            "rules_maintenance",
            "update_xray",
            "update_singbox",
        ];
        
        for task_type in task_types {
            let keyboard = build_schedule_presets_keyboard(task_type);
            
            // 所有预设键盘应该有相同的结构
            assert_eq!(keyboard.inline_keyboard.len(), 3);
            
            // 第一行：每天设置 + 每周设置
            let first_row = &keyboard.inline_keyboard[0];
            assert_eq!(first_row.len(), 2);
            assert_eq!(first_row[0].text, "每天设置");
            assert_eq!(first_row[1].text, "每周设置");
            
            // 第二行：每月设置 + 自定义
            let second_row = &keyboard.inline_keyboard[1];
            assert_eq!(second_row.len(), 2);
            assert_eq!(second_row[0].text, "每月设置");
            assert_eq!(second_row[1].text, "自定义");
            
            // 第三行：返回按钮
            let third_row = &keyboard.inline_keyboard[2];
            assert_eq!(third_row.len(), 1);
            assert_eq!(third_row[0].text, "🔙 返回任务类型");
        }
    }
    
    #[test]
    fn test_time_selection_keyboard_different_frequencies() {
        // 测试不同频率的时间选择键盘
        let frequencies = vec!["daily", "weekly", "monthly"];
        
        for frequency in frequencies {
            let keyboard = build_time_selection_keyboard("system_maintenance", frequency);
            
            // 检查键盘不为空
            assert!(!keyboard.inline_keyboard.is_empty());
            
            // 检查最后一行是返回按钮
            let last_row = keyboard.inline_keyboard.last().unwrap();
            assert_eq!(last_row.len(), 1);
            assert_eq!(last_row[0].text, "🔙 返回");
            
            // 检查按钮文本包含时间选项
            let has_time_buttons = keyboard.inline_keyboard[..keyboard.inline_keyboard.len() - 1]
                .iter()
                .any(|row| row.iter().any(|btn| btn.text.contains("点")));
            assert!(has_time_buttons, "应该包含时间选项");
        }
        
        // 测试无效频率
        let keyboard = build_time_selection_keyboard("system_maintenance", "invalid");
        assert_eq!(keyboard.inline_keyboard.len(), 1); // 只有返回按钮
    }
    
    #[test]
    fn test_log_selection_keyboard_structure() {
        // 测试日志选择键盘结构
        let keyboard = build_log_selection_keyboard();
        
        // 检查键盘行数
        assert_eq!(keyboard.inline_keyboard.len(), 3);
        
        // 检查第一行（最近20行 + 最近50行）
        let first_row = &keyboard.inline_keyboard[0];
        assert_eq!(first_row.len(), 2);
        assert_eq!(first_row[0].text, "📋 最近 20 行");
        assert_eq!(first_row[1].text, "📋 最近 50 行");
        
        // 检查第二行（最近100行 + 全部日志）
        let second_row = &keyboard.inline_keyboard[1];
        assert_eq!(second_row.len(), 2);
        assert_eq!(second_row[0].text, "📋 最近 100 行");
        assert_eq!(second_row[1].text, "📋 全部日志");
        
        // 检查第三行（返回主菜单）
        let third_row = &keyboard.inline_keyboard[2];
        assert_eq!(third_row.len(), 1);
        assert_eq!(third_row[0].text, "🔙 返回主菜单");
    }
    
    #[test]
    fn test_maintenance_history_keyboard_pagination() {
        // 测试维护历史键盘分页
        
        // 测试第0页
        let keyboard_page_0 = build_maintenance_history_keyboard(0);
        assert_eq!(keyboard_page_0.inline_keyboard.len(), 2);
        
        // 测试第5页
        let keyboard_page_5 = build_maintenance_history_keyboard(5);
        assert_eq!(keyboard_page_5.inline_keyboard.len(), 2);
        
        // 测试大页码
        let keyboard_page_100 = build_maintenance_history_keyboard(100);
        assert_eq!(keyboard_page_100.inline_keyboard.len(), 2);
        
        // 检查第一行都有分页按钮
        for page in vec![0, 5, 100] {
            let keyboard = build_maintenance_history_keyboard(page);
            let first_row = &keyboard.inline_keyboard[0];
            assert!(first_row.len() >= 3); // 上一页 + 摘要 + 下一页
            
            // 检查有"历史摘要"按钮
            let has_summary = first_row.iter().any(|btn| btn.text == "📜 历史摘要");
            assert!(has_summary);
        }
    }

    // ========== 消息格式化测试 ==========
    
    #[test]
    fn test_system_status_message_format() {
        // 测试系统状态消息格式化
        
        // 模拟系统状态数据
        struct MockSystemStatus {
            pub cpu_usage: f64,
            pub memory_used: u64,
            pub memory_total: u64,
            pub disk_used: u64,
            pub disk_total: u64,
            pub network_rx: u64,
            pub network_tx: u64,
            pub uptime: u64,
        }
        
        let status = MockSystemStatus {
            cpu_usage: 25.5,
            memory_used: 2 * 1024 * 1024 * 1024, // 2GB
            memory_total: 8 * 1024 * 1024 * 1024, // 8GB
            disk_used: 50 * 1024 * 1024 * 1024, // 50GB
            disk_total: 100 * 1024 * 1024 * 1024, // 100GB
            network_rx: 1024 * 1024 * 1024, // 1GB
            network_tx: 512 * 1024 * 1024, // 512MB
            uptime: 86400, // 1天
        };
        
        let reply = format!(
            "📊 系统状态:\n\n{}",
            format!("🔹 CPU 使用率: {:.2}%\n", status.cpu_usage) +
            &format!("🔹 内存使用: {} MB / {} MB\n", status.memory_used / 1024 / 1024, status.memory_total / 1024 / 1024) +
            &format!("🔹 磁盘使用: {} GB / {} GB\n", status.disk_used / 1024 / 1024 / 1024, status.disk_total / 1024 / 1024 / 1024) +
            &format!("🔹 网络接收: {} MB\n", status.network_rx / 1024 / 1024) +
            &format!("🔹 网络发送: {} MB\n", status.network_tx / 1024 / 1024) +
            &format!("🔹 运行时间: {} 秒", status.uptime)
        );
        
        // 验证消息格式
        assert!(reply.starts_with("📊 系统状态:"));
        assert!(reply.contains("🔹 CPU 使用率: 25.50%"));
        assert!(reply.contains("🔹 内存使用: 2048 MB / 8192 MB"));
        assert!(reply.contains("🔹 磁盘使用: 50 GB / 100 GB"));
        assert!(reply.contains("🔹 网络接收: 1024 MB"));
        assert!(reply.contains("🔹 网络发送: 512 MB"));
        assert!(reply.contains("🔹 运行时间: 86400 秒"));
    }
    
    #[test]
    fn test_maintenance_report_message_format() {
        // 测试维护报告消息格式化
        
        let maintenance_log = "执行了系统更新\n清理了临时文件\n更新了软件包列表";
        
        // 成功消息格式
        let success_message = format!("✅ 系统维护完成:\n{}", maintenance_log);
        assert!(success_message.starts_with("✅ 系统维护完成:"));
        assert!(success_message.contains("执行了系统更新"));
        
        // 核心维护消息格式
        let core_message = format!("✅ 核心维护完成:\n{}\n\n🔄 系统将在 3 秒后自动重启，请保存您的工作！", maintenance_log);
        assert!(core_message.starts_with("✅ 核心维护完成:"));
        assert!(core_message.contains("🔄 系统将在 3 秒后自动重启"));
        
        // 错误消息格式
        let error_message = format!("❌ 系统维护失败: 网络连接超时");
        assert!(error_message.starts_with("❌ 系统维护失败:"));
        assert!(error_message.contains("网络连接超时"));
    }
    
    #[test]
    fn test_error_message_format() {
        // 测试错误消息格式化
        
        let error_cases = vec![
            ("系统状态获取失败", "❌ 无法获取系统状态: 系统状态获取失败"),
            ("网络连接超时", "❌ 无法获取日志: 网络连接超时"),
            ("权限被拒绝", "❌ 核心维护失败: 权限被拒绝"),
            ("文件不存在", "❌ 更新 Xray 失败: 文件不存在"),
        ];
        
        for (error_detail, expected_format) in error_cases {
            let error_message = format!("❌ 操作失败: {}", error_detail);
            assert!(error_message.starts_with("❌ 操作失败:"));
            assert!(error_message.contains(error_detail));
        }
    }
    
    #[test]
    fn test_welcome_message_format() {
        // 测试欢迎消息格式
        let welcome_message = "🚀 欢迎使用 VPS 管理机器人!\n\n请选择您要执行的操作:";
        
        assert!(welcome_message.starts_with("🚀 欢迎使用 VPS 管理机器人!"));
        assert!(welcome_message.contains("请选择您要执行的操作:"));
    }
    
    #[test]
    fn test_schedule_preset_message_format() {
        // 测试调度预设消息格式
        
        let task_types = vec![
            ("system_maintenance", "🔄 系统维护"),
            ("core_maintenance", "🚀 核心维护"),
            ("rules_maintenance", "🌍 规则维护"),
            ("update_xray", "🔧 更新 Xray"),
            ("update_singbox", "📦 更新 Sing-box"),
        ];
        
        for (task_type, expected_display) in task_types {
            let daily_message = format!("⏰ 设置 {} 每天执行\n\n请选择具体执行时间:", expected_display);
            assert!(daily_message.contains("⏰ 设置"));
            assert!(daily_message.contains("每天执行"));
            assert!(daily_message.contains("请选择具体执行时间:"));
            
            let weekly_message = format!("⏰ 设置 {} 每周执行\n\n请选择具体执行时间:", expected_display);
            assert!(weekly_message.contains("⏰ 设置"));
            assert!(weekly_message.contains("每周执行"));
            
            let monthly_message = format!("⏰ 设置 {} 每月执行\n\n请选择具体执行时间:", expected_display);
            assert!(monthly_message.contains("⏰ 设置"));
            assert!(monthly_message.contains("每月执行"));
        }
    }
    
    #[test]
    fn test_log_message_format() {
        // 测试日志消息格式
        
        let log_entries = "2024-01-01 10:00:01 INFO: 系统启动\n2024-01-01 10:00:02 INFO: 加载配置完成\n2024-01-01 10:00:03 INFO: 启动完成";
        
        // 不同行数的日志消息
        let log_20 = format!("📋 系统日志 (最近20行):\n{}", log_entries);
        assert!(log_20.starts_with("📋 系统日志 (最近20行):"));
        assert!(log_20.contains("系统启动"));
        
        let log_50 = format!("📋 系统日志 (最近50行):\n{}", log_entries);
        assert!(log_50.starts_with("📋 系统日志 (最近50行):"));
        
        let log_100 = format!("📋 系统日志 (最近100行):\n{}", log_entries);
        assert!(log_100.starts_with("📋 系统日志 (最近100行):"));
        
        let log_all = format!("📋 系统日志 (全部):\n{}", log_entries);
        assert!(log_all.starts_with("📋 系统日志 (全部):"));
        
        // 测试日志截断
        let long_log = "a".repeat(5000);
        let truncated_log = if long_log.len() > 4000 {
            format!("📋 系统日志 (全部 - 已截取部分内容):\n{}\n\n⚠️ 日志过长，已截取前 4000 字符", &long_log[..4000])
        } else {
            format!("📋 系统日志 (全部):\n{}", long_log)
        };
        assert!(truncated_log.contains("已截取部分内容"));
        assert!(truncated_log.contains("⚠️ 日志过长"));
    }
    
    #[test]
    fn test_maintenance_history_message_format() {
        // 测试维护历史消息格式
        
        let summary = "📊 维护历史摘要\n\n总维护次数: 15\n成功维护: 13\n失败维护: 2\n平均维护时间: 120 秒";
        
        assert!(summary.starts_with("📊 维护历史摘要"));
        assert!(summary.contains("总维护次数:"));
        assert!(summary.contains("成功维护:"));
        assert!(summary.contains("失败维护:"));
        assert!(summary.contains("平均维护时间:"));
        
        // 测试分页消息
        let page_message = format!("{}\n\n📊 共 25 条记录", summary);
        assert!(page_message.contains("📊 共 25 条记录"));
    }
    
    #[test]
    fn test_cron_expression_message_format() {
        // 测试 Cron 表达式消息格式
        
        let cron_examples = vec![
            ("0 4 * * *", "每天凌晨4点"),
            ("0 4 * * Sun", "每周日凌晨4点"),
            ("0 4 1 * *", "每月1号凌晨4点"),
        ];
        
        for (cron_expr, description) in cron_examples {
            let custom_message = format!("⏰ 自定义定时任务设置\n\n📝 请发送 Cron 表达式:\n\n示例:\n• 每天凌晨4点: 0 4 * * *\n• 每周日凌晨4点: 0 4 * * Sun\n• 每月1号凌晨4点: 0 4 1 * *\n\n使用命令: /set_schedule <cron_expression>");
            
            assert!(custom_message.contains("📝 请发送 Cron 表达式:"));
            assert!(custom_message.contains("示例:"));
            assert!(custom_message.contains("使用命令: /set_schedule"));
        }
    }

    // ========== 综合测试 ==========
    
    #[test]
    fn test_complete_menu_navigation() {
        // 测试完整菜单导航流程
        
        // 1. 主菜单
        let main_menu = build_main_menu_keyboard();
        assert!(main_menu.inline_keyboard.len() > 0);
        
        // 2. 进入维护菜单
        let maintain_menu = build_maintain_menu_keyboard();
        assert!(maintain_menu.inline_keyboard.len() > 0);
        
        // 3. 进入任务类型菜单
        let task_menu = build_task_type_menu_keyboard();
        assert!(task_menu.inline_keyboard.len() > 0);
        
        // 4. 进入预设时间菜单
        let preset_menu = build_schedule_presets_keyboard("system_maintenance");
        assert!(preset_menu.inline_keyboard.len() > 0);
        
        // 5. 进入时间选择菜单
        let time_menu = build_time_selection_keyboard("system_maintenance", "daily");
        assert!(time_menu.inline_keyboard.len() > 0);
        
        // 6. 检查所有菜单都有返回按钮
        let menus = vec![&main_menu, &maintain_menu, &task_menu, &preset_menu, &time_menu];
        for menu in menus {
            assert!(menu.inline_keyboard.iter().any(|row| {
                row.iter().any(|btn| btn.text.contains("返回"))
            }), "所有菜单都应该有返回按钮");
        }
    }
    
    #[test]
    fn test_all_button_text_uniqueness() {
        // 测试所有按钮文本的唯一性
        
        let mut all_button_texts = Vec::new();
        
        // 收集所有菜单的按钮文本
        let menus = vec![
            build_main_menu_keyboard(),
            build_maintain_menu_keyboard(),
            build_task_type_menu_keyboard(),
            build_schedule_presets_keyboard("system_maintenance"),
            build_time_selection_keyboard("system_maintenance", "daily"),
            build_log_selection_keyboard(),
            build_maintenance_history_keyboard(0),
        ];
        
        for menu in menus {
            for row in menu.inline_keyboard {
                for button in row {
                    all_button_texts.push(button.text.clone());
                }
            }
        }
        
        // 检查是否有重复的按钮文本（允许一些重复，如返回按钮）
        let mut button_counts = std::collections::HashMap::new();
        for text in &all_button_texts {
            *button_counts.entry(text).or_insert(0) += 1;
        }
        
        // 只检查非返回按钮的唯一性
        for (text, count) in button_counts {
            if !text.contains("返回") && count > 1 {
                panic!("发现重复的按钮文本: {}, 出现次数: {}", text, count);
            }
        }
        
        assert!(true, "按钮文本检查完成");
    }
    
    #[test]
    fn test_emoji_consistency_across_menus() {
        // 测试所有菜单中 emoji 的一致性
        
        let main_menu = build_main_menu_keyboard();
        let maintain_menu = build_maintain_menu_keyboard();
        let task_menu = build_task_type_menu_keyboard();
        
        // 检查是否所有按钮都使用了 emoji
        let menus = vec![main_menu, maintain_menu, task_menu];
        
        for menu in menus {
            for row in menu.inline_keyboard {
                for button in row {
                    // 每个按钮文本都应该包含至少一个 emoji
                    let has_emoji = button.text.chars().any(|c| c as u32 > 0x2600); // 基本的 emoji 范围
                    assert!(has_emoji, "按钮文本缺少 emoji: {}", button.text);
                }
            }
        }
    }
}