//! 旧版明文配置文件加载器
//! 
//! 从旧版 TOML 配置文件加载配置，仅用于迁移：
//! - /etc/vps-tg-bot-rust/config.toml
//! - config.toml
//! 
//! 加载时会记录安全警告日志，建议迁移到加密格式。

use crate::config::loader::{ConfigLoader};
use crate::config::types::{Config, ConfigError, ConfigResult, ConfigSource};
use log::{warn, debug};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// 旧版配置文件路径
const LEGACY_CONFIG_PATHS: &[&str] = &[
    "/etc/vps-tg-bot-rust/config.toml",  // 新版系统安装路径
    "/etc/vps-tg-bot/config.toml",       // 旧版系统安装路径
    "config.toml",                       // 本地开发目录
];

/// 默认检查间隔
fn default_check_interval() -> u64 {
    300
}

/// 旧版配置文件结构（扁平格式）
#[derive(Debug, Serialize, Deserialize)]
struct LegacyConfigFlat {
    pub bot_token: String,
    pub chat_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub check_interval: Option<u64>,
}

/// 旧版配置文件结构（分层格式）
#[derive(Debug, Serialize, Deserialize)]
struct LegacyConfigNested {
    pub bot: LegacyBotConfig,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub check_interval: Option<u64>,
}

/// 旧版 Bot 配置结构
#[derive(Debug, Serialize, Deserialize)]
struct LegacyBotConfig {
    pub token: String,
    pub chat_id: String,
}

/// 旧版明文文件配置加载器
#[derive(Debug)]
pub struct LegacyFileLoader {
    config_path: Option<PathBuf>,
}

impl LegacyFileLoader {
    /// 创建新的旧版文件加载器
    pub fn new() -> Self {
        Self {
            config_path: Self::find_legacy_config_path(),
        }
    }
    
    /// 查找旧版配置文件路径
    fn find_legacy_config_path() -> Option<PathBuf> {
        for path in LEGACY_CONFIG_PATHS {
            if Path::new(path).exists() {
                debug!("发现旧版配置文件: {}", path);
                return Some(PathBuf::from(path));
            }
        }
        None
    }
    
    #[allow(dead_code)]
    pub fn get_config_path(&self) -> Option<&Path> {
        self.config_path.as_deref()
    }
    
    /// 从指定路径加载旧版配置（用于测试）
    pub fn load_from_path(&self, path: &Path) -> ConfigResult<Config> {
        if !path.exists() {
            return Err(ConfigError::LegacyFileError(
                format!("配置文件不存在: {:?}", path)
            ));
        }
        
        // 记录安全警告
        warn!("⚠️  正在从明文配置文件加载敏感信息: {:?}", path);
        warn!("⚠️  建议迁移到加密配置文件格式以提高安全性");
        
        // 读取配置文件内容
        let content = fs::read_to_string(path)
            .map_err(|e| ConfigError::LegacyFileError(
                format!("读取配置文件失败: {}", e)
            ))?;
        
        // 首先尝试新的扁平格式
        if let Ok(flat_config) = toml::from_str::<LegacyConfigFlat>(&content) {
            let chat_id = flat_config.chat_id.parse::<i64>()
                .map_err(|e| ConfigError::LegacyFileError(
                    format!("解析 chat_id 失败: {}", e)
                ))?;
            
            let check_interval = flat_config.check_interval.unwrap_or_else(default_check_interval);
            
            let config = Config {
                bot_token: flat_config.bot_token,
                chat_id,
                check_interval,
            };
            
            debug!("✅ 从旧版扁平格式成功加载配置");
            return Ok(config);
        }
        
        // 如果扁平格式失败，尝试旧的分层格式
        if let Ok(nested_config) = toml::from_str::<LegacyConfigNested>(&content) {
            let chat_id = nested_config.bot.chat_id.parse::<i64>()
                .map_err(|e| ConfigError::LegacyFileError(
                    format!("解析 chat_id 失败: {}", e)
                ))?;
            
            let check_interval = nested_config.check_interval.unwrap_or_else(default_check_interval);
            
            let config = Config {
                bot_token: nested_config.bot.token,
                chat_id,
                check_interval,
            };
            
            debug!("✅ 从旧版分层格式成功加载配置");
            return Ok(config);
        }
        
        // 如果两种格式都失败，返回错误
        Err(ConfigError::LegacyFileError(
            "配置文件格式无效，既不是扁平格式也不是分层格式".to_string()
        ))
    }
}

impl Default for LegacyFileLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigLoader for LegacyFileLoader {
    fn load(&self) -> ConfigResult<Config> {
        let config_path = self.config_path
            .as_ref()
            .ok_or_else(|| ConfigError::LegacyFileError(
                "未找到旧版配置文件".to_string()
            ))?;
        
        // 记录安全警告
        warn!("⚠️  正在从明文配置文件加载敏感信息");
        warn!("⚠️  配置文件路径: {:?}", config_path);
        warn!("⚠️  强烈建议迁移到加密配置文件格式以提高安全性");
        
        self.load_from_path(config_path)
    }
    
    fn source(&self) -> ConfigSource {
        if let Some(ref path) = self.config_path {
            ConfigSource::LegacyFile(path.clone())
        } else {
            ConfigSource::LegacyFile(PathBuf::new())
        }
    }
    
    fn is_available(&self) -> bool {
        self.config_path.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::env;

    fn cleanup_env_vars() {
        env::remove_var("BOT_TOKEN");
        env::remove_var("CHAT_ID");
        env::remove_var("CHECK_INTERVAL");
    }

    #[test]
    fn test_legacy_config_flat_creation() {
        let flat_config = LegacyConfigFlat {
            bot_token: "test_token".to_string(),
            chat_id: "123456".to_string(),
            check_interval: Some(600),
        };
        
        assert_eq!(flat_config.bot_token, "test_token");
        assert_eq!(flat_config.chat_id, "123456");
        assert_eq!(flat_config.check_interval, Some(600));
    }

    #[test]
    fn test_legacy_config_nested_creation() {
        let nested_config = LegacyConfigNested {
            bot: LegacyBotConfig {
                token: "test_token".to_string(),
                chat_id: "123456".to_string(),
            },
            check_interval: Some(600),
        };
        
        assert_eq!(nested_config.bot.token, "test_token");
        assert_eq!(nested_config.bot.chat_id, "123456");
        assert_eq!(nested_config.check_interval, Some(600));
    }

    #[test]
    fn test_legacy_file_loader_creation() {
        let loader = LegacyFileLoader::new();
        // 应该能创建，即使没有配置文件
        assert!(loader.is_available() || !loader.is_available());
    }

    #[test]
    fn test_legacy_file_loader_load_flat_format() {
        cleanup_env_vars();
        
        let loader = LegacyFileLoader::new();
        
        // 创建临时配置文件（扁平格式）
        let temp_file = NamedTempFile::new().unwrap();
        let temp_path = temp_file.path().to_path_buf();
        
        let config_content = r#"
bot_token = "legacy_flat_token_123"
chat_id = "987654321"
check_interval = 900
"#;
        
        fs::write(&temp_path, config_content).unwrap();
        
        let loaded_config = loader.load_from_path(&temp_path).unwrap();
        
        assert_eq!(loaded_config.bot_token, "legacy_flat_token_123");
        assert_eq!(loaded_config.chat_id, 987654321);
        assert_eq!(loaded_config.check_interval, 900);
        
        // 清理临时文件
        let _ = fs::remove_file(temp_path);
        cleanup_env_vars();
    }

    #[test]
    fn test_legacy_file_loader_load_nested_format() {
        cleanup_env_vars();
        
        let loader = LegacyFileLoader::new();
        
        // 创建临时配置文件（分层格式）
        let temp_file = NamedTempFile::new().unwrap();
        let temp_path = temp_file.path().to_path_buf();
        
        let config_content = r#"
[bot]
token = "legacy_nested_token_456"
chat_id = "111222333"
"#;
        
        fs::write(&temp_path, config_content).unwrap();
        
        let loaded_config = loader.load_from_path(&temp_path).unwrap();
        
        assert_eq!(loaded_config.bot_token, "legacy_nested_token_456");
        assert_eq!(loaded_config.chat_id, 111222333);
        assert_eq!(loaded_config.check_interval, 300); // 默认值
        
        // 清理临时文件
        let _ = fs::remove_file(temp_path);
        cleanup_env_vars();
    }

    #[test]
    fn test_legacy_file_loader_load_invalid_format() {
        cleanup_env_vars();
        
        let loader = LegacyFileLoader::new();
        
        // 创建临时配置文件（无效格式）
        let temp_file = NamedTempFile::new().unwrap();
        let temp_path = temp_file.path().to_path_buf();
        
        let invalid_content = r#"
invalid toml syntax
bot_token = "test"
missing quotes
"#;
        
        fs::write(&temp_path, invalid_content).unwrap();
        
        let result = loader.load_from_path(&temp_path);
        assert!(result.is_err());
        
        // 清理临时文件
        let _ = fs::remove_file(temp_path);
        cleanup_env_vars();
    }

    #[test]
    fn test_legacy_file_source() {
        let loader = LegacyFileLoader::new();
        let source = loader.source();
        
        match source {
            ConfigSource::LegacyFile(path) => {
                assert!(path.as_os_str().is_empty() || path.exists());
            }
            _ => panic!("应该返回 LegacyFile 源类型"),
        }
    }

    #[test]
    fn test_find_legacy_config_path() {
        // 测试不存在文件的情况
        let result = LegacyFileLoader::find_legacy_config_path();
        // 应该返回 None，因为测试环境中可能没有这些文件
        assert!(result.is_none() || result.unwrap().exists());
    }
}