//! AES-256-GCM 加解密模块
//! 
//! 提供对称加密功能，使用 AES-256-GCM 算法进行安全加密和解密。

use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce
};
use anyhow::Result;
use log::debug;
use rand::RngCore;

/// AES-256-GCM Nonce 长度
const NONCE_SIZE: usize = 12;
/// AES-256-GCM Tag 长度  
const TAG_SIZE: usize = 16;
/// 密钥长度（256 bits）
const KEY_SIZE: usize = 32;

/// 使用 AES-256-GCM 加密数据
pub fn encrypt(plaintext: &[u8], key: &[u8]) -> Result<Vec<u8>> {
    if key.len() != KEY_SIZE {
        return Err(anyhow::anyhow!(
            "Invalid key length: {} bytes, expected {} bytes", 
            key.len(), KEY_SIZE
        ));
    }

    debug!("开始 AES-256-GCM 加密，数据长度: {} 字节", plaintext.len());

    // 生成随机 12 字节 Nonce
    let mut nonce_bytes = [0u8; NONCE_SIZE];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    // 创建 AES-256-GCM 密码实例
    let cipher = Aes256Gcm::new(key.into());
    
    // 执行加密
    let ciphertext = cipher.encrypt(nonce, plaintext)
        .map_err(|e| anyhow::anyhow!("AES-256-GCM encryption failed: {}", e))?;

    debug!("AES-256-GCM 加密成功，密文长度: {} 字节", ciphertext.len());

    // 组装输出格式: nonce(12) + ciphertext + tag(16)
    let mut result = Vec::new();
    result.extend_from_slice(&nonce_bytes);
    result.extend_from_slice(&ciphertext);

    debug!("最终密文格式: nonce(12) + ciphertext + tag(16), 总长度: {} 字节", result.len());

    Ok(result)
}

/// 使用 AES-256-GCM 解密数据
pub fn decrypt(ciphertext_with_nonce: &[u8], key: &[u8]) -> Result<Vec<u8>> {
    if key.len() != KEY_SIZE {
        return Err(anyhow::anyhow!(
            "Invalid key length: {} bytes, expected {} bytes", 
            key.len(), KEY_SIZE
        ));
    }

    if ciphertext_with_nonce.len() < NONCE_SIZE + TAG_SIZE {
        return Err(anyhow::anyhow!(
            "Invalid ciphertext length: {} bytes, minimum expected: {} bytes", 
            ciphertext_with_nonce.len(), NONCE_SIZE + TAG_SIZE
        ));
    }

    debug!("开始 AES-256-GCM 解密，输入长度: {} 字节", ciphertext_with_nonce.len());

    // 提取 Nonce
    let nonce_bytes = &ciphertext_with_nonce[..NONCE_SIZE];
    let nonce = Nonce::from_slice(nonce_bytes);

    // 提取密文（包含 Tag）
    let ciphertext = &ciphertext_with_nonce[NONCE_SIZE..];

    // 创建 AES-256-GCM 密码实例
    let cipher = Aes256Gcm::new(key.into());

    // 执行解密
    let plaintext = cipher.decrypt(nonce, ciphertext)
        .map_err(|e| anyhow::anyhow!("AES-256-GCM decryption failed: {}", e))?;

    debug!("AES-256-GCM 解密成功，明文长度: {} 字节", plaintext.len());

    Ok(plaintext)
}

/// 生成随机密钥（用于测试）
#[cfg(test)]
pub fn generate_random_key() -> Vec<u8> {
    let mut key = vec![0u8; KEY_SIZE];
    OsRng.fill_bytes(&mut key);
    key
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let key = generate_random_key();
        let plaintext = b"Hello, AES-256-GCM encryption! This is test data.";
        
        // 加密
        let encrypted = encrypt(plaintext, &key).unwrap();
        assert!(!encrypted.is_empty(), "加密结果不应该为空");
        assert!(encrypted.len() >= NONCE_SIZE + plaintext.len(), 
               "加密结果长度应该包含 nonce + 密文 + tag");
        
        // 解密
        let decrypted = decrypt(&encrypted, &key).unwrap();
        assert_eq!(decrypted, plaintext, "解密后的数据应该与原始数据一致");
    }

    #[test]
    fn test_encrypt_empty_data() {
        let key = generate_random_key();
        let plaintext = b"";
        
        let encrypted = encrypt(plaintext, &key).unwrap();
        let decrypted = decrypt(&encrypted, &key).unwrap();
        
        assert_eq!(decrypted, plaintext, "空数据的加密解密应该成功");
    }

    #[test]
    fn test_encrypt_large_data() {
        let key = generate_random_key();
        let plaintext = vec![42u8; 1024 * 1024]; // 1MB 测试数据
        
        let encrypted = encrypt(&plaintext, &key).unwrap();
        let decrypted = decrypt(&encrypted, &key).unwrap();
        
        assert_eq!(decrypted, plaintext, "大数据量的加密解密应该成功");
    }

    #[test]
    fn test_wrong_key_rejection() {
        let key1 = generate_random_key();
        let key2 = generate_random_key();
        let plaintext = b"Secret message";
        
        let encrypted = encrypt(plaintext, &key1).unwrap();
        
        // 使用错误的密钥解密应该失败
        let result = decrypt(&encrypted, &key2);
        assert!(result.is_err(), "使用错误密钥解密应该失败");
    }

    #[test]
    fn test_invalid_key_length() {
        let key_too_short = vec![0u8; 16]; // 128 bits instead of 256
        let key_too_long = vec![0u8; 64]; // 512 bits instead of 256
        let plaintext = b"test data";
        
        assert!(encrypt(plaintext, &key_too_short).is_err(), "短密钥应该被拒绝");
        assert!(encrypt(plaintext, &key_too_long).is_err(), "长密钥应该被拒绝");
    }

    #[test]
    fn test_invalid_ciphertext() {
        let key = generate_random_key();
        let invalid_ciphertext = vec![0u8; 10]; // 太短，无法包含 nonce + tag
        
        let result = decrypt(&invalid_ciphertext, &key);
        assert!(result.is_err(), "无效的密文应该被拒绝");
    }

    #[test]
    fn test_ciphertext_corruption_detection() {
        let key = generate_random_key();
        let plaintext = b"Original message";
        
        let mut encrypted = encrypt(plaintext, &key).unwrap();
        
        // 篡改密文中间部分
        if encrypted.len() > 20 {
            encrypted[10] = encrypted[10].wrapping_add(1);
        }
        
        let result = decrypt(&encrypted, &key);
        assert!(result.is_err(), "篡改的密文应该被检测并拒绝");
    }

    #[test]
    fn test_nonce_uniqueness() {
        let key = generate_random_key();
        let plaintext = b"Same message";
        
        // 加密相同消息多次
        let encrypted1 = encrypt(plaintext, &key).unwrap();
        let encrypted2 = encrypt(plaintext, &key).unwrap();
        
        // 不同的 Nonce 应该产生不同的密文
        assert_ne!(encrypted1, encrypted2, "相同消息的加密结果应该不同（不同的 nonce）");
        
        // 但解密应该都能得到相同结果
        let decrypted1 = decrypt(&encrypted1, &key).unwrap();
        let decrypted2 = decrypt(&encrypted2, &key).unwrap();
        
        assert_eq!(decrypted1, decrypted2, "不同的加密应该解密出相同的明文");
    }

    #[test]
    fn test_nonce_extraction() {
        let key = generate_random_key();
        let plaintext = b"Test nonce extraction";
        
        let encrypted = encrypt(plaintext, &key).unwrap();
        
        // 前 12 字节应该是 nonce
        let nonce = &encrypted[..NONCE_SIZE];
        assert_eq!(nonce.len(), NONCE_SIZE, "nonce 长度应该为 {} 字节", NONCE_SIZE);
        
        // nonce 不应该全为 0
        assert!(!nonce.iter().all(|&x| x == 0), "nonce 不应该全为 0");
    }
}