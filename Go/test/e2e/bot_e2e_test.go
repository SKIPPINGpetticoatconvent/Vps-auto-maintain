// Package e2e 提供 Telegram Bot 的端到端测试
// 模拟真实用户与 Bot 按钮的交互，验证程序行为和脚本执行
package e2e

import (
	"fmt"
	"os"
	"sync"
	"testing"
	"time"
	"vps-tg-bot/pkg/bot"
	"vps-tg-bot/pkg/config"
	"vps-tg-bot/pkg/scheduler"
	"vps-tg-bot/pkg/system"

	tgbotapi "github.com/go-telegram-bot-api/telegram-bot-api/v5"
)

// MockTelegramAPI 模拟 Telegram API，记录所有发送的消息
type MockTelegramAPI struct {
	mu            sync.Mutex
	SentMessages  []tgbotapi.Chattable
	Responses     []tgbotapi.Message
	CallbackCount int
}

func NewMockTelegramAPI() *MockTelegramAPI {
	return &MockTelegramAPI{
		SentMessages: make([]tgbotapi.Chattable, 0),
		Responses:    make([]tgbotapi.Message, 0),
	}
}

func (m *MockTelegramAPI) Send(c tgbotapi.Chattable) (tgbotapi.Message, error) {
	m.mu.Lock()
	defer m.mu.Unlock()
	m.SentMessages = append(m.SentMessages, c)
	return tgbotapi.Message{MessageID: len(m.SentMessages)}, nil
}

func (m *MockTelegramAPI) Request(c tgbotapi.Chattable) (*tgbotapi.APIResponse, error) {
	m.mu.Lock()
	defer m.mu.Unlock()
	m.CallbackCount++
	return &tgbotapi.APIResponse{Ok: true}, nil
}

func (m *MockTelegramAPI) GetSentCount() int {
	m.mu.Lock()
	defer m.mu.Unlock()
	return len(m.SentMessages)
}

// E2ETestSuite 端到端测试套件
type E2ETestSuite struct {
	api        *MockTelegramAPI
	botHandler bot.BotHandler
	mockSys    *system.MockSystemExecutor
	jobManager scheduler.JobManager
	cfg        *config.Config
	adminChat  int64
	stateFile  string
	t          *testing.T
}

// NewE2ETestSuite 创建测试套件
func NewE2ETestSuite(t *testing.T) *E2ETestSuite {
	cwd, _ := os.Getwd()
	stateFile := fmt.Sprintf("%s/test_e2e_state_%d.json", cwd, time.Now().UnixNano())
	historyFile := fmt.Sprintf("%s/test_e2e_history_%d.json", cwd, time.Now().UnixNano())

	// 创建测试脚本
	coreScript := cwd + "/test_core.sh"
	rulesScript := cwd + "/test_rules.sh"
	os.WriteFile(coreScript, []byte("#!/bin/bash\necho 'Core maintenance completed'"), 0755)
	os.WriteFile(rulesScript, []byte("#!/bin/bash\necho 'Rules update completed'"), 0755)

	// 设置环境变量
	os.Setenv("TG_TOKEN", "123456789:ABCdefGHIjklMNOpqrsTUVwxyz1234567")
	os.Setenv("TG_CHAT_ID", "123456789")
	os.Setenv("STATE_FILE", stateFile)
	os.Setenv("CORE_SCRIPT", coreScript)
	os.Setenv("RULES_SCRIPT", rulesScript)
	os.Setenv("HISTORY_FILE", historyFile)

	cfg, err := config.LoadConfig()
	if err != nil {
		t.Fatalf("加载配置失败: %v", err)
	}

	mockSys := system.NewMockSystemExecutor()
	setupMockSystemCommands(mockSys)

	jobManager := scheduler.NewCronJobManagerWithExecutor(stateFile, mockSys)
	jobManager.Start()

	api := NewMockTelegramAPI()
	botHandler := bot.NewTGBotHandler(api, cfg, mockSys, jobManager)

	return &E2ETestSuite{
		api:        api,
		botHandler: botHandler,
		mockSys:    mockSys,
		jobManager: jobManager,
		cfg:        cfg,
		adminChat:  cfg.AdminChatID,
		stateFile:  stateFile,
		t:          t,
	}
}

// Cleanup 清理测试环境
func (s *E2ETestSuite) Cleanup() {
	s.jobManager.Stop()
	os.Unsetenv("TG_TOKEN")
	os.Unsetenv("TG_CHAT_ID")
	os.Unsetenv("STATE_FILE")
	os.Unsetenv("CORE_SCRIPT")
	os.Unsetenv("RULES_SCRIPT")
	os.Unsetenv("HISTORY_FILE")
	os.Remove(s.stateFile)
	os.Remove("maintain_history.json")
	cwd, _ := os.Getwd()
	os.Remove(cwd + "/test_core.sh")
	os.Remove(cwd + "/test_rules.sh")
}

// setupMockSystemCommands 设置模拟系统命令
func setupMockSystemCommands(mockSys *system.MockSystemExecutor) {
	mockSys.CommandOutput["uptime -p"] = "up 2 days, 5 hours"
	mockSys.CommandOutput["cat /proc/loadavg"] = "0.25 0.15 0.10 2/256 12345"
	mockSys.CommandOutput["free -h"] = "              total        used        free      shared  buff/cache   available\nMem:           2Gi       512Mi       1.2Gi        16Mi       256Mi       1.5Gi"
	mockSys.CommandOutput["df -h /"] = "Filesystem      Size  Used Avail Use% Mounted on\n/dev/sda1        20G   8.0G   12G  40% /"
	mockSys.CommandOutput["ps -e --no-headers"] = "1\n2\n3\n4\n5"
	mockSys.CommandOutput["core_maintain"] = "Core maintenance executed successfully\nPackages updated: 5\nSystem cleaned"
	mockSys.CommandOutput["rules_maintain"] = "Rules updated successfully\nNew rules: 150"
	mockSys.CommandOutput["update_xray"] = "Xray updated to version 1.8.0"
	mockSys.CommandOutput["update_singbox"] = "Sing-box updated to version 1.5.0"
	mockSys.CommandOutput["journalctl"] = "Dec 27 10:00:00 vps systemd[1]: Started VPS Bot\nDec 27 10:01:00 vps vps-tg-bot: Bot running"
	mockSys.CommandOutput["reboot"] = ""
	mockSys.SystemTime = time.Now()
	mockSys.Timezone = "Asia/Shanghai"
}

// SimulateStartCommand 模拟发送 /start 命令
func (s *E2ETestSuite) SimulateStartCommand() error {
	update := tgbotapi.Update{
		Message: &tgbotapi.Message{
			Chat: &tgbotapi.Chat{ID: s.adminChat},
			Text: "/start",
			Entities: []tgbotapi.MessageEntity{
				{Type: "bot_command", Offset: 0, Length: 6},
			},
		},
	}
	return s.botHandler.HandleUpdate(update)
}

// SimulateButtonClick 模拟按钮点击
func (s *E2ETestSuite) SimulateButtonClick(callbackData string) error {
	update := tgbotapi.Update{
		CallbackQuery: &tgbotapi.CallbackQuery{
			ID: fmt.Sprintf("cb_%d", time.Now().UnixNano()),
			From: &tgbotapi.User{ID: s.adminChat},
			Message: &tgbotapi.Message{
				Chat:      &tgbotapi.Chat{ID: s.adminChat},
				MessageID: 1,
			},
			Data: callbackData,
		},
	}
	return s.botHandler.HandleUpdate(update)
}

// SimulateUnauthorizedClick 模拟未授权用户点击
func (s *E2ETestSuite) SimulateUnauthorizedClick(callbackData string) error {
	unauthorizedChat := s.adminChat + 999
	update := tgbotapi.Update{
		CallbackQuery: &tgbotapi.CallbackQuery{
			ID: fmt.Sprintf("cb_%d", time.Now().UnixNano()),
			From: &tgbotapi.User{ID: unauthorizedChat},
			Message: &tgbotapi.Message{
				Chat:      &tgbotapi.Chat{ID: unauthorizedChat},
				MessageID: 1,
			},
			Data: callbackData,
		},
	}
	return s.botHandler.HandleUpdate(update)
}

// ===================== 测试用例 =====================

// TestE2E_StartCommand 测试 /start 命令
func TestE2E_StartCommand(t *testing.T) {
	suite := NewE2ETestSuite(t)
	defer suite.Cleanup()

	err := suite.SimulateStartCommand()
	if err != nil {
		t.Errorf("/start 命令处理失败: %v", err)
	}

	if suite.api.GetSentCount() == 0 {
		t.Error("未发送任何消息")
	}
}

// TestE2E_MainMenuNavigation 测试主菜单导航
func TestE2E_MainMenuNavigation(t *testing.T) {
	suite := NewE2ETestSuite(t)
	defer suite.Cleanup()

	// 测试主菜单各按钮
	testCases := []struct {
		name         string
		callbackData string
		shouldError  bool
	}{
		{"系统状态", "status", false},
		{"立即维护菜单", "maintain_now", false},
		{"调度设置菜单", "schedule_menu", false},
		{"查看日志", "view_logs", false},
		{"维护历史", "view_history", false},
		{"返回主菜单", "back_main", false},
	}

	for _, tc := range testCases {
		t.Run(tc.name, func(t *testing.T) {
			err := suite.SimulateButtonClick(tc.callbackData)
			if tc.shouldError && err == nil {
				t.Errorf("期望错误但未发生: %s", tc.name)
			}
			if !tc.shouldError && err != nil {
				t.Errorf("%s 按钮处理失败: %v", tc.name, err)
			}
		})
	}
}

// TestE2E_MaintenanceFlow 测试维护流程
func TestE2E_MaintenanceFlow(t *testing.T) {
	suite := NewE2ETestSuite(t)
	defer suite.Cleanup()

	// 进入维护菜单
	err := suite.SimulateButtonClick("maintain_now")
	if err != nil {
		t.Fatalf("进入维护菜单失败: %v", err)
	}

	// 测试各维护操作
	maintenanceTests := []struct {
		name         string
		callbackData string
	}{
		{"核心维护", "maintain_core"},
		{"规则维护", "maintain_rules"},
		{"完整维护", "maintain_full"},
		{"Xray更新", "update_xray"},
		{"Sing-box更新", "update_singbox"},
	}

	for _, tc := range maintenanceTests {
		t.Run(tc.name, func(t *testing.T) {
			initialCount := suite.api.GetSentCount()
			err := suite.SimulateButtonClick(tc.callbackData)
			if err != nil {
				t.Errorf("%s 处理失败: %v", tc.name, err)
			}

			// 等待异步操作
			time.Sleep(100 * time.Millisecond)

			if suite.api.GetSentCount() <= initialCount {
				t.Logf("警告: %s 未发送新消息", tc.name)
			}
		})
	}
}

// TestE2E_ScheduleFlow 测试调度设置流程
func TestE2E_ScheduleFlow(t *testing.T) {
	suite := NewE2ETestSuite(t)
	defer suite.Cleanup()

	// 测试设置各种调度任务
	scheduleTests := []struct {
		name         string
		callbackData string
		expectedJob  string
	}{
		{"设置核心维护调度", "schedule_core", "core_maintain"},
		{"设置规则维护调度", "schedule_rules", "rules_maintain"},
		{"设置Xray重启调度", "schedule_xray_restart", "restart_xray"},
		{"设置Sing-box重启调度", "schedule_sb_restart", "restart_singbox"},
	}

	for _, tc := range scheduleTests {
		t.Run(tc.name, func(t *testing.T) {
			err := suite.SimulateButtonClick(tc.callbackData)
			if err != nil {
				t.Errorf("%s 失败: %v", tc.name, err)
			}

			// 验证任务已添加
			status := suite.jobManager.GetJobStatus(tc.expectedJob)
			if status != "✅ Schedule" {
				t.Errorf("任务 %s 未正确设置，状态: %s", tc.expectedJob, status)
			}
		})
	}

	// 测试清除所有调度
	t.Run("清除所有调度", func(t *testing.T) {
		err := suite.SimulateButtonClick("schedule_clear")
		if err != nil {
			t.Errorf("清除调度失败: %v", err)
		}
	})
}

// TestE2E_MultiLevelMenuFlow 测试多级菜单流程
func TestE2E_MultiLevelMenuFlow(t *testing.T) {
	suite := NewE2ETestSuite(t)
	defer suite.Cleanup()

	// 模拟完整的多级菜单操作流程
	// 1. 进入调度设置
	err := suite.SimulateButtonClick("schedule_menu")
	if err != nil {
		t.Fatalf("进入调度菜单失败: %v", err)
	}

	// 2. 选择任务类型：核心维护
	err = suite.SimulateButtonClick("menu_task_core_maintain")
	if err != nil {
		t.Fatalf("选择核心维护任务失败: %v", err)
	}

	// 3. 选择频率：每日
	err = suite.SimulateButtonClick("menu_freq_core_maintain_daily")
	if err != nil {
		t.Fatalf("选择每日频率失败: %v", err)
	}

	// 4. 选择时间：凌晨4点
	err = suite.SimulateButtonClick("menu_time_core_maintain_daily_4")
	if err != nil {
		t.Fatalf("选择时间失败: %v", err)
	}

	// 等待异步任务设置完成
	time.Sleep(200 * time.Millisecond)
}

// TestE2E_ViewTasksList 测试查看任务列表
func TestE2E_ViewTasksList(t *testing.T) {
	suite := NewE2ETestSuite(t)
	defer suite.Cleanup()

	// 先添加一个任务
	err := suite.SimulateButtonClick("schedule_core")
	if err != nil {
		t.Fatalf("设置调度失败: %v", err)
	}

	// 查看任务列表
	err = suite.SimulateButtonClick("menu_view_tasks")
	if err != nil {
		t.Errorf("查看任务列表失败: %v", err)
	}
}

// TestE2E_UnauthorizedAccess 测试未授权访问
func TestE2E_UnauthorizedAccess(t *testing.T) {
	suite := NewE2ETestSuite(t)
	defer suite.Cleanup()

	// 测试各种未授权操作
	unauthorizedTests := []string{
		"status",
		"maintain_core",
		"schedule_core",
		"reboot_confirm",
	}

	for _, callbackData := range unauthorizedTests {
		t.Run("未授权_"+callbackData, func(t *testing.T) {
			err := suite.SimulateUnauthorizedClick(callbackData)
			// 未授权访问应该被静默拒绝，不返回错误
			if err != nil {
				t.Errorf("未授权访问处理异常: %v", err)
			}
		})
	}
}

// TestE2E_RebootConfirm 测试重启确认
func TestE2E_RebootConfirm(t *testing.T) {
	suite := NewE2ETestSuite(t)
	defer suite.Cleanup()

	err := suite.SimulateButtonClick("reboot_confirm")
	if err != nil {
		t.Errorf("重启确认处理失败: %v", err)
	}

	// 验证发送了确认消息
	if suite.api.GetSentCount() == 0 {
		t.Error("重启确认未发送消息")
	}
}

// TestE2E_StatusCheck 测试系统状态查询
func TestE2E_StatusCheck(t *testing.T) {
	suite := NewE2ETestSuite(t)
	defer suite.Cleanup()

	err := suite.SimulateButtonClick("status")
	if err != nil {
		t.Errorf("状态查询失败: %v", err)
	}

	if suite.api.GetSentCount() == 0 {
		t.Error("状态查询未发送消息")
	}
}

// TestE2E_ConcurrentButtonClicks 测试并发按钮点击
func TestE2E_ConcurrentButtonClicks(t *testing.T) {
	suite := NewE2ETestSuite(t)
	defer suite.Cleanup()

	buttons := []string{"status", "maintain_now", "schedule_menu", "view_logs"}
	var wg sync.WaitGroup
	errors := make(chan error, len(buttons))

	for _, btn := range buttons {
		wg.Add(1)
		go func(callbackData string) {
			defer wg.Done()
			if err := suite.SimulateButtonClick(callbackData); err != nil {
				errors <- fmt.Errorf("并发点击 %s 失败: %v", callbackData, err)
			}
		}(btn)
	}

	wg.Wait()
	close(errors)

	for err := range errors {
		t.Error(err)
	}
}

// TestE2E_RapidButtonClicks 测试快速连续点击
func TestE2E_RapidButtonClicks(t *testing.T) {
	suite := NewE2ETestSuite(t)
	defer suite.Cleanup()

	// 快速点击同一按钮多次
	for i := 0; i < 10; i++ {
		err := suite.SimulateButtonClick("status")
		if err != nil {
			t.Errorf("第 %d 次快速点击失败: %v", i+1, err)
		}
	}
}

// TestE2E_FullUserJourney 测试完整用户旅程
func TestE2E_FullUserJourney(t *testing.T) {
	suite := NewE2ETestSuite(t)
	defer suite.Cleanup()

	// 模拟完整的用户操作流程
	steps := []struct {
		name   string
		action func() error
	}{
		{"发送 /start", suite.SimulateStartCommand},
		{"查看系统状态", func() error { return suite.SimulateButtonClick("status") }},
		{"进入维护菜单", func() error { return suite.SimulateButtonClick("maintain_now") }},
		{"执行核心维护", func() error { return suite.SimulateButtonClick("maintain_core") }},
		{"返回主菜单", func() error { return suite.SimulateButtonClick("back_main") }},
		{"进入调度设置", func() error { return suite.SimulateButtonClick("schedule_menu") }},
		{"设置核心调度", func() error { return suite.SimulateButtonClick("schedule_core") }},
		{"查看日志", func() error { return suite.SimulateButtonClick("view_logs") }},
		{"查看历史", func() error { return suite.SimulateButtonClick("view_history") }},
	}

	for _, step := range steps {
		t.Run(step.name, func(t *testing.T) {
			err := step.action()
			if err != nil {
				t.Errorf("步骤 '%s' 失败: %v", step.name, err)
			}
			// 短暂等待异步操作
			time.Sleep(50 * time.Millisecond)
		})
	}
}

// TestE2E_InvalidCallbackData 测试无效回调数据
func TestE2E_InvalidCallbackData(t *testing.T) {
	suite := NewE2ETestSuite(t)
	defer suite.Cleanup()

	invalidCallbacks := []string{
		"invalid_action",
		"",
		"menu_invalid",
		"unknown_command",
	}

	for _, callbackData := range invalidCallbacks {
		t.Run("无效回调_"+callbackData, func(t *testing.T) {
			// 无效回调应该被静默忽略，不应该panic
			err := suite.SimulateButtonClick(callbackData)
			if err != nil {
				t.Logf("无效回调 '%s' 返回错误: %v", callbackData, err)
			}
		})
	}
}

// TestE2E_FrequencySelectionMenu 测试频率选择菜单
func TestE2E_FrequencySelectionMenu(t *testing.T) {
	suite := NewE2ETestSuite(t)
	defer suite.Cleanup()

	taskTypes := []string{"core_maintain", "rules_maintain", "update_xray", "update_singbox"}
	frequencies := []string{"daily", "weekly"}

	for _, taskType := range taskTypes {
		for _, freq := range frequencies {
			t.Run(fmt.Sprintf("%s_%s", taskType, freq), func(t *testing.T) {
				callbackData := fmt.Sprintf("menu_freq_%s_%s", taskType, freq)
				err := suite.SimulateButtonClick(callbackData)
				if err != nil {
					t.Errorf("频率选择失败: %v", err)
				}
			})
		}
	}
}

// TestE2E_TimeSelectionMenu 测试时间选择菜单
func TestE2E_TimeSelectionMenu(t *testing.T) {
	suite := NewE2ETestSuite(t)
	defer suite.Cleanup()

	// 测试不同时间选项
	timeTests := []struct {
		taskType  string
		frequency string
		timeValue string
	}{
		{"core_maintain", "daily", "4"},
		{"core_maintain", "daily", "12"},
		{"rules_maintain", "weekly", "0 7"},
	}

	for _, tc := range timeTests {
		t.Run(fmt.Sprintf("%s_%s_%s", tc.taskType, tc.frequency, tc.timeValue), func(t *testing.T) {
			callbackData := fmt.Sprintf("menu_time_%s_%s_%s", tc.taskType, tc.frequency, tc.timeValue)
			err := suite.SimulateButtonClick(callbackData)
			if err != nil {
				t.Errorf("时间选择失败: %v", err)
			}
			// 等待异步任务设置
			time.Sleep(100 * time.Millisecond)
		})
	}
}

// TestE2E_BackNavigation 测试返回导航
func TestE2E_BackNavigation(t *testing.T) {
	suite := NewE2ETestSuite(t)
	defer suite.Cleanup()

	backTests := []struct {
		name         string
		callbackData string
	}{
		{"返回主菜单", "back_main"},
		{"返回任务类型", "menu_back_task_types"},
	}

	for _, tc := range backTests {
		t.Run(tc.name, func(t *testing.T) {
			err := suite.SimulateButtonClick(tc.callbackData)
			if err != nil {
				t.Errorf("%s 失败: %v", tc.name, err)
			}
		})
	}
}
