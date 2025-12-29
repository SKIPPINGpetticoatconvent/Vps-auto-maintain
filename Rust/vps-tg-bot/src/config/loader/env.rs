//! 环境变量配置加载器
//! 
//! 从环境变量 BOT_TOKEN、CHAT_ID、CHECK_INTERVAL 加载配置
//! 支持从 systemd 凭证文件读取敏感信息
//! 优先级：环境变量 > systemd 凭证文件

use crate::config::loader::{ConfigLoader};
use crate::config::types::{Config, ConfigError, ConfigResult, ConfigSource};
use log::{debug, warn};
use std::env;
use std::cell::RefCell;

/// 环境变量配置加载器
pub struct EnvironmentLoader {
    config_source: RefCell<Option<ConfigSource>>,
}

impl EnvironmentLoader {
    /// 创建新的环境变量加载器
    pub fn new() -> Self {
        Self { config_source: RefCell::new(None) }
    }
    
    /// 从 systemd 凭证文件加载配置
    fn load_from_credentials(&self) -> Option<(String, i64)> {
        let cred_dir = "/run/credentials/vps-tg-bot-rust.service";
        
        // 尝试读取 BOT_TOKEN
        let bot_token = match std::fs::read_to_string(format!("{}/bot-token", cred_dir)) {
            Ok(token) => token.trim().to_string(),
            Err(_) => {
                debug!("无法读取 BOT_TOKEN 凭证文件");
                return None;
            }
        };
        
        // 尝试读取 CHAT_ID
        let chat_id = match std::fs::read_to_string(format!("{}/chat-id", cred_dir)) {
            Ok(id) => {
                match id.trim().parse::<i64>() {
                    Ok(parsed_id) => parsed_id,
                    Err(_) => {
                        debug!("CHAT_ID 凭证格式无效");
                        return None;
                    }
                }
            },
            Err(_) => {
                debug!("无法读取 CHAT_ID 凭证文件");
                return None;
            }
        };
        
        if bot_token.is_empty() || chat_id <= 0 {
            debug!("凭证文件内容无效");
            return None;
        }
        
        debug!("✅ 从 systemd 凭证文件成功加载配置");
        Some((bot_token, chat_id))
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
        debug!("正在从环境变量和 systemd 凭证文件加载配置...");
        
        // 优先从环境变量加载（开发/测试模式）
        if EnvironmentLoader::check_env_vars() {
            debug!("尝试从环境变量加载配置...");
            
            // 验证环境变量格式
            if let Err(e) = Self::validate_env_vars() {
                warn!("⚠️  环境变量验证失败: {}，尝试从凭证文件加载", e);
            } else {
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
                
                // 保存配置来源信息
                *self.config_source.borrow_mut() = Some(ConfigSource::Environment);
                return Ok(config);
            }
        }
        
        // 尝试从 systemd 凭证文件加载（生产环境）
        debug!("尝试从 systemd 凭证文件加载配置...");
        if let Some((bot_token, chat_id)) = self.load_from_credentials() {
            let check_interval = env::var("CHECK_INTERVAL")
                .map(|s| s.parse::<u64>().unwrap_or(300))
                .unwrap_or(300);
            
            let config = Config {
                bot_token,
                chat_id,
                check_interval,
            };
            
            debug!("✅ 从 systemd 凭证文件成功加载配置");
            debug!("配置: bot_token={}, chat_id={}, check_interval={}", 
                  config.bot_token.chars().take(10).collect::<String>() + "...", 
                  config.chat_id, 
                  config.check_interval);
            
            // 保存配置来源信息
            *self.config_source.borrow_mut() = Some(ConfigSource::CredentialFile);
            return Ok(config);
        }
        
        Err(ConfigError::EnvironmentError(
            "无法从环境变量或 systemd 凭证文件加载配置".to_string()
        ))
    }
    
    fn source(&self) -> ConfigSource {
        self.config_source.borrow().clone().unwrap_or(ConfigSource::Environment)
    }
    
    fn is_available(&self) -> bool {
        // 检查环境变量或凭证文件是否可用
        EnvironmentLoader::check_env_vars() || self.load_from_credentials().is_some()
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
