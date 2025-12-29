//! 加密模块
//! 
//! 提供配置安全增强功能，包括密钥衍生、对称加密和机器指纹等功能。

pub mod kdf;
pub mod aes_gcm;
pub mod fingerprint;

use anyhow::Result;
use std::path::Path;

/// 安全存储 trait，定义配置加密存储的接口
pub trait SecureStorage {
    /// 加密配置数据
    fn encrypt_config(&self, data: &[u8]) -> Result<Vec<u8>>;
    
    /// 解密配置数据
    fn decrypt_config(&self, encrypted_data: &[u8]) -> Result<Vec<u8>>;
    
    /// 获取或生成加密密钥
    #[allow(dead_code)]
    fn get_or_generate_key(&self, config_path: &Path) -> Result<Vec<u8>>;
}

/// 配置加密存储结构
#[derive(Debug)]
pub struct ConfigCrypto {
    _private: (),
}

impl ConfigCrypto {
    /// 创建新的配置加密实例
    pub fn new() -> Self {
        Self { _private: () }
    }
}

impl SecureStorage for ConfigCrypto {
    fn encrypt_config(&self, data: &[u8]) -> Result<Vec<u8>> {
        // 获取密钥
        let key = kdf::derive_key_from_fingerprint()?;
        
        // 使用 AES-256-GCM 加密
        aes_gcm::encrypt(data, &key)
    }
    
    fn decrypt_config(&self, encrypted_data: &[u8]) -> Result<Vec<u8>> {
        // 获取密钥
        let key = kdf::derive_key_from_fingerprint()?;
        
        // 使用 AES-256-GCM 解密
        aes_gcm::decrypt(encrypted_data, &key)
    }
    
    fn get_or_generate_key(&self, _config_path: &Path) -> Result<Vec<u8>> {
        // 基于机器指纹生成密钥
        kdf::derive_key_from_fingerprint()
    }
}

impl Default for ConfigCrypto {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_config_crypto_creation() {
        let crypto = ConfigCrypto::new();
        assert!(crypto.get_or_generate_key(Path::new("test")).is_ok());
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let crypto = ConfigCrypto::new();
        let test_data = b"test config data";
        
        // 加密
        let encrypted = crypto.encrypt_config(test_data).unwrap();
        assert!(!encrypted.is_empty());
        
        // 解密
        let decrypted = crypto.decrypt_config(&encrypted).unwrap();
        assert_eq!(decrypted, test_data);
    }
}