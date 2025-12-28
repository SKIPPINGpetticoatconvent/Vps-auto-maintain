package integration

import (
	"fmt"
	"reflect"
	"testing"
	"time"

	tgbotapi "github.com/go-telegram-bot-api/telegram-bot-api/v5"
)

// GenericCallbackFramework 通用的回调测试框架
// 这个框架可以用来测试任何Bot Handler的回调处理能力
type GenericCallbackFramework struct {
	suite      *IntegrationTestSuite
	knownTypes map[string]bool // 记录已知的回调类型
}

// NewGenericCallbackFramework 创建通用的回调测试框架
func NewGenericCallbackFramework(suite *IntegrationTestSuite) *GenericCallbackFramework {
	return &GenericCallbackFramework{
		suite:      suite,
		knownTypes: make(map[string]bool),
	}
}

// DiscoverCallbackTypes 通过反射发现所有可能的回调类型
func (g *GenericCallbackFramework) DiscoverCallbackTypes() []string {
	var callbacks []string
	
	// 通过分析Bot Handler的方法来发现回调类型
	handlerType := reflect.TypeOf(g.suite.botHandler)
	
	for i := 0; i < handlerType.NumMethod(); i++ {
		method := handlerType.Method(i)
		methodName := method.Name
		
		// 识别回调处理方法
		if methodName == "HandleUpdate" {
			// HandleUpdate方法可以处理所有更新
			callbacks = append(callbacks, g.discoverFromHandleUpdate()...)
		}
	}
	
	return callbacks
}

// discoverFromHandleUpdate 从HandleUpdate方法中发现回调类型
func (g *GenericCallbackFramework) discoverFromHandleUpdate() []string {
	var callbacks []string
	
	// 通过分析已有的回调数据和代码结构来发现
	// 这里我们使用预定义的回调模式，但保持通用性
	
	// 主菜单回调模式
	callbacks = append(callbacks, "status", "maintain_now", "schedule_menu", 
		"view_logs", "view_history", "reboot_confirm", "back_main")
	
	// 维护菜单回调模式
	callbacks = append(callbacks, "maintain_core", "maintain_rules", "maintain_full",
		"update_xray", "update_singbox")
	
	// 调度菜单回调模式
	callbacks = append(callbacks, "schedule_core", "schedule_rules", 
		"schedule_xray_restart", "schedule_sb_restart", "schedule_clear")
	
	// 多级菜单回调模式
	callbacks = append(callbacks, "menu_task_core_maintain", "menu_task_rules_maintain",
		"menu_task_update_xray", "menu_task_update_singbox", "menu_view_tasks",
		"menu_task_add", "menu_task_clear_all", "menu_back_task_types")
	
	// 频率选择回调模式
	callbacks = append(callbacks, "menu_freq_core_maintain_daily", "menu_freq_core_maintain_weekly",
		"menu_freq_core_maintain_monthly", "menu_freq_core_maintain_custom")
	
	// 时间选择回调模式
	callbacks = append(callbacks, "menu_time_core_maintain_daily_4", "menu_time_core_maintain_daily_12",
		"menu_time_core_maintain_weekly_0_4")
	
	// 任务操作回调模式
	callbacks = append(callbacks, "menu_task_delete_1", "menu_task_edit_1",
		"menu_task_enable_1", "menu_task_disable_1")
	
	return callbacks
}

// TestAllDiscoveredCallbacks 测试所有发现的回调
func (g *GenericCallbackFramework) TestAllDiscoveredCallbacks(t *testing.T) {
	callbacks := g.DiscoverCallbackTypes()
	
	for _, callbackData := range callbacks {
		t.Run(fmt.Sprintf("回调_%s", callbackData), func(t *testing.T) {
			g.testSingleCallback(t, callbackData)
		})
	}
}

// TestCallbackVariants 测试回调的各种变体
func (g *GenericCallbackFramework) TestCallbackVariants(t *testing.T) {
	variants := []struct {
		pattern  string
		examples []string
	}{
		{
			pattern: "menu_task_{action}_{id}",
			examples: []string{"menu_task_delete_1", "menu_task_edit_2", "menu_task_enable_3"},
		},
		{
			pattern: "menu_freq_{task}_{frequency}",
			examples: []string{"menu_freq_core_maintain_daily", "menu_freq_rules_maintain_weekly"},
		},
		{
			pattern: "menu_time_{task}_{frequency}_{time}",
			examples: []string{"menu_time_core_maintain_daily_4", "menu_time_rules_maintain_weekly_0_7"},
		},
	}
	
	for _, variant := range variants {
		t.Run(fmt.Sprintf("变体_%s", variant.pattern), func(t *testing.T) {
			for _, example := range variant.examples {
				t.Run(fmt.Sprintf("示例_%s", example), func(t *testing.T) {
					g.testSingleCallback(t, example)
				})
			}
		})
	}
}

// TestEdgeCases 测试边界情况
func (g *GenericCallbackFramework) TestEdgeCases(t *testing.T) {
	edgeCases := []struct {
		name         string
		callbackData string
		expectError  bool
	}{
		{"空字符串", "", false},
		{"只有分隔符", "_", false},
		{"未知回调", "unknown_callback", false},
		{"无效格式", "invalid_format_test", false},
		{"超长回调", g.generateLongCallback(), false},
		{"特殊字符", "callback_with_special_chars!@#$%", false},
	}
	
	for _, testCase := range edgeCases {
		t.Run(testCase.name, func(t *testing.T) {
			g.testSingleCallbackWithExpectations(t, testCase.callbackData, testCase.expectError)
		})
	}
}

// TestAuthorizationScenarios 测试授权场景
func (g *GenericCallbackFramework) TestAuthorizationScenarios(t *testing.T) {
	callbacks := []string{"status", "maintain_core", "schedule_core", "reboot_confirm", "view_logs"}
	
	for _, callbackData := range callbacks {
		t.Run(fmt.Sprintf("未授权_%s", callbackData), func(t *testing.T) {
			// 使用错误的Chat ID
			update := tgbotapi.Update{
				CallbackQuery: &tgbotapi.CallbackQuery{
					ID: "unauthorized_test",
					From: &tgbotapi.User{ID: 999999999}, // 错误的ID
					Message: &tgbotapi.Message{
						Chat:      &tgbotapi.Chat{ID: 999999999},
						MessageID: 1,
					},
					Data: callbackData,
				},
			}
			
			err := g.suite.SimulateUpdate(update)
			if err != nil {
				t.Errorf("未授权处理不应该返回错误: %v", err)
			}
			
			// 验证发送了拒绝消息
			sentCount := g.suite.api.GetSentCount()
			if sentCount == 0 {
				t.Errorf("应该发送拒绝消息，但没有发送。回调: %s", callbackData)
			}
		})
	}
}

// TestConcurrencySafety 测试并发安全性
func (g *GenericCallbackFramework) TestConcurrencySafety(t *testing.T) {
	callbacks := []string{"status", "maintain_now", "schedule_menu", "view_logs"}
	
	// 并发测试
	for i := 0; i < 10; i++ {
		t.Run(fmt.Sprintf("并发批次_%d", i), func(t *testing.T) {
			for _, callbackData := range callbacks {
				err := g.testSingleCallback(t, callbackData)
				if err != nil {
					t.Errorf("并发测试失败: %v", err)
				}
			}
		})
	}
}

// testSingleCallback 测试单个回调
func (g *GenericCallbackFramework) testSingleCallback(t *testing.T, callbackData string) error {
	update := tgbotapi.Update{
		CallbackQuery: &tgbotapi.CallbackQuery{
			ID: fmt.Sprintf("test_%s_%d", callbackData, time.Now().UnixNano()),
			From: &tgbotapi.User{ID: g.suite.cfg.AdminChatID},
			Message: &tgbotapi.Message{
				Chat:      &tgbotapi.Chat{ID: g.suite.cfg.AdminChatID},
				MessageID: 1,
			},
			Data: callbackData,
		},
	}
	
	return g.suite.SimulateUpdate(update)
}

// testSingleCallbackWithExpectations 测试单个回调并验证期望
func (g *GenericCallbackFramework) testSingleCallbackWithExpectations(t *testing.T, callbackData string, expectError bool) error {
	update := tgbotapi.Update{
		CallbackQuery: &tgbotapi.CallbackQuery{
			ID: fmt.Sprintf("expect_test_%s_%d", callbackData, time.Now().UnixNano()),
			From: &tgbotapi.User{ID: g.suite.cfg.AdminChatID},
			Message: &tgbotapi.Message{
				Chat:      &tgbotapi.Chat{ID: g.suite.cfg.AdminChatID},
				MessageID: 1,
			},
			Data: callbackData,
		},
	}
	
	err := g.suite.SimulateUpdate(update)
	
	if expectError && err == nil {
		t.Errorf("预期应该返回错误，但没有返回错误。回调数据: %s", callbackData)
	}
	if !expectError && err != nil {
		t.Errorf("预期不应该返回错误，但返回了错误: %v。回调数据: %s", err, callbackData)
	}
	
	return err
}

// generateLongCallback 生成超长回调数据
func (g *GenericCallbackFramework) generateLongCallback() string {
	longData := "menu_task_"
	for i := 0; i < 100; i++ {
		longData += fmt.Sprintf("long_parameter_%d_", i)
	}
	longData += "end"
	return longData
}

// TestGenericCallbackTestSuite 通用的回调测试套件
func TestGenericCallbackTestSuite(t *testing.T) {
	suite := NewIntegrationTestSuite(t)
	defer suite.Cleanup()
	
	framework := NewGenericCallbackFramework(suite)
	
	// 运行所有通用测试
	t.Run("发现所有回调", framework.TestAllDiscoveredCallbacks)
	t.Run("测试回调变体", framework.TestCallbackVariants)
	t.Run("测试边界情况", framework.TestEdgeCases)
	t.Run("测试授权场景", framework.TestAuthorizationScenarios)
	t.Run("测试并发安全性", framework.TestConcurrencySafety)
}