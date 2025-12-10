package main

import (
	"os"
	"testing"
	"vps-tg-bot/pkg/bot"
	"vps-tg-bot/pkg/config"
	"vps-tg-bot/pkg/scheduler"
	"vps-tg-bot/pkg/system"

	tgbotapi "github.com/go-telegram-bot-api/telegram-bot-api/v5"
)

// MockTelegramAPI implements bot.TelegramAPI for testing
type MockTelegramAPI struct{}

func (m *MockTelegramAPI) Send(c tgbotapi.Chattable) (tgbotapi.Message, error) {
	return tgbotapi.Message{}, nil
}

func (m *MockTelegramAPI) Request(c tgbotapi.Chattable) (*tgbotapi.APIResponse, error) {
	return &tgbotapi.APIResponse{Ok: true}, nil
}

func TestIntegration_SystemStartupAndWiring(t *testing.T) {
	// 1. Setup Environment
	// Use a valid format token for validation logic
	cwd, _ := os.Getwd()
	stateFile := cwd + "/test_integration_state.json"
	
	// Create dummy script files to pass validation
	coreScript := cwd + "/test_core.sh"
	rulesScript := cwd + "/test_rules.sh"
	os.WriteFile(coreScript, []byte("#!/bin/bash\necho ok"), 0755)
	os.WriteFile(rulesScript, []byte("#!/bin/bash\necho ok"), 0755)

	os.Setenv("TG_TOKEN", "123456789:ABCdefGHIjklMNOpqrsTUVwxyz1234567")
	os.Setenv("TG_CHAT_ID", "123456789")
	os.Setenv("STATE_FILE", stateFile)
	os.Setenv("CORE_SCRIPT", coreScript)
	os.Setenv("RULES_SCRIPT", rulesScript)
	
	// Clean up
	defer func() {
		os.Unsetenv("TG_TOKEN")
		os.Unsetenv("TG_CHAT_ID")
		os.Unsetenv("STATE_FILE")
		os.Unsetenv("CORE_SCRIPT")
		os.Unsetenv("RULES_SCRIPT")
		os.Remove(stateFile)
		os.Remove(coreScript)
		os.Remove(rulesScript)
		os.Remove("maintain_history.json")
	}()

	// 2. Load Config
	cfg, err := config.LoadConfig()
	if err != nil {
		t.Fatalf("Failed to load config: %v", err)
	}

	// Verify Config
	if cfg.TelegramToken != "123456789:ABCdefGHIjklMNOpqrsTUVwxyz1234567" {
		t.Errorf("Config token mismatch")
	}

	// 3. Initialize Components
	// Use MockSystemExecutor to avoid side effects
	mockSys := system.NewMockSystemExecutor()
	
	// Pre-fill some mock data for status check
	mockSys.CommandOutput["uptime -p"] = "up 1 hour"
	mockSys.CommandOutput["cat /proc/loadavg"] = "0.1 0.2 0.3 1/100 1234"
	mockSys.CommandOutput["free -h"] = "Mem: 100 50 20 0 10 20"
	mockSys.CommandOutput["df -h /"] = "Filesystem Size Used Avail Use% Mounted\n/dev/sda1 10G 5G 5G 50% /"
	mockSys.CommandOutput["ps -e --no-headers"] = "1\n2\n3"
	
	// JobManager
	jobManager := scheduler.NewCronJobManager(cfg.StateFile)
	
	// Mock Telegram API
	mockAPI := &MockTelegramAPI{}

	// 4. Initialize Bot Handler
	botHandler := bot.NewTGBotHandler(mockAPI, cfg, mockSys, jobManager)

	if botHandler == nil {
		t.Fatal("BotHandler failed to initialize")
	}

	// 5. Test Interaction: /start command
	updateStart := tgbotapi.Update{
		Message: &tgbotapi.Message{
			Chat: &tgbotapi.Chat{ID: cfg.AdminChatID},
			Text: "/start",
			Entities: []tgbotapi.MessageEntity{
				{Type: "bot_command", Offset: 0, Length: 6},
			},
		},
	}

	err = botHandler.HandleUpdate(updateStart)
	if err != nil {
		t.Errorf("HandleUpdate (/start) failed: %v", err)
	}

	// 6. Test Interaction: Status Callback
	// This verifies that BotHandler correctly uses SystemExecutor
	updateStatus := tgbotapi.Update{
		CallbackQuery: &tgbotapi.CallbackQuery{
			ID: "1",
			From: &tgbotapi.User{ID: 123},
			Message: &tgbotapi.Message{
				Chat: &tgbotapi.Chat{ID: cfg.AdminChatID},
				MessageID: 1,
			},
			Data: "status",
		},
	}
	
	err = botHandler.HandleUpdate(updateStatus)
	if err != nil {
		t.Errorf("HandleUpdate (callback status) failed: %v", err)
	}

	// 7. Test Interaction: Schedule Setup
	// This verifies that BotHandler correctly uses JobManager
	updateSchedule := tgbotapi.Update{
		CallbackQuery: &tgbotapi.CallbackQuery{
			ID: "2",
			From: &tgbotapi.User{ID: 123},
			Message: &tgbotapi.Message{
				Chat: &tgbotapi.Chat{ID: cfg.AdminChatID},
				MessageID: 1,
			},
			Data: "schedule_core",
		},
	}

	err = botHandler.HandleUpdate(updateSchedule)
	if err != nil {
		t.Errorf("HandleUpdate (schedule_core) failed: %v", err)
	}

	// Verify job was set
	if status := jobManager.GetJobStatus("core_maintain"); status != "✅ Schedule" {
		t.Errorf("Job 'core_maintain' was not scheduled")
	}
}