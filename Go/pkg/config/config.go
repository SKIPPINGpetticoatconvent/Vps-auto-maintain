package config

import (
	"fmt"
	"os"
	"strconv"
)

// Config 应用程序配置结构体
type Config struct {
	// Telegram 相关配置（必需）
	TelegramToken string // Telegram Bot Token
	AdminChatID   int64  // 管理员 Chat ID

	// 文件路径配置（可选）
	StateFile  string // 状态持久化文件路径
	CoreScript string // 核心维护脚本路径
	RulesScript string // 规则维护脚本路径

	// 系统配置（可选）
	TimeZone      string // 系统时区
	LogLevel      string // 日志级别
	CommandTimeout int    // 命令执行超时时间（秒）

	// 高级配置（可选）
	EnableNotifications bool   // 是否启用通知
	MaintenanceWindow   string // 维护窗口时间
	SystemUser          string // 系统用户
}

// LoadConfig 从环境变量加载配置（保持向后兼容性）
func LoadConfig() (*Config, error) {
	// 获取默认配置
	config := GetDefaultConfig()

	// 加载必需字段
	token := os.Getenv("TG_TOKEN")
	if token == "" {
		return nil, fmt.Errorf("TG_TOKEN is required")
	}
	config.TelegramToken = token

	chatIDStr := os.Getenv("TG_CHAT_ID")
	if chatIDStr == "" {
		return nil, fmt.Errorf("TG_CHAT_ID is required")
	}

	chatID, err := strconv.ParseInt(chatIDStr, 10, 64)
	if err != nil {
		return nil, fmt.Errorf("invalid TG_CHAT_ID: %v", err)
	}
	config.AdminChatID = chatID

	// 加载可选字段
	if stateFile := os.Getenv("STATE_FILE"); stateFile != "" {
		config.StateFile = stateFile
	}

	if coreScript := os.Getenv("CORE_SCRIPT"); coreScript != "" {
		config.CoreScript = coreScript
	}

	if rulesScript := os.Getenv("RULES_SCRIPT"); rulesScript != "" {
		config.RulesScript = rulesScript
	}

	if timeZone := os.Getenv("TIME_ZONE"); timeZone != "" {
		config.TimeZone = timeZone
	}

	if logLevel := os.Getenv("LOG_LEVEL"); logLevel != "" {
		config.LogLevel = logLevel
	}

	if cmdTimeout := os.Getenv("CMD_TIMEOUT"); cmdTimeout != "" {
		if timeout, err := strconv.Atoi(cmdTimeout); err != nil {
			return nil, fmt.Errorf("invalid CMD_TIMEOUT: %v", err)
		} else {
			config.CommandTimeout = timeout
		}
	}

	if enableNotifications := os.Getenv("ENABLE_NOTIFICATIONS"); enableNotifications != "" {
		if parsed, err := strconv.ParseBool(enableNotifications); err != nil {
			return nil, fmt.Errorf("invalid ENABLE_NOTIFICATIONS: %v", err)
		} else {
			config.EnableNotifications = parsed
		}
	}

	if maintenanceWindow := os.Getenv("MAINTENANCE_WINDOW"); maintenanceWindow != "" {
		config.MaintenanceWindow = maintenanceWindow
	}

	if systemUser := os.Getenv("SYSTEM_USER"); systemUser != "" {
		config.SystemUser = systemUser
	}

	// 验证配置
	validator := NewConfigValidator(config)
	if err := validator.Validate(); err != nil {
		return nil, fmt.Errorf("配置验证失败: %v", err)
	}

	return config, nil
}
