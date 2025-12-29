use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;

// 移除加密相关模块导入
pub mod loader;
pub mod types;

// 使用新的类型定义
use crate::config::types::Config as NewConfig;
use crate::config::types::{ConfigError, ConfigResult};
use crate::config::loader::{load_config, get_available_sources};

// 保留旧的结构体定义以确保向后兼容
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub bot_token: String,
    pub chat_id: i64,
    #[serde(default = "default_check_interval")]
    pub check_interval: u64,
}

fn default_check_interval() -> u64 {
    300
}

impl Config {
    /// 主配置加载函数 - 现在只使用环境变量
    pub fn load() -> Result<Self> {
        match load_config() {
            Ok(new_config) => {
                // 转换为旧的结构体格式
                Ok(Config {
                    bot_token: new_config.bot_token,
                    chat_id: new_config.chat_id,
                    check_interval: new_config.check_interval,
                })
            }
            Err(e) => {
                // 将 ConfigError 转换为 anyhow::Error
                Err(anyhow::anyhow!("配置加载失败: {}", e))
            }
        }
    }
    
    #[allow(dead_code)]
    pub fn get_available_sources() -> Vec<String> {
        get_available_sources()
            .into_iter()
            .map(|source| match source {
                crate::config::types::ConfigSource::Environment => 
                    "环境变量".to_string(),
                crate::config::types::ConfigSource::CredentialFile => 
                    "systemd 凭证文件".to_string(),
            })
            .collect()
    }
    
    #[allow(dead_code)]
    pub fn save(&self, path: &str) -> Result<()> {
        let content = toml::to_string(self)
            .context("Failed to serialize config")?;
        fs::write(path, content)
            .context(format!("Failed to write config to: {}", path))?;
        Ok(())
    }
    
    #[allow(dead_code)]
    pub fn validate(&self) -> ConfigResult<()> {
        let new_config = NewConfig {
            bot_token: self.bot_token.clone(),
            chat_id: self.chat_id,
            check_interval: self.check_interval,
        };
        
        new_config.validate()
            .map_err(|e| ConfigError::ValidationError(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    // 辅助函数：清理环境变量
    fn cleanup_env_vars() {
        unsafe {
            std::env::remove_var("BOT_TOKEN");
            std::env::remove_var("CHAT_ID");
            std::env::remove_var("CHECK_INTERVAL");
        }
    }

    #[test]
    fn test_config_save_and_load() {
        let config = Config {
            bot_token: "test_save_token".to_string(),
            chat_id: 555666777,
            check_interval: 1200,
        };

        let temp_path = "test_config_save.toml";
        
        // 保存配置
        config.save(temp_path).unwrap();
        
        // 读取并验证
        let content = fs::read_to_string(temp_path).unwrap();
        assert!(content.contains("test_save_token"));
        assert!(content.contains("555666777"));
        assert!(content.contains("1200"));

        // 清理
        let _ = fs::remove_file(temp_path);
    }

    #[test]
    fn test_config_get_available_sources() {
        cleanup_env_vars();
        
        // 获取可用配置源
        let sources = Config::get_available_sources();
        println!("可用配置源: {:?}", sources);
        
        cleanup_env_vars();
    }

    #[test]
    fn test_config_validation() {
        let valid_config = Config {
            bot_token: "123456789:ABCdefGHIjklMNOpqrsTUVwxyz".to_string(),
            chat_id: 123456789,
            check_interval: 300,
        };
        
        assert!(valid_config.validate().is_ok());
        
        let invalid_config = Config {
            bot_token: "".to_string(),
            chat_id: 0,
            check_interval: 30,
        };
        
        assert!(invalid_config.validate().is_err());
    }
}