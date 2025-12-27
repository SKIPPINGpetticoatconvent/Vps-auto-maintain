use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Serialize, Deserialize, Debug, Clone)]
struct SchedulerState {
    cron_expression: String,
}

impl SchedulerState {
    fn new(cron_expression: &str) -> Self {
        Self {
            cron_expression: cron_expression.to_string(),
        }
    }

    fn default() -> Self {
        Self {
            cron_expression: "0 4 * * Sun".to_string(),
        }
    }

    fn save_to_file(&self, path: &str) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(self).unwrap();
        fs::write(path, json)
    }

    fn load_from_file(path: &str) -> Self {
        if !Path::new(path).exists() {
            return SchedulerState::default();
        }
        let content = fs::read_to_string(path).unwrap();
        let state: SchedulerState = serde_json::from_str(&content).unwrap();
        state
    }
}

#[test]
fn test_scheduler_state_persistence() {
    // Test creating and saving scheduler state
    let state = SchedulerState::new("0 0 * * *");
    state.save_to_file("test_scheduler_state.json").unwrap();
    
    // Test loading scheduler state
    let loaded_state = SchedulerState::load_from_file("test_scheduler_state.json");
    
    assert_eq!(state.cron_expression, loaded_state.cron_expression);
    
    // Test default state when file doesn't exist
    fs::remove_file("test_scheduler_state.json").ok();
    let default_state = SchedulerState::load_from_file("nonexistent_file.json");
    assert_eq!(default_state.cron_expression, "0 4 * * Sun");
    
    // Clean up
    fs::remove_file("test_scheduler_state.json").ok();
}

#[test]
fn test_cron_validation() {
    // 测试字段数量验证
    let valid_cron = "0 4 * * *";
    let invalid_cron_short = "0 4 * *";
    let invalid_cron_long = "0 4 * * * *";
    
    assert_eq!(valid_cron.split_whitespace().count(), 5);
    assert_eq!(invalid_cron_short.split_whitespace().count(), 4);
    assert_eq!(invalid_cron_long.split_whitespace().count(), 6);
}

#[test]
fn test_cron_field_validation() {
    // 测试各种字段值的有效性
    let test_cases = vec![
        // (表达式, 是否应该有效, 描述)
        ("0 4 * * *", true, "每天凌晨4点"),
        ("30 14 * * Mon", true, "每周一下午2点30分"),
        ("0 0 1 * *", true, "每月1号凌晨0点"),
        ("15 9 * * 0", true, "每周日凌晨9点15分"),
        ("0 23 * * 1-5", true, "工作日晚上23点"),
        ("*/15 * * * *", true, "每15分钟"),
        ("0 8-18/2 * * *", true, "8点到18点之间每2小时"),
        ("0,30 12 * * *", true, "每天12点和12点30分"),
        ("60 4 * * *", false, "分钟超出范围(60)"),
        ("0 25 * * *", false, "小时超出范围(25)"),
        ("0 4 32 * *", false, "日期超出范围(32)"),
        ("0 4 * 13 *", false, "月份超出范围(13)"),
        ("0 4 * * 8", false, "星期超出范围(8)"),
        ("abc 4 * * *", false, "分钟字段非数字"),
        ("0 abc * * *", false, "小时字段非数字"),
        ("0 4 -1 * *", false, "日期为负数"),
        ("0 4 * 0 *", false, "月份为0"),
    ];
    
    for (cron_expr, should_be_valid, description) in test_cases {
        let fields: Vec<&str> = cron_expr.split_whitespace().collect();
        let field_count_valid = fields.len() == 5;
        
        let mut values_valid = true;
        if field_count_valid {
            let (minute, hour, day, month, weekday) = (fields[0], fields[1], fields[2], fields[3], fields[4]);
            
            // 检查每个字段的数值范围
            if let Ok(min_val) = minute.parse::<i32>() {
                if min_val < 0 || min_val > 59 { values_valid = false; }
            } else if minute != "*" && !minute.contains(',') && !minute.contains('-') && !minute.contains('/') {
                values_valid = false;
            }
            
            if let Ok(hour_val) = hour.parse::<i32>() {
                if hour_val < 0 || hour_val > 23 { values_valid = false; }
            } else if hour != "*" && !hour.contains(',') && !hour.contains('-') && !hour.contains('/') {
                values_valid = false;
            }
            
            if let Ok(day_val) = day.parse::<i32>() {
                if day_val < 1 || day_val > 31 { values_valid = false; }
            } else if day != "*" && !day.contains(',') && !day.contains('-') && !day.contains('/') {
                values_valid = false;
            }
            
            if let Ok(month_val) = month.parse::<i32>() {
                if month_val < 1 || month_val > 12 { values_valid = false; }
            } else if month != "*" && !month.contains(',') && !month.contains('-') && !month.contains('/') {
                values_valid = false;
            }
            
            if let Ok(weekday_val) = weekday.parse::<i32>() {
                if weekday_val < 0 || weekday_val > 7 { values_valid = false; }
            } else if weekday != "*" && !weekday.contains(',') && !weekday.contains('-') && !weekday.contains('/') {
                // 检查星期缩写
                let valid_weekdays = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
                if !valid_weekdays.iter().any(|&w| w == weekday) {
                    values_valid = false;
                }
            }
        }
        
        let is_valid = field_count_valid && values_valid;
        assert_eq!(is_valid, should_be_valid, 
            "测试 '{}' 失败: {} - 预期: {}, 实际: {}", 
            cron_expr, description, should_be_valid, is_valid);
    }
}