#[cfg(test)]
mod tests {
    use super::*;

    // ============ TaskType Display Name Tests ============

    #[test]
    fn test_task_type_system_maintenance_display() {
        let task_type = TaskType::SystemMaintenance;
        assert_eq!(task_type.get_display_name(), "🔄 系统维护");
    }

    #[test]
    fn test_task_type_core_maintenance_display() {
        let task_type = TaskType::CoreMaintenance;
        assert_eq!(task_type.get_display_name(), "🚀 核心维护");
    }

    #[test]
    fn test_task_type_rules_maintenance_display() {
        let task_type = TaskType::RulesMaintenance;
        assert_eq!(task_type.get_display_name(), "🌍 规则维护");
    }

    #[test]
    fn test_task_type_update_xray_display() {
        let task_type = TaskType::UpdateXray;
        assert_eq!(task_type.get_display_name(), "🔧 更新 Xray");
    }

    #[test]
    fn test_task_type_update_singbox_display() {
        let task_type = TaskType::UpdateSingbox;
        assert_eq!(task_type.get_display_name(), "📦 更新 Sing-box");
    }

    // ============ ScheduledTask Constructor Tests ============

    #[test]
    fn test_scheduled_task_new() {
        let task = ScheduledTask::new(TaskType::SystemMaintenance, "0 4 * * *");
        assert_eq!(task.task_type, TaskType::SystemMaintenance);
        assert_eq!(task.cron_expression, "0 4 * * *");
        assert!(task.enabled);
    }

    #[test]
    fn test_scheduled_task_get_display_name() {
        let task = ScheduledTask::new(TaskType::CoreMaintenance, "0 5 * * Sun");
        let display = task.get_display_name();
        assert!(display.contains("🚀 核心维护"));
        assert!(display.contains("0 5 * * Sun"));
    }

    #[test]
    fn test_scheduled_task_default_enabled() {
        let task = ScheduledTask::new(TaskType::RulesMaintenance, "0 3 * * *");
        assert!(task.enabled);
    }

    // ============ TaskType Cron Suggestions Tests ============

    #[test]
    fn test_system_maintenance_cron_suggestions() {
        let suggestions = TaskType::SystemMaintenance.get_cron_suggestions();
        assert_eq!(suggestions.len(), 3);
        assert!(suggestions.iter().any(|(desc, _)| desc.contains("每天")));
        assert!(suggestions.iter().any(|(desc, _)| desc.contains("每周日")));
        assert!(suggestions.iter().any(|(desc, _)| desc.contains("每月")));
    }

    #[test]
    fn test_core_maintenance_cron_suggestions() {
        let suggestions = TaskType::CoreMaintenance.get_cron_suggestions();
        assert_eq!(suggestions.len(), 3);
        // 验证包含每两周的建议
        assert!(suggestions.iter().any(|(_, cron)| cron.contains("*/14")));
    }

    #[test]
    fn test_rules_maintenance_cron_suggestions() {
        let suggestions = TaskType::RulesMaintenance.get_cron_suggestions();
        assert_eq!(suggestions.len(), 3);
        // 验证包含每6小时的建议
        assert!(suggestions.iter().any(|(_, cron)| cron.contains("*/6")));
    }

    #[test]
    fn test_update_xray_cron_suggestions() {
        let suggestions = TaskType::UpdateXray.get_cron_suggestions();
        assert_eq!(suggestions.len(), 3);
        // 验证cron表达式的格式
        for (_, cron) in &suggestions {
            let parts: Vec<&str> = cron.split_whitespace().collect();
            assert_eq!(parts.len(), 5);
        }
    }

    #[test]
    fn test_update_singbox_cron_suggestions() {
        let suggestions = TaskType::UpdateSingbox.get_cron_suggestions();
        assert_eq!(suggestions.len(), 3);
        // 验证所有建议都有不同的执行时间
        let times: Vec<&str> = suggestions.iter().map(|(_, cron)| cron).collect();
        assert_ne!(times[0], times[1]);
        assert_ne!(times[1], times[2]);
    }

    // ============ Edge Cases Tests ============

    #[test]
    fn test_scheduled_task_with_special_characters_in_cron() {
        let cron = "*/15 0-6 * * 1-5";
        let task = ScheduledTask::new(TaskType::SystemMaintenance, cron);
        assert_eq!(task.cron_expression, cron);
    }

    #[test]
    fn test_all_task_types_have_display_names() {
        let task_types = [
            TaskType::SystemMaintenance,
            TaskType::CoreMaintenance,
            TaskType::RulesMaintenance,
            TaskType::UpdateXray,
            TaskType::UpdateSingbox,
        ];
        
        for task_type in task_types.iter() {
            let name = task_type.get_display_name();
            assert!(!name.is_empty());
            assert!(name.starts_with(|c: char| c.is_emoji() || c.is_alphanumeric()));
        }
    }

    #[test]
    fn test_all_task_types_have_cron_suggestions() {
        let task_types = [
            TaskType::SystemMaintenance,
            TaskType::CoreMaintenance,
            TaskType::RulesMaintenance,
            TaskType::UpdateXray,
            TaskType::UpdateSingbox,
        ];
        
        for task_type in task_types.iter() {
            let suggestions = task_type.get_cron_suggestions();
            assert!(!suggestions.is_empty());
            for (desc, cron) in suggestions.iter() {
                assert!(!desc.is_empty());
                assert!(!cron.is_empty());
            }
        }
    }

    // ============ Serialization Tests ============

    #[test]
    fn test_scheduled_task_serialize_deserialize() {
        use serde_json;
        
        let task = ScheduledTask::new(TaskType::SystemMaintenance, "0 4 * * *");
        let json = serde_json::to_string(&task).expect("Failed to serialize");
        let decoded: ScheduledTask = serde_json::from_str(&json).expect("Failed to deserialize");
        
        assert_eq!(decoded.task_type, task.task_type);
        assert_eq!(decoded.cron_expression, task.cron_expression);
        assert_eq!(decoded.enabled, task.enabled);
    }
}
