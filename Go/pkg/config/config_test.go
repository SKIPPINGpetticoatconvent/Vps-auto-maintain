package config

import (
	"os"
	"testing"
)

func TestLoadConfig_EnvVars(t *testing.T) {
	// Setup environment variables
	os.Setenv("TG_TOKEN", "test_token")
	os.Setenv("TG_CHAT_ID", "123456789")
	defer func() {
		os.Unsetenv("TG_TOKEN")
		os.Unsetenv("TG_CHAT_ID")
	}()

	cfg, err := LoadConfig()
	if err != nil {
		t.Fatalf("Failed to load config: %v", err)
	}

	if cfg.TelegramToken != "test_token" {
		t.Errorf("Expected TelegramToken to be 'test_token', got '%s'", cfg.TelegramToken)
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

	os.Setenv("TG_TOKEN", "test_token")
	defer os.Unsetenv("TG_TOKEN")
	
	_, err = LoadConfig()
	if err == nil {
		t.Error("Expected error when TG_CHAT_ID is missing, got nil")
	}
}
