//! 环境变量配置加载器
//! 
//! 从环境变量 BOT_TOKEN、CHAT_ID、CHECK_INTERVAL 加载配置
//! 环境变量具有最高优先级

use crate::config::loader::{ConfigLoader};
use crate::config::types::{Config, ConfigError, ConfigResult, ConfigSource};
use log::{debug};
use std::env;

/// 环境变量配置加载器
pub struct EnvironmentLoader {
    _private: (),
}

impl EnvironmentLoader {
    /// 创建新的环境变量加载器
    pub fn new() -> Self {
        Self { _private: () }
    }
    
    /// 检查环境变量是否设置且有效
    fn check_env_vars() -> bool {
        // 检查必需的变量
        let has_token = env::var("BOT_TOKEN").is_ok_and(|s| !s.is_empty());
        let has_chat_id = env::var("CHAT_ID").is_ok_and(|s| !s.is_empty());
        
        has_token && has_chat_id
    }
    
    /// 验证环境变量值的格式
    fn validate_env_vars() -> ConfigResult<()> {
        // 验证 BOT_TOKEN 格式
        let bot_token = env::var("BOT_TOKEN")
            .map_err(|e| ConfigError::EnvironmentError(format!("无法读取 BOT_TOKEN: {}", e)))?;
            
        if bot_token.is_empty() {
            return Err(ConfigError::EnvironmentError(
                "BOT_TOKEN 不能为空".to_string()
            ));
        }
        
        // 简单的 token 格式验证（应该以数字开头）
        if !bot_token.chars().next().unwrap_or(' ').is_ascii_digit() {
            return Err(ConfigError::EnvironmentError(
                "BOT_TOKEN 格式无效".to_string()
            ));
        }
        
        // 验证 CHAT_ID 格式
        let chat_id_str = env::var("CHAT_ID")
            .map_err(|e| ConfigError::EnvironmentError(format!("无法读取 CHAT_ID: {}", e)))?;
            
        let chat_id: i64 = chat_id_str.parse()
            .map_err(|e| ConfigError::EnvironmentError(format!("CHAT_ID 格式无效: {}", e)))?;
            
        if chat_id <= 0 {
            return Err(ConfigError::EnvironmentError(
                "CHAT_ID 必须为正整数".to_string()
            ));
        }
        
        // 验证 CHECK_INTERVAL（可选）
        if let Ok(interval_str) = env::var("CHECK_INTERVAL") {
            if !interval_str.is_empty() {
                let check_interval: u64 = interval_str.parse()
                    .map_err(|e| ConfigError::EnvironmentError(format!("CHECK_INTERVAL 格式无效: {}", e)))?;
                    
                if check_interval < 60 {
                    return Err(ConfigError::EnvironmentError(
                        "CHECK_INTERVAL 不能小于60秒".to_string()
                    ));
                }
            }
        }
        
        Ok(())
    }
}

impl Default for EnvironmentLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigLoader for EnvironmentLoader {
    fn load(&self) -> ConfigResult<Config> {
        debug!("正在从环境变量加载配置...");
        
        // 检查环境变量是否存在
        if !self.is_available() {
            return Err(ConfigError::EnvironmentError(
                "环境变量未设置或无效".to_string()
            ));
        }
        
        // 验证环境变量格式
        Self::validate_env_vars()?;
        
        // 加载配置
        let bot_token = env::var("BOT_TOKEN")
            .map_err(|e| ConfigError::EnvironmentError(format!("读取 BOT_TOKEN 失败: {}", e)))?;
            
        let chat_id: i64 = env::var("CHAT_ID")
            .map_err(|e| ConfigError::EnvironmentError(format!("读取 CHAT_ID 失败: {}", e)))?
            .parse()
            .map_err(|e| ConfigError::EnvironmentError(format!("解析 CHAT_ID 失败: {}", e)))?;
            
        let check_interval = env::var("CHECK_INTERVAL")
            .map(|s| s.parse::<u64>().unwrap_or(300))
            .unwrap_or(300);
        
        let config = Config {
            bot_token,
            chat_id,
            check_interval,
        };
        
        debug!("✅ 从环境变量成功加载配置");
        debug!("配置: bot_token={}, chat_id={}, check_interval={}", 
              config.bot_token.chars().take(10).collect::<String>() + "...", 
              config.chat_id, 
              config.check_interval);
        
        Ok(config)
    }
    
    fn source(&self) -> ConfigSource {
        ConfigSource::Environment
    }
    
    fn is_available(&self) -> bool {
        Self::check_env_vars()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn setup_test_env() {
        env::set_var("BOT_TOKEN", "123456789:ABCdefGHIjklMNOpqrsTUVwxyz");
        env::set_var("CHAT_ID", "123456789");
        env::set_var("CHECK_INTERVAL", "600");
    }

    fn cleanup_test_env() {
        env::remove_var("BOT_TOKEN");
        env::remove_var("CHAT_ID");
        env::remove_var("CHECK_INTERVAL");
    }

    #[test]
    fn test_environment_loader_creation() {
        let loader = EnvironmentLoader::new();
        assert!(loader.is_available() || !loader.is_available()); // 应该能创建
    }

    #[test]
    fn test_environment_loader_available_with_valid_env() {
        setup_test_env();
        
        let loader = EnvironmentLoader::new();
        assert!(loader.is_available());
        
        cleanup_test_env();
    }

    #[test]
    fn test_environment_loader_not_available_without_env() {
        cleanup_test_env();
        
        let loader = EnvironmentLoader::new();
        assert!(!loader.is_available());
    }

    #[test]
    fn test_environment_loader_load_success() {
        setup_test_env();
        
        let loader = EnvironmentLoader::new();
        let result = loader.load();
        
        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.bot_token, "123456789:ABCdefGHIjklMNOpqrsTUVwxyz");
        assert_eq!(config.chat_id, 123456789);
        assert_eq!(config.check_interval, 600);
        
        cleanup_test_env();
    }

    #[test]
    fn test_environment_loader_load_invalid_token() {
        env::set_var("BOT_TOKEN", "invalid_token");
        env::set_var("CHAT_ID", "123456789");
        env::remove_var("CHECK_INTERVAL");
        
        let loader = EnvironmentLoader::new();
        let result = loader.load();
        
        assert!(result.is_err());
        
        cleanup_test_env();
    }

    #[test]
    fn test_environment_loader_load_invalid_chat_id() {
        env::set_var("BOT_TOKEN", "123456789:ABCdefGHIjklMNOpqrsTUVwxyz");
        env::set_var("CHAT_ID", "invalid_chat_id");
        env::remove_var("CHECK_INTERVAL");
        
        let loader = EnvironmentLoader::new();
        let result = loader.load();
        
        assert!(result.is_err());
        
        cleanup_test_env();
    }

    #[test]
    fn test_environment_loader_source() {
        let loader = EnvironmentLoader::new();
        assert_eq!(loader.source(), ConfigSource::Environment);
    }

    #[test]
    fn test_check_env_vars() {
        // 测试没有环境变量的情况
        cleanup_test_env();
        assert!(!EnvironmentLoader::check_env_vars());
        
        // 测试部分环境变量设置
        env::set_var("BOT_TOKEN", "test_token");
        assert!(!EnvironmentLoader::check_env_vars());
        
        env::set_var("CHAT_ID", "123456");
        assert!(EnvironmentLoader::check_env_vars());
        
        cleanup_test_env();
    }
}
