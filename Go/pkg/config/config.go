package config

import (
	"bufio"
	"fmt"
	"os"
	"strconv"
	"strings"

	"golang.org/x/term"
)

// Config 存储 Bot 配置信息
type Config struct {
	Token       string
	AdminChatID int64
	CoreScript  string
	RulesScript string
}

// isInteractive 检查是否在交互式终端中运行
func isInteractive() bool {
	return term.IsTerminal(int(os.Stdin.Fd()))
}

// readInput 读取用户输入
func readInput(prompt string, sensitive bool) (string, error) {
	reader := bufio.NewReader(os.Stdin)
	fmt.Print(prompt)

	var input string
	var err error

	if sensitive {
		// 对于敏感信息（如 Token），使用隐藏输入
		bytePassword, err := term.ReadPassword(int(os.Stdin.Fd()))
		if err != nil {
			return "", fmt.Errorf("读取输入失败: %v", err)
		}
		input = string(bytePassword)
		fmt.Println() // 换行
	} else {
		input, err = reader.ReadString('\n')
		if err != nil {
			return "", fmt.Errorf("读取输入失败: %v", err)
		}
		input = strings.TrimSpace(input)
	}

	return input, nil
}

// promptForConfig 交互式提示用户输入配置
func promptForConfig() (*Config, error) {
	fmt.Println("============================================================")
	fmt.Println("VPS Telegram Bot 配置")
	fmt.Println("============================================================")
	fmt.Println()

	// 读取 Token
	token, err := readInput("请输入 Telegram Bot Token: ", true)
	if err != nil {
		return nil, err
	}
	if token == "" {
		return nil, &ConfigError{Message: "TG_TOKEN 不能为空"}
	}

	// 读取 Chat ID
	chatIDStr, err := readInput("请输入 Telegram Chat ID (管理员): ", false)
	if err != nil {
		return nil, err
	}
	if chatIDStr == "" {
		return nil, &ConfigError{Message: "TG_CHAT_ID 不能为空"}
	}

	chatID, err := strconv.ParseInt(chatIDStr, 10, 64)
	if err != nil {
		return nil, &ConfigError{Message: "TG_CHAT_ID 格式错误: " + err.Error()}
	}

	// 读取核心维护脚本路径（可选）
	coreScript, err := readInput("请输入核心维护脚本路径 (默认: /usr/local/bin/vps-maintain-core.sh): ", false)
	if err != nil {
		return nil, err
	}
	if coreScript == "" {
		coreScript = "/usr/local/bin/vps-maintain-core.sh"
	}

	// 读取规则更新脚本路径（可选）
	rulesScript, err := readInput("请输入规则更新脚本路径 (默认: /usr/local/bin/vps-maintain-rules.sh): ", false)
	if err != nil {
		return nil, err
	}
	if rulesScript == "" {
		rulesScript = "/usr/local/bin/vps-maintain-rules.sh"
	}

	fmt.Println()
	fmt.Println("✅ 配置读取完成")

	return &Config{
		Token:       token,
		AdminChatID: chatID,
		CoreScript:  coreScript,
		RulesScript: rulesScript,
	}, nil
}

// Load 从环境变量加载配置，如果未设置则交互式输入
func Load() (*Config, error) {
	token := os.Getenv("TG_TOKEN")
	chatIDStr := os.Getenv("TG_CHAT_ID")

	// 如果环境变量未设置，且处于交互式终端，则提示用户输入
	if (token == "" || chatIDStr == "") && isInteractive() {
		fmt.Println("⚠️  检测到环境变量未设置，将使用交互式配置")
		fmt.Println()
		return promptForConfig()
	}

	// 从环境变量加载
	if token == "" {
		return nil, &ConfigError{Message: "TG_TOKEN 环境变量未设置，且不在交互式终端中"}
	}

	if chatIDStr == "" {
		return nil, &ConfigError{Message: "TG_CHAT_ID 环境变量未设置，且不在交互式终端中"}
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
