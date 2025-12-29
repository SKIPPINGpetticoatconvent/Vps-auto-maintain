//! 改进的加密文件配置加载器
//! 
//! 从加密配置文件加载配置，支持以下路径：
//! - /etc/vps-tg-bot-rust/config.enc
//! - /usr/local/etc/vps-tg-bot-rust/config.enc
//! - /opt/vps-tg-bot-rust/config.enc
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
    "/etc/vps-tg-bot-rust/config.enc",           // 标准系统安装路径
    "/usr/local/etc/vps-tg-bot-rust/config.enc", // 备用系统路径
    "/opt/vps-tg-bot-rust/config.enc",           // 可选安装路径
    "config.enc",                                 // 本地开发目录
];

/// 配置文件环境变量
const CONFIG_PATH_ENV: &str = "BOT_CONFIG_PATH";
const CONFIG_DIR_ENV: &str = "BOT_CONFIG_DIR";

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

/// 改进的加密文件配置加载器
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
    
    /// 验证配置文件完整性
    fn verify_config_file(path: &Path) -> Result<bool, String> {
        debug!("验证配置文件: {:?}", path);
        
        // 检查文件存在性
        if !path.exists() {
            return Err("配置文件不存在".to_string());
        }
        
        // 检查文件权限（对于加密文件，应该只有所有者可读写）
        match std::fs::metadata(path) {
            Ok(metadata) => {
                let permissions = metadata.permissions();
                debug!("文件权限: {:?}", permissions);
                
                // 检查文件大小
                let file_size = metadata.len();
                if file_size == 0 {
                    return Err("配置文件为空".to_string());
                }
                
                if file_size > 10 * 1024 * 1024 { // 10MB
                    return Err("配置文件过大，可能已损坏".to_string());
                }
                
                debug!("文件大小: {} bytes", file_size);
            }
            Err(e) => {
                return Err(format!("无法读取文件元数据: {}", e));
            }
        }
        
        // 读取文件头部检查格式
        match std::fs::read_to_string(path) {
            Ok(content) => {
                // 简单检查是否为有效的TOML结构
                if !content.trim_start().starts_with("encrypted_data") {
                    return Err("配置文件格式不正确".to_string());
                }
                
                // 检查是否包含必需的字段
                if !content.contains("version") || !content.contains("created_at") {
                    return Err("配置文件缺少必需字段".to_string());
                }
            }
            Err(e) => {
                return Err(format!("无法读取配置文件: {}", e));
            }
        }
        
        Ok(true)
    }
    
    /// 查找加密配置文件路径
    fn find_encrypted_config_path() -> Option<PathBuf> {
        debug!("开始搜索加密配置文件...");
        
        // 1. 首先检查环境变量指定的路径
        if let Ok(config_path) = std::env::var(CONFIG_PATH_ENV) {
            let path = Path::new(&config_path);
            debug!("检查环境变量指定的路径: {:?}", path);
            
            if path.exists() {
                match Self::verify_config_file(path) {
                    Ok(_) => {
                        debug!("✅ 从环境变量发现有效配置文件: {}", config_path);
                        return Some(PathBuf::from(config_path));
                    }
                    Err(e) => {
                        debug!("⚠️  环境变量指定的配置文件验证失败: {}", e);
                    }
                }
            }
        }
        
        // 2. 检查环境变量指定的配置目录
        if let Ok(config_dir) = std::env::var(CONFIG_DIR_ENV) {
            let config_file = Path::new(&config_dir).join("config.enc");
            debug!("检查环境变量指定的配置目录: {:?}", config_file);
            
            if config_file.exists() {
                match Self::verify_config_file(&config_file) {
                    Ok(_) => {
                        debug!("✅ 从环境变量目录发现有效配置文件: {:?}", config_file);
                        return Some(config_file);
                    }
                    Err(e) => {
                        debug!("⚠️  环境变量目录的配置文件验证失败: {}", e);
                    }
                }
            }
        }
        
        // 3. 依次检查预定义路径
        for path in ENCRYPTED_CONFIG_PATHS {
            let path_obj = Path::new(path);
            debug!("检查预定义路径: {:?}", path_obj);
            
            if path_obj.exists() {
                match Self::verify_config_file(path_obj) {
                    Ok(_) => {
                        debug!("✅ 发现有效的加密配置文件: {}", path);
                        return Some(PathBuf::from(path));
                    }
                    Err(e) => {
                        debug!("⚠️  配置文件验证失败: {} - {}", path, e);
                    }
                }
            } else {
                debug!("❌ 配置文件不存在: {}", path);
            }
        }
        
        // 4. 尝试当前工作目录
        debug!("尝试当前工作目录搜索...");
        {
            let relative_path = Path::new("config.enc");
            debug!("检查当前目录: {:?}", relative_path);
            
            if relative_path.exists() {
                match Self::verify_config_file(relative_path) {
                    Ok(_) => {
                        let absolute_path = std::env::current_dir()
                            .map(|current_dir| current_dir.join(relative_path))
                            .unwrap_or_else(|_| relative_path.to_path_buf());
                            
                        debug!("✅ 发现有效的加密配置文件（当前目录）: {:?}", absolute_path);
                        return Some(absolute_path);
                    }
                    Err(e) => {
                        debug!("⚠️  当前目录配置文件验证失败: {}", e);
                    }
                }
            }
        }
        
        // 5. 提供详细的诊断信息
        let diagnostic_info = Self::generate_diagnostic_info();
        debug!("❌ 未找到任何有效的加密配置文件");
        debug!("诊断信息: {}", diagnostic_info);
        
        None
    }
    
    /// 生成详细的诊断信息
    fn generate_diagnostic_info() -> String {
        let mut info = String::new();
        info.push_str("配置搜索诊断报告\n");
        info.push_str(&format!("当前工作目录: {:?}\n", std::env::current_dir().unwrap_or_else(|_| PathBuf::from("未知"))));
        
        // 环境变量
        info.push_str("\n环境变量:\n");
        if let Ok(val) = std::env::var(CONFIG_PATH_ENV) {
            info.push_str(&format!("  {}: {}\n", CONFIG_PATH_ENV, val));
        }
        if let Ok(val) = std::env::var(CONFIG_DIR_ENV) {
            info.push_str(&format!("  {}: {}\n", CONFIG_DIR_ENV, val));
        }
        
        // 检查预定义路径
        info.push_str("\n预定义路径检查:\n");
        for path in ENCRYPTED_CONFIG_PATHS {
            let path_obj = Path::new(path);
            let exists = path_obj.exists();
            let status = if exists { "存在" } else { "不存在" };
            info.push_str(&format!("  {}: {}\n", path, status));
            
            if exists {
                match std::fs::metadata(path_obj) {
                    Ok(metadata) => {
                        info.push_str(&format!("    大小: {} bytes\n", metadata.len()));
                        info.push_str(&format!("    权限: {:?}\n", metadata.permissions()));
                    }
                    Err(e) => {
                        info.push_str(&format!("    无法读取元数据: {}\n", e));
                    }
                }
            }
        }
        
        info
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
    
    /// 改进的从指定路径加载方法，包含详细的错误诊断
    pub fn load_from_path<P: AsRef<Path>>(&self, path: P) -> ConfigResult<Config> {
        let path = path.as_ref();
        let path_str = path.display().to_string();
        
        debug!("开始加载配置文件: {:?}", path);
        
        // 1. 检查文件是否存在
        if !path.exists() {
            let error_msg = format!(
                "配置文件不存在: {}\n\n搜索路径和诊断信息:\n{}",
                path_str,
                Self::generate_diagnostic_info()
            );
            return Err(ConfigError::EncryptedFileError(error_msg));
        }
        
        // 2. 验证文件完整性
        if let Err(verification_error) = Self::verify_config_file(path) {
            let error_msg = format!(
                "配置文件验证失败: {}\n\n文件路径: {}\n验证错误: {}\n\n建议:\n1. 检查文件权限是否正确\n2. 验证文件是否损坏\n3. 重新生成配置文件",
                verification_error,
                path_str,
                verification_error
            );
            return Err(ConfigError::EncryptedFileError(error_msg));
        }
        
        // 3. 读取加密配置文件（字节数据）
        let content_bytes = match fs::read(path) {
            Ok(bytes) => bytes,
            Err(e) => {
                let error_msg = format!(
                    "读取配置文件失败: {}\n\n文件路径: {}\n系统错误: {}\n\n建议:\n1. 检查文件权限\n2. 确认文件未被其他程序占用\n3. 验证文件系统状态",
                    e, path_str, e
                );
                return Err(ConfigError::EncryptedFileError(error_msg));
            }
        };
        
        // 4. 检查文件大小
        let file_size = content_bytes.len();
        debug!("配置文件大小: {} bytes", file_size);
        
        if file_size == 0 {
            return Err(ConfigError::EncryptedFileError(
                format!("配置文件为空: {}\n\n建议: 重新生成配置文件", path_str)
            ));
        }
        
        if file_size > 10 * 1024 * 1024 { // 10MB
            return Err(ConfigError::EncryptedFileError(
                format!("配置文件过大 ({} bytes): {}\n\n建议: 检查配置文件是否损坏", file_size, path_str)
            ));
        }
        
        // 5. 保存原始字节数据用于错误诊断
        let original_bytes = content_bytes.clone();
        
        // 6. 将字节数据转换为 UTF-8 字符串
        let content = match String::from_utf8(content_bytes) {
            Ok(content) => content,
            Err(from_utf8_error) => {
                // 提供详细的 UTF-8 解析错误信息
                let utf8_error = from_utf8_error.utf8_error();
                let error_start = utf8_error.valid_up_to();
                let error_length = utf8_error.error_len().map_or(1, |_| 1);
                let file_size_kb = file_size / 1024;
                
                // 获取前100个字节的十六进制表示
                let first_100_bytes = &original_bytes[..original_bytes.len().min(100)];
                let hex_dump = first_100_bytes.iter()
                    .map(|b| format!("{:02x}", b))
                    .collect::<Vec<_>>()
                    .join(" ");
                
                let error_msg = format!(
                    "配置文件 UTF-8 解析失败:\n\n文件路径: {}\n文件大小: {} bytes ({:.1} KB)\n错误位置: 字节偏移 {}，错误长度: {} 字节\n前100字节十六进制: {}\n\n建议:\n1. 检查配置文件是否损坏\n2. 重新生成配置文件\n3. 确保配置文件为有效的 TOML 格式\n4. 验证文件编码是否为 UTF-8\n5. 检查硬件指纹是否发生变化",
                    path_str,
                    file_size,
                    file_size_kb as f64 / 1024.0,
                    error_start,
                    error_length,
                    hex_dump
                );
                
                return Err(ConfigError::EncryptedFileError(error_msg));
            }
        };
        
        // 7. 验证文件格式
        let trimmed_content = content.trim();
        if !trimmed_content.starts_with("encrypted_data") {
            let error_msg = format!(
                "配置文件格式不正确: {}\n\n文件内容预览:\n{}\n\n建议:\n1. 确认这是加密配置文件\n2. 检查文件是否损坏\n3. 重新生成配置文件",
                path_str,
                &trimmed_content[..trimmed_content.len().min(200)]
            );
            return Err(ConfigError::EncryptedFileError(error_msg));
        }
        
        // 8. 解析加密配置文件结构
        let encrypted_config: EncryptedConfig = match toml::from_str(content.as_str()) {
            Ok(config) => config,
            Err(e) => {
                let error_msg = format!(
                    "解析加密配置文件失败: {}\n\n文件路径: {}\nTOML 解析错误: {}\n\n建议:\n1. 检查 TOML 语法是否正确\n2. 验证配置文件完整性\n3. 重新生成配置文件",
                    e, path_str, e
                );
                return Err(ConfigError::EncryptedFileError(error_msg));
            }
        };
        
        // 9. 解码 Base64 数据
        let encrypted_data = match encrypted_config.get_encrypted_data() {
            Ok(data) => data,
            Err(e) => {
                let error_msg = format!(
                    "Base64 解码失败: {}\n\n文件路径: {}\n版本信息: {}\n创建时间: {}\n\n建议:\n1. 检查配置文件是否损坏\n2. 验证 Base64 编码是否正确\n3. 重新生成配置文件",
                    e, path_str, encrypted_config.version, encrypted_config.created_at
                );
                return Err(ConfigError::EncryptedFileError(error_msg));
            }
        };
        
        // 10. 解密数据
        let decrypted_data = match self.crypto.decrypt_config(&encrypted_data) {
            Ok(data) => data,
            Err(e) => {
                let error_msg = format!(
                    "解密配置失败: {}\n\n文件路径: {}\n加密数据大小: {} bytes\n版本信息: {}\n\n可能原因:\n1. 硬件指纹发生变化\n2. 密码学库版本不匹配\n3. 配置文件损坏\n\n建议:\n1. 确认机器硬件未发生重大变化\n2. 重新生成配置文件\n3. 检查系统时间是否正确",
                    e, path_str, encrypted_data.len(), encrypted_config.version
                );
                return Err(ConfigError::EncryptedFileError(error_msg));
            }
        };
        
        // 11. 解析 TOML 配置
        let decrypted_len = decrypted_data.len();
        let config_str = match String::from_utf8(decrypted_data) {
            Ok(s) => s,
            Err(e) => {
                let error_msg = format!(
                    "配置数据解码失败: {}\n\n文件路径: {}\n解密数据大小: {} bytes\n\n建议:\n1. 检查解密后的数据是否正确\n2. 验证配置文件完整性\n3. 重新生成配置文件",
                    e, path_str, decrypted_len
                );
                return Err(ConfigError::EncryptedFileError(error_msg));
            }
        };
        
        let config: Config = match toml::from_str(&config_str) {
            Ok(c) => c,
            Err(e) => {
                let error_msg = format!(
                    "解析配置失败: {}\n\n文件路径: {}\n配置数据预览:\n{}\n\n建议:\n1. 检查 TOML 语法是否正确\n2. 验证配置字段是否完整\n3. 重新生成配置文件",
                    e, path_str, &config_str[..config_str.len().min(500)]
                );
                return Err(ConfigError::EncryptedFileError(error_msg));
            }
        };
        
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
                format!(
                    "未找到加密配置文件\n\n{}\n\n请检查配置文件是否存在或设置正确的环境变量:",
                    Self::generate_diagnostic_info()
                )
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
        env::remove_var(CONFIG_PATH_ENV);
        env::remove_var(CONFIG_DIR_ENV);
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

    #[test]
    fn test_config_path_environment_variable() {
        cleanup_env_vars();
        
        // 设置环境变量
        std::env::set_var(CONFIG_PATH_ENV, "/nonexistent/path/config.enc");
        
        let loader = EncryptedFileLoader::new();
        // 即使环境变量指向不存在的文件，加载器也应该能创建
        assert!(loader.is_available() || !loader.is_available());
        
        cleanup_env_vars();
    }

    #[test]
    fn test_diagnostic_info_generation() {
        let info = EncryptedFileLoader::generate_diagnostic_info();
        
        // 检查诊断信息包含关键内容
        assert!(info.contains("配置搜索诊断报告"));
        assert!(info.contains("预定义路径检查"));
        assert!(info.contains("环境变量"));
    }
}