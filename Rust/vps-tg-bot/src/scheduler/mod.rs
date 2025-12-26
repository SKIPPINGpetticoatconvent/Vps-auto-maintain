use tokio_cron_scheduler::{JobScheduler, Job};
use teloxide::Bot;
use teloxide::types::ChatId;
use teloxide::prelude::Requester;
use crate::config::Config;
use crate::system::ops;
use anyhow::Result;
use serde::{Serialize, Deserialize};
use std::fs;
use std::path::Path;
use tokio::sync::Mutex;
use std::sync::Arc;
use once_cell::sync::Lazy;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SchedulerState {
    pub cron_expression: String,
}

impl SchedulerState {
    pub fn new(cron_expression: &str) -> Self {
        Self {
            cron_expression: cron_expression.to_string(),
        }
    }

    pub fn default() -> Self {
        Self {
            cron_expression: "0 0 4 * * Sun".to_string(),
        }
    }

    pub fn save_to_file(&self, path: &str) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;
        Ok(())
    }

    pub fn load_from_file(path: &str) -> Result<Self> {
        if !Path::new(path).exists() {
            return Ok(SchedulerState::default());
        }
        let content = fs::read_to_string(path)?;
        let state: SchedulerState = serde_json::from_str(&content)?;
        Ok(state)
    }
}

pub struct SchedulerManager {
    pub scheduler: Arc<Mutex<Option<JobScheduler>>>, 
    pub state: Arc<Mutex<SchedulerState>>,
}

impl SchedulerManager {
    pub async fn new(config: Config, bot: Bot) -> Result<Self> {
        let state_path = "scheduler_state.json";
        let state = SchedulerState::load_from_file(state_path)?;
        
        let sched = JobScheduler::new().await?;
        let scheduler = Arc::new(Mutex::new(Some(sched)));
        let state = Arc::new(Mutex::new(state.clone()));
        
        let manager = Self { scheduler, state };
        manager.start_scheduler(config, bot).await?;
        
        Ok(manager)
    }

    pub async fn start_scheduler(&self, config: Config, bot: Bot) -> Result<()> {
        let state = self.state.lock().await;
        let cron_expr = state.cron_expression.clone();
        drop(state); // 释放锁
        
        let mut scheduler_guard = self.scheduler.lock().await;
        if let Some(sched) = scheduler_guard.as_mut() {
            let job = Job::new_async(cron_expr.as_str(), move |_uuid, _l| {
                let bot = bot.clone();
                let chat_id = config.chat_id;
                Box::pin(async move {
                    match ops::perform_maintenance().await {
                        Ok(log) => {
                            let _ = bot.send_message(ChatId(chat_id), format!("✅ 计划维护已完成:\n{}", log)).await;
                        }
                        Err(e) => {
                            let _ = bot.send_message(ChatId(chat_id), format!("❌ 计划维护失败: {}", e)).await;
                        }
                    }
                })
            })?;

            sched.add(job).await?;
            sched.start().await?;
        }
        
        Ok(())
    }

    pub async fn update_schedule(&self, new_cron: &str) -> Result<String> {
        // 验证 Cron 表达式
        match self.validate_cron_expression(new_cron) {
            Err(validation_error) => {
                return Ok(format!("❌ {}", validation_error));
            }
            Ok(_) => {}
        }

        let mut state_guard = self.state.lock().await;
        state_guard.cron_expression = new_cron.to_string();
        let state_path = "scheduler_state.json";
        state_guard.save_to_file(state_path)?;
        drop(state_guard);

        // 重新启动调度器
        let mut scheduler_guard = self.scheduler.lock().await;
        if let Some(mut sched) = scheduler_guard.take() {
            sched.shutdown().await?;
        }
        
        let new_sched = JobScheduler::new().await?;
        *scheduler_guard = Some(new_sched);
        
        Ok(format!("✅ 调度已更新为: {}", new_cron))
    }

    fn validate_cron_expression(&self, cron_expr: &str) -> Result<(), String> {
        let fields: Vec<&str> = cron_expr.split_whitespace().collect();
        
        // 检查字段数量
        if fields.len() != 5 {
            return Err(format!("无效的 Cron 表达式。应为 5 个字段（分钟 小时 日 月 周几），当前有 {} 个字段", fields.len()));
        }
        
        let (minute, hour, day, month, weekday) = (fields[0], fields[1], fields[2], fields[3], fields[4]);
        
        // 验证分钟字段 (0-59)
        if !self.is_valid_field(minute, 0, 59) {
            return Err(format!("分钟字段无效。应在 0-59 之间，当前值: {}", minute));
        }
        
        // 验证小时字段 (0-23)
        if !self.is_valid_field(hour, 0, 23) {
            return Err(format!("小时字段无效。应在 0-23 之间，当前值: {}", hour));
        }
        
        // 验证日期字段 (1-31)
        if !self.is_valid_field(day, 1, 31) {
            return Err(format!("日期字段无效。应在 1-31 之间，当前值: {}", day));
        }
        
        // 验证月份字段 (1-12)
        if !self.is_valid_field(month, 1, 12) {
            return Err(format!("月份字段无效。应在 1-12 之间，当前值: {}", month));
        }
        
        // 验证星期字段 (0-7, 0 和 7 都表示周日)
        if !self.is_valid_weekday_field(weekday) {
            return Err(format!("星期字段无效。应在 0-7 之间（0 和 7 都表示周日），当前值: {}", weekday));
        }
        
        Ok(())
    }
    
    fn is_valid_field(&self, field: &str, min: i32, max: i32) -> bool {
        // 处理特殊字符 *
        if field == "*" {
            return true;
        }
        
        // 处理列表 (如: 1,3,5)
        if field.contains(',') {
            return field.split(',').all(|part| self.is_valid_single_value(part, min, max));
        }
        
        // 处理范围 (如: 1-5)
        if field.contains('-') {
            let parts: Vec<&str> = field.split('-').collect();
            if parts.len() != 2 {
                return false;
            }
            return self.is_valid_single_value(parts[0], min, max) && 
                   self.is_valid_single_value(parts[1], min, max);
        }
        
        // 处理步长 (如: */5 或 1-10/2)
        if field.contains('/') {
            let parts: Vec<&str> = field.split('/').collect();
            if parts.len() != 2 {
                return false;
            }
            let base = parts[0];
            let step = parts[1];
            
            // 步长必须是数字
            if step.parse::<i32>().is_err() {
                return false;
            }
            
            // 基础部分可以是 * 或具体值或范围
            if base == "*" {
                return true;
            }
            if base.contains('-') {
                let base_parts: Vec<&str> = base.split('-').collect();
                return base_parts.len() == 2 &&
                       self.is_valid_single_value(base_parts[0], min, max) &&
                       self.is_valid_single_value(base_parts[1], min, max);
            }
            return self.is_valid_single_value(base, min, max);
        }
        
        // 单个数字值
        self.is_valid_single_value(field, min, max)
    }
    
    fn is_valid_single_value(&self, value: &str, min: i32, max: i32) -> bool {
        if let Ok(num) = value.parse::<i32>() {
            num >= min && num <= max
        } else {
            false
        }
    }
    
    fn is_valid_weekday_field(&self, field: &str) -> bool {
        // 特殊处理星期字段，接受数字和缩写
        if field == "*" {
            return true;
        }
        
        // 处理缩写 (Sun, Mon, Tue, Wed, Thu, Fri, Sat)
        let weekdays = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
        if weekdays.iter().any(|&w| w == field) {
            return true;
        }
        
        // 使用通用的字段验证逻辑，但范围是 0-7
        self.is_valid_field(field, 0, 7)
    }
}

// 全局调度器管理器实例
static SCHEDULER_MANAGER: Lazy<Arc<Mutex<Option<SchedulerManager>>>> = Lazy::new(|| Arc::new(Mutex::new(None)));

pub async fn start_scheduler(config: Config, bot: Bot) -> Result<()> {
    let manager = SchedulerManager::new(config, bot).await?;
    let mut manager_guard = SCHEDULER_MANAGER.lock().await;
    *manager_guard = Some(manager);
    drop(manager_guard);
    
    // 保持调度器运行
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;
    }
}

pub async fn get_current_schedule() -> Result<String> {
    let manager_guard = SCHEDULER_MANAGER.lock().await;
    if let Some(manager) = &*manager_guard {
        let state_guard = manager.state.lock().await;
        Ok(state_guard.cron_expression.clone())
    } else {
        Ok("❌ 调度器尚未初始化".to_string())
    }
}

pub async fn update_schedule(new_cron: &str) -> Result<String> {
    let manager_guard = SCHEDULER_MANAGER.lock().await;
    if let Some(manager) = &*manager_guard {
        manager.update_schedule(new_cron).await
    } else {
        Ok("❌ 调度器尚未初始化".to_string())
    }
}