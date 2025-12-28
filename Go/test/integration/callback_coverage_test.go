package integration

import (
	"testing"
	"time"

	tgbotapi "github.com/go-telegram-bot-api/telegram-bot-api/v5"
)

// TestCallbackCoverage 测试所有回调的覆盖情况
// 这个测试专门用来验证所有回调都能正确处理，包括之前遗漏的重启回调
func TestCallbackCoverage(t *testing.T) {
	suite := NewIntegrationTestSuite(t)
	defer suite.Cleanup()

	// 定义所有需要测试的回调数据
	callbackTests := []struct {
		name         string
		callbackData string
		shouldSendMessage bool // 是否应该发送消息
		description  string
	}{
		// 主菜单回调
		{"状态查询", "status", true, "显示系统状态"},
		{"维护菜单", "maintain_now", true, "显示维护菜单"},
		{"调度设置", "schedule_menu", true, "显示多级菜单"},
		{"查看日志", "view_logs", true, "显示系统日志"},
		{"维护历史", "view_history", true, "显示维护历史"},
		{"重启确认", "reboot_confirm", true, "显示重启确认界面"},
		{"返回主菜单", "back_main", true, "返回主菜单"},

		// 维护菜单回调
		{"核心维护", "maintain_core", true, "执行核心维护"},
		{"规则维护", "maintain_rules", true, "执行规则维护"},
		{"完整维护", "maintain_full", true, "执行完整维护"},
		{"Xray更新", "update_xray", true, "更新Xray"},
		{"Singbox更新", "update_singbox", true, "更新Singbox"},

		// 调度菜单回调
		{"核心调度", "schedule_core", true, "设置核心维护调度"},
		{"规则调度", "schedule_rules", true, "设置规则维护调度"},
		{"Xray重启调度", "schedule_xray_restart", true, "设置Xray重启调度"},
		{"Singbox重启调度", "schedule_sb_restart", true, "设置Singbox重启调度"},
		{"清除调度", "schedule_clear", true, "清除所有调度"},

		// 多级菜单回调
		{"核心维护任务", "menu_task_core_maintain", true, "进入核心维护设置"},
		{"规则维护任务", "menu_task_rules_maintain", true, "进入规则维护设置"},
		{"Xray更新任务", "menu_task_update_xray", true, "进入Xray更新设置"},
		{"Singbox更新任务", "menu_task_update_singbox", true, "进入Singbox更新设置"},
		{"查看任务列表", "menu_view_tasks", true, "显示任务列表"},
		{"添加任务", "menu_task_add", true, "重新构建任务类型菜单"},
		{"清除所有任务", "menu_task_clear_all", true, "清除所有任务"},
		{"返回任务类型", "menu_back_task_types", true, "返回任务类型菜单"},

		// 频率选择回调
		{"核心维护每日", "menu_freq_core_maintain_daily", true, "处理核心维护每日频率"},
		{"核心维护每周", "menu_freq_core_maintain_weekly", true, "处理核心维护每周频率"},
		{"核心维护每月", "menu_freq_core_maintain_monthly", true, "处理核心维护每月频率"},
		{"核心维护自定义", "menu_freq_core_maintain_custom", true, "处理核心维护自定义频率"},

		// 时间选择回调
		{"时间选择-每日4点", "menu_time_core_maintain_daily_4", true, "设置每日凌晨4点执行"},
		{"时间选择-每日12点", "menu_time_core_maintain_daily_12", true, "设置每日中午12点执行"},
		{"时间选择-每周4点", "menu_time_core_maintain_weekly_0_4", true, "设置每周日凌晨4点执行"},

		// 关键测试：重启执行回调（这个测试本应该发现之前的缺失）
		{"重启执行", "reboot_execute", true, "执行VPS重启操作"},

		// 任务操作回调
		{"删除任务", "menu_task_delete_1", true, "删除ID为1的任务"},
		{"编辑任务", "menu_task_edit_1", true, "编辑ID为1的任务"},
		{"启用任务", "menu_task_enable_1", true, "启用ID为1的任务"},
		{"禁用任务", "menu_task_disable_1", true, "禁用ID为1的任务"},

		// 边界情况测试
		{"未知回调", "unknown_callback", false, "未知回调应该被忽略"},
		{"无效格式", "invalid_format", false, "无效格式应该被忽略"},
	}

	// 逐个测试每个回调
	for _, test := range callbackTests {
		t.Run(test.name, func(t *testing.T) {
			// 重置消息计数
			initialCount := suite.api.GetSentCount()

			// 创建模拟的回调查询
			update := tgbotapi.Update{
				CallbackQuery: &tgbotapi.CallbackQuery{
					ID: test.name + "_test",
					From: &tgbotapi.User{ID: suite.cfg.AdminChatID},
					Message: &tgbotapi.Message{
						Chat:      &tgbotapi.Chat{ID: suite.cfg.AdminChatID},
						MessageID: 1,
					},
					Data: test.callbackData,
				},
			}

			// 执行处理
			err := suite.SimulateUpdate(update)

			// 对于未知回调，我们不期望错误
			if err != nil && test.callbackData != "unknown_callback" && test.callbackData != "invalid_format" {
				t.Errorf("回调处理失败: %v，回调: %s，描述: %s", err, test.callbackData, test.description)
			}

			// 验证消息发送
			finalCount := suite.api.GetSentCount()
			if test.shouldSendMessage {
				if finalCount <= initialCount {
					t.Errorf("应该发送消息，但没有发送。回调: %s，描述: %s", test.callbackData, test.description)
				}
			}

			// 等待异步操作完成
			if test.callbackData == "maintain_core" || test.callbackData == "maintain_rules" || 
			   test.callbackData == "maintain_full" || test.callbackData == "update_xray" || 
			   test.callbackData == "update_singbox" || test.callbackData == "reboot_execute" {
				time.Sleep(200 * time.Millisecond)
			}
		})
	}
}

// TestMissingCallbackRegression 测试回归：确保重启回调现在工作
// 这个测试专门用来确保我们修复的重启回调问题不会再次出现
func TestMissingCallbackRegression(t *testing.T) {
	suite := NewIntegrationTestSuite(t)
	defer suite.Cleanup()

	// 这个测试验证重启回调的完整流程
	t.Run("重启回调完整流程", func(t *testing.T) {
		// 1. 进入重启确认
		confirmUpdate := tgbotapi.Update{
			CallbackQuery: &tgbotapi.CallbackQuery{
				ID: "reboot_confirm_test",
				From: &tgbotapi.User{ID: suite.cfg.AdminChatID},
				Message: &tgbotapi.Message{
					Chat:      &tgbotapi.Chat{ID: suite.cfg.AdminChatID},
					MessageID: 1,
				},
				Data: "reboot_confirm",
			},
		}

		// 验证能进入重启确认界面
		err := suite.SimulateUpdate(confirmUpdate)
		if err != nil {
			t.Errorf("进入重启确认失败: %v", err)
		}

		initialCount := suite.api.GetSentCount()
		if initialCount == 0 {
			t.Error("进入重启确认应该发送消息")
		}

		// 2. 执行重启（这是关键的测试）
		executeUpdate := tgbotapi.Update{
			CallbackQuery: &tgbotapi.CallbackQuery{
				ID: "reboot_execute_test",
				From: &tgbotapi.User{ID: suite.cfg.AdminChatID},
				Message: &tgbotapi.Message{
					Chat:      &tgbotapi.Chat{ID: suite.cfg.AdminChatID},
					MessageID: 2,
				},
				Data: "reboot_execute",
			},
		}

		// 验证重启执行不会崩溃
		err = suite.SimulateUpdate(executeUpdate)
		if err != nil {
			t.Errorf("执行重启失败: %v", err)
		}

		// 验证发送了重启确认消息
		finalCount := suite.api.GetSentCount()
		if finalCount <= initialCount {
			t.Error("执行重启应该发送确认消息")
		}
	})
}

// TestAuthorizationCoverage 测试授权覆盖
func TestAuthorizationCoverage(t *testing.T) {
	suite := NewIntegrationTestSuite(t)
	defer suite.Cleanup()

	// 测试未授权用户的关键回调
	unauthorizedCallbacks := []string{
		"status",
		"maintain_core",
		"schedule_core",
		"reboot_confirm",
		"reboot_execute", // 包括我们修复的重启执行
		"menu_task_core_maintain",
		"menu_freq_core_maintain_daily",
		"menu_time_core_maintain_daily_4",
	}

	for _, callbackData := range unauthorizedCallbacks {
		t.Run("未授权-"+callbackData, func(t *testing.T) {
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

			err := suite.SimulateUpdate(update)
			if err != nil {
				t.Errorf("未授权处理不应该返回错误: %v", err)
			}

			// 验证发送了拒绝消息
			sentCount := suite.api.GetSentCount()
			if sentCount == 0 {
				t.Errorf("应该发送拒绝消息，但没有发送。回调: %s", callbackData)
			}
		})
	}
}