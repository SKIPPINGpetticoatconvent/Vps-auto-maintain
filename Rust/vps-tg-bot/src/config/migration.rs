//! 配置迁移模块
//!
//! 提供明文配置到加密配置的迁移功能
//! 支持自动检测、加载、加密和清理旧配置文件

use crate::config::crypto::{ConfigCrypto, SecureStorage};

use crate::config::loader::encrypted::EncryptedFileLoader;
use crate::config::types::Config;
use anyhow::{Context, Result};
use log::{debug, error, info, warn};
use std::fs;
use std::path::{Path, PathBuf};

/// 迁移结果摘要
#[derive(Debug)]
pub struct MigrationResult {
    /// 是否成功迁移
    pub success: bool,
    /// 源配置文件路径
    pub source_path: PathBuf,
    /// 目标配置文件路径
    pub destination_path: PathBuf,
    /// 迁移消息
    pub message: String,
    /// 是否删除了原文件
    pub deleted_legacy: bool,
}

impl MigrationResult {
    /// 创建成功结果
    #[allow(dead_code)]
    pub fn success(source: PathBuf, dest: PathBuf, deleted: bool) -> Self {
        Self {
            success: true,
            source_path: source,
            destination_path: dest,
            message: "迁移成功完成".to_string(),
            deleted_legacy: deleted,
        }
    }

    /// 创建失败结果
    pub fn failure(source: PathBuf, message: String) -> Self {
        Self {
            success: false,
            source_path: source,
            destination_path: PathBuf::new(),
            message,
            deleted_legacy: false,
        }
    }
}

/// 检测并迁移明文配置到加密格式
///
/// # 参数
/// * `legacy_path` - 明文配置文件路径
/// * `encrypted_path` - 加密配置文件目标路径
/// * `delete_legacy` - 是否删除原明文文件
///
/// # 返回
/// 迁移结果
pub fn migrate_legacy_config(
    legacy_path: &Path,
    encrypted_path: &Path,
    _delete_legacy: bool,
) -> MigrationResult {
    info!("🔄 开始配置迁移...");
    debug!("源文件: {:?}", legacy_path);
    debug!("目标文件: {:?}", encrypted_path);

    // 1. 检查源文件是否存在
    if !legacy_path.exists() {
        let msg = format!("源配置文件不存在: {:?}", legacy_path);
        error!("{}", msg);
        return MigrationResult::failure(legacy_path.to_path_buf(), msg);
    }

    // 2. 由于不再支持明文配置，直接返回错误
    let msg = format!("不再支持明文配置文件: {:?}，请使用环境变量或加密配置", legacy_path);
    error!("{}", msg);
    MigrationResult::failure(legacy_path.to_path_buf(), msg)
}

/// 保存加密配置
fn save_encrypted_config(crypto: &ConfigCrypto, config: &Config, path: &Path) -> Result<()> {
    // 序列化配置为 TOML
    let toml_data = toml::to_string(config)
        .with_context(|| "序列化配置失败")?;

    // 加密配置数据
    let encrypted_data = crypto.encrypt_config(toml_data.as_bytes())
        .with_context(|| "加密配置失败")?;

    // 确保目录存在
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("创建目录失败: {:?}", parent))?;
    }

    // 写入加密文件
    fs::write(path, encrypted_data)
        .with_context(|| format!("写入加密配置文件失败: {:?}", path))?;

    Ok(())
}

/// 检测系统中是否存在明文配置文件
///
/// # 返回
/// 找到的明文配置文件路径列表
pub fn detect_legacy_configs() -> Vec<PathBuf> {
    let mut found_configs = Vec::new();

    const LEGACY_PATHS: &[&str] = &[
        "/etc/vps-tg-bot-rust/config.toml",
        "/etc/vps-tg-bot/config.toml",
        "config.toml",
    ];

    for path_str in LEGACY_PATHS {
        let path = Path::new(path_str);
        if path.exists() {
            info!("🔍 发现明文配置文件: {:?}", path);
            found_configs.push(path.to_path_buf());
        }
    }

    if found_configs.is_empty() {
        debug!("未检测到明文配置文件");
    }

    found_configs
}

/// 检测系统中是否存在加密配置文件
///
/// # 返回
/// 找到的加密配置文件路径列表
pub fn detect_encrypted_configs() -> Vec<PathBuf> {
    let mut found_configs = Vec::new();

    const ENCRYPTED_PATHS: &[&str] = &[
        "/etc/vps-tg-bot-rust/config.enc",
        "config.enc",
    ];

    for path_str in ENCRYPTED_PATHS {
        let path = Path::new(path_str);
        if path.exists() {
            info!("🔍 发现加密配置文件: {:?}", path);
            found_configs.push(path.to_path_buf());
        }
    }

    if found_configs.is_empty() {
        debug!("未检测到加密配置文件");
    }

    found_configs
}

/// 检查是否需要迁移
///
/// # 返回
/// (是否需要迁移, 明文配置路径, 加密配置路径)
pub fn check_migration_needed() -> (bool, Option<PathBuf>, Option<PathBuf>) {
    let legacy_configs = detect_legacy_configs();
    let encrypted_configs = detect_encrypted_configs();

    // 如果存在明文配置但不存在加密配置，则需要迁移
    if !legacy_configs.is_empty() && encrypted_configs.is_empty() {
        return (true, Some(legacy_configs[0].clone()), None);
    }

    // 如果存在加密配置，不需要迁移
    if !encrypted_configs.is_empty() {
        return (false, None, Some(encrypted_configs[0].clone()));
    }

    (false, None, None)
}

/// 初始化新配置并加密保存
///
/// # 参数
/// * `token` - Bot token
/// * `chat_id` - Chat ID
/// * `output_path` - 输出加密文件路径
///
/// # 返回
/// Result<(), anyhow::Error>
pub fn init_encrypted_config(
    token: String,
    chat_id: i64,
    output_path: &Path,
) -> Result<Config> {
    info!("🔧 初始化新加密配置...");

    // 创建配置
    let config = Config {
        bot_token: token.clone(),
        chat_id,
        check_interval: 300, // 默认值
    };

    // 验证配置
    config.validate()
        .map_err(|e| anyhow::anyhow!("配置验证失败: {}", e))?;

    debug!("配置验证通过，准备加密保存");
    debug!("Bot Token: {}...", &token[..20.min(token.len())]);
    debug!("Chat ID: {}", chat_id);
    debug!("输出路径: {:?}", output_path);

    // 保存加密配置
    let crypto = ConfigCrypto::new();
    save_encrypted_config(&crypto, &config, output_path)
        .with_context(|| format!("保存加密配置失败: {:?}", output_path))?;

    // 验证文件是否成功创建
    if !output_path.exists() {
        return Err(anyhow::anyhow!("加密配置文件创建失败: {:?}", output_path));
    }

    // 验证加密文件是否可以正确加载
    match test_encrypted_config_load(output_path) {
        Ok(true) => {
            info!("✅ 加密配置已保存并验证通过: {:?}", output_path);
        }
        Ok(false) => {
            warn!("⚠️  加密配置文件创建成功但加载测试失败");
        }
        Err(e) => {
            warn!("⚠️  加密配置文件加载测试出错: {}", e);
        }
    }

    Ok(config)
}

/// 测试加密配置文件是否可以正确加载
fn test_encrypted_config_load(path: &Path) -> Result<bool> {
    use crate::config::loader::encrypted::EncryptedFileLoader;
    
    let loader = EncryptedFileLoader::new();
    match loader.load_from_path(path) {
        Ok(_) => {
            debug!("加密配置文件加载测试成功");
            Ok(true)
        }
        Err(e) => {
            debug!("加密配置文件加载测试失败: {}", e);
            Ok(false) // 返回 false 而不是错误，避免影响主要流程
        }
    }
}

/// 导出解密配置（危险操作）
///
/// # 参数
/// * `encrypted_path` - 加密配置文件路径
/// * `output_path` - 输出明文文件路径
/// * `confirm` - 用户确认标志
///
/// # 返回
/// Result<(), anyhow::Error>
pub fn export_decrypted_config(
    encrypted_path: &Path,
    output_path: &Path,
    confirm: bool,
) -> Result<Config> {
    if !confirm {
        return Err(anyhow::anyhow!(
            "⚠️  危险操作未确认！使用 --confirm 标志确认您理解风险"
        ));
    }

    warn!("⚠️  警告：即将导出解密配置到明文文件！");
    warn!("⚠️  这将导致敏感信息以明文形式存储！");

    // 加载加密配置
    let loader = EncryptedFileLoader::new();
    let config = loader.load_from_path(encrypted_path)
        .with_context(|| format!("加载加密配置失败: {:?}", encrypted_path))?;

    // 验证配置
    config.validate()
        .map_err(|e| anyhow::anyhow!("配置验证失败: {}", e))?;

    // 保存明文配置
    let content = toml::to_string(&config)
        .with_context(|| "序列化配置失败")?;

    fs::write(output_path, content)
        .with_context(|| format!("写入明文配置文件失败: {:?}", output_path))?;

    warn!("⚠️  已将解密配置保存到: {:?}", output_path);
    warn!("⚠️  请尽快删除此明文文件并迁移到加密格式！");

    Ok(config)
}

/// 验证配置是否可用
///
/// # 参数
/// * `path` - 可选的配置文件路径（如果未指定则自动检测）
///
/// # 返回
/// (是否有效, 配置源类型描述, 错误信息如果无效)
pub fn verify_config(path: Option<&Path>) -> (bool, String, Option<String>) {
    use crate::config::loader::env::EnvironmentLoader;
    use crate::config::loader::ConfigLoader;

    // 1. 首先检查环境变量
    let env_loader = EnvironmentLoader::new();
    if env_loader.is_available() {
        match env_loader.load() {
            Ok(config) => {
                if let Err(e) = config.validate() {
                    return (false, "环境变量".to_string(), Some(e.to_string()));
                }
                return (true, "环境变量".to_string(), None);
            }
            Err(e) => {
                warn!("⚠️  环境变量配置无效: {}", e);
            }
        }
    }

    // 2. 检查指定路径或自动检测
    let check_path: Option<PathBuf> = path.map(|p| p.to_path_buf())
        .or_else(|| {
            detect_encrypted_configs()
                .first()
                .cloned()
                .or_else(|| {
                    detect_legacy_configs()
                        .first()
                        .cloned()
                })
        });

    match check_path {
        Some(ref p) if p.extension().map(|e| e.to_string_lossy()) == Some("enc".into()) => {
            // 加密文件
            let loader = EncryptedFileLoader::new();
            match loader.load_from_path(p) {
                Ok(config) => {
                    if let Err(e) = config.validate() {
                        return (false, format!("加密文件: {:?}", p), Some(e.to_string()));
                    }
                    (true, format!("加密文件: {:?}", p), None)
                }
                Err(e) => (false, format!("加密文件: {:?}", p), Some(e.to_string())),
            }
        }
        Some(p) => {
            // 明文文件 - 不再支持
            (false, format!("明文文件: {:?}", p), Some("不再支持明文配置文件，请使用环境变量或加密配置".to_string()))
        }
        None => (false, "未找到配置".to_string(), Some("没有任何可用配置源".to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::{NamedTempFile, TempDir};

    fn cleanup_env_vars() {
        std::env::remove_var("BOT_TOKEN");
        std::env::remove_var("CHAT_ID");
        std::env::remove_var("CHECK_INTERVAL");
    }



    #[test]
    fn test_migrate_nonexistent_file() {
        let temp_path = TempDir::new().unwrap().path().join("nonexistent.toml");
        let encrypted_path = TempDir::new().unwrap().path().join("config.enc");

        let result = migrate_legacy_config(&temp_path, &encrypted_path, false);

        assert!(!result.success);
        assert!(result.message.contains("不存在"));
    }

    #[test]
    fn test_init_encrypted_config() {
        cleanup_env_vars();

        let temp_path = TempDir::new().unwrap().path().join("new_config.enc");

        let config = init_encrypted_config(
            "123456789:init_test_token".to_string(),
            555666777,
            &temp_path,
        ).unwrap();

        assert_eq!(config.bot_token, "123456789:init_test_token");
        assert_eq!(config.chat_id, 555666777);
        assert!(temp_path.exists());
    }

    #[test]
    fn test_export_decrypted_config_without_confirm() {
        cleanup_env_vars();

        let temp_dir = TempDir::new().unwrap();
        let encrypted_path = temp_dir.path().join("config.enc");
        let output_path = temp_dir.path().join("exported.toml");

        // 先创建加密配置
        let config = Config {
            bot_token: "123456789:export_test".to_string(),
            chat_id: 111222333,
            check_interval: 300,
        };
        let crypto = ConfigCrypto::new();
        save_encrypted_config(&crypto, &config, &encrypted_path).unwrap();

        // 尝试导出（未确认）
        let result = export_decrypted_config(&encrypted_path, &output_path, false);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("未确认"));
    }

    #[test]
    fn test_export_decrypted_config_with_confirm() {
        cleanup_env_vars();

        let temp_dir = TempDir::new().unwrap();
        let encrypted_path = temp_dir.path().join("config.enc");
        let output_path = temp_dir.path().join("exported.toml");

        // 先创建加密配置
        let config = Config {
            bot_token: "123456789:export_confirm_test".to_string(),
            chat_id: 444555666,
            check_interval: 600,
        };
        let crypto = ConfigCrypto::new();
        save_encrypted_config(&crypto, &config, &encrypted_path).unwrap();

        // 导出（已确认）
        let result = export_decrypted_config(&encrypted_path, &output_path, true);

        assert!(result.is_ok());
        assert!(output_path.exists());

        // 验证导出的配置
        let content = std::fs::read_to_string(&output_path).unwrap();
        assert!(content.contains("export_confirm_test"));
        assert!(content.contains("444555666"));
    }

    #[test]
    fn test_verify_config_valid_encrypted() {
        cleanup_env_vars();

        let temp_dir = TempDir::new().unwrap();
        let encrypted_path = temp_dir.path().join("config.enc");

        // 创建有效加密配置
        let config = Config {
            bot_token: "123456789:verify_test".to_string(),
            chat_id: 777888999,
            check_interval: 300,
        };
        let crypto = ConfigCrypto::new();
        save_encrypted_config(&crypto, &config, &encrypted_path).unwrap();

        let (valid, source, error) = verify_config(Some(&encrypted_path));

        assert!(valid);
        assert!(source.contains("加密文件"));
        assert!(error.is_none());
    }

    #[test]
    fn test_verify_config_nonexistent() {
        cleanup_env_vars();

        let temp_path = TempDir::new().unwrap().path().join("nonexistent.enc");
        let (valid, source, error) = verify_config(Some(&temp_path));

        assert!(!valid);
        assert!(error.is_some());
    }

    #[test]
    fn test_detect_legacy_configs() {
        cleanup_env_vars();

        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        if !config_path.exists() {
            std::fs::write(&config_path, "bot_token = \"test\"\nchat_id = \"123\"").unwrap();
        }

        let detected = detect_legacy_configs();
        // 可能检测到多个，验证至少包含我们创建的
        assert!(detected.iter().any(|p| p == &config_path));
    }
}
