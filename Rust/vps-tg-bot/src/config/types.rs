//! 配置类型定义
//! 
//! 定义配置相关的类型和结构体

use serde::{Deserialize, Serialize};

/// 配置结构体
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

/// 配置来源枚举
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub enum ConfigSource {
    /// 环境变量配置
    Environment,
    /// systemd 凭证文件配置
    CredentialFile,
}

/// 配置加载错误
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("环境变量加载失败: {0}")]
    EnvironmentError(String),
    
    #[error("配置验证失败: {0}")]
    ValidationError(String),
    
    #[error("未找到有效的配置源")]
    NoValidSource,
}

/// 配置加载结果类型
pub type ConfigResult<T> = std::result::Result<T, ConfigError>;

impl Config {
    /// 验证配置的有效性
    pub fn validate(&self) -> ConfigResult<()> {
        // 验证 bot_token 格式
        if self.bot_token.is_empty() {
            return Err(ConfigError::ValidationError(
                "Bot token 不能为空".to_string()
            ));
        }
        
        // 简单的 token 格式验证（应该以数字开头）
        if !self.bot_token.chars().next().unwrap_or(' ').is_ascii_digit() {
            return Err(ConfigError::ValidationError(
                "Bot token 格式无效".to_string()
            ));
        }
        
        // 验证 chat_id
        if self.chat_id <= 0 {
            return Err(ConfigError::ValidationError(
                "Chat ID 必须为正整数".to_string()
            ));
        }
        
        // 验证 check_interval
        if self.check_interval < 60 {
            return Err(ConfigError::ValidationError(
                "检查间隔不能小于60秒".to_string()
            ));
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_validation_success() {
        let config = Config {
            bot_token: "123456789:ABCdefGHIjklMNOpqrsTUVwxyz".to_string(),
            chat_id: 123456789,
            check_interval: 300,
        };
        
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validation_empty_token() {
        let config = Config {
            bot_token: "".to_string(),
            chat_id: 123456789,
            check_interval: 300,
        };
        
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_invalid_chat_id() {
        let config = Config {
            bot_token: "123456789:ABCdefGHIjklMNOpqrsTUVwxyz".to_string(),
            chat_id: 0,
            check_interval: 300,
        };
        
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_invalid_interval() {
        let config = Config {
            bot_token: "123456789:ABCdefGHIjklMNOpqrsTUVwxyz".to_string(),
            chat_id: 123456789,
            check_interval: 30,
        };
        
        assert!(config.validate().is_err());
    }
}