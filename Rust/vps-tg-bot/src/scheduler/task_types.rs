use serde::{Serialize, Deserialize};
use teloxide::Bot;
use teloxide::types::ChatId;
use teloxide::prelude::Requester;
use crate::config::Config;
use crate::system::ops;
use anyhow::Result;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ScheduledTask {
    pub task_type: TaskType,
    pub cron_expression: String,
    pub enabled: bool,
}

impl ScheduledTask {
    pub fn new(task_type: TaskType, cron_expression: &str) -> Self {
        Self {
            task_type,
            cron_expression: cron_expression.to_string(),
            enabled: true,
        }
    }

    pub fn get_display_name(&self) -> String {
        format!("{} ({})", self.task_type.get_display_name(), self.cron_expression)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum TaskType {
    SystemMaintenance,    // 系统维护
    CoreMaintenance,      // 核心维护（系统更新+重启）
    RulesMaintenance,     // 规则维护
    UpdateXray,          // 更新 Xray
    UpdateSingbox,       // 更新 Sing-box
}

impl TaskType {
    pub fn get_display_name(&self) -> &'static str {
        match self {
            TaskType::SystemMaintenance => "🔄 系统维护",
            TaskType::CoreMaintenance => "🚀 核心维护",
            TaskType::RulesMaintenance => "🌍 规则维护",
            TaskType::UpdateXray => "🔧 更新 Xray",
            TaskType::UpdateSingbox => "📦 更新 Sing-box",
        }
    }

    pub fn get_cron_suggestions(&self) -> Vec<(&'static str, &'static str)> {
        match self {
            TaskType::SystemMaintenance => vec![
                ("每天凌晨4点", "0 4 * * *"),
                ("每周日凌晨4点", "0 4 * * Sun"),
                ("每月1号凌晨4点", "0 4 1 * *"),
            ],
            TaskType::CoreMaintenance => vec![
                ("每周日凌晨5点", "0 5 * * Sun"),
                ("每月1号凌晨5点", "0 5 1 * *"),
                ("每两周日凌晨5点", "0 5 */14 * *"),
            ],
            TaskType::RulesMaintenance => vec![
                ("每天凌晨3点", "0 3 * * *"),
                ("每6小时", "0 */6 * * *"),
                ("每周日凌晨3点", "0 3 * * Sun"),
            ],
            TaskType::UpdateXray => vec![
                ("每周日凌晨6点", "0 6 * * Sun"),
                ("每月1号凌晨6点", "0 6 1 * *"),
                ("每两周日凌晨6点", "0 6 */14 * *"),
            ],
            TaskType::UpdateSingbox => vec![
                ("每周日凌晨7点", "0 7 * * Sun"),
                ("每月1号凌晨7点", "0 7 1 * *"),
                ("每两周日凌晨7点", "0 7 */14 * *"),
            ],
        }
    }

    pub async fn execute(&self, bot: &Bot, chat_id: i64) -> Result<String> {
        let task_name = self.get_display_name();
        
        match self {
            TaskType::SystemMaintenance => {
                match ops::perform_maintenance().await {
                    Ok(log) => {
                        let _ = bot.send_message(ChatId(chat_id), 
                            format!("✅ {} 任务已完成:\n{}", task_name, log)).await;
                        Ok(format!("{} 完成", task_name))
                    }
                    Err(e) => {
                        let _ = bot.send_message(ChatId(chat_id), 
                            format!("❌ {} 任务失败: {}", task_name, e)).await;
                        Err(e)
                    }
                }
            }
            TaskType::CoreMaintenance => {
                match ops::maintain_core().await {
                    Ok(log) => {
                        let _ = bot.send_message(ChatId(chat_id), 
                            format!("✅ {} 任务已完成:\n{}", task_name, log)).await;
                        Ok(format!("{} 完成", task_name))
                    }
                    Err(e) => {
                        let _ = bot.send_message(ChatId(chat_id), 
                            format!("❌ {} 任务失败: {}", task_name, e)).await;
                        Err(e)
                    }
                }
            }
            TaskType::RulesMaintenance => {
                match ops::maintain_rules().await {
                    Ok(log) => {
                        let _ = bot.send_message(ChatId(chat_id), 
                            format!("✅ {} 任务已完成:\n{}", task_name, log)).await;
                        Ok(format!("{} 完成", task_name))
                    }
                    Err(e) => {
                        let _ = bot.send_message(ChatId(chat_id), 
                            format!("❌ {} 任务失败: {}", task_name, e)).await;
                        Err(e)
                    }
                }
            }
            TaskType::UpdateXray => {
                match ops::update_xray().await {
                    Ok(log) => {
                        let _ = bot.send_message(ChatId(chat_id), 
                            format!("✅ {} 任务已完成:\n{}", task_name, log)).await;
                        Ok(format!("{} 完成", task_name))
                    }
                    Err(e) => {
                        let _ = bot.send_message(ChatId(chat_id), 
                            format!("❌ {} 任务失败: {}", task_name, e)).await;
                        Err(e)
                    }
                }
            }
            TaskType::UpdateSingbox => {
                match ops::update_singbox().await {
                    Ok(log) => {
                        let _ = bot.send_message(ChatId(chat_id), 
                            format!("✅ {} 任务已完成:\n{}", task_name, log)).await;
                        Ok(format!("{} 完成", task_name))
                    }
                    Err(e) => {
                        let _ = bot.send_message(ChatId(chat_id), 
                            format!("❌ {} 任务失败: {}", task_name, e)).await;
                        Err(e)
                    }
                }
            }
        }
    }
}