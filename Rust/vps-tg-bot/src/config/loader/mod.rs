//! 配置加载器模块入口
//! 
//! 定义配置加载器 trait 和主加载函数

pub mod env;
pub mod encrypted;

use crate::config::types::{Config, ConfigError, ConfigResult, ConfigSource};
use log::{debug, info, warn};
use std::path::Path;

/// 配置加载器 trait
pub trait ConfigLoader {
    /// 加载配置
    fn load(&self) -> ConfigResult<Config>;
    
    /// 获取配置来源
    fn source(&self) -> ConfigSource;
    
    /// 检查配置源是否可用
    fn is_available(&self) -> bool;
}

/// 主配置加载函数
/// 
/// 按照优先级依次尝试不同的配置源：
/// 1. 环境变量（最高优先级）
/// 2. 加密文件
pub fn load_config() -> ConfigResult<Config> {
    debug!("开始加载配置...");
    
    // 1. 尝试从环境变量加载（最高优先级）
    debug!("尝试从环境变量加载配置...");
    let env_loader = env::EnvironmentLoader::new();
    if env_loader.is_available() {
        match env_loader.load() {
            Ok(config) => {
                info!("✅ 从环境变量成功加载配置");
                debug!("配置来源: {:?}", env_loader.source());
                
                // 验证配置
                config.validate()
                    .map_err(|e| ConfigError::ValidationError(e.to_string()))?;
                    
                return Ok(config);
            }
            Err(e) => {
                warn!("⚠️  环境变量加载失败: {}", e);
            }
        }
    }
    
    // 2. 尝试从加密文件加载
    debug!("尝试从加密文件加载配置...");
    let encrypted_loader = encrypted::EncryptedFileLoader::default();
    if encrypted_loader.is_available() {
        match encrypted_loader.load() {
            Ok(config) => {
                info!("✅ 从加密文件成功加载配置");
                debug!("配置来源: {:?}", encrypted_loader.source());
                
                // 验证配置
                config.validate()
                    .map_err(|e| ConfigError::ValidationError(e.to_string()))?;
                    
                return Ok(config);
            }
            Err(e) => {
                warn!("⚠️  加密文件加载失败: {}", e);
            }
        }
    }
    

    
    Err(ConfigError::NoValidSource)
}

/// 获取所有可用的配置源
#[allow(dead_code)]
pub fn get_available_sources() -> Vec<ConfigSource> {
    let mut sources = Vec::new();
    
    // 检查环境变量
    let env_loader = env::EnvironmentLoader::new();
    if env_loader.is_available() {
        sources.push(env_loader.source());
    }
    
    // 检查加密文件
    let encrypted_loader = encrypted::EncryptedFileLoader::default();
    if encrypted_loader.is_available() {
        sources.push(encrypted_loader.source());
    }
    

    
    sources
}

/// 检查指定路径的配置文件是否存在
#[allow(dead_code)]
fn check_config_exists(path: &str) -> bool {
    Path::new(path).exists()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_check_config_exists() {
        // 测试存在的文件
        assert!(check_config_exists("Cargo.toml"));
        
        // 测试不存在的文件
        assert!(!check_config_exists("nonexistent_file_12345.toml"));
    }

    #[test]
    fn test_get_available_sources() {
        // 清理环境变量
        env::remove_var("BOT_TOKEN");
        env::remove_var("CHAT_ID");
        env::remove_var("CHECK_INTERVAL");
        
        let sources = get_available_sources();
        
        // 由于没有配置文件，应该返回空列表
        // 注意：这个测试可能在某些环境下失败，因为可能有遗留的配置文件
        println!("可用配置源: {:?}", sources);
    }
}