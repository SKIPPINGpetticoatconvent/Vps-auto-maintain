//! 配置加载器模块入口
//! 
//! 支持环境变量和 systemd 凭证文件配置加载
//! 优先级：环境变量 > systemd 凭证文件

pub mod env;

use crate::config::types::{Config, ConfigError, ConfigResult, ConfigSource};
use log::{debug, info, warn};

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
/// 优先从环境变量加载，失败后尝试从 systemd 凭证文件加载
pub fn load_config() -> ConfigResult<Config> {
    debug!("开始加载配置...");
    
    // 使用统一的配置加载器，它会自动处理优先级
    debug!("使用配置加载器尝试加载配置...");
    let env_loader = env::EnvironmentLoader::new();
    if env_loader.is_available() {
        match env_loader.load() {
            Ok(config) => {
                let source_name = match env_loader.source() {
                    ConfigSource::Environment => "环境变量",
                    ConfigSource::CredentialFile => "systemd 凭证文件"
                };
                info!("✅ 从 {} 成功加载配置", source_name);
                debug!("配置来源: {:?}", env_loader.source());
                
                // 验证配置
                config.validate()
                    .map_err(|e| ConfigError::ValidationError(e.to_string()))?;
                    
                return Ok(config);
            }
            Err(e) => {
                warn!("⚠️  配置加载失败: {}", e);
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
    
    sources
}

/// 检查指定路径的配置文件是否存在
#[allow(dead_code)]
fn check_config_exists(path: &str) -> bool {
    std::path::Path::new(path).exists()
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
        
        println!("可用配置源: {:?}", sources);
    }
}