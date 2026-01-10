use serde::{Serialize, Deserialize};
use teloxide::Bot;
use teloxide::types::ChatId;
use teloxide::prelude::Requester;
use crate::system::ops;
use crate::scheduler::maintenance_history::{record_maintenance, MaintenanceResult};
use anyhow::{Result, anyhow};

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

    #[allow(dead_code)]
    pub fn get_display_name(&self) -> String {
        format!("{} ({})", self.task_type.get_display_name(), self.cron_expression)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
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

    #[allow(dead_code)]
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

        // 发送任务开始执行通知
        let _ = bot.send_message(ChatId(chat_id),
            format!("🔄 [定时任务] {} 开始执行...", task_name)).await;

        match self {
            TaskType::SystemMaintenance => {
                match ops::perform_maintenance().await {
                    Ok(log) => {
                        let _ = bot.send_message(ChatId(chat_id),
                            format!("✅ [定时任务] {} 执行成功:\n{}", task_name, log)).await;
                        // 记录到维护历史
                        record_maintenance(task_name, MaintenanceResult::Success, &log, None).await;
                        Ok(format!("{} 完成", task_name))
                    }
                    Err(e) => {
                        let user_message = e.user_message();
                        let error_msg = format!("{}", e);
                        let _ = bot.send_message(ChatId(chat_id),
                            format!("❌ [定时任务] {} 执行失败:\n{}\n\n建议: {}", task_name, e,
                                if e.is_retryable() { "可以稍后重试" } else { "请检查系统配置" })).await;
                        // 记录到维护历史
                        record_maintenance(task_name, MaintenanceResult::Failed, &user_message, Some(&error_msg)).await;
                        Err(anyhow!("{}", user_message))
                    }
                }
            }
            TaskType::CoreMaintenance => {
                match ops::maintain_core().await {
                    Ok(log) => {
                        let _ = bot.send_message(ChatId(chat_id),
                            format!("✅ [定时任务] {} 执行成功:\n{}", task_name, log)).await;
                        // 记录到维护历史
                        record_maintenance(task_name, MaintenanceResult::Success, &log, None).await;
                        Ok(format!("{} 完成", task_name))
                    }
                    Err(e) => {
                        let user_message = e.user_message();
                        let error_msg = format!("{}", e);
                        let _ = bot.send_message(ChatId(chat_id),
                            format!("❌ [定时任务] {} 执行失败:\n{}\n\n建议: {}", task_name, e,
                                if e.is_retryable() { "可以稍后重试" } else { "请检查系统配置" })).await;
                        // 记录到维护历史
                        record_maintenance(task_name, MaintenanceResult::Failed, &user_message, Some(&error_msg)).await;
                        Err(anyhow!("{}", user_message))
                    }
                }
            }
            TaskType::RulesMaintenance => {
                match ops::maintain_rules().await {
                    Ok(log) => {
                        let _ = bot.send_message(ChatId(chat_id),
                            format!("✅ [定时任务] {} 执行成功:\n{}", task_name, log)).await;
                        // 记录到维护历史
                        record_maintenance(task_name, MaintenanceResult::Success, &log, None).await;
                        Ok(format!("{} 完成", task_name))
                    }
                    Err(e) => {
                        let user_message = e.user_message();
                        let error_msg = format!("{}", e);
                        let _ = bot.send_message(ChatId(chat_id),
                            format!("❌ [定时任务] {} 执行失败:\n{}\n\n建议: {}", task_name, e,
                                if e.is_retryable() { "可以稍后重试" } else { "请检查系统配置" })).await;
                        // 记录到维护历史
                        record_maintenance(task_name, MaintenanceResult::Failed, &user_message, Some(&error_msg)).await;
                        Err(anyhow!("{}", user_message))
                    }
                }
            }
            TaskType::UpdateXray => {
                match ops::update_xray().await {
                    Ok(log) => {
                        let _ = bot.send_message(ChatId(chat_id),
                            format!("✅ [定时任务] {} 执行成功:\n{}", task_name, log)).await;
                        // 记录到维护历史
                        record_maintenance(task_name, MaintenanceResult::Success, &log, None).await;
                        Ok(format!("{} 完成", task_name))
                    }
                    Err(e) => {
                        let user_message = e.user_message();
                        let error_msg = format!("{}", e);
                        let _ = bot.send_message(ChatId(chat_id),
                            format!("❌ [定时任务] {} 执行失败:\n{}\n\n建议: {}", task_name, e,
                                if e.is_retryable() { "可以稍后重试" } else { "请检查系统配置" })).await;
                        // 记录到维护历史
                        record_maintenance(task_name, MaintenanceResult::Failed, &user_message, Some(&error_msg)).await;
                        Err(anyhow!("{}", user_message))
                    }
                }
            }
            TaskType::UpdateSingbox => {
                match ops::update_singbox().await {
                    Ok(log) => {
                        let _ = bot.send_message(ChatId(chat_id),
                            format!("✅ [定时任务] {} 执行成功:\n{}", task_name, log)).await;
                        // 记录到维护历史
                        record_maintenance(task_name, MaintenanceResult::Success, &log, None).await;
                        Ok(format!("{} 完成", task_name))
                    }
                    Err(e) => {
                        let user_message = e.user_message();
                        let error_msg = format!("{}", e);
                        let _ = bot.send_message(ChatId(chat_id),
                            format!("❌ [定时任务] {} 执行失败:\n{}\n\n建议: {}", task_name, e,
                                if e.is_retryable() { "可以稍后重试" } else { "请检查系统配置" })).await;
                        // 记录到维护历史
                        record_maintenance(task_name, MaintenanceResult::Failed, &user_message, Some(&error_msg)).await;
                        Err(anyhow!("{}", user_message))
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_scheduled_task_new() {
        let task = ScheduledTask::new(TaskType::SystemMaintenance, "0 4 * * *");
        assert_eq!(task.cron_expression, "0 4 * * *");
        assert!(task.enabled);
        assert_eq!(task.task_type, TaskType::SystemMaintenance);
    }

    #[test]
    fn test_scheduled_task_display_name() {
        let task = ScheduledTask::new(TaskType::SystemMaintenance, "0 4 * * *");
        let display = task.get_display_name();
        assert!(display.contains("系统维护"));
        assert!(display.contains("0 4 * * *"));
    }

    #[test]
    fn test_task_type_display_names() {
        assert_eq!(TaskType::SystemMaintenance.get_display_name(), "🔄 系统维护");
        assert_eq!(TaskType::CoreMaintenance.get_display_name(), "🚀 核心维护");
        assert_eq!(TaskType::RulesMaintenance.get_display_name(), "🌍 规则维护");
        assert_eq!(TaskType::UpdateXray.get_display_name(), "🔧 更新 Xray");
        assert_eq!(TaskType::UpdateSingbox.get_display_name(), "📦 更新 Sing-box");
    }

    #[test]
    fn test_task_type_cron_suggestions_count() {
        assert_eq!(TaskType::SystemMaintenance.get_cron_suggestions().len(), 3);
        assert_eq!(TaskType::CoreMaintenance.get_cron_suggestions().len(), 3);
        assert_eq!(TaskType::RulesMaintenance.get_cron_suggestions().len(), 3);
        assert_eq!(TaskType::UpdateXray.get_cron_suggestions().len(), 3);
        assert_eq!(TaskType::UpdateSingbox.get_cron_suggestions().len(), 3);
    }

    #[test]
    fn test_serialization() {
        let task = ScheduledTask::new(TaskType::SystemMaintenance, "0 4 * * *");
        let json = serde_json::to_string(&task).unwrap();
        assert!(json.contains("\"task_type\":\"SystemMaintenance\""));
        assert!(json.contains("\"cron_expression\":\"0 4 * * *\""));
        assert!(json.contains("\"enabled\":true"));
    }

    #[test]
    fn test_deserialization() {
        let json = r#"{"task_type":"SystemMaintenance","cron_expression":"0 4 * * *","enabled":true}"#;
        let task: ScheduledTask = serde_json::from_str(json).unwrap();
        assert_eq!(task.task_type, TaskType::SystemMaintenance);
        assert_eq!(task.cron_expression, "0 4 * * *");
        assert!(task.enabled);
    }

    #[test]
    fn test_all_task_types_count() {
        // 验证所有任务类型都被正确定义
        let variants = [
            TaskType::SystemMaintenance,
            TaskType::CoreMaintenance,
            TaskType::RulesMaintenance,
            TaskType::UpdateXray,
            TaskType::UpdateSingbox,
        ];
        assert_eq!(variants.len(), 5);
    }

    #[test]
    fn test_scheduled_task_display_name_with_long_cron() {
        let task = ScheduledTask::new(TaskType::SystemMaintenance, "0 4 * * 0,1,2,3,4,5,6");
        let display = task.get_display_name();
        assert!(display.contains("系统维护"));
        assert!(display.contains("0 4 * * 0,1,2,3,4,5,6"));
    }

    #[test]
    fn test_scheduled_task_disabled() {
        let mut task = ScheduledTask::new(TaskType::UpdateXray, "0 6 * * Sun");
        task.enabled = false;
        assert!(!task.enabled);
        
        let display = task.get_display_name();
        assert!(display.contains("更新 Xray"));
        assert!(display.contains("0 6 * * Sun"));
    }

    #[test]
    fn test_task_type_display_names_consistency() {
        let task_types = vec![
            TaskType::SystemMaintenance,
            TaskType::CoreMaintenance,
            TaskType::RulesMaintenance,
            TaskType::UpdateXray,
            TaskType::UpdateSingbox,
        ];
        
        for task_type in task_types {
            let display_name = task_type.get_display_name();
            // 验证显示名称不为空
            assert!(!display_name.is_empty());
            // 验证显示名称包含emoji
            assert!(display_name.chars().any(|c| c as u32 > 255));
        }
    }

    #[test]
    fn test_task_type_cron_suggestions_format() {
        // 测试系统维护的建议格式
        let suggestions = TaskType::SystemMaintenance.get_cron_suggestions();
        assert_eq!(suggestions.len(), 3);
        
        for (description, cron) in suggestions {
            // 验证描述不为空
            assert!(!description.is_empty());
            // 验证Cron表达式格式正确（5个字段）
            let fields: Vec<&str> = cron.split_whitespace().collect();
            assert_eq!(fields.len(), 5);
        }
    }

    #[test]
    fn test_task_type_cron_suggestions_uniqueness() {
        // 测试不同任务类型的Cron建议不重复
        let system_suggestions = TaskType::SystemMaintenance.get_cron_suggestions();
        let core_suggestions = TaskType::CoreMaintenance.get_cron_suggestions();
        let rules_suggestions = TaskType::RulesMaintenance.get_cron_suggestions();
        
        let all_suggestions: Vec<&str> = system_suggestions.iter()
            .chain(core_suggestions.iter())
            .chain(rules_suggestions.iter())
            .map(|(_, cron)| *cron)
            .collect();
        
        // 检查是否有重复的Cron表达式
        let unique_crons: std::collections::HashSet<&str> = all_suggestions.iter().cloned().collect();
        assert_eq!(all_suggestions.len(), unique_crons.len());
    }

    #[test]
    fn test_serialization_round_trip() {
        let original_task = ScheduledTask::new(TaskType::RulesMaintenance, "0 3 * * Sun");
        
        // 序列化
        let json = serde_json::to_string(&original_task).unwrap();
        
        // 反序列化
        let deserialized_task: ScheduledTask = serde_json::from_str(&json).unwrap();
        
        // 验证字段一致
        assert_eq!(original_task.task_type, deserialized_task.task_type);
        assert_eq!(original_task.cron_expression, deserialized_task.cron_expression);
        assert_eq!(original_task.enabled, deserialized_task.enabled);
    }

    #[test]
    fn test_serialization_with_disabled_task() {
        let mut task = ScheduledTask::new(TaskType::UpdateSingbox, "0 7 * * Mon");
        task.enabled = false;
        
        let json = serde_json::to_string(&task).unwrap();
        let deserialized_task: ScheduledTask = serde_json::from_str(&json).unwrap();
        
        assert!(!deserialized_task.enabled);
        assert_eq!(deserialized_task.task_type, TaskType::UpdateSingbox);
    }

    #[test]
    fn test_deserialization_with_invalid_task_type() {
        let invalid_json = r#"{\"task_type\":\"InvalidType\",\"cron_expression\":\"0 4 * * *\",\"enabled\":true}"#;
        
        let result: Result<ScheduledTask, _> = serde_json::from_str(invalid_json);
        assert!(result.is_err());
    }

    #[test]
    fn test_scheduled_task_with_empty_cron() {
        let task = ScheduledTask::new(TaskType::SystemMaintenance, "");
        assert_eq!(task.cron_expression, "");
        assert!(task.enabled);
    }

    #[test]
    fn test_scheduled_task_with_special_characters() {
        let special_cron = "0 4 * * Sun,Mon,Tue";
        let task = ScheduledTask::new(TaskType::CoreMaintenance, special_cron);
        assert_eq!(task.cron_expression, special_cron);
        assert_eq!(task.task_type, TaskType::CoreMaintenance);
    }
}