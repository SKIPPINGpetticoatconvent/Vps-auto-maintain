package config

import (
	"os"
	"strconv"
)

// Config 存储 Bot 配置信息
type Config struct {
	Token        string
	AdminChatID  int64
	CoreScript   string
	RulesScript  string
}

// Load 从环境变量加载配置
func Load() (*Config, error) {
	token := os.Getenv("TG_TOKEN")
	if token == "" {
		return nil, &ConfigError{Message: "TG_TOKEN 环境变量未设置"}
	}

	chatIDStr := os.Getenv("TG_CHAT_ID")
	if chatIDStr == "" {
		return nil, &ConfigError{Message: "TG_CHAT_ID 环境变量未设置"}
	}

	chatID, err := strconv.ParseInt(chatIDStr, 10, 64)
	if err != nil {
		return nil, &ConfigError{Message: "TG_CHAT_ID 格式错误: " + err.Error()}
	}

	coreScript := os.Getenv("CORE_SCRIPT")
	if coreScript == "" {
		coreScript = "/usr/local/bin/vps-maintain-core.sh"
	}

	rulesScript := os.Getenv("RULES_SCRIPT")
	if rulesScript == "" {
		rulesScript = "/usr/local/bin/vps-maintain-rules.sh"
	}

	return &Config{
		Token:       token,
		AdminChatID: chatID,
		CoreScript:  coreScript,
		RulesScript: rulesScript,
	}, nil
}

// ConfigError 配置错误类型
type ConfigError struct {
	Message string
}

func (e *ConfigError) Error() string {
	return e.Message
}
