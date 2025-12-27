package bot

import (
	"context"
	"fmt"
	"log"
	"strings"
	"sync"
	"time"
	"vps-tg-bot/pkg/config"
	"vps-tg-bot/pkg/scheduler"
	"vps-tg-bot/pkg/system"

	tgbotapi "github.com/go-telegram-bot-api/telegram-bot-api/v5"
)

// BotHandler 接口定义
type BotHandler interface {
	HandleUpdate(update tgbotapi.Update) error
	SendMessage(chatID int64, text string) error
	SendInlineKeyboard(chatID int64, text string, keyboard [][]tgbotapi.InlineKeyboardButton) error
}

// TGBotHandler 实现 BotHandler 接口
type TGBotHandler struct {
	api                 TelegramAPI
	config              *config.Config
	systemExec          system.SystemExecutor
	jobManager          scheduler.JobManager
	adminChatID         int64
	historyRecorder     system.HistoryRecorder
	// 维护状态管理
	isMaintenanceRunning bool
	maintenanceMutex     sync.Mutex
}

// TelegramAPI 定义 Telegram API 的接口
type TelegramAPI interface {
	Send(c tgbotapi.Chattable) (tgbotapi.Message, error)
	Request(c tgbotapi.Chattable) (*tgbotapi.APIResponse, error)
}

// NewTGBotHandler 创建新的 TGBotHandler
func NewTGBotHandler(api TelegramAPI, config *config.Config, systemExec system.SystemExecutor, jobManager scheduler.JobManager) BotHandler {
	return &TGBotHandler{
		api:             api,
		config:          config,
		systemExec:      systemExec,
		jobManager:      jobManager,
		adminChatID:     config.AdminChatID,
		historyRecorder: system.NewFileHistoryRecorder("maintain_history.json"),
	}
}

// HandleUpdate 处理 Telegram 更新
func (t *TGBotHandler) HandleUpdate(update tgbotapi.Update) error {
	if update.Message != nil {
		return t.handleMessage(update.Message)
	}
	
	if update.CallbackQuery != nil {
		return t.handleCallback(update.CallbackQuery)
	}
	
	return nil
}

// handleMessage 处理消息
func (t *TGBotHandler) handleMessage(message *tgbotapi.Message) error {
	// 权限验证
	if message.Chat.ID != t.adminChatID {
		return t.SendMessage(message.Chat.ID, "❌ 无权限访问此 Bot")
	}
	
	// 处理命令
	if message.IsCommand() {
		switch message.Command() {
		case "start":
			return t.ShowMainMenu(message.Chat.ID)
		case "help":
			return t.SendMessage(message.Chat.ID, "📖 *帮助信息*\n\n使用按钮进行操作，或发送 /start 打开菜单")

		}
	}
	
	// 处理自定义 Cron 输入（简单处理）
	// 这里可以扩展为更复杂的状态管理
	if strings.Contains(message.Text, "0") && strings.Contains(message.Text, "*") {
		// 简单的 Cron 表达式检测
		if err := t.validateCronExpression(message.Text); err == nil {
			// 假设用户要设置一个核心维护任务（这里可以扩展为更智能的识别）
			taskName := "核心维护 自定义定时任务"
			_, err := t.jobManager.AddJob(taskName, string(TaskTypeCore), strings.TrimSpace(message.Text))
			if err != nil {
				return t.SendMessage(message.Chat.ID, fmt.Sprintf("❌ 设置定时任务失败: %v", err))
			}
			return t.SendMessage(message.Chat.ID, fmt.Sprintf("✅ 定时任务设置成功\n\n🆔 Cron: `%s`", strings.TrimSpace(message.Text)))
		}
	}
	
	return nil
}

// handleCallback 处理回调查询
func (t *TGBotHandler) handleCallback(query *tgbotapi.CallbackQuery) error {
	// 权限验证
	if query.Message.Chat.ID != t.adminChatID {
		callback := tgbotapi.NewCallback(query.ID, "❌ 无权限访问")
		t.api.Request(callback)
		return nil
	}
	
	// 确认回调查询
	callback := tgbotapi.NewCallback(query.ID, "")
	t.api.Request(callback)
	
	// 处理回调数据
	switch query.Data {
	case "status":
		return t.handleStatusCallback(query)
	case "maintain_now":
		return t.handleMaintainMenu(query)
	case "maintain_core":
		return t.handleCoreMaintain(query)
	case "maintain_rules":
		return t.handleRulesMaintain(query)
	case "maintain_full":
		return t.handleFullMaintain(query)
	case "update_xray":
		return t.handleUpdateXray(query)
	case "update_singbox":
		return t.handleUpdateSingbox(query)
	case "schedule_menu":
		return t.BuildTaskTypeMenu(query.Message.Chat.ID)
	case "schedule_core":
		return t.handleSetCoreSchedule(query)
	case "schedule_rules":
		return t.handleSetRulesSchedule(query)
	case "schedule_xray_restart":
		return t.handleSetXrayRestartSchedule(query)
	case "schedule_sb_restart":
		return t.handleSetSingboxRestartSchedule(query)
	case "schedule_clear":
		return t.handleClearSchedule(query)
	case "view_logs":
		return t.handleViewLogs(query)
	case "view_history":
		return t.handleViewHistory(query)
	case "reboot_confirm":
		return t.handleRebootConfirm(query)
	case "back_main":
		return t.handleBackToMain(query)
	
	// 新增多级菜单系统处理
	case "menu_task_core_maintain":
		return t.HandleTaskTypeSelection(query, TaskTypeCore)
	case "menu_task_rules_maintain":
		return t.HandleTaskTypeSelection(query, TaskTypeRules)
	case "menu_task_update_xray":
		return t.HandleTaskTypeSelection(query, TaskTypeUpdateXray)
	case "menu_task_update_singbox":
		return t.HandleTaskTypeSelection(query, TaskTypeUpdateSing)
	case "menu_view_tasks":
		return t.HandleViewTasks(query)
	case "menu_task_add":
		return t.BuildTaskTypeMenu(query.Message.Chat.ID)
	case "menu_back_task_types":
		return t.BuildTaskTypeMenu(query.Message.Chat.ID)
	
	default:
		// 处理动态回调数据
		if strings.HasPrefix(query.Data, "menu_freq_") {
			parts := strings.Split(query.Data, "_")
			if len(parts) >= 4 {
				taskType := TaskType(parts[2])
				frequency := Frequency(parts[3])
				return t.HandleFrequencySelection(query, taskType, frequency)
			}
		} else if strings.HasPrefix(query.Data, "menu_time_") {
			parts := strings.Split(query.Data, "_")
			if len(parts) >= 5 {
				taskType := TaskType(parts[2])
				frequency := Frequency(parts[3])
				timeValue := strings.Join(parts[4:], "_")
				return t.HandleTimeSelection(query, taskType, frequency, timeValue)
			}
		} else {
			log.Printf("未知的回调数据: %s", query.Data)
		}
	}
	
	return nil
}

// SendMessage 发送消息
func (t *TGBotHandler) SendMessage(chatID int64, text string) error {
	// 简单的 Markdown 转义，防止格式错误
	// 注意：这里假设 text 已经是 Markdown 格式，或者需要被转义
	// 为了安全起见，如果 text 包含用户输入，应该进行转义。
	// 但由于这是 Admin Bot，且大部分 text 是系统生成的，我们主要关注防止意外的格式错误。
	// 更好的做法是使用 MarkdownV2 并转义所有特殊字符，或者提供一个 SafeSendMessage 方法。
	msg := tgbotapi.NewMessage(chatID, text)
	msg.ParseMode = tgbotapi.ModeMarkdown
	_, err := t.api.Send(msg)
	return err
}

// SendInlineKeyboard 发送内联键盘
func (t *TGBotHandler) SendInlineKeyboard(chatID int64, text string, keyboard [][]tgbotapi.InlineKeyboardButton) error {
	msg := tgbotapi.NewMessage(chatID, text)
	msg.ParseMode = tgbotapi.ModeMarkdown
	msg.ReplyMarkup = tgbotapi.NewInlineKeyboardMarkup(keyboard...)
	_, err := t.api.Send(msg)
	return err
}

// ShowMainMenu 显示主菜单
func (t *TGBotHandler) ShowMainMenu(chatID int64) error {
	keyboard := [][]tgbotapi.InlineKeyboardButton{
		{tgbotapi.NewInlineKeyboardButtonData("📊 系统状态", "status")},
		{tgbotapi.NewInlineKeyboardButtonData("🔧 立即维护", "maintain_now"), tgbotapi.NewInlineKeyboardButtonData("⚙️ 调度设置", "schedule_menu")},
		{tgbotapi.NewInlineKeyboardButtonData("📋 查看日志", "view_logs"), tgbotapi.NewInlineKeyboardButtonData("📜 维护历史", "view_history")},
		{tgbotapi.NewInlineKeyboardButtonData("🔄 重启 VPS", "reboot_confirm")},
	}
	
	text := "🤖 *VPS 管理 Bot*\n\n请选择操作："
	return t.SendInlineKeyboard(chatID, text, keyboard)
}

// handleMaintainMenu 显示维护菜单
func (t *TGBotHandler) handleMaintainMenu(query *tgbotapi.CallbackQuery) error {
	keyboard := [][]tgbotapi.InlineKeyboardButton{
		{tgbotapi.NewInlineKeyboardButtonData("🔧 核心维护", "maintain_core"), tgbotapi.NewInlineKeyboardButtonData("📜 规则更新", "maintain_rules")},
		{tgbotapi.NewInlineKeyboardButtonData("🔄 Xray 更新", "update_xray"), tgbotapi.NewInlineKeyboardButtonData("🔄 Sing-box 更新", "update_singbox")},
		{tgbotapi.NewInlineKeyboardButtonData("🔄 完整维护", "maintain_full"), tgbotapi.NewInlineKeyboardButtonData("🔙 返回", "back_main")},
	}
	
	text := "🔧 *维护菜单*\n\n请选择维护类型："
	
	msg := tgbotapi.NewEditMessageText(query.Message.Chat.ID, query.Message.MessageID, text)
	msg.ParseMode = tgbotapi.ModeMarkdown
	keyboardMarkup := tgbotapi.NewInlineKeyboardMarkup(keyboard...)
	msg.ReplyMarkup = &keyboardMarkup
	_, err := t.api.Send(msg)
	return err
}

// handleScheduleMenu 显示调度菜单
func (t *TGBotHandler) handleScheduleMenu(query *tgbotapi.CallbackQuery) error {
	coreStatus := t.jobManager.GetJobStatus("core_maintain")
	rulesStatus := t.jobManager.GetJobStatus("rules_maintain")
	xrayRestartStatus := t.jobManager.GetJobStatus("restart_xray")
	sbRestartStatus := t.jobManager.GetJobStatus("restart_singbox")

	keyboard := [][]tgbotapi.InlineKeyboardButton{
		{tgbotapi.NewInlineKeyboardButtonData("⏰ 设置核心 (每日04:00)", "schedule_core")},
		{tgbotapi.NewInlineKeyboardButtonData("📅 设置规则 (周日07:00)", "schedule_rules")},
		{tgbotapi.NewInlineKeyboardButtonData("🔄 Xray重启 (每日02:00)", "schedule_xray_restart")},
		{tgbotapi.NewInlineKeyboardButtonData("🔄 Sing-box重启 (每日03:00)", "schedule_sb_restart")},
		{tgbotapi.NewInlineKeyboardButtonData("🗑️ 清除所有", "schedule_clear"), tgbotapi.NewInlineKeyboardButtonData("🔙 返回", "back_main")},
	}
	
	text := fmt.Sprintf("⚙️ *调度菜单*\n\n"+
		"核心维护: %s\n"+
		"规则更新: %s\n"+
		"Xray 重启: %s\n"+
		"Sing-box 重启: %s\n\n"+
		"配置定时任务：",
		coreStatus, rulesStatus, xrayRestartStatus, sbRestartStatus)
	
	msg := tgbotapi.NewEditMessageText(query.Message.Chat.ID, query.Message.MessageID, text)
	msg.ParseMode = tgbotapi.ModeMarkdown
	keyboardMarkup := tgbotapi.NewInlineKeyboardMarkup(keyboard...)
	msg.ReplyMarkup = &keyboardMarkup
	_, err := t.api.Send(msg)
	return err
}

// handleStatusCallback 处理状态查询
func (t *TGBotHandler) handleStatusCallback(query *tgbotapi.CallbackQuery) error {
	// 获取系统时间
	systemTime, timezone := t.systemExec.GetSystemTime()
	
	// 获取详细系统状态
	status, err := t.systemExec.GetSystemStatus()
	if err != nil {
		log.Printf("获取系统状态失败: %v", err)
		// 降级显示
		text := fmt.Sprintf("📊 *系统状态*\n\n时间: %s %s\n状态: ⚠️ 获取详细信息失败",
			systemTime.Format("2006-01-02 15:04:05"), timezone)
		return t.SendMessage(query.Message.Chat.ID, text)
	}

	// 获取服务状态
	xrayStatus, _ := t.systemExec.GetServiceStatus("xray")
	sbStatus, _ := t.systemExec.GetServiceStatus("sing-box")
	
	text := fmt.Sprintf("📊 *系统状态*\n\n"+
		"🕒 时间: %s %s\n"+
		"⏱️ 运行时间: %s\n"+
		"📈 负载: %s\n"+
		"💾 内存: %s\n"+
		"💿 磁盘: %s\n"+
		"💻 CPU: %s\n"+
		"🔢 进程数: %d\n\n"+
		"*服务状态:*\n"+
		"Xray: %s\n"+
		"Sing-box: %s",
		systemTime.Format("2006-01-02 15:04:05"), timezone,
		status.Uptime,
		status.LoadAverage,
		status.MemoryUsage,
		status.DiskUsage,
		status.CPUUsage,
		status.ProcessCount,
		getStatusIcon(xrayStatus),
		getStatusIcon(sbStatus))
	
	return t.SendMessage(query.Message.Chat.ID, text)
}

func getStatusIcon(status string) string {
	if status == "active" {
		return "🟢 运行中"
	}
	return "🔴 已停止"
}

// handleCoreMaintain 处理核心维护
func (t *TGBotHandler) handleCoreMaintain(query *tgbotapi.CallbackQuery) error {
	// 在后台执行维护
	go func() {
		startTime := time.Now()
		result, err := t.systemExec.RunCoreMaintain()
		endTime := time.Now()

		record := &system.MaintainHistoryRecord{
			ID:        fmt.Sprintf("%d", startTime.Unix()),
			Type:      "核心维护",
			StartTime: startTime,
			EndTime:   endTime,
			Status:    "success",
			Result:    result,
		}

		if err != nil {
			record.Status = "failed"
			record.Error = err.Error()
			t.historyRecorder.AddRecord(record)
			t.SendMessage(query.Message.Chat.ID, fmt.Sprintf("❌ 核心维护失败: %v", err))
			return
		}
		
		t.historyRecorder.AddRecord(record)
		t.SendMessage(query.Message.Chat.ID, fmt.Sprintf("✅ *核心维护完成*\n\n```\n%s\n```", result))
	}()
	
	text := "⏳ 正在执行核心维护，请稍候..."
	
	msg := tgbotapi.NewEditMessageText(query.Message.Chat.ID, query.Message.MessageID, text)
	_, err := t.api.Send(msg)
	return err
}

// handleRulesMaintain 处理规则维护
func (t *TGBotHandler) handleRulesMaintain(query *tgbotapi.CallbackQuery) error {
	// 在后台执行维护
	go func() {
		startTime := time.Now()
		result, err := t.systemExec.RunRulesMaintain()
		endTime := time.Now()

		record := &system.MaintainHistoryRecord{
			ID:        fmt.Sprintf("%d", startTime.Unix()),
			Type:      "规则维护",
			StartTime: startTime,
			EndTime:   endTime,
			Status:    "success",
			Result:    result,
		}

		if err != nil {
			record.Status = "failed"
			record.Error = err.Error()
			t.historyRecorder.AddRecord(record)
			t.SendMessage(query.Message.Chat.ID, fmt.Sprintf("❌ 规则维护失败: %v", err))
			return
		}
		
		t.historyRecorder.AddRecord(record)
		t.SendMessage(query.Message.Chat.ID, fmt.Sprintf("✅ *规则维护完成*\n\n```\n%s\n```", result))
	}()
	
	text := "⏳ 正在执行规则维护，请稍候..."
	
	msg := tgbotapi.NewEditMessageText(query.Message.Chat.ID, query.Message.MessageID, text)
	_, err := t.api.Send(msg)
	return err
}

// handleFullMaintain 处理完整维护
func (t *TGBotHandler) handleFullMaintain(query *tgbotapi.CallbackQuery) error {
	// 检查维护状态
	t.maintenanceMutex.Lock()
	if t.isMaintenanceRunning {
		t.maintenanceMutex.Unlock()
		return t.SendMessage(query.Message.Chat.ID, "⏳ 维护任务正在进行中，请稍候...")
	}
	t.isMaintenanceRunning = true
	t.maintenanceMutex.Unlock()

	// 确保在函数结束时重置状态
	defer func() {
		t.maintenanceMutex.Lock()
		t.isMaintenanceRunning = false
		t.maintenanceMutex.Unlock()
	}()

	// 在后台执行完整维护
	go func() {
		// 设置30分钟超时
		ctx, cancel := context.WithTimeout(context.Background(), 30*time.Minute)
		defer cancel()

		// 发送开始消息
		t.SendMessage(query.Message.Chat.ID, "⏳ 正在执行完整维护（超时时间：30分钟），请稍候...")

		startTime := time.Now()
		
		// 执行核心维护
		coreResult, err := t.runWithTimeout(ctx, func() (string, error) {
			return t.systemExec.RunCoreMaintain()
		})
		
		if err != nil {
			endTime := time.Now()
			record := &system.MaintainHistoryRecord{
				ID:        fmt.Sprintf("%d", startTime.Unix()),
				Type:      "完整维护",
				StartTime: startTime,
				EndTime:   endTime,
				Status:    "failed",
				Error:     err.Error(),
			}
			t.historyRecorder.AddRecord(record)

			if ctx.Err() == context.DeadlineExceeded {
				t.SendMessage(query.Message.Chat.ID, "❌ 维护任务超时，已取消")
			} else {
				t.SendMessage(query.Message.Chat.ID, fmt.Sprintf("❌ 核心维护失败: %v", err))
			}
			return
		}

		// 执行规则维护
		rulesResult, err := t.runWithTimeout(ctx, func() (string, error) {
			return t.systemExec.RunRulesMaintain()
		})
		
		if err != nil {
			endTime := time.Now()
			record := &system.MaintainHistoryRecord{
				ID:        fmt.Sprintf("%d", startTime.Unix()),
				Type:      "完整维护",
				StartTime: startTime,
				EndTime:   endTime,
				Status:    "failed",
				Error:     err.Error(),
			}
			t.historyRecorder.AddRecord(record)

			if ctx.Err() == context.DeadlineExceeded {
				t.SendMessage(query.Message.Chat.ID, "❌ 维护任务超时，已取消")
			} else {
				t.SendMessage(query.Message.Chat.ID, fmt.Sprintf("❌ 规则维护失败: %v", err))
			}
			return
		}

		endTime := time.Now()
		result := fmt.Sprintf("核心维护:\n%s\n\n规则维护:\n%s", coreResult, rulesResult)
		
		record := &system.MaintainHistoryRecord{
			ID:        fmt.Sprintf("%d", startTime.Unix()),
			Type:      "完整维护",
			StartTime: startTime,
			EndTime:   endTime,
			Status:    "success",
			Result:    result,
		}
		t.historyRecorder.AddRecord(record)

		t.SendMessage(query.Message.Chat.ID, fmt.Sprintf("✅ *完整维护已完成*\n\n```\n%s\n```", result))
	}()

	return nil
}

// runWithTimeout 带超时的函数执行
func (t *TGBotHandler) runWithTimeout(ctx context.Context, fn func() (string, error)) (string, error) {
	done := make(chan struct{})
	var result string
	var err error

	go func() {
		defer close(done)
		result, err = fn()
	}()

	select {
	case <-ctx.Done():
		if ctx.Err() == context.DeadlineExceeded {
			return "", fmt.Errorf("任务执行超时")
		}
		return "", ctx.Err()
	case <-done:
		return result, err
	}
}

// handleSetCoreSchedule 处理设置核心维护调度
func (t *TGBotHandler) handleSetCoreSchedule(query *tgbotapi.CallbackQuery) error {
	// 设置每日04:00执行核心维护
	task := func() {
		result, err := t.systemExec.RunCoreMaintain()
		if err != nil {
			log.Printf("定时核心维护失败: %v", err)
		} else {
			log.Printf("定时核心维护完成: %s", result)
		}
	}
	
	err := t.jobManager.SetJob("core_maintain", "0 0 4 * * *", task)
	if err != nil {
		return t.SendMessage(query.Message.Chat.ID, fmt.Sprintf("❌ 设置调度失败: %v", err))
	}
	
	return t.SendMessage(query.Message.Chat.ID, "✅ 已设置核心维护调度：每日 04:00")
}

// handleSetRulesSchedule 处理设置规则维护调度
func (t *TGBotHandler) handleSetRulesSchedule(query *tgbotapi.CallbackQuery) error {
	// 设置每周日07:00执行规则维护
	task := func() {
		result, err := t.systemExec.RunRulesMaintain()
		if err != nil {
			log.Printf("定时规则维护失败: %v", err)
		} else {
			log.Printf("定时规则维护完成: %s", result)
		}
	}
	
	err := t.jobManager.SetJob("rules_maintain", "0 0 7 * * 0", task)
	if err != nil {
		return t.SendMessage(query.Message.Chat.ID, fmt.Sprintf("❌ 设置调度失败: %v", err))
	}
	
	return t.SendMessage(query.Message.Chat.ID, "✅ 已设置规则维护调度：每周日 07:00")
}

// handleSetXrayRestartSchedule 处理设置 Xray 重启调度
func (t *TGBotHandler) handleSetXrayRestartSchedule(query *tgbotapi.CallbackQuery) error {
	// 设置每日02:00执行 Xray 重启
	task := func() {
		log.Println("执行定时 Xray 重启...")
		result, err := t.systemExec.RestartService("xray")
		if err != nil {
			log.Printf("定时 Xray 重启失败: %v", err)
			t.SendMessage(t.adminChatID, fmt.Sprintf("❌ 定时 Xray 重启失败: %v", err))
		} else {
			log.Printf("定时 Xray 重启完成: %s", result)
			t.SendMessage(t.adminChatID, fmt.Sprintf("✅ 定时 Xray 重启完成\n\n```\n%s\n```", result))
		}
	}
	
	err := t.jobManager.SetJob("restart_xray", "0 0 2 * * *", task)
	if err != nil {
		return t.SendMessage(query.Message.Chat.ID, fmt.Sprintf("❌ 设置调度失败: %v", err))
	}
	
	return t.SendMessage(query.Message.Chat.ID, "✅ 已设置 Xray 重启调度：每日 02:00")
}

// handleSetSingboxRestartSchedule 处理设置 Sing-box 重启调度
func (t *TGBotHandler) handleSetSingboxRestartSchedule(query *tgbotapi.CallbackQuery) error {
	// 设置每日03:00执行 Sing-box 重启
	task := func() {
		log.Println("执行定时 Sing-box 重启...")
		result, err := t.systemExec.RestartService("sing-box")
		if err != nil {
			log.Printf("定时 Sing-box 重启失败: %v", err)
			t.SendMessage(t.adminChatID, fmt.Sprintf("❌ 定时 Sing-box 重启失败: %v", err))
		} else {
			log.Printf("定时 Sing-box 重启完成: %s", result)
			t.SendMessage(t.adminChatID, fmt.Sprintf("✅ 定时 Sing-box 重启完成\n\n```\n%s\n```", result))
		}
	}
	
	err := t.jobManager.SetJob("restart_singbox", "0 0 3 * * *", task)
	if err != nil {
		return t.SendMessage(query.Message.Chat.ID, fmt.Sprintf("❌ 设置调度失败: %v", err))
	}
	
	return t.SendMessage(query.Message.Chat.ID, "✅ 已设置 Sing-box 重启调度：每日 03:00")
}

// handleClearSchedule 处理清除调度
func (t *TGBotHandler) handleClearSchedule(query *tgbotapi.CallbackQuery) error {
	t.jobManager.ClearAll()
	return t.SendMessage(query.Message.Chat.ID, "✅ 已清除所有调度任务")
}

// handleViewLogs 处理查看日志
func (t *TGBotHandler) handleViewLogs(query *tgbotapi.CallbackQuery) error {
	logs, err := t.systemExec.GetLogs(20)
	if err != nil {
		return t.SendMessage(query.Message.Chat.ID, fmt.Sprintf("❌ 获取日志失败: %v", err))
	}
	
	return t.SendMessage(query.Message.Chat.ID, fmt.Sprintf("📋 *服务日志*\n\n```\n%s\n```", logs))
}

// handleViewHistory 处理查看历史
func (t *TGBotHandler) handleViewHistory(query *tgbotapi.CallbackQuery) error {
	records, err := t.historyRecorder.GetRecords(10)
	if err != nil {
		return t.SendMessage(query.Message.Chat.ID, fmt.Sprintf("❌ 获取历史记录失败: %v", err))
	}

	if len(records) == 0 {
		return t.SendMessage(query.Message.Chat.ID, "📭 暂无维护历史记录")
	}

	var text string
	text = "📜 *最近 10 条维护记录*\n\n"
	
	for _, record := range records {
		statusIcon := "✅"
		if record.Status != "success" {
			statusIcon = "❌"
		}
		
		duration := record.EndTime.Sub(record.StartTime)
		
		text += fmt.Sprintf("%s *%s*\n", statusIcon, record.Type)
		text += fmt.Sprintf("时间: %s\n", record.StartTime.Format("2006-01-02 15:04:05"))
		text += fmt.Sprintf("耗时: %s\n", duration)
		if record.Error != "" {
			text += fmt.Sprintf("错误: %s\n", record.Error)
		}
		text += "-------------------\n"
	}

	return t.SendMessage(query.Message.Chat.ID, text)
}

// handleRebootConfirm 处理重启确认
func (t *TGBotHandler) handleRebootConfirm(query *tgbotapi.CallbackQuery) error {
	// 在后台执行重启
	go func() {
		time.Sleep(5 * time.Second)
		t.systemExec.Reboot()
	}()
	
	return t.SendMessage(query.Message.Chat.ID, "⚠️ 系统将在 5 秒后重启...")
}

// handleBackToMain 处理返回主菜单
func (t *TGBotHandler) handleBackToMain(query *tgbotapi.CallbackQuery) error {
	return t.ShowMainMenu(query.Message.Chat.ID)
}

// handleUpdateXray 处理 Xray 更新
func (t *TGBotHandler) handleUpdateXray(query *tgbotapi.CallbackQuery) error {
	// 在后台执行更新
	go func() {
		startTime := time.Now()
		result, err := t.systemExec.UpdateXray()
		endTime := time.Now()

		record := &system.MaintainHistoryRecord{
			ID:        fmt.Sprintf("%d", startTime.Unix()),
			Type:      "Xray 更新",
			StartTime: startTime,
			EndTime:   endTime,
			Status:    "success",
			Result:    result,
		}

		if err != nil {
			record.Status = "failed"
			record.Error = err.Error()
			t.historyRecorder.AddRecord(record)
			t.SendMessage(query.Message.Chat.ID, fmt.Sprintf("❌ Xray 更新失败: %v", err))
			return
		}
		
		t.historyRecorder.AddRecord(record)
		t.SendMessage(query.Message.Chat.ID, fmt.Sprintf("✅ *Xray 更新完成*\n\n```\n%s\n```", result))
	}()
	
	text := "⏳ 正在更新 Xray 核心，请稍候..."
	
	msg := tgbotapi.NewEditMessageText(query.Message.Chat.ID, query.Message.MessageID, text)
	_, err := t.api.Send(msg)
	return err
}

// handleUpdateSingbox 处理 Sing-box 更新
func (t *TGBotHandler) handleUpdateSingbox(query *tgbotapi.CallbackQuery) error {
	// 在后台执行更新
	go func() {
		startTime := time.Now()
		result, err := t.systemExec.UpdateSingbox()
		endTime := time.Now()

		record := &system.MaintainHistoryRecord{
			ID:        fmt.Sprintf("%d", startTime.Unix()),
			Type:      "Sing-box 更新",
			StartTime: startTime,
			EndTime:   endTime,
			Status:    "success",
			Result:    result,
		}

		if err != nil {
			record.Status = "failed"
			record.Error = err.Error()
			t.historyRecorder.AddRecord(record)
			t.SendMessage(query.Message.Chat.ID, fmt.Sprintf("❌ Sing-box 更新失败: %v", err))
			return
		}
		
		t.historyRecorder.AddRecord(record)
		t.SendMessage(query.Message.Chat.ID, fmt.Sprintf("✅ *Sing-box 更新完成*\n\n```\n%s\n```", result))
	}()
	
	text := "⏳ 正在更新 Sing-box 核心，请稍候..."
	
	msg := tgbotapi.NewEditMessageText(query.Message.Chat.ID, query.Message.MessageID, text)
	_, err := t.api.Send(msg)
	return err
}