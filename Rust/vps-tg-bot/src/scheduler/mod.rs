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
        // 验证 Cron 表达式（简单验证）
        if new_cron.split_whitespace().count() != 5 {
            return Ok("❌ 无效的 Cron 表达式。应为 5 个字段（分钟 小时 日 月 周几）".to_string());
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