package config

import (
	"path/filepath"
	"os"
)

// DefaultValues 定义所有配置项的默认值
type DefaultValues struct {
	StateFile     string
	CoreScript    string
	RulesScript   string
	TimeZone      string
	LogLevel      string
	CommandTimeout int
}

// GetDefaultValues 返回默认配置值
func GetDefaultValues() *DefaultValues {
	return &DefaultValues{
		StateFile:     getDefaultStateFile(),
		CoreScript:    "/usr/local/bin/vps-maintain-core.sh",
		RulesScript:   "/usr/local/bin/vps-maintain-rules.sh",
		TimeZone:      "Asia/Shanghai",
		LogLevel:      "info",
		CommandTimeout: 30, // 30秒
	}
}

// getDefaultStateFile 获取默认状态文件路径
func getDefaultStateFile() string {
	// 获取当前工作目录
	wd, err := os.Getwd()
	if err != nil {
		return "state.json"
	}
	return filepath.Join(wd, "state.json")
}

// GetDefaultConfig 返回包含默认值的Config
func GetDefaultConfig() *Config {
	defaults := GetDefaultValues()
	return &Config{
		TelegramToken:  "", // 必须从环境变量设置
		AdminChatID:    0,  // 必须从环境变量设置
		StateFile:      defaults.StateFile,
		CoreScript:     defaults.CoreScript,
		RulesScript:    defaults.RulesScript,
		TimeZone:       defaults.TimeZone,
		LogLevel:       defaults.LogLevel,
		CommandTimeout: defaults.CommandTimeout,
	}
}

// GetEnvironmentVariables 返回所有支持的 环境变量
func GetEnvironmentVariables() map[string]string {
	return map[string]string{
		// 必需变量
		"TG_TOKEN":     "Telegram Bot Token (必需)",
		"TG_CHAT_ID":   "管理员 Telegram Chat ID (必需)",
		
		// 可选变量
		"STATE_FILE":     "状态持久化文件路径 (可选)",
		"CORE_SCRIPT":    "核心维护脚本路径 (可选)",
		"RULES_SCRIPT":   "规则维护脚本路径 (可选)",
		"TIME_ZONE":      "系统时区 (可选)",
		"LOG_LEVEL":      "日志级别 (可选)",
		"CMD_TIMEOUT":    "命令执行超时时间(秒) (可选)",
	}
}