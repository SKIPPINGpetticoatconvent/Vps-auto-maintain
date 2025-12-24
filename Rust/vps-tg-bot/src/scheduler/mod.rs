use tokio_cron_scheduler::{JobScheduler, Job};
use teloxide::Bot;
use teloxide::types::ChatId;
use teloxide::prelude::Requester;
use crate::config::Config;
use crate::system::ops;
use anyhow::Result;

pub async fn start_scheduler(config: Config, bot: Bot) -> Result<()> {
    let sched = JobScheduler::new().await?;

    let job = Job::new_async("0 0 4 * * Sun", move |_uuid, _l| {
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

    Ok(())
}