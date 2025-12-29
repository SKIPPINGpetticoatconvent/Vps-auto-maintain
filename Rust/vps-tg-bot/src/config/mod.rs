use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub bot_token: String,
    pub chat_id: i64,
    #[serde(default = "default_check_interval")]
    pub check_interval: u64,
}

fn default_check_interval() -> u64 {
    300
}

impl Config {
    pub fn load() -> Result<Self> {
        // 优先从环境变量读取
        if let (Ok(bot_token), Ok(chat_id)) = (
            env::var("BOT_TOKEN"),
            env::var("CHAT_ID").map(|s| s.parse::<i64>().unwrap_or(-1)),
        ) {
            if !bot_token.is_empty() && chat_id != -1 {
                return Ok(Config {
                    bot_token,
                    chat_id,
                    check_interval: env::var("CHECK_INTERVAL")
                        .map(|s| s.parse::<u64>().unwrap_or(300))
                        .unwrap_or(300),
                });
            }
        }

        // 尝试读取配置文件
        let config_paths = [
            "/etc/vps-tg-bot-rust/config.toml",  // 与安装脚本一致
            "/etc/vps-tg-bot/config.toml",       // 保留兼容性
            "config.toml",                        // 本地开发目录
        ];

        for path in config_paths {
            if Path::new(path).exists() {
                let content = fs::read_to_string(path)
                    .with_context(|| format!("无法读取配置文件: {}", path))?;
                 
                // 尝试直接解析为 Config 结构
                let config: Result<Config, _> = toml::from_str(&content);
                 
                // 如果直接解析失败，尝试兼容旧格式
                if config.is_err() {
                    // 尝试解析为旧格式
                    #[derive(Deserialize)]
                    struct LegacyConfig {
                        bot: LegacyBotConfig,
                    }
                     
                    #[derive(Deserialize)]
                    struct LegacyBotConfig {
                        token: String,
                        chat_id: String,
                    }
                     
                    if let Ok(legacy_config) = toml::from_str::<LegacyConfig>(&content) {
                        return Ok(Config {
                            bot_token: legacy_config.bot.token,
                            chat_id: legacy_config.bot.chat_id.parse::<i64>().unwrap_or(-1),
                            check_interval: default_check_interval(),
                        });
                    }
                }
                 
                let config: Config = config?;
                return Ok(config);
            }
        }

        Err(anyhow::anyhow!("未找到有效的配置源"))
    }

    pub fn save(&self, path: &str) -> Result<()> {
        let content = toml::to_string(self)
            .with_context(|| "Failed to serialize config")?;
        fs::write(path, content)
            .with_context(|| format!("Failed to write config to: {}", path))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::fs;

    // 辅助函数：清理环境变量
    fn cleanup_env_vars() {
        unsafe {
            std::env::remove_var("BOT_TOKEN");
            std::env::remove_var("CHAT_ID");
            std::env::remove_var("CHECK_INTERVAL");
        }
    }

    #[test]
    fn test_config_from_file() {
        cleanup_env_vars();
        
        // 创建临时配置文件
        let config_content = r#"
bot_token = "file_bot_token_456"
chat_id = "987654321"
check_interval = 900
"#;

        // 创建config.toml文件
        let config_path = "config.toml";
        fs::write(config_path, config_content).unwrap();

        let config = Config::load().unwrap();
        
        assert_eq!(config.bot_token, "file_bot_token_456");
        assert_eq!(config.chat_id, 987654321);
        assert_eq!(config.check_interval, 900);

        // 清理
        let _ = fs::remove_file(config_path);
        cleanup_env_vars();
    }

    #[test]
    fn test_config_legacy_format() {
        cleanup_env_vars();
        
        // 测试旧格式配置文件
        let legacy_config_content = r#"
[bot]
token = "legacy_bot_token_789"
chat_id = "111222333"
"#;

        // 创建config.toml文件
        let config_path = "config.toml";
        fs::write(config_path, legacy_config_content).unwrap();

        let config = Config::load().unwrap();
        
        assert_eq!(config.bot_token, "legacy_bot_token_789");
        assert_eq!(config.chat_id, 111222333);
        assert_eq!(config.check_interval, 300); // 默认值

        // 清理
        let _ = fs::remove_file(config_path);
        cleanup_env_vars();
    }

    #[test]
    fn test_config_save_and_load() {
        let config = Config {
            bot_token: "test_save_token".to_string(),
            chat_id: 555666777,
            check_interval: 1200,
        };

        let temp_path = "test_config_save.toml";
        
        // 保存配置
        config.save(temp_path).unwrap();
        
        // 读取并验证
        let content = fs::read_to_string(temp_path).unwrap();
        assert!(content.contains("test_save_token"));
        assert!(content.contains("555666777"));
        assert!(content.contains("1200"));

        // 清理
        let _ = fs::remove_file(temp_path);
    }

    #[test]
    fn test_config_default_check_interval() {
        cleanup_env_vars();
        
        // 测试默认值
        let config_content = r#"
bot_token = "test_token"
chat_id = "123456"
"#;

        let config_path = "config.toml";
        fs::write(config_path, config_content).unwrap();

        let config = Config::load().unwrap();
        assert_eq!(config.check_interval, 300); // 默认值

        // 清理
        let _ = fs::remove_file(config_path);
        cleanup_env_vars();
    }

    #[test]
    fn test_config_invalid_toml_format() {
        cleanup_env_vars();
        
        // 无效的TOML格式
        let invalid_content = r#"
bot_token = "test_token"
chat_id = "123456"
# 无效的语法
invalid line without quotes
"#;

        let config_path = "config.toml";
        fs::write(config_path, invalid_content).unwrap();

        let result = Config::load();
        assert!(result.is_err());

        // 清理
        let _ = fs::remove_file(config_path);
        cleanup_env_vars();
    }

    #[test]
    fn test_config_no_valid_sources() {
        cleanup_env_vars();
        
        // 确保没有配置文件存在
        let config_path = "config.toml";
        let _ = fs::remove_file(config_path);

        // 应该返回错误，因为没有有效的配置源
        let result = Config::load();
        assert!(result.is_err());
        
        cleanup_env_vars();
    }

    #[test]
    fn test_config_deserialization() {
        // 测试配置反序列化
        let config_str = r#"
bot_token = "test_token_123"
chat_id = "987654321"
check_interval = 600
"#;

        let config: Result<Config, _> = toml::from_str(config_str);
        assert!(config.is_ok());
        
        let config = config.unwrap();
        assert_eq!(config.bot_token, "test_token_123");
        assert_eq!(config.chat_id, 987654321);
        assert_eq!(config.check_interval, 600);
    }

    #[test]
    fn test_config_serialization() {
        let config = Config {
            bot_token: "test_token".to_string(),
            chat_id: 123456789,
            check_interval: 300,
        };

        let serialized = toml::to_string(&config).unwrap();
        assert!(serialized.contains("test_token"));
        assert!(serialized.contains("123456789"));
        assert!(serialized.contains("300"));
    }

    #[test]
    fn test_config_clone() {
        let config = Config {
            bot_token: "clone_test".to_string(),
            chat_id: 111222333,
            check_interval: 600,
        };

        let cloned = config.clone();
        assert_eq!(config.bot_token, cloned.bot_token);
        assert_eq!(config.chat_id, cloned.chat_id);
        assert_eq!(config.check_interval, cloned.check_interval);
    }
}