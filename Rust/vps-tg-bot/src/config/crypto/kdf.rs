//! 密钥衍生函数模块
//! 
//! 使用 Argon2id 算法从机器指纹衍生出 256-bit 加密密钥。

use anyhow::{Context, Result};
use argon2::{Algorithm, Argon2, Params, Version};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use log::debug;

/// Argon2 密钥衍生参数
fn create_kdf_params() -> Params {
    Params::new(
        64 * 1024, // 64MB 内存
        3,         // 3 次迭代
        1,         // 并行度 1
        Some(32),  // 32 字节输出密钥长度
    ).expect("Failed to create Argon2 parameters")
}

/// 固定盐值，用于密钥衍生
const FIXED_SALT: &[u8] = b"vps-tg-bot-rust-v1";

/// 从机器指纹衍生密钥
pub fn derive_key_from_fingerprint() -> Result<Vec<u8>> {
    debug!("开始从机器指纹衍生密钥");
    
    // 采集机器指纹
    let fingerprint = crate::config::crypto::fingerprint::collect_machine_fingerprint()
        .context("Failed to collect machine fingerprint")?;
    
    debug!("成功采集机器指纹，长度: {} 字节", fingerprint.len());
    
    // 使用 Argon2id 衍生密钥
    let derived_key = derive_key_argon2id(fingerprint.as_bytes(), FIXED_SALT)
        .context("Failed to derive key with Argon2id")?;
    
    debug!("成功衍生密钥，长度: {} 字节", derived_key.len());
    
    Ok(derived_key)
}

/// 使用 Argon2id 算法衍生密钥
fn derive_key_argon2id(password: &[u8], salt: &[u8]) -> Result<Vec<u8>> {
    // 创建 Argon2id 上下文
    let params = create_kdf_params();
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    
    let mut key = vec![0u8; 32]; // 256-bit 密钥
    
    // 执行密钥衍生
    argon2.hash_password_into(password, salt, &mut key)
        .map_err(|e| anyhow::anyhow!("Argon2 key derivation failed: {}", e))?;
    
    // 将密钥转换为 base64 格式便于调试
    let key_b64 = BASE64.encode(&key);
    debug!("密钥衍生成功 (base64): {}... (长度: {})", 
           &key_b64[..std::cmp::min(32, key_b64.len())], key_b64.len());
    
    Ok(key)
}

/// 从自定义指纹数据衍生密钥（用于测试）
#[cfg(test)]
pub fn derive_key_from_fingerprint_data(fingerprint_data: &str) -> Result<Vec<u8>> {
    let params = create_kdf_params();
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    
    let mut key = vec![0u8; 32];
    let password = fingerprint_data.as_bytes();
    
    argon2.hash_password_into(password, FIXED_SALT, &mut key)
        .map_err(|e| anyhow::anyhow!("Argon2 key derivation failed: {}", e))?;
    
    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::NamedTempFile;

    #[test]
    fn test_key_derivation_deterministic() {
        // 测试相同的指纹数据会产生相同的密钥
        let fingerprint_data = "test-fingerprint-data-12345";
        
        let key1 = derive_key_from_fingerprint_data(fingerprint_data).unwrap();
        let key2 = derive_key_from_fingerprint_data(fingerprint_data).unwrap();
        
        assert_eq!(key1, key2, "相同的指纹数据应该产生相同的密钥");
        assert_eq!(key1.len(), 32, "密钥长度应该是 32 字节");
    }

    #[test]
    fn test_key_derivation_different_fingerprints() {
        // 测试不同的指纹数据会产生不同的密钥
        let key1 = derive_key_from_fingerprint_data("fingerprint-1").unwrap();
        let key2 = derive_key_from_fingerprint_data("fingerprint-2").unwrap();
        
        assert_ne!(key1, key2, "不同的指纹数据应该产生不同的密钥");
        assert_eq!(key1.len(), 32, "密钥长度应该是 32 字节");
        assert_eq!(key2.len(), 32, "密钥长度应该是 32 字节");
    }

    #[test]
    fn test_key_derivation_with_empty_input() {
        // 测试空输入的处理
        let result = derive_key_from_fingerprint_data("");
        assert!(result.is_ok(), "空输入应该能够成功处理");
        
        let key = result.unwrap();
        assert_eq!(key.len(), 32, "密钥长度应该是 32 字节");
    }

    #[test]
    fn test_argon2_parameters() {
        // 验证 Argon2 参数配置
        let params = create_kdf_params();
        assert_eq!(params.m_cost(), 64 * 1024, "内存成本应该是 64MB");
        assert_eq!(params.t_cost(), 3, "时间成本应该是 3 次迭代");
        assert_eq!(params.p_cost(), 1, "并行度应该是 1");
        assert_eq!(params.output_len(), Some(32), "输出长度应该是 32 字节");
    }

    #[test]
    fn test_fixed_salt() {
        // 验证固定盐值
        assert_eq!(FIXED_SALT, b"vps-tg-bot-rust-v1", "固定盐值应该正确");
        assert!(!FIXED_SALT.is_empty(), "固定盐值不应该为空");
    }

    #[test]
    fn test_key_derivation_consistency() {
        // 测试密钥衍生的稳定性（相同输入产生相同输出）
        let inputs = [
            "cpu-id-mac-uuid-hostname",
            "different-combination-123",
            "simple-test",
        ];
        
        for input in &inputs {
            let key1 = derive_key_from_fingerprint_data(input).unwrap();
            let key2 = derive_key_from_fingerprint_data(input).unwrap();
            assert_eq!(key1, key2, "相同输入应该产生相同的密钥");
        }
    }
}