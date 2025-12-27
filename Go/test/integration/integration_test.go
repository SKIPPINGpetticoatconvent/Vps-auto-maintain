// Package integration 提供 VPS Telegram Bot 的集成测试
// 测试各模块之间的协作和真实场景
package integration

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

// MockTelegramAPI 模拟 Telegram API
type MockTelegramAPI struct {
	mu           sync.Mutex
	SentMessages []tgbotapi.Chattable
	CallbackIDs  []string
}

func NewMockTelegramAPI() *MockTelegramAPI {
	return &MockTelegramAPI{
		SentMessages: make([]tgbotapi.Chattable, 0),
		CallbackIDs:  make([]string, 0),
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
	return &tgbotapi.APIResponse{Ok: true}, nil
}

func (m *MockTelegramAPI) GetSentCount() int {
	m.mu.Lock()
	defer m.mu.Unlock()
	return len(m.SentMessages)
}

// IntegrationTestSuite 集成测试套件
type IntegrationTestSuite struct {
	api        *MockTelegramAPI
	botHandler bot.BotHandler
	mockSys    *system.MockSystemExecutor
	jobManager scheduler.JobManager
	cfg        *config.Config
	stateFile  string
	t          *testing.T
}

// NewIntegrationTestSuite 创建集成测试套件
func NewIntegrationTestSuite(t *testing.T) *IntegrationTestSuite {
	cwd, _ := os.Getwd()
	stateFile := fmt.Sprintf("%s/test_integration_state_%d.json", cwd, time.Now().UnixNano())

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

	cfg, err := config.LoadConfig()
	if err != nil {
		t.Fatalf("加载配置失败: %v", err)
	}

	mockSys := system.NewMockSystemExecutor()
	setupMockSystem(mockSys)

	jobManager := scheduler.NewCronJobManagerWithExecutor(stateFile, mockSys)
	jobManager.Start()

	api := NewMockTelegramAPI()
	botHandler := bot.NewTGBotHandler(api, cfg, mockSys, jobManager)

	return &IntegrationTestSuite{
		api:        api,
		botHandler: botHandler,
		mockSys:    mockSys,
		jobManager: jobManager,
		cfg:        cfg,
		stateFile:  stateFile,
		t:          t,
	}
}

// Cleanup 清理测试环境
func (s *IntegrationTestSuite) Cleanup() {
	s.jobManager.Stop()
	os.Unsetenv("TG_TOKEN")
	os.Unsetenv("TG_CHAT_ID")
	os.Unsetenv("STATE_FILE")
	os.Unsetenv("CORE_SCRIPT")
	os.Unsetenv("RULES_SCRIPT")
	os.Remove(s.stateFile)
	os.Remove("maintain_history.json")
	cwd, _ := os.Getwd()
	os.Remove(cwd + "/test_core.sh")
	os.Remove(cwd + "/test_rules.sh")
}

// setupMockSystem 设置模拟系统
func setupMockSystem(mockSys *system.MockSystemExecutor) {
	mockSys.CommandOutput["uptime -p"] = "up 2 days, 5 hours"
	mockSys.CommandOutput["cat /proc/loadavg"] = "0.25 0.15 0.10 2/256 12345"
	mockSys.CommandOutput["free -h"] = "              total        used        free\nMem:           2Gi       512Mi       1.2Gi"
	mockSys.CommandOutput["df -h /"] = "Filesystem      Size  Used Avail Use% Mounted on\n/dev/sda1        20G   8.0G   12G  40% /"
	mockSys.CommandOutput["ps -e --no-headers"] = "1\n2\n3\n4\n5"
	mockSys.CommandOutput["core_maintain"] = "Core maintenance executed\nPackages updated: 5"
	mockSys.CommandOutput["rules_maintain"] = "Rules updated\nNew rules: 150"
	mockSys.CommandOutput["update_xray"] = "Xray updated to v1.8.0"
	mockSys.CommandOutput["update_singbox"] = "Sing-box updated to v1.5.0"
	mockSys.CommandOutput["journalctl"] = "Dec 27 10:00:00 vps Bot started"
	mockSys.CommandOutput["reboot"] = ""
	mockSys.SystemTime = time.Now()
	mockSys.Timezone = "Asia/Shanghai"
}

// SimulateUpdate 模拟 Telegram 更新
func (s *IntegrationTestSuite) SimulateUpdate(update tgbotapi.Update) error {
	return s.botHandler.HandleUpdate(update)
}

// ===================== 集成测试用例 =====================

// TestIntegration_ConfigToBot 测试配置到 Bot 的集成
func TestIntegration_ConfigToBot(t *testing.T) {
	suite := NewIntegrationTestSuite(t)
	defer suite.Cleanup()

	// 验证配置正确加载
	if suite.cfg.TelegramToken == "" {
		t.Error("Telegram Token 未加载")
	}
	if suite.cfg.AdminChatID == 0 {
		t.Error("Admin Chat ID 未加载")
	}

	// 验证 Bot Handler 创建成功
	if suite.botHandler == nil {
		t.Error("Bot Handler 创建失败")
	}
}

// TestIntegration_BotToSystem 测试 Bot 到系统执行器的集成
func TestIntegration_BotToSystem(t *testing.T) {
	suite := NewIntegrationTestSuite(t)
	defer suite.Cleanup()

	// 模拟状态查询
	update := tgbotapi.Update{
		CallbackQuery: &tgbotapi.CallbackQuery{
			ID: "cb_status",
			From: &tgbotapi.User{ID: suite.cfg.AdminChatID},
			Message: &tgbotapi.Message{
				Chat:      &tgbotapi.Chat{ID: suite.cfg.AdminChatID},
				MessageID: 1,
			},
			Data: "status",
		},
	}

	err := suite.SimulateUpdate(update)
	if err != nil {
		t.Errorf("状态查询集成失败: %v", err)
	}

	// 验证消息发送
	if suite.api.GetSentCount() == 0 {
		t.Error("状态查询未发送消息")
	}
}

// TestIntegration_BotToScheduler 测试 Bot 到调度器的集成
func TestIntegration_BotToScheduler(t *testing.T) {
	suite := NewIntegrationTestSuite(t)
	defer suite.Cleanup()

	// 模拟设置调度
	update := tgbotapi.Update{
		CallbackQuery: &tgbotapi.CallbackQuery{
			ID: "cb_schedule",
			From: &tgbotapi.User{ID: suite.cfg.AdminChatID},
			Message: &tgbotapi.Message{
				Chat:      &tgbotapi.Chat{ID: suite.cfg.AdminChatID},
				MessageID: 1,
			},
			Data: "schedule_core",
		},
	}

	err := suite.SimulateUpdate(update)
	if err != nil {
		t.Errorf("调度设置集成失败: %v", err)
	}

	// 验证任务已添加
	status := suite.jobManager.GetJobStatus("core_maintain")
	if status != "✅ Schedule" {
		t.Errorf("调度任务未正确设置，状态: %s", status)
	}
}

// TestIntegration_SchedulerToSystem 测试调度器到系统的集成
func TestIntegration_SchedulerToSystem(t *testing.T) {
	suite := NewIntegrationTestSuite(t)
	defer suite.Cleanup()

	// 设置调度任务
	task := func() {
		// 任务执行
	}

	err := suite.jobManager.SetJob("test_task", "0 0 * * * *", task)
	if err != nil {
		t.Errorf("设置调度任务失败: %v", err)
	}

	// 验证任务状态
	status := suite.jobManager.GetJobStatus("test_task")
	if status != "✅ Schedule" {
		t.Errorf("任务状态不正确: %s", status)
	}
}

// TestIntegration_MaintenanceWorkflow 测试完整维护工作流
func TestIntegration_MaintenanceWorkflow(t *testing.T) {
	suite := NewIntegrationTestSuite(t)
	defer suite.Cleanup()

	// 1. 进入维护菜单
	update1 := tgbotapi.Update{
		CallbackQuery: &tgbotapi.CallbackQuery{
			ID: "cb_maintain_menu",
			From: &tgbotapi.User{ID: suite.cfg.AdminChatID},
			Message: &tgbotapi.Message{
				Chat:      &tgbotapi.Chat{ID: suite.cfg.AdminChatID},
				MessageID: 1,
			},
			Data: "maintain_now",
		},
	}
	err := suite.SimulateUpdate(update1)
	if err != nil {
		t.Errorf("进入维护菜单失败: %v", err)
	}

	// 2. 执行核心维护
	update2 := tgbotapi.Update{
		CallbackQuery: &tgbotapi.CallbackQuery{
			ID: "cb_maintain_core",
			From: &tgbotapi.User{ID: suite.cfg.AdminChatID},
			Message: &tgbotapi.Message{
				Chat:      &tgbotapi.Chat{ID: suite.cfg.AdminChatID},
				MessageID: 1,
			},
			Data: "maintain_core",
		},
	}
	err = suite.SimulateUpdate(update2)
	if err != nil {
		t.Errorf("核心维护失败: %v", err)
	}

	// 等待异步操作
	time.Sleep(100 * time.Millisecond)
}

// TestIntegration_ScheduleWorkflow 测试完整调度工作流
func TestIntegration_ScheduleWorkflow(t *testing.T) {
	suite := NewIntegrationTestSuite(t)
	defer suite.Cleanup()

	// 1. 设置核心维护调度
	update1 := tgbotapi.Update{
		CallbackQuery: &tgbotapi.CallbackQuery{
			ID: "cb_schedule_core",
			From: &tgbotapi.User{ID: suite.cfg.AdminChatID},
			Message: &tgbotapi.Message{
				Chat:      &tgbotapi.Chat{ID: suite.cfg.AdminChatID},
				MessageID: 1,
			},
			Data: "schedule_core",
		},
	}
	err := suite.SimulateUpdate(update1)
	if err != nil {
		t.Errorf("设置核心调度失败: %v", err)
	}

	// 2. 设置规则维护调度
	update2 := tgbotapi.Update{
		CallbackQuery: &tgbotapi.CallbackQuery{
			ID: "cb_schedule_rules",
			From: &tgbotapi.User{ID: suite.cfg.AdminChatID},
			Message: &tgbotapi.Message{
				Chat:      &tgbotapi.Chat{ID: suite.cfg.AdminChatID},
				MessageID: 1,
			},
			Data: "schedule_rules",
		},
	}
	err = suite.SimulateUpdate(update2)
	if err != nil {
		t.Errorf("设置规则调度失败: %v", err)
	}

	// 3. 验证任务状态
	coreStatus := suite.jobManager.GetJobStatus("core_maintain")
	rulesStatus := suite.jobManager.GetJobStatus("rules_maintain")

	if coreStatus != "✅ Schedule" {
		t.Errorf("核心维护调度状态不正确: %s", coreStatus)
	}
	if rulesStatus != "✅ Schedule" {
		t.Errorf("规则维护调度状态不正确: %s", rulesStatus)
	}

	// 4. 清除所有调度
	update3 := tgbotapi.Update{
		CallbackQuery: &tgbotapi.CallbackQuery{
			ID: "cb_schedule_clear",
			From: &tgbotapi.User{ID: suite.cfg.AdminChatID},
			Message: &tgbotapi.Message{
				Chat:      &tgbotapi.Chat{ID: suite.cfg.AdminChatID},
				MessageID: 1,
			},
			Data: "schedule_clear",
		},
	}
	err = suite.SimulateUpdate(update3)
	if err != nil {
		t.Errorf("清除调度失败: %v", err)
	}
}

// TestIntegration_StatePersistence 测试状态持久化
func TestIntegration_StatePersistence(t *testing.T) {
	suite := NewIntegrationTestSuite(t)
	
	// 设置调度任务
	update := tgbotapi.Update{
		CallbackQuery: &tgbotapi.CallbackQuery{
			ID: "cb_schedule",
			From: &tgbotapi.User{ID: suite.cfg.AdminChatID},
			Message: &tgbotapi.Message{
				Chat:      &tgbotapi.Chat{ID: suite.cfg.AdminChatID},
				MessageID: 1,
			},
			Data: "schedule_core",
		},
	}
	suite.SimulateUpdate(update)

	// 保存状态
	err := suite.jobManager.SaveState()
	if err != nil {
		t.Errorf("保存状态失败: %v", err)
	}

	// 验证状态文件存在
	if _, err := os.Stat(suite.stateFile); os.IsNotExist(err) {
		t.Error("状态文件未创建")
	}

	suite.Cleanup()
}

// TestIntegration_ConcurrentRequests 测试并发请求
func TestIntegration_ConcurrentRequests(t *testing.T) {
	suite := NewIntegrationTestSuite(t)
	defer suite.Cleanup()

	var wg sync.WaitGroup
	errors := make(chan error, 10)

	// 并发发送多个请求
	for i := 0; i < 10; i++ {
		wg.Add(1)
		go func(idx int) {
			defer wg.Done()
			update := tgbotapi.Update{
				CallbackQuery: &tgbotapi.CallbackQuery{
					ID: fmt.Sprintf("cb_%d", idx),
					From: &tgbotapi.User{ID: suite.cfg.AdminChatID},
					Message: &tgbotapi.Message{
						Chat:      &tgbotapi.Chat{ID: suite.cfg.AdminChatID},
						MessageID: 1,
					},
					Data: "status",
				},
			}
			if err := suite.SimulateUpdate(update); err != nil {
				errors <- err
			}
		}(i)
	}

	wg.Wait()
	close(errors)

	for err := range errors {
		t.Errorf("并发请求错误: %v", err)
	}
}

// TestIntegration_ErrorHandling 测试错误处理
func TestIntegration_ErrorHandling(t *testing.T) {
	suite := NewIntegrationTestSuite(t)
	defer suite.Cleanup()

	// 模拟系统命令失败
	suite.mockSys.CommandError["core_maintain"] = fmt.Errorf("模拟错误")

	update := tgbotapi.Update{
		CallbackQuery: &tgbotapi.CallbackQuery{
			ID: "cb_error",
			From: &tgbotapi.User{ID: suite.cfg.AdminChatID},
			Message: &tgbotapi.Message{
				Chat:      &tgbotapi.Chat{ID: suite.cfg.AdminChatID},
				MessageID: 1,
			},
			Data: "maintain_core",
		},
	}

	// 应该不会 panic
	err := suite.SimulateUpdate(update)
	if err != nil {
		t.Errorf("错误处理失败: %v", err)
	}

	// 等待异步操作
	time.Sleep(100 * time.Millisecond)
}

// TestIntegration_AuthorizationChain 测试授权链
func TestIntegration_AuthorizationChain(t *testing.T) {
	suite := NewIntegrationTestSuite(t)
	defer suite.Cleanup()

	// 测试未授权用户
	unauthorizedUpdate := tgbotapi.Update{
		CallbackQuery: &tgbotapi.CallbackQuery{
			ID: "cb_unauth",
			From: &tgbotapi.User{ID: suite.cfg.AdminChatID + 999},
			Message: &tgbotapi.Message{
				Chat:      &tgbotapi.Chat{ID: suite.cfg.AdminChatID + 999},
				MessageID: 1,
			},
			Data: "status",
		},
	}

	// 未授权请求应该被拒绝但不报错
	err := suite.SimulateUpdate(unauthorizedUpdate)
	if err != nil {
		t.Logf("未授权请求处理: %v", err)
	}
}

// TestIntegration_MultiLevelMenu 测试多级菜单集成
func TestIntegration_MultiLevelMenu(t *testing.T) {
	suite := NewIntegrationTestSuite(t)
	defer suite.Cleanup()

	// 1. 进入调度菜单
	update1 := tgbotapi.Update{
		CallbackQuery: &tgbotapi.CallbackQuery{
			ID: "cb_1",
			From: &tgbotapi.User{ID: suite.cfg.AdminChatID},
			Message: &tgbotapi.Message{
				Chat:      &tgbotapi.Chat{ID: suite.cfg.AdminChatID},
				MessageID: 1,
			},
			Data: "schedule_menu",
		},
	}
	if err := suite.SimulateUpdate(update1); err != nil {
		t.Errorf("进入调度菜单失败: %v", err)
	}

	// 2. 选择任务类型
	update2 := tgbotapi.Update{
		CallbackQuery: &tgbotapi.CallbackQuery{
			ID: "cb_2",
			From: &tgbotapi.User{ID: suite.cfg.AdminChatID},
			Message: &tgbotapi.Message{
				Chat:      &tgbotapi.Chat{ID: suite.cfg.AdminChatID},
				MessageID: 1,
			},
			Data: "menu_task_core_maintain",
		},
	}
	if err := suite.SimulateUpdate(update2); err != nil {
		t.Errorf("选择任务类型失败: %v", err)
	}

	// 3. 选择频率
	update3 := tgbotapi.Update{
		CallbackQuery: &tgbotapi.CallbackQuery{
			ID: "cb_3",
			From: &tgbotapi.User{ID: suite.cfg.AdminChatID},
			Message: &tgbotapi.Message{
				Chat:      &tgbotapi.Chat{ID: suite.cfg.AdminChatID},
				MessageID: 1,
			},
			Data: "menu_freq_core_maintain_daily",
		},
	}
	if err := suite.SimulateUpdate(update3); err != nil {
		t.Errorf("选择频率失败: %v", err)
	}

	// 4. 选择时间
	update4 := tgbotapi.Update{
		CallbackQuery: &tgbotapi.CallbackQuery{
			ID: "cb_4",
			From: &tgbotapi.User{ID: suite.cfg.AdminChatID},
			Message: &tgbotapi.Message{
				Chat:      &tgbotapi.Chat{ID: suite.cfg.AdminChatID},
				MessageID: 1,
			},
			Data: "menu_time_core_maintain_daily_4",
		},
	}
	if err := suite.SimulateUpdate(update4); err != nil {
		t.Errorf("选择时间失败: %v", err)
	}

	// 等待异步操作
	time.Sleep(200 * time.Millisecond)
}

// TestIntegration_ServiceRestart 测试服务重启集成
func TestIntegration_ServiceRestart(t *testing.T) {
	suite := NewIntegrationTestSuite(t)
	defer suite.Cleanup()

	// 设置 Xray 重启调度
	update := tgbotapi.Update{
		CallbackQuery: &tgbotapi.CallbackQuery{
			ID: "cb_xray_restart",
			From: &tgbotapi.User{ID: suite.cfg.AdminChatID},
			Message: &tgbotapi.Message{
				Chat:      &tgbotapi.Chat{ID: suite.cfg.AdminChatID},
				MessageID: 1,
			},
			Data: "schedule_xray_restart",
		},
	}

	err := suite.SimulateUpdate(update)
	if err != nil {
		t.Errorf("Xray 重启调度设置失败: %v", err)
	}

	// 验证任务状态
	status := suite.jobManager.GetJobStatus("restart_xray")
	if status != "✅ Schedule" {
		t.Errorf("Xray 重启调度状态不正确: %s", status)
	}
}

// TestIntegration_UpdateOperations 测试更新操作集成
func TestIntegration_UpdateOperations(t *testing.T) {
	suite := NewIntegrationTestSuite(t)
	defer suite.Cleanup()

	// 测试 Xray 更新
	update1 := tgbotapi.Update{
		CallbackQuery: &tgbotapi.CallbackQuery{
			ID: "cb_update_xray",
			From: &tgbotapi.User{ID: suite.cfg.AdminChatID},
			Message: &tgbotapi.Message{
				Chat:      &tgbotapi.Chat{ID: suite.cfg.AdminChatID},
				MessageID: 1,
			},
			Data: "update_xray",
		},
	}
	if err := suite.SimulateUpdate(update1); err != nil {
		t.Errorf("Xray 更新失败: %v", err)
	}

	// 测试 Sing-box 更新
	update2 := tgbotapi.Update{
		CallbackQuery: &tgbotapi.CallbackQuery{
			ID: "cb_update_singbox",
			From: &tgbotapi.User{ID: suite.cfg.AdminChatID},
			Message: &tgbotapi.Message{
				Chat:      &tgbotapi.Chat{ID: suite.cfg.AdminChatID},
				MessageID: 1,
			},
			Data: "update_singbox",
		},
	}
	if err := suite.SimulateUpdate(update2); err != nil {
		t.Errorf("Sing-box 更新失败: %v", err)
	}

	// 等待异步操作
	time.Sleep(100 * time.Millisecond)
}
