package config

import (
	"flag"
	"fmt"
	"os"
	"strconv"
	"strings"
)

// ConfigLoader 配置加载器
type ConfigLoader struct {
	config         *Config
	envVars        map[string]string
	flagOverrides  map[string]string
	interactiveMode bool
}

// NewConfigLoader 创建配置加载器
func NewConfigLoader() *ConfigLoader {
	return &ConfigLoader{
		config:        GetDefaultConfig(),
		envVars:       make(map[string]string),
		flagOverrides: make(map[string]string),
		interactiveMode: isInteractive(),
	}
}

// Load 从环境变量和标志加载配置
func (cl *ConfigLoader) Load() (*Config, error) {
	// 1. 加载环境变量
	if err := cl.loadEnvironmentVariables(); err != nil {
		return nil, err
	}

	// 2. 解析命令行标志
	if err := cl.parseFlags(); err != nil {
		return nil, err
	}

	// 3. 应用标志覆盖
	cl.applyFlagOverrides()

	// 4. 如果是交互式模式且缺少必需字段，提示用户输入
	if cl.interactiveMode && (cl.config.TelegramToken == "" || cl.config.AdminChatID == 0) {
		if err := cl.promptForMissingValues(); err != nil {
			return nil, err
		}
	}

	// 5. 验证配置
	validator := NewConfigValidator(cl.config)
	if err := validator.Validate(); err != nil {
		return nil, err
	}

	return cl.config, nil
}

// loadEnvironmentVariables 加载环境变量
func (cl *ConfigLoader) loadEnvironmentVariables() error {
	// 必需字段
	if token := os.Getenv("TG_TOKEN"); token != "" {
		cl.config.TelegramToken = token
	} else if cl.interactiveMode {
		// 交互式模式下不报错，等待提示输入
	} else {
		return fmt.Errorf("TG_TOKEN 环境变量未设置")
	}

	if chatIDStr := os.Getenv("TG_CHAT_ID"); chatIDStr != "" {
		chatID, err := strconv.ParseInt(chatIDStr, 10, 64)
		if err != nil {
			return fmt.Errorf("无效的 TG_CHAT_ID: %v", err)
		}
		cl.config.AdminChatID = chatID
	} else if cl.interactiveMode {
		// 交互式模式下不报错，等待提示输入
	} else {
		return fmt.Errorf("TG_CHAT_ID 环境变量未设置")
	}

	// 可选字段
	if stateFile := os.Getenv("STATE_FILE"); stateFile != "" {
		cl.config.StateFile = stateFile
	}

	if coreScript := os.Getenv("CORE_SCRIPT"); coreScript != "" {
		cl.config.CoreScript = coreScript
	}

	if rulesScript := os.Getenv("RULES_SCRIPT"); rulesScript != "" {
		cl.config.RulesScript = rulesScript
	}

	if timeZone := os.Getenv("TIME_ZONE"); timeZone != "" {
		cl.config.TimeZone = timeZone
	}

	if logLevel := os.Getenv("LOG_LEVEL"); logLevel != "" {
		cl.config.LogLevel = logLevel
	}

	if cmdTimeout := os.Getenv("CMD_TIMEOUT"); cmdTimeout != "" {
		timeout, err := strconv.Atoi(cmdTimeout)
		if err != nil {
			return fmt.Errorf("无效的 CMD_TIMEOUT: %v", err)
		}
		cl.config.CommandTimeout = timeout
	}

	if enableNotifications := os.Getenv("ENABLE_NOTIFICATIONS"); enableNotifications != "" {
		parsed, err := strconv.ParseBool(enableNotifications)
		if err != nil {
			return fmt.Errorf("无效的 ENABLE_NOTIFICATIONS: %v", err)
		}
		cl.config.EnableNotifications = parsed
	}

	if maintenanceWindow := os.Getenv("MAINTENANCE_WINDOW"); maintenanceWindow != "" {
		cl.config.MaintenanceWindow = maintenanceWindow
	}

	if systemUser := os.Getenv("SYSTEM_USER"); systemUser != "" {
		cl.config.SystemUser = systemUser
	}

	// 保存环境变量映射
	cl.envVars = map[string]string{
		"TG_TOKEN":            cl.config.TelegramToken,
		"TG_CHAT_ID":          strconv.FormatInt(cl.config.AdminChatID, 10),
		"STATE_FILE":          cl.config.StateFile,
		"CORE_SCRIPT":         cl.config.CoreScript,
		"RULES_SCRIPT":        cl.config.RulesScript,
		"TIME_ZONE":           cl.config.TimeZone,
		"LOG_LEVEL":           cl.config.LogLevel,
		"CMD_TIMEOUT":         strconv.Itoa(cl.config.CommandTimeout),
		"ENABLE_NOTIFICATIONS": strconv.FormatBool(cl.config.EnableNotifications),
		"MAINTENANCE_WINDOW":  cl.config.MaintenanceWindow,
		"SYSTEM_USER":         cl.config.SystemUser,
	}

	return nil
}

// parseFlags 解析命令行标志
func (cl *ConfigLoader) parseFlags() error {
	// 定义标志
	tokenFlag := flag.String("token", "", "Telegram Bot Token")
	chatIDFlag := flag.Int64("chat-id", 0, "Admin Chat ID")
	stateFileFlag := flag.String("state-file", "", "State file path")
	coreScriptFlag := flag.String("core-script", "", "Core maintenance script path")
	rulesScriptFlag := flag.String("rules-script", "", "Rules maintenance script path")
	timeZoneFlag := flag.String("timezone", "", "System timezone")
	logLevelFlag := flag.String("log-level", "", "Log level")
	timeoutFlag := flag.Int("timeout", 0, "Command timeout in seconds")
	notificationsFlag := flag.Bool("notifications", false, "Enable notifications")
	maintenanceWindowFlag := flag.String("maintenance-window", "", "Maintenance window")
	systemUserFlag := flag.String("system-user", "", "System user")

	flag.Parse()

	// 保存标志值
	cl.flagOverrides["token"] = *tokenFlag
	cl.flagOverrides["chat-id"] = strconv.FormatInt(*chatIDFlag, 10)
	cl.flagOverrides["state-file"] = *stateFileFlag
	cl.flagOverrides["core-script"] = *coreScriptFlag
	cl.flagOverrides["rules-script"] = *rulesScriptFlag
	cl.flagOverrides["timezone"] = *timeZoneFlag
	cl.flagOverrides["log-level"] = *logLevelFlag
	cl.flagOverrides["timeout"] = strconv.Itoa(*timeoutFlag)
	cl.flagOverrides["notifications"] = strconv.FormatBool(*notificationsFlag)
	cl.flagOverrides["maintenance-window"] = *maintenanceWindowFlag
	cl.flagOverrides["system-user"] = *systemUserFlag

	return nil
}

// applyFlagOverrides 应用标志覆盖
func (cl *ConfigLoader) applyFlagOverrides() {
	if token := cl.flagOverrides["token"]; token != "" {
		cl.config.TelegramToken = token
	}

	if chatIDStr := cl.flagOverrides["chat-id"]; chatIDStr != "" && chatIDStr != "0" {
		if chatID, err := strconv.ParseInt(chatIDStr, 10, 64); err == nil {
			cl.config.AdminChatID = chatID
		}
	}

	if stateFile := cl.flagOverrides["state-file"]; stateFile != "" {
		cl.config.StateFile = stateFile
	}

	if coreScript := cl.flagOverrides["core-script"]; coreScript != "" {
		cl.config.CoreScript = coreScript
	}

	if rulesScript := cl.flagOverrides["rules-script"]; rulesScript != "" {
		cl.config.RulesScript = rulesScript
	}

	if timeZone := cl.flagOverrides["timezone"]; timeZone != "" {
		cl.config.TimeZone = timeZone
	}

	if logLevel := cl.flagOverrides["log-level"]; logLevel != "" {
		cl.config.LogLevel = logLevel
	}

	if timeoutStr := cl.flagOverrides["timeout"]; timeoutStr != "" {
		if timeout, err := strconv.Atoi(timeoutStr); err == nil {
			cl.config.CommandTimeout = timeout
		}
	}

	if notifications := cl.flagOverrides["notifications"]; notifications != "" {
		if parsed, err := strconv.ParseBool(notifications); err == nil {
			cl.config.EnableNotifications = parsed
		}
	}

	if maintenanceWindow := cl.flagOverrides["maintenance-window"]; maintenanceWindow != "" {
		cl.config.MaintenanceWindow = maintenanceWindow
	}

	if systemUser := cl.flagOverrides["system-user"]; systemUser != "" {
		cl.config.SystemUser = systemUser
	}
}

// promptForMissingValues 提示用户输入缺失的值
func (cl *ConfigLoader) promptForMissingValues() error {
	var err error

	// 提示输入 Telegram Token
	if cl.config.TelegramToken == "" {
		fmt.Print("请输入 Telegram Bot Token: ")
		cl.config.TelegramToken, err = readPassword()
		if err != nil {
			return fmt.Errorf("读取 Token 失败: %v", err)
		}
		fmt.Println() // 换行
	}

	// 提示输入 Chat ID
	if cl.config.AdminChatID == 0 {
		fmt.Print("请输入管理员 Chat ID: ")
		var chatIDStr string
		fmt.Scanln(&chatIDStr)
		
		chatID, err := strconv.ParseInt(chatIDStr, 10, 64)
		if err != nil {
			return fmt.Errorf("无效的 Chat ID: %v", err)
		}
		cl.config.AdminChatID = chatID
	}

	// 提示输入可选配置
	if cl.config.StateFile == "" {
		fmt.Printf("状态文件路径 (默认: %s): ", cl.config.StateFile)
		var input string
		fmt.Scanln(&input)
		if input != "" {
			cl.config.StateFile = input
		}
	}

	if cl.config.CoreScript == "" {
		fmt.Printf("核心维护脚本路径 (默认: %s): ", cl.config.CoreScript)
		var input string
		fmt.Scanln(&input)
		if input != "" {
			cl.config.CoreScript = input
		}
	}

	if cl.config.RulesScript == "" {
		fmt.Printf("规则维护脚本路径 (默认: %s): ", cl.config.RulesScript)
		var input string
		fmt.Scanln(&input)
		if input != "" {
			cl.config.RulesScript = input
		}
	}

	return nil
}

// isInteractive 检查是否在交互式终端中运行
func isInteractive() bool {
	// 检查是否是TTY
	return isTerm(os.Stdin.Fd())
}

// isTerm 检查文件描述符是否为终端
func isTerm(fd uintptr) bool {
	// 在Unix系统上，我们可以使用系统调用检查
	// 这里简化处理，假设在Linux环境下
	return true // 简化实现，实际项目中可能需要更复杂的检查
}

// readPassword 读取密码（隐藏输入）
func readPassword() (string, error) {
	// 这里简化实现，实际项目中可能需要使用termios库
	var password string
	fmt.Scanln(&password)
	return password, nil
}

// GetEnvironmentSummary 获取环境变量摘要
func (cl *ConfigLoader) GetEnvironmentSummary() map[string]interface{} {
	summary := make(map[string]interface{})
	
	for key, value := range cl.envVars {
		// 隐藏敏感信息
		if key == "TG_TOKEN" {
			if value != "" {
				summary[key] = "***" + value[len(value)-4:]
			} else {
				summary[key] = ""
			}
		} else {
			summary[key] = value
		}
	}
	
	return summary
}

// GetFlagOverridesSummary 获取标志覆盖摘要
func (cl *ConfigLoader) GetFlagOverridesSummary() map[string]interface{} {
	summary := make(map[string]interface{})
	
	for key, value := range cl.flagOverrides {
		if value != "" {
			if key == "token" {
				summary[key] = "***" + value[len(value)-4:]
			} else {
				summary[key] = value
			}
		}
	}
	
	return summary
}

// LoadFromString 从字符串加载配置（用于测试）
func LoadFromString(configStr string) (*Config, error) {
	lines := strings.Split(configStr, "\n")
	config := GetDefaultConfig()

	for _, line := range lines {
		line = strings.TrimSpace(line)
		if line == "" || strings.HasPrefix(line, "#") {
			continue
		}

		parts := strings.SplitN(line, "=", 2)
		if len(parts) != 2 {
			continue
		}

		key := strings.TrimSpace(parts[0])
		value := strings.TrimSpace(parts[1])

		switch key {
		case "TG_TOKEN":
			config.TelegramToken = value
		case "TG_CHAT_ID":
			if chatID, err := strconv.ParseInt(value, 10, 64); err == nil {
				config.AdminChatID = chatID
			}
		case "STATE_FILE":
			config.StateFile = value
		case "CORE_SCRIPT":
			config.CoreScript = value
		case "RULES_SCRIPT":
			config.RulesScript = value
		case "TIME_ZONE":
			config.TimeZone = value
		case "LOG_LEVEL":
			config.LogLevel = value
		case "CMD_TIMEOUT":
			if timeout, err := strconv.Atoi(value); err == nil {
				config.CommandTimeout = timeout
			}
		case "ENABLE_NOTIFICATIONS":
			if enabled, err := strconv.ParseBool(value); err == nil {
				config.EnableNotifications = enabled
			}
		case "MAINTENANCE_WINDOW":
			config.MaintenanceWindow = value
		case "SYSTEM_USER":
			config.SystemUser = value
		}
	}

	return config, nil
}