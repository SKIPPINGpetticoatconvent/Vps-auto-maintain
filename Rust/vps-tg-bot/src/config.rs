use std::path::PathBuf;
use crate::error::ConfigError;
use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub tg_token: String,
    pub tg_chat_id: i64,
    pub state_path: PathBuf,
    pub scripts_path: PathBuf,
    pub logs_service: String,
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        let token = env::var("TG_TOKEN").map_err(|_| ConfigError::MissingEnv("TG_TOKEN".to_string()))?;
        let chat_id_str = env::var("TG_CHAT_ID").map_err(|_| ConfigError::MissingEnv("TG_CHAT_ID".to_string()))?;
        let chat_id = chat_id_str.parse::<i64>().map_err(|_| ConfigError::InvalidValue("TG_CHAT_ID".to_string(), chat_id_str))?;
        
        let state_path = env::var("STATE_PATH").unwrap_or_else(|_| "/var/lib/vps-tg-bot".to_string());
        let scripts_path = env::var("SCRIPTS_PATH").unwrap_or_else(|_| "/usr/local/bin/vps-tg-bot/scripts".to_string());
        let logs_service = env::var("LOGS_SERVICE").unwrap_or_else(|_| "vps-tg-bot".to_string());

        Ok(Self {
            tg_token: token,
            tg_chat_id: chat_id,
            state_path: PathBuf::from(state_path),
            scripts_path: PathBuf::from(scripts_path),
            logs_service,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    // Helper to safely run tests with env vars
    fn with_env_vars<F>(vars: Vec<(&str, &str)>, test: F)
    where
        F: FnOnce(),
    {
        // Set env vars
        for (k, v) in &vars {
            env::set_var(k, v);
        }

        // Run test
        test();

        // Cleanup
        for (k, _) in vars {
            env::remove_var(k);
        }
    }

    #[test]
    fn test_config_from_env_success() {
        with_env_vars(
            vec![
                ("TG_TOKEN", "test_token"),
                ("TG_CHAT_ID", "123456789"),
                ("STATE_PATH", "/tmp/state"),
                ("SCRIPTS_PATH", "/tmp/scripts"),
                ("LOGS_SERVICE", "test-service"),
            ],
            || {
                let config = Config::from_env().unwrap();
                assert_eq!(config.tg_token, "test_token");
                assert_eq!(config.tg_chat_id, 123456789);
                assert_eq!(config.state_path, PathBuf::from("/tmp/state"));
                assert_eq!(config.scripts_path, PathBuf::from("/tmp/scripts"));
                assert_eq!(config.logs_service, "test-service");
            },
        );
    }

    #[test]
    fn test_config_from_env_missing_token() {
        with_env_vars(vec![("TG_CHAT_ID", "123456789")], || {
            let err = Config::from_env().unwrap_err();
            match err {
                ConfigError::MissingEnv(var) => assert_eq!(var, "TG_TOKEN"),
                _ => panic!("Unexpected error: {:?}", err),
            }
        });
    }

    #[test]
    fn test_config_from_env_invalid_chat_id() {
        with_env_vars(
            vec![("TG_TOKEN", "test_token"), ("TG_CHAT_ID", "invalid")],
            || {
                let err = Config::from_env().unwrap_err();
                match err {
                    ConfigError::InvalidValue(var, val) => {
                        assert_eq!(var, "TG_CHAT_ID");
                        assert_eq!(val, "invalid");
                    }
                    _ => panic!("Unexpected error: {:?}", err),
                }
            },
        );
    }
}
