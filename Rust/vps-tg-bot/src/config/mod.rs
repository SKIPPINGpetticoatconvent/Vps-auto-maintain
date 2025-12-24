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
            "/etc/vps-tg-bot/config.toml",
            "config.toml",
        ];

        for path in config_paths {
            if Path::new(path).exists() {
                let content = fs::read_to_string(path)
                    .with_context(|| format!("Failed to read config file: {}", path))?;
                
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

        Err(anyhow::anyhow!("No valid config source found"))
    }

    pub fn save(&self, path: &str) -> Result<()> {
        let content = toml::to_string(self)
            .with_context(|| "Failed to serialize config")?;
        fs::write(path, content)
            .with_context(|| format!("Failed to write config to: {}", path))?;
        Ok(())
    }
}