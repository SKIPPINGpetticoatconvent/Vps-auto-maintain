package config

import (
	"fmt"
	"os"
	"strconv"
)

type Config struct {
	TelegramToken string
	AdminChatID   int64
	StateFile     string
}

func LoadConfig() (*Config, error) {
	token := os.Getenv("TG_TOKEN")
	if token == "" {
		return nil, fmt.Errorf("TG_TOKEN is required")
	}

	chatIDStr := os.Getenv("TG_CHAT_ID")
	if chatIDStr == "" {
		return nil, fmt.Errorf("TG_CHAT_ID is required")
	}

	stateFile := os.Getenv("STATE_FILE")
	if stateFile == "" {
		stateFile = "state.json"
	}

	chatID, err := strconv.ParseInt(chatIDStr, 10, 64)
	if err != nil {
		return nil, fmt.Errorf("invalid TG_CHAT_ID: %v", err)
	}

	return &Config{
		TelegramToken: token,
		AdminChatID:   chatID,
		StateFile:     stateFile,
	}, nil
}
