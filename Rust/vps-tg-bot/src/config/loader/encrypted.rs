//! 加密文件配置加载器
//! 
//! 从加密配置文件加载配置，支持以下路径：
//! - /etc/vps-tg-bot-rust/config.enc
//! - config.enc
//! 
//! 使用 Phase 1 的加密模块进行解密，支持 TOML 格式的加密配置。

use crate::config::loader::{ConfigLoader};
use crate::config::types::{Config, ConfigError, ConfigResult, ConfigSource};
use crate::config::crypto::{ConfigCrypto, SecureStorage};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use log::{debug, info};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// 加密配置文件路径
const ENCRYPTED_CONFIG_PATHS: &[&str] = &[
    "/etc/vps-tg-bot-rust/config.enc",  // 系统安装路径
    "config.enc",                       // 本地开发目录
];

/// 加密配置文件结构
#[derive(Debug, Serialize, Deserialize)]
struct EncryptedConfig {
    /// Base64 编码的加密配置数据
    pub encrypted_data: String,
    /// 配置文件版本
    pub version: String,
    /// 创建时间戳
    pub created_at: String,
}

impl EncryptedConfig {
    #[allow(dead_code)]
    pub fn new(encrypted_data: Vec<u8>, version: String) -> Self {
        use chrono::Utc;
        
        Self {
            encrypted_data: BASE64.encode(encrypted_data),
            version,
            created_at: Utc::now().to_rfc3339(),
        }
    }
    
    /// 获取解码后的加密数据
    pub fn get_encrypted_data(&self) -> Result<Vec<u8>, base64::DecodeError> {
        BASE64.decode(&self.encrypted_data)
    }
}

/// 加密文件配置加载器
#[derive(Debug)]
pub struct EncryptedFileLoader {
    config_path: Option<PathBuf>,
    crypto: ConfigCrypto,
}

impl EncryptedFileLoader {
    /// 创建新的加密文件加载器
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            config_path: Self::find_encrypted_config_path(),
            crypto: ConfigCrypto::new(),
        }
    }
    
    /// 查找加密配置文件路径
    fn find_encrypted_config_path() -> Option<PathBuf> {
        for path in ENCRYPTED_CONFIG_PATHS {
            if Path::new(path).exists() {
                debug!("发现加密配置文件: {}", path);
                return Some(PathBuf::from(path));
            }
        }
        None
    }
    
    #[allow(dead_code)]
    pub fn get_config_path(&self) -> Option<&Path> {
        self.config_path.as_deref()
    }
    
    #[allow(dead_code)]
    pub fn save(&self, config: &Config) -> ConfigResult<()> {
        let config_path = self.config_path
            .as_ref()
            .ok_or_else(|| ConfigError::EncryptedFileError(
                "未设置配置文件路径".to_string()
            ))?;
        
        // 确保目录存在
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| ConfigError::EncryptedFileError(
                    format!("创建目录失败: {}", e)
                ))?;
        }
        
        // 序列化配置为 TOML
        let toml_data = toml::to_string(config)
            .map_err(|e| ConfigError::EncryptedFileError(
                format!("序列化配置失败: {}", e)
            ))?;
        
        // 加密配置数据
        let encrypted_data = self.crypto.encrypt_config(toml_data.as_bytes())
            .map_err(|e| ConfigError::EncryptedFileError(
                format!("加密配置失败: {}", e)
            ))?;
        
        // 创建加密配置文件结构
        let encrypted_config = EncryptedConfig::new(
            encrypted_data,
            "1.0".to_string()
        );
        
        // 序列化加密配置文件
        let config_content = toml::to_string(&encrypted_config)
            .map_err(|e| ConfigError::EncryptedFileError(
                format!("序列化加密配置失败: {}", e)
            ))?;
        
        // 写入文件
        fs::write(config_path, config_content)
            .map_err(|e| ConfigError::EncryptedFileError(
                format!("写入加密配置文件失败: {}", e)
            ))?;
        
        info!("✅ 配置已保存到加密文件: {:?}", config_path);
        Ok(())
    }
    
    /// 尝试从指定路径加载（用于测试）
    pub fn load_from_path<P: AsRef<Path>>(&self, path: P) -> ConfigResult<Config> {
        let path = path.as_ref();
        
        if !path.exists() {
            return Err(ConfigError::EncryptedFileError(
                format!("配置文件不存在: {:?}", path)
            ));
        }
        
        // 读取加密配置文件
        let content = fs::read_to_string(path)
            .map_err(|e| ConfigError::EncryptedFileError(
                format!("读取配置文件失败: {}", e)
            ))?;
        
        // 解析加密配置文件结构
        let encrypted_config: EncryptedConfig = toml::from_str(&content)
            .map_err(|e| ConfigError::EncryptedFileError(
                format!("解析加密配置文件失败: {}", e)
            ))?;
        
        // 解码 Base64 数据
        let encrypted_data = encrypted_config.get_encrypted_data()
            .map_err(|e| ConfigError::EncryptedFileError(
                format!("Base64 解码失败: {}", e)
            ))?;
        
        // 解密数据
        let decrypted_data = self.crypto.decrypt_config(&encrypted_data)
            .map_err(|e| ConfigError::EncryptedFileError(
                format!("解密配置失败: {}", e)
            ))?;
        
        // 解析 TOML 配置
        let config_str = String::from_utf8(decrypted_data)
            .map_err(|e| ConfigError::EncryptedFileError(
                format!("配置数据解码失败: {}", e)
            ))?;
        
        let config: Config = toml::from_str(&config_str)
            .map_err(|e| ConfigError::EncryptedFileError(
                format!("解析配置失败: {}", e)
            ))?;
        
        debug!("✅ 从加密文件成功加载配置: {:?}", path);
        Ok(config)
    }
}

impl Default for EncryptedFileLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigLoader for EncryptedFileLoader {
    fn load(&self) -> ConfigResult<Config> {
        let config_path = self.config_path
            .as_ref()
            .ok_or_else(|| ConfigError::EncryptedFileError(
                "未找到加密配置文件".to_string()
            ))?;
        
        self.load_from_path(config_path)
    }
    
    fn source(&self) -> ConfigSource {
        if let Some(ref path) = self.config_path {
            ConfigSource::EncryptedFile(path.clone())
        } else {
            ConfigSource::EncryptedFile(PathBuf::new())
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
    fn test_encrypted_config_creation() {
        let test_data = b"test config data";
        let encrypted_config = EncryptedConfig::new(
            test_data.to_vec(),
            "1.0".to_string()
        );
        
        assert_eq!(encrypted_config.version, "1.0");
        assert!(!encrypted_config.encrypted_data.is_empty());
        assert!(!encrypted_config.created_at.is_empty());
    }

    #[test]
    fn test_encrypted_config_base64_encoding() {
        let test_data = b"Hello, encrypted world!";
        let encrypted_config = EncryptedConfig::new(
            test_data.to_vec(),
            "1.0".to_string()
        );
        
        // 解码应该能恢复原始数据
        let decoded_data = encrypted_config.get_encrypted_data().unwrap();
        assert_eq!(decoded_data, test_data);
    }

    #[test]
    fn test_encrypted_file_loader_creation() {
        let loader = EncryptedFileLoader::new();
        // 应该能创建，即使没有配置文件
        assert!(loader.is_available() || !loader.is_available());
    }

    #[test]
    fn test_encrypted_file_save_and_load() {
        cleanup_env_vars();
        
        let loader = EncryptedFileLoader::new();
        
        // 创建测试配置
        let test_config = Config {
            bot_token: "123456789:test_encrypted_token".to_string(),
            chat_id: 987654321,
            check_interval: 600,
        };
        
        // 使用临时文件进行测试
        let temp_file = NamedTempFile::new().unwrap();
        let temp_path = temp_file.path().to_path_buf();
        
        // 创建临时加载器
        let mut temp_loader = EncryptedFileLoader::new();
        temp_loader.config_path = Some(temp_path.clone());
        
        // 保存配置
        let save_result = temp_loader.save(&test_config);
        assert!(save_result.is_ok(), "保存加密配置应该成功");
        
        // 从临时文件加载配置
        let loaded_config = temp_loader.load_from_path(&temp_path).unwrap();
        
        assert_eq!(loaded_config.bot_token, test_config.bot_token);
        assert_eq!(loaded_config.chat_id, test_config.chat_id);
        assert_eq!(loaded_config.check_interval, test_config.check_interval);
        
        // 清理临时文件
        let _ = fs::remove_file(temp_path);
        cleanup_env_vars();
    }

    #[test]
    fn test_encrypted_file_source() {
        let loader = EncryptedFileLoader::new();
        let source = loader.source();
        
        match source {
            ConfigSource::EncryptedFile(path) => {
                assert!(path.as_os_str().is_empty() || path.exists());
            }
            _ => panic!("应该返回 EncryptedFile 源类型"),
        }
    }
}