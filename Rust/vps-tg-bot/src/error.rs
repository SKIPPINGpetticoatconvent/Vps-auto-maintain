use std::time::Duration;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BotError {
    #[error("Configuration error")]
    Config(#[from] ConfigError),
    
    #[error("System operation failed")]
    System(#[from] SystemError),
    
    #[error("Telegram API error")]
    Telegram(#[from] teloxide::RequestError),
    
    #[error("Scheduler error")]
    Scheduler(#[from] SchedulerError),
    
    #[error("Operation timeout")]
    Timeout(Duration),
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Missing required environment variable")]
    MissingEnv(String),
    #[error("Invalid configuration value")]
    InvalidValue(String, String),
}

#[derive(Debug, Error)]
pub enum SystemError {
    #[error("IO operation failed")]
    IoError,
    #[error("Execution failed")]
    ExecutionFailed(String),
    #[error("Script execution failed")]
    ScriptFailed(i32),
    #[error("Timeout executing script")]
    Timeout(String),
}

#[derive(Debug, Error)]
pub enum SchedulerError {
    #[error("Job not found")]
    JobNotFound,
    #[error("Invalid cron expression")]
    InvalidCron(String),
    #[error("IO operation failed")]
    IoError(#[from] std::io::Error),
    #[error("Data serialization failed")]
    SerializationError(#[from] serde_json::Error),
}
