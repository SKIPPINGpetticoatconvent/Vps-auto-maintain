package config

import (
	"fmt"
	"os"
	"path/filepath"
	"strconv"
	"strings"
	"time"
)

// ConfigValidator 配置验证器
type ConfigValidator struct {
	config           *Config
	skipScriptCheck bool
}

// NewConfigValidator 创建配置验证器
func NewConfigValidator(config *Config) *ConfigValidator {
	return &ConfigValidator{config: config, skipScriptCheck: false}
}

// NewConfigValidatorWithOptions 创建配置验证器（带选项）
func NewConfigValidatorWithOptions(config *Config, skipScriptCheck bool) *ConfigValidator {
	return &ConfigValidator{config: config, skipScriptCheck: skipScriptCheck}
}

// Validate 执行完整配置验证
func (cv *ConfigValidator) Validate() error {
	var errs []string

	// 验证必需字段
	if err := cv.validateRequiredFields(); err != nil {
		errs = append(errs, err.Error())
	}

	// 验证可选字段
	if err := cv.validateOptionalFields(); err != nil {
		errs = append(errs, err.Error())
	}

	// 验证业务逻辑
	if err := cv.validateBusinessRules(); err != nil {
		errs = append(errs, err.Error())
	}

	if len(errs) > 0 {
		return fmt.Errorf("配置验证失败:\n%s", strings.Join(errs, "\n"))
	}

	return nil
}

// validateRequiredFields 验证必需字段
func (cv *ConfigValidator) validateRequiredFields() error {
	if cv.config.TelegramToken == "" {
		return fmt.Errorf("TelegramToken 不能为空，请设置 TG_TOKEN 环境变量")
	}

	if cv.config.AdminChatID == 0 {
		return fmt.Errorf("AdminChatID 不能为空，请设置 TG_CHAT_ID 环境变量")
	}

	return nil
}

// validateOptionalFields 验证可选字段
func (cv *ConfigValidator) validateOptionalFields() error {
	// 验证状态文件路径
	if cv.config.StateFile != "" {
		if err := cv.validateStateFilePath(); err != nil {
			return err
		}
	}

	// 验证脚本路径
	if err := cv.validateScriptPaths(); err != nil {
		return err
	}

	// 验证时区
	if err := cv.validateTimeZone(); err != nil {
		return err
	}

	// 验证日志级别
	if err := cv.validateLogLevel(); err != nil {
		return err
	}

	// 验证命令超时时间
	if err := cv.validateCommandTimeout(); err != nil {
		return err
	}

	return nil
}

// validateBusinessRules 验证业务规则
func (cv *ConfigValidator) validateBusinessRules() error {
	// 验证 Telegram Token 格式
	if err := cv.validateTelegramToken(); err != nil {
		return err
	}

	// 验证 Chat ID 范围
	if err := cv.validateChatID(); err != nil {
		return err
	}

	// 验证脚本文件是否存在（可选）
	if err := cv.validateScriptFiles(); err != nil {
		return err
	}

	return nil
}

// validateStateFilePath 验证状态文件路径
func (cv *ConfigValidator) validateStateFilePath() error {
	if !filepath.IsAbs(cv.config.StateFile) {
		return fmt.Errorf("状态文件路径必须是绝对路径: %s", cv.config.StateFile)
	}

	// 检查路径是否包含 .. 以防止路径遍历（虽然 IsAbs 已经部分缓解，但显式检查更好）
	if strings.Contains(cv.config.StateFile, "..") {
		return fmt.Errorf("状态文件路径包含非法字符 '..'")
	}

	// 检查父目录是否存在
	dir := filepath.Dir(cv.config.StateFile)
	if _, err := os.Stat(dir); os.IsNotExist(err) {
		return fmt.Errorf("状态文件目录不存在: %s", dir)
	}

	return nil
}

// validateScriptPaths 验证脚本路径
func (cv *ConfigValidator) validateScriptPaths() error {
	// 验证核心维护脚本
	if cv.config.CoreScript != "" && !filepath.IsAbs(cv.config.CoreScript) {
		return fmt.Errorf("核心维护脚本路径必须是绝对路径: %s", cv.config.CoreScript)
	}

	// 验证规则维护脚本
	if cv.config.RulesScript != "" && !filepath.IsAbs(cv.config.RulesScript) {
		return fmt.Errorf("规则维护脚本路径必须是绝对路径: %s", cv.config.RulesScript)
	}

	return nil
}

// validateTimeZone 验证时区
func (cv *ConfigValidator) validateTimeZone() error {
	if cv.config.TimeZone == "" {
		return nil // 使用默认值
	}

	_, err := time.LoadLocation(cv.config.TimeZone)
	if err != nil {
		return fmt.Errorf("无效的时区设置: %s (错误: %v)", cv.config.TimeZone, err)
	}

	return nil
}

// validateLogLevel 验证日志级别
func (cv *ConfigValidator) validateLogLevel() error {
	if cv.config.LogLevel == "" {
		return nil // 使用默认值
	}

	validLevels := map[string]bool{
		"debug":   true,
		"info":    true,
		"warning": true,
		"error":   true,
		"fatal":   true,
	}

	if !validLevels[cv.config.LogLevel] {
		return fmt.Errorf("无效的日志级别: %s (支持的值: debug, info, warning, error, fatal)", cv.config.LogLevel)
	}

	return nil
}

// validateCommandTimeout 验证命令超时时间
func (cv *ConfigValidator) validateCommandTimeout() error {
	if cv.config.CommandTimeout <= 0 {
		return fmt.Errorf("命令超时时间必须大于0: %d", cv.config.CommandTimeout)
	}

	if cv.config.CommandTimeout > 3600 { // 1小时
		return fmt.Errorf("命令超时时间不能超过3600秒: %d", cv.config.CommandTimeout)
	}

	return nil
}

// validateTelegramToken 验证 Telegram Token 格式
func (cv *ConfigValidator) validateTelegramToken() error {
	token := cv.config.TelegramToken

	// Telegram Bot Token 格式: <bot_id>:<token>
	parts := strings.Split(token, ":")
	if len(parts) != 2 {
		return fmt.Errorf("Telegram Token 格式不正确，应为 '<bot_id>:<token>' 格式")
	}

	// 验证 bot_id 是纯数字
	if _, err := strconv.ParseInt(parts[0], 10, 64); err != nil {
		return fmt.Errorf("Telegram Token 中的 bot_id 必须是纯数字")
	}

	// 验证 token 部分不为空且包含字母数字字符
	if len(parts[1]) < 20 || !strings.ContainsAny(parts[1], "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789") {
		return fmt.Errorf("Telegram Token 中的 token 部分格式不正确")
	}

	return nil
}

// validateChatID 验证 Chat ID 范围
func (cv *ConfigValidator) validateChatID() error {
	chatID := cv.config.AdminChatID

	// Chat ID 必须是正数（群组和频道也可能是负数，但用户ID通常是正数）
	if chatID == 0 {
		return fmt.Errorf("Chat ID 不能为 0")
	}

	// 检查是否为合理的用户ID范围（通常用户ID是正数，超级群组可能是负数）
	if chatID > 9000000000000000000 { // 9e18，这个值超出了 int64 的合理范围
		return fmt.Errorf("Chat ID 超出合理范围: %d", chatID)
	}

	return nil
}

// validateScriptFiles 验证脚本文件是否存在
func (cv *ConfigValidator) validateScriptFiles() error {
	// 如果跳过脚本检查（测试模式），直接返回
	if cv.skipScriptCheck {
		return nil
	}

	var missingFiles []string

	// 检查核心维护脚本
	if cv.config.CoreScript != "" {
		if _, err := os.Stat(cv.config.CoreScript); os.IsNotExist(err) {
			missingFiles = append(missingFiles, fmt.Sprintf("核心维护脚本: %s", cv.config.CoreScript))
		}
	}

	// 检查规则维护脚本
	if cv.config.RulesScript != "" {
		if _, err := os.Stat(cv.config.RulesScript); os.IsNotExist(err) {
			missingFiles = append(missingFiles, fmt.Sprintf("规则维护脚本: %s", cv.config.RulesScript))
		}
	}

	if len(missingFiles) > 0 {
		return fmt.Errorf("以下脚本文件不存在:\n%s", strings.Join(missingFiles, "\n"))
	}

	return nil
}

// GetValidationSummary 获取配置验证摘要
func (cv *ConfigValidator) GetValidationSummary() map[string]interface{} {
	return map[string]interface{}{
		"telegram_token_set": cv.config.TelegramToken != "",
		"admin_chat_id_set":  cv.config.AdminChatID != 0,
		"state_file":         cv.config.StateFile,
		"core_script":        cv.config.CoreScript,
		"rules_script":       cv.config.RulesScript,
		"time_zone":          cv.config.TimeZone,
		"log_level":          cv.config.LogLevel,
		"command_timeout":    cv.config.CommandTimeout,
	}
}