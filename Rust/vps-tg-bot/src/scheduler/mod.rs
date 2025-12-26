use tokio_cron_scheduler::{JobScheduler, Job, JobSchedulerError};
use teloxide::Bot;
use teloxide::types::ChatId;
use teloxide::prelude::Requester;
use crate::config::Config;
use crate::system::ops;
use crate::scheduler::task_types::{TaskType, ScheduledTask};
use anyhow::Result;
use serde::{Serialize, Deserialize};
use std::fs;
use std::path::Path;
use tokio::sync::Mutex;
use std::sync::Arc;
use once_cell::sync::Lazy;

pub mod task_types;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SchedulerState {
    pub tasks: Vec<ScheduledTask>,
}

impl SchedulerState {
    pub fn new() -> Self {
        Self {
            tasks: vec![
                ScheduledTask::new(TaskType::SystemMaintenance, "0 4 * * Sun"),
            ],
        }
    }

    pub fn default() -> Self {
        Self::new()
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

    pub fn add_task(&mut self, task: ScheduledTask) {
        self.tasks.push(task);
    }

    pub fn remove_task(&mut self, index: usize) -> Result<()> {
        if index < self.tasks.len() {
            self.tasks.remove(index);
            Ok(())
        } else {
            Err(anyhow::anyhow!("任务索引超出范围"))
        }
    }

    pub fn get_task(&self, index: usize) -> Option<&ScheduledTask> {
        self.tasks.get(index)
    }

    pub fn update_task(&mut self, index: usize, new_cron: &str) -> Result<()> {
        if index < self.tasks.len() {
            // 验证 Cron 表达式
            let validator = SchedulerValidator::new();
            match validator.validate_cron_expression(new_cron) {
                Err(validation_error) => {
                    return Err(anyhow::anyhow!("{}", validation_error));
                }
                Ok(_) => {}
            }
            
            self.tasks[index].cron_expression = new_cron.to_string();
            Ok(())
        } else {
            Err(anyhow::anyhow!("任务索引超出范围"))
        }
    }

    pub fn toggle_task(&mut self, index: usize) -> Result<()> {
        if index < self.tasks.len() {
            self.tasks[index].enabled = !self.tasks[index].enabled;
            Ok(())
        } else {
            Err(anyhow::anyhow!("任务索引超出范围"))
        }
    }

    pub fn get_all_tasks_summary(&self) -> String {
        if self.tasks.is_empty() {
            return "📝 暂无定时任务".to_string();
        }

        let mut summary = String::new();
        summary.push_str("⏰ 定时任务列表:\n\n");
        
        for (i, task) in self.tasks.iter().enumerate() {
            let status = if task.enabled { "✅" } else { "⏸️" };
            summary.push_str(&format!("{}. {} {}\n   Cron: {}\n\n", 
                i + 1, status, task.task_type.get_display_name(), task.cron_expression));
        }
        
        summary
    }
}

#[derive(Clone)]
pub struct SchedulerManager {
    pub scheduler: Arc<Mutex<Option<JobScheduler>>>, 
    pub state: Arc<Mutex<SchedulerState>>,
}

impl SchedulerManager {
    pub async fn new(config: Config, bot: Bot) -> Result<Self, JobSchedulerError> {
        let state_path = "scheduler_state.json";
        let state = SchedulerState::load_from_file(state_path).unwrap_or_else(|_| SchedulerState::default());
        
        let sched = JobScheduler::new().await?;
        let scheduler = Arc::new(Mutex::new(Some(sched)));
        let state = Arc::new(Mutex::new(state.clone()));
        
        let manager = Self { scheduler, state };
        let _ = manager.start_all_tasks(config, bot).await;
        
        Ok(manager)
    }

    pub async fn start_all_tasks(&self, config: Config, bot: Bot) -> Result<(), JobSchedulerError> {
        let state = self.state.lock().await;
        let tasks = state.tasks.clone();
        drop(state);
        
        let mut scheduler_guard = self.scheduler.lock().await;
        if let Some(sched) = scheduler_guard.as_mut() {
            // 清除现有任务
            let _ = sched.shutdown().await;
            *scheduler_guard = Some(JobScheduler::new().await?);
            
            let sched = scheduler_guard.as_mut().unwrap();
            
            // 添加所有启用的任务
            for task in tasks.iter() {
                if task.enabled {
                    let job = Job::new_async(task.cron_expression.as_str(), {
                        let bot = bot.clone();
                        let task_type = task.task_type.clone();
                        let chat_id = config.chat_id;
                        move |_uuid, _l| {
                            let bot = bot.clone();
                            let task_type = task_type.clone();
                            let chat_id = chat_id;
                            Box::pin(async move {
                                match task_type.execute(&bot, chat_id).await {
                                    Ok(_) => {},
                                    Err(e) => {
                                        eprintln!("任务执行失败: {}", e);
                                    }
                                }
                            })
                        }
                    });

                    if let Ok(job) = job {
                        let _ = sched.add(job).await;
                    }
                }
            }
            
            let _ = sched.start().await;
        }
        
        Ok(())
    }

    pub async fn add_new_task(&self, config: Config, bot: Bot, task_type: TaskType, cron_expression: &str) -> Result<String, JobSchedulerError> {
        let validator = SchedulerValidator::new();
        match validator.validate_cron_expression(cron_expression) {
            Err(validation_error) => {
                return Ok(format!("❌ {}", validation_error));
            }
            Ok(_) => {}
        }

        let new_task = ScheduledTask::new(task_type.clone(), cron_expression);
        
        let mut state_guard = self.state.lock().await;
        state_guard.add_task(new_task);
        let state_path = "scheduler_state.json";
        if let Err(e) = state_guard.save_to_file(state_path) {
            log::error!("保存任务状态失败: {}", e);
        }
        drop(state_guard);

        // 重新启动调度器
        self.restart_scheduler(config, bot).await?;
        
        Ok(format!("✅ 新任务已添加: {} ({})", 
            task_type.get_display_name(), cron_expression))
    }

    pub async fn remove_task_by_index(&self, config: Config, bot: Bot, index: usize) -> Result<String> {
        let mut state_guard = self.state.lock().await;
        match state_guard.remove_task(index) {
            Ok(_) => {
                let state_path = "scheduler_state.json";
                state_guard.save_to_file(state_path)?;
                drop(state_guard);

                // 重新启动调度器
                self.restart_scheduler(config, bot).await?;
                
                Ok("✅ 任务已删除".to_string())
            }
            Err(e) => {
                Ok(format!("❌ 删除任务失败: {}", e))
            }
        }
    }

    pub async fn toggle_task_by_index(&self, config: Config, bot: Bot, index: usize) -> Result<String> {
        let mut state_guard = self.state.lock().await;
        match state_guard.toggle_task(index) {
            Ok(_) => {
                let state_path = "scheduler_state.json";
                state_guard.save_to_file(state_path)?;
                drop(state_guard);

                // 重新启动调度器
                self.restart_scheduler(config, bot).await?;
                
                Ok("✅ 任务状态已切换".to_string())
            }
            Err(e) => {
                Ok(format!("❌ 切换任务状态失败: {}", e))
            }
        }
    }

    pub async fn update_task_by_index(&self, config: Config, bot: Bot, index: usize, new_cron: &str) -> Result<String> {
        let mut state_guard = self.state.lock().await;
        match state_guard.update_task(index, new_cron) {
            Ok(_) => {
                let state_path = "scheduler_state.json";
                state_guard.save_to_file(state_path)?;
                drop(state_guard);

                // 重新启动调度器
                self.restart_scheduler(config, bot).await?;
                
                Ok(format!("✅ 任务 {} 已更新为: {}", index + 1, new_cron))
            }
            Err(e) => {
                Ok(format!("❌ 更新任务失败: {}", e))
            }
        }
    }

    async fn restart_scheduler(&self, config: Config, bot: Bot) -> Result<(), JobSchedulerError> {
        let mut scheduler_guard = self.scheduler.lock().await;
        if let Some(mut sched) = scheduler_guard.take() {
            sched.shutdown().await?;
        }
        
        let new_sched = JobScheduler::new().await?;
        *scheduler_guard = Some(new_sched);
        
        // 重新启动所有任务
        drop(scheduler_guard);
        self.start_all_tasks(config, bot).await?;
        
        Ok(())
    }

    pub async fn get_tasks_summary(&self) -> String {
        let state_guard = self.state.lock().await;
        state_guard.get_all_tasks_summary()
    }
}

// Cron 表达式验证器
pub struct SchedulerValidator;

impl SchedulerValidator {
    pub fn new() -> Self {
        Self
    }

    pub fn validate_cron_expression(&self, cron_expr: &str) -> Result<(), String> {
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
pub static SCHEDULER_MANAGER: Lazy<Arc<Mutex<Option<SchedulerManager>>>> = Lazy::new(|| Arc::new(Mutex::new(None)));

pub async fn start_scheduler(config: Config, bot: Bot) -> Result<(), JobSchedulerError> {
    log::info!("⏰ 开始初始化调度器...");
    
    let manager = SchedulerManager::new(config.clone(), bot.clone()).await?;
    let mut manager_guard = SCHEDULER_MANAGER.lock().await;
    *manager_guard = Some(manager);
    drop(manager_guard);
    
    log::info!("✅ 调度器初始化完成");
    
    // 添加关闭处理器
    if let Some(manager) = &mut *SCHEDULER_MANAGER.lock().await {
        let scheduler = &mut manager.scheduler;
        if let Some(job_scheduler) = &mut *scheduler.lock().await {
            job_scheduler.set_shutdown_handler(Box::new(|| {
                Box::pin(async move {
                    log::info!("🔄 调度器正在关闭...");
                })
            }));
        }
    }
    
    Ok(())
}

pub async fn get_tasks_summary() -> Result<String> {
    let manager_guard = SCHEDULER_MANAGER.lock().await;
    if let Some(manager) = &*manager_guard {
        Ok(manager.get_tasks_summary().await)
    } else {
        Ok("❌ 调度器尚未初始化".to_string())
    }
}

// 向后兼容的函数
pub async fn update_schedule(new_cron: &str) -> Result<String> {
    let manager_guard = SCHEDULER_MANAGER.lock().await;
    if let Some(manager) = &*manager_guard {
        // 使用第一个任务的类型来保持兼容性
        let config = Config::load().unwrap_or_else(|_| Config { bot_token: "".to_string(), chat_id: 0, check_interval: 300 });
        let bot = Bot::new(config.bot_token.clone());
        
        match manager.add_new_task(config, bot, TaskType::SystemMaintenance, new_cron).await {
            Ok(msg) => Ok(msg),
            Err(e) => Ok(format!("❌ 更新调度失败: {}", e))
        }
    } else {
        Ok("❌ 调度器尚未初始化".to_string())
    }
}