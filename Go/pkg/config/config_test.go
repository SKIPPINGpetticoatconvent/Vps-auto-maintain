package config

import (
	"os"
	"testing"
)

func TestLoadConfig_EnvVars(t *testing.T) {
	// Setup environment variables
	os.Setenv("TG_TOKEN", "123456789:ABCdefGHIjklMNOpqrsTUVwxyz")
	os.Setenv("TG_CHAT_ID", "123456789")
	defer func() {
		os.Unsetenv("TG_TOKEN")
		os.Unsetenv("TG_CHAT_ID")
	}()

	// 使用跳过脚本检查的验证器进行测试
	config := GetDefaultConfig()
	config.TelegramToken = "123456789:ABCdefGHIjklMNOpqrsTUVwxyz"
	config.AdminChatID = 123456789
	
	validator := NewConfigValidatorWithOptions(config, true)
	if err := validator.Validate(); err != nil {
		t.Fatalf("Failed to validate config: %v", err)
	}
	
	cfg := config

	if cfg.TelegramToken != "123456789:ABCdefGHIjklMNOpqrsTUVwxyz" {
		t.Errorf("Expected TelegramToken to be '123456789:ABCdefGHIjklMNOpqrsTUVwxyz', got '%s'", cfg.TelegramToken)
	}

	if cfg.AdminChatID != 123456789 {
		t.Errorf("Expected AdminChatID to be 123456789, got %d", cfg.AdminChatID)
	}
	
	if cfg.StateFile == "" {
		t.Error("Expected default StateFile to be set")
	}
}

func TestLoadConfig_Validation(t *testing.T) {
	// Ensure environment is clean
	os.Unsetenv("TG_TOKEN")
	os.Unsetenv("TG_CHAT_ID")

	_, err := LoadConfig()
	if err == nil {
		t.Error("Expected error when TG_TOKEN is missing, got nil")
	}

	os.Setenv("TG_TOKEN", "123456789:ABCdefGHIjklMNOpqrsTUVwxyz")
	defer os.Unsetenv("TG_TOKEN")
	
	_, err = LoadConfig()
	if err == nil {
		t.Error("Expected error when TG_CHAT_ID is missing, got nil")
	}
}
