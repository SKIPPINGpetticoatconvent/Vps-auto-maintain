package bot

import (
	"fmt"
	"log"
	"strconv"
	"strings"
	"sync"
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
		log.Printf("用户进入调度设置，显示多级菜单")
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
	case "menu_task_clear_all":
		return t.HandleClearAllTasks(query)
	case "menu_back_task_types":
		return t.BuildTaskTypeMenu(query.Message.Chat.ID)
	
	// 处理动态回调数据
	default:
		// 处理动态任务操作回调数据
		if strings.HasPrefix(query.Data, "menu_task_delete_") {
			parts := strings.Split(query.Data, "_")
			if len(parts) >= 4 {
				if taskID, err := strconv.Atoi(parts[3]); err == nil {
					return t.HandleDeleteTask(query, taskID)
				}
			}
		} else if strings.HasPrefix(query.Data, "menu_task_edit_") {
			parts := strings.Split(query.Data, "_")
			if len(parts) >= 4 {
				if taskID, err := strconv.Atoi(parts[3]); err == nil {
					return t.HandleEditTask(query, taskID)
				}
			}
		} else if strings.HasPrefix(query.Data, "menu_task_enable_") {
			parts := strings.Split(query.Data, "_")
			if len(parts) >= 5 {
				if taskID, err := strconv.Atoi(parts[4]); err == nil {
					return t.HandleToggleTask(query, taskID, true)
				}
			}
		} else if strings.HasPrefix(query.Data, "menu_task_disable_") {
			parts := strings.Split(query.Data, "_")
			if len(parts) >= 5 {
				if taskID, err := strconv.Atoi(parts[4]); err == nil {
					return t.HandleToggleTask(query, taskID, false)
				}
			}
		} else if strings.HasPrefix(query.Data, "menu_freq_") {
			parts := strings.Split(query.Data, "_")
			log.Printf("解析频率菜单回调数据: %s, 分割结果: %v", query.Data, parts)
			if len(parts) >= 5 {
				// 修复解析逻辑: menu_freq_core_maintain_daily
				// parts: [menu, freq, core, maintain, daily]
				taskType := TaskType(fmt.Sprintf("%s_%s", parts[2], parts[3]))
				frequency := Frequency(parts[4])
				log.Printf("解析结果 - 任务类型: %s, 频率: %s", taskType, frequency)
				return t.HandleFrequencySelection(query, taskType, frequency)
			}
		} else if strings.HasPrefix(query.Data, "menu_time_") {
			parts := strings.Split(query.Data, "_")
			log.Printf("解析时间选择回调数据: %s, 分割结果: %v", query.Data, parts)
			if len(parts) >= 6 {
				// 修复解析逻辑: menu_time_core_maintain_daily_4
				// parts: [menu, time, core, maintain, daily, 4]
				taskType := TaskType(fmt.Sprintf("%s_%s", parts[2], parts[3]))
				frequency := Frequency(parts[4])
				timeValue := parts[5]
				log.Printf("解析结果 - 任务类型: %s, 频率: %s, 时间: %s", taskType, frequency, timeValue)
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
	
	text := fmt.Sprintf("📊 *系统状态*\n\n时间: %s %s\n\n📈 运行时间: %s\n📊 负载: %s\n💾 内存: %s\n💿 磁盘: %s\n🖥️ CPU: %s\n🔄 进程数: %d",
		systemTime.Format("2006-01-02 15:04:05"), timezone,
		status.Uptime, status.LoadAverage, status.MemoryUsage, status.DiskUsage, status.CPUUsage, status.ProcessCount)
	
	return t.SendMessage(query.Message.Chat.ID, text)
}

// handleCoreMaintain 处理核心维护
func (t *TGBotHandler) handleCoreMaintain(query *tgbotapi.CallbackQuery) error {
	t.maintenanceMutex.Lock()
	defer t.maintenanceMutex.Unlock()
	
	if t.isMaintenanceRunning {
		text := "⚠️ 维护任务正在执行中，请稍后再试"
		return t.SendMessage(query.Message.Chat.ID, text)
	}
	
	t.isMaintenanceRunning = true
	defer func() { t.isMaintenanceRunning = false }()
	
	// 发送开始消息
	text := "🔄 开始执行核心维护任务..."
	msg := tgbotapi.NewMessage(query.Message.Chat.ID, text)
	t.api.Send(msg)
	
	// 在 goroutine 中执行维护任务
	go func() {
		// 执行核心维护
		result, err := t.systemExec.RunCoreMaintain()
		if err != nil {
			t.SendMessage(query.Message.Chat.ID, fmt.Sprintf("❌ 核心维护失败: %v\n\n输出:\n%s", err, result))
		} else {
			t.SendMessage(query.Message.Chat.ID, fmt.Sprintf("✅ 核心维护完成\n\n输出:\n%s", result))
		}
	}()
	
	return nil
}

// handleRulesMaintain 处理规则维护
func (t *TGBotHandler) handleRulesMaintain(query *tgbotapi.CallbackQuery) error {
	t.maintenanceMutex.Lock()
	defer t.maintenanceMutex.Unlock()
	
	if t.isMaintenanceRunning {
		text := "⚠️ 维护任务正在执行中，请稍后再试"
		return t.SendMessage(query.Message.Chat.ID, text)
	}
	
	t.isMaintenanceRunning = true
	defer func() { t.isMaintenanceRunning = false }()
	
	// 发送开始消息
	text := "📜 开始执行规则维护任务..."
	msg := tgbotapi.NewMessage(query.Message.Chat.ID, text)
	t.api.Send(msg)
	
	// 在 goroutine 中执行维护任务
	go func() {
		// 执行规则维护
		result, err := t.systemExec.RunRulesMaintain()
		if err != nil {
			t.SendMessage(query.Message.Chat.ID, fmt.Sprintf("❌ 规则维护失败: %v\n\n输出:\n%s", err, result))
		} else {
			t.SendMessage(query.Message.Chat.ID, fmt.Sprintf("✅ 规则维护完成\n\n输出:\n%s", result))
		}
	}()
	
	return nil
}

// handleFullMaintain 处理完整维护
func (t *TGBotHandler) handleFullMaintain(query *tgbotapi.CallbackQuery) error {
	t.maintenanceMutex.Lock()
	defer t.maintenanceMutex.Unlock()
	
	if t.isMaintenanceRunning {
		text := "⚠️ 维护任务正在执行中，请稍后再试"
		return t.SendMessage(query.Message.Chat.ID, text)
	}
	
	t.isMaintenanceRunning = true
	defer func() { t.isMaintenanceRunning = false }()
	
	// 发送开始消息
	text := "🔄 开始执行完整维护任务..."
	msg := tgbotapi.NewMessage(query.Message.Chat.ID, text)
	t.api.Send(msg)
	
	// 在 goroutine 中执行维护任务
	go func() {
		// 执行完整维护
		result1, err1 := t.systemExec.RunCoreMaintain()
		result2, err2 := t.systemExec.RunRulesMaintain()
		
		var allResult string
		var allErr error
		
		if err1 != nil {
			allResult += fmt.Sprintf("核心维护失败: %v\n输出:\n%s\n\n", err1, result1)
			allErr = err1
		} else {
			allResult += fmt.Sprintf("核心维护完成:\n%s\n\n", result1)
		}
		
		if err2 != nil {
			allResult += fmt.Sprintf("规则维护失败: %v\n输出:\n%s", err2, result2)
			allErr = err2
		} else {
			allResult += fmt.Sprintf("规则维护完成:\n%s", result2)
		}
		
		if allErr != nil {
			t.SendMessage(query.Message.Chat.ID, fmt.Sprintf("❌ 完整维护部分失败\n\n%s", allResult))
		} else {
			t.SendMessage(query.Message.Chat.ID, fmt.Sprintf("✅ 完整维护完成\n\n%s", allResult))
		}
	}()
	
	return nil
}

// handleUpdateXray 处理 Xray 更新
func (t *TGBotHandler) handleUpdateXray(query *tgbotapi.CallbackQuery) error {
	if !t.systemExec.IsInstalled("x-ui") {
		return t.SendMessage(query.Message.Chat.ID, "❌ 未检测到 Xray/X-UI，请确保已安装")
	}
	
	text := "🔄 开始更新 Xray/X-UI..."
	t.SendMessage(query.Message.Chat.ID, text)
	
	go func() {
		result, err := t.systemExec.RunCommand("x-ui", "update")
		if err != nil {
			t.SendMessage(query.Message.Chat.ID, fmt.Sprintf("❌ Xray 更新失败: %v\n\n输出:\n%s", err, result))
		} else {
			t.SendMessage(query.Message.Chat.ID, fmt.Sprintf("✅ Xray 更新完成\n\n输出:\n%s", result))
		}
	}()
	
	return nil
}

// handleUpdateSingbox 处理 Sing-box 更新
func (t *TGBotHandler) handleUpdateSingbox(query *tgbotapi.CallbackQuery) error {
	if !t.systemExec.IsInstalled("sb") {
		return t.SendMessage(query.Message.Chat.ID, "❌ 未检测到 Sing-box，请确保已安装")
	}
	
	text := "🔄 开始更新 Sing-box..."
	t.SendMessage(query.Message.Chat.ID, text)
	
	go func() {
		result, err := t.systemExec.RunCommand("sb", "update")
		if err != nil {
			t.SendMessage(query.Message.Chat.ID, fmt.Sprintf("❌ Sing-box 更新失败: %v\n\n输出:\n%s", err, result))
		} else {
			t.SendMessage(query.Message.Chat.ID, fmt.Sprintf("✅ Sing-box 更新完成\n\n输出:\n%s", result))
		}
	}()
	
	return nil
}

// handleSetCoreSchedule 处理设置核心维护调度
func (t *TGBotHandler) handleSetCoreSchedule(query *tgbotapi.CallbackQuery) error {
	// 创建核心维护任务函数
	coreTask := func() {
		result, err := t.systemExec.RunCoreMaintain()
		// 发送执行结果通知
		if err != nil {
			t.jobManager.SetNotificationCallback(t.adminChatID, func(chatID int64, message string) {
				t.SendMessage(chatID, fmt.Sprintf("❌ 定时核心维护执行失败: %v\n\n输出:\n%s", err, result))
			})
		} else {
			t.jobManager.SetNotificationCallback(t.adminChatID, func(chatID int64, message string) {
				t.SendMessage(chatID, fmt.Sprintf("✅ 定时核心维护执行完成\n\n输出:\n%s", result))
			})
		}
	}
	
	err := t.jobManager.SetJob("core_maintain", "0 0 4 * * *", coreTask)
	if err != nil {
		return t.SendMessage(query.Message.Chat.ID, fmt.Sprintf("❌ 设置核心维护调度失败: %v", err))
	}
	
	return t.SendMessage(query.Message.Chat.ID, "✅ 核心维护调度设置成功 (每日凌晨4点)")
}

// handleSetRulesSchedule 处理设置规则维护调度
func (t *TGBotHandler) handleSetRulesSchedule(query *tgbotapi.CallbackQuery) error {
	// 创建规则维护任务函数
	rulesTask := func() {
		result, err := t.systemExec.RunRulesMaintain()
		// 发送执行结果通知
		if err != nil {
			t.jobManager.SetNotificationCallback(t.adminChatID, func(chatID int64, message string) {
				t.SendMessage(chatID, fmt.Sprintf("❌ 定时规则维护执行失败: %v\n\n输出:\n%s", err, result))
			})
		} else {
			t.jobManager.SetNotificationCallback(t.adminChatID, func(chatID int64, message string) {
				t.SendMessage(chatID, fmt.Sprintf("✅ 定时规则维护执行完成\n\n输出:\n%s", result))
			})
		}
	}
	
	err := t.jobManager.SetJob("rules_maintain", "0 0 7 * * 0", rulesTask)
	if err != nil {
		return t.SendMessage(query.Message.Chat.ID, fmt.Sprintf("❌ 设置规则维护调度失败: %v", err))
	}
	
	return t.SendMessage(query.Message.Chat.ID, "✅ 规则维护调度设置成功 (每周日上午7点)")
}

// handleSetXrayRestartSchedule 处理设置 Xray 重启调度
func (t *TGBotHandler) handleSetXrayRestartSchedule(query *tgbotapi.CallbackQuery) error {
	// 创建 Xray 重启任务函数
	xrayRestartTask := func() {
		result, err := t.systemExec.RestartService("xray")
		// 发送执行结果通知
		if err != nil {
			t.jobManager.SetNotificationCallback(t.adminChatID, func(chatID int64, message string) {
				t.SendMessage(chatID, fmt.Sprintf("❌ 定时 Xray 重启执行失败: %v\n\n输出:\n%s", err, result))
			})
		} else {
			t.jobManager.SetNotificationCallback(t.adminChatID, func(chatID int64, message string) {
				t.SendMessage(chatID, fmt.Sprintf("✅ 定时 Xray 重启执行完成\n\n输出:\n%s", result))
			})
		}
	}
	
	err := t.jobManager.SetJob("restart_xray", "0 0 2 * * *", xrayRestartTask)
	if err != nil {
		return t.SendMessage(query.Message.Chat.ID, fmt.Sprintf("❌ 设置 Xray 重启调度失败: %v", err))
	}
	
	return t.SendMessage(query.Message.Chat.ID, "✅ Xray 重启调度设置成功 (每日凌晨2点)")
}

// handleSetSingboxRestartSchedule 处理设置 Sing-box 重启调度
func (t *TGBotHandler) handleSetSingboxRestartSchedule(query *tgbotapi.CallbackQuery) error {
	// 创建 Sing-box 重启任务函数
	sbRestartTask := func() {
		result, err := t.systemExec.RestartService("sing-box")
		// 发送执行结果通知
		if err != nil {
			t.jobManager.SetNotificationCallback(t.adminChatID, func(chatID int64, message string) {
				t.SendMessage(chatID, fmt.Sprintf("❌ 定时 Sing-box 重启执行失败: %v\n\n输出:\n%s", err, result))
			})
		} else {
			t.jobManager.SetNotificationCallback(t.adminChatID, func(chatID int64, message string) {
				t.SendMessage(chatID, fmt.Sprintf("✅ 定时 Sing-box 重启执行完成\n\n输出:\n%s", result))
			})
		}
	}
	
	err := t.jobManager.SetJob("restart_singbox", "0 0 3 * * *", sbRestartTask)
	if err != nil {
		return t.SendMessage(query.Message.Chat.ID, fmt.Sprintf("❌ 设置 Sing-box 重启调度失败: %v", err))
	}
	
	return t.SendMessage(query.Message.Chat.ID, "✅ Sing-box 重启调度设置成功 (每日凌晨3点)")
}

// handleClearSchedule 处理清除调度
func (t *TGBotHandler) handleClearSchedule(query *tgbotapi.CallbackQuery) error {
	t.jobManager.ClearAll()
	return t.SendMessage(query.Message.Chat.ID, "✅ 所有调度任务已清除")
}

// handleViewLogs 处理查看日志
func (t *TGBotHandler) handleViewLogs(query *tgbotapi.CallbackQuery) error {
	logs, err := t.systemExec.GetLogs(50)
	if err != nil {
		return t.SendMessage(query.Message.Chat.ID, fmt.Sprintf("❌ 获取日志失败: %v", err))
	}
	
	// 限制日志长度
	if len(logs) > 3000 {
		logs = logs[:3000] + "\n\n... (日志过长，已截断)"
	}
	
	text := fmt.Sprintf("📋 *系统日志 (最近50条)*\n\n```\n%s\n```", logs)
	return t.SendMessage(query.Message.Chat.ID, text)
}

// handleViewHistory 处理查看维护历史
func (t *TGBotHandler) handleViewHistory(query *tgbotapi.CallbackQuery) error {
	history, err := t.historyRecorder.GetRecords(20)
	if err != nil {
		return t.SendMessage(query.Message.Chat.ID, fmt.Sprintf("❌ 获取维护历史失败: %v", err))
	}
	
	if len(history) == 0 {
		return t.SendMessage(query.Message.Chat.ID, "📜 暂无维护历史记录")
	}
	
	text := "📜 *维护历史记录*\n\n"
	for _, record := range history {
		text += fmt.Sprintf("⏰ %s\n", record.StartTime.Format("2006-01-02 15:04:05"))
		text += fmt.Sprintf("   类型: %s\n", record.Type)
		text += fmt.Sprintf("   结果: %s\n", record.Status)
		if record.Result != "" {
			// 限制输出长度
			output := record.Result
			if len(output) > 200 {
				output = output[:200] + "..."
			}
			text += fmt.Sprintf("   输出: %s\n\n", output)
		}
	}
	
	return t.SendMessage(query.Message.Chat.ID, text)
}

// handleRebootConfirm 处理重启确认
func (t *TGBotHandler) handleRebootConfirm(query *tgbotapi.CallbackQuery) error {
	keyboard := [][]tgbotapi.InlineKeyboardButton{
		{
			tgbotapi.NewInlineKeyboardButtonData("✅ 确认重启", "reboot_execute"),
			tgbotapi.NewInlineKeyboardButtonData("❌ 取消", "back_main"),
		},
	}
	
	text := "⚠️ *确认重启 VPS*\n\n⚠️ VPS 将在 30 秒后重启，请确保所有工作已保存。\n\n请确认是否继续？"
	return t.SendInlineKeyboard(query.Message.Chat.ID, text, keyboard)
}

// handleBackToMain 处理返回主菜单
func (t *TGBotHandler) handleBackToMain(query *tgbotapi.CallbackQuery) error {
	return t.ShowMainMenu(query.Message.Chat.ID)
}

// HandleTimeSelection 处理时间选择
func (t *TGBotHandler) HandleTimeSelection(query *tgbotapi.CallbackQuery, taskType TaskType, frequency Frequency, timeValue string) error {
	// 生成 Cron 表达式
	cronExpr := t.generateCronExpression(frequency, timeValue)
	
	// 验证 Cron 表达式
	if err := t.validateCronExpression(cronExpr); err != nil {
		return t.SendMessage(query.Message.Chat.ID, fmt.Sprintf("❌ Cron 表达式无效: %v", err))
	}
	
	// 生成任务名称
	taskName := t.generateTaskName(taskType, frequency, timeValue)
	
	// 添加任务
	_, err := t.jobManager.AddJob(taskName, string(taskType), cronExpr)
	if err != nil {
		return t.SendMessage(query.Message.Chat.ID, fmt.Sprintf("❌ 添加任务失败: %v", err))
	}
	
	// 确认消息
	taskDisplayName := getTaskDisplayName(string(taskType))
	frequencyDisplayName := getFrequencyDisplayName(frequency)
	text := fmt.Sprintf("✅ 任务设置确认\n\n🆔 任务: %s\n⏰ 频率: %s\n🕒 时间: %s\n📅 Cron: `%s`",
		taskDisplayName, frequencyDisplayName, timeValue, cronExpr)
	
	return t.SendMessage(query.Message.Chat.ID, text)
}

// HandleClearAllTasks 处理清除所有任务
func (t *TGBotHandler) HandleClearAllTasks(query *tgbotapi.CallbackQuery) error {
	t.jobManager.ClearAll()
	
	keyboard := [][]tgbotapi.InlineKeyboardButton{
		{
			tgbotapi.NewInlineKeyboardButtonData("➕ 添加任务", "menu_task_add"),
			tgbotapi.NewInlineKeyboardButtonData("🔙 返回", "menu_back_task_types"),
		},
	}
	
	text := "🗑️ *所有任务已清除*\n\n📭 暂无定时任务"
	return t.SendInlineKeyboard(query.Message.Chat.ID, text, keyboard)
}

// HandleDeleteTask 处理删除任务
func (t *TGBotHandler) HandleDeleteTask(query *tgbotapi.CallbackQuery, taskID int) error {
	err := t.jobManager.RemoveJobByID(taskID)
	if err != nil {
		return t.SendMessage(query.Message.Chat.ID, fmt.Sprintf("❌ 删除任务失败: %v", err))
	}
	
	// 重新显示任务列表
	return t.HandleViewTasks(query)
}

// HandleEditTask 处理编辑任务（简化版 - 目前只是显示任务信息）
func (t *TGBotHandler) HandleEditTask(query *tgbotapi.CallbackQuery, taskID int) error {
	jobList := t.jobManager.GetJobList()
	
	// 查找指定任务
	var targetJob *scheduler.JobEntry
	for _, job := range jobList {
		if job.ID == taskID {
			targetJob = &job
			break
		}
	}
	
	if targetJob == nil {
		return t.SendMessage(query.Message.Chat.ID, "❌ 未找到指定任务")
	}
	
	taskDisplayName := getTaskDisplayName(targetJob.Type)
	text := fmt.Sprintf("✏️ *编辑任务*\n\n🆔 任务名称: %s\n📝 任务类型: %s\n⏰ Cron: `%s`\n\n⚠️ 编辑功能正在开发中，当前只能查看任务信息。", 
		targetJob.Name, taskDisplayName, targetJob.Spec)
	
	keyboard := [][]tgbotapi.InlineKeyboardButton{
		{
			tgbotapi.NewInlineKeyboardButtonData("🔙 返回列表", "menu_view_tasks"),
		},
	}
	
	return t.SendInlineKeyboard(query.Message.Chat.ID, text, keyboard)
}

// HandleToggleTask 处理启用/禁用任务
func (t *TGBotHandler) HandleToggleTask(query *tgbotapi.CallbackQuery, taskID int, enable bool) error {
	// 目前调度器可能没有直接的支持启用/禁用功能
	// 这里先返回提示信息
	action := "启用"
	if !enable {
		action = "禁用"
	}
	
	return t.SendMessage(query.Message.Chat.ID, fmt.Sprintf("⚠️ %s任务功能正在开发中，当前版本暂不支持。", action))
}

// generateTaskName 生成任务名称
func (t *TGBotHandler) generateTaskName(taskType TaskType, frequency Frequency, timeValue string) string {
	taskDisplayName := getTaskDisplayName(string(taskType))
	frequencyDisplayName := getFrequencyDisplayName(frequency)
	return fmt.Sprintf("%s %s %s", taskDisplayName, frequencyDisplayName, timeValue)
}

// generateCronExpression 生成 Cron 表达式
func (t *TGBotHandler) generateCronExpression(frequency Frequency, timeValue string) string {
	switch frequency {
	case FrequencyDaily:
		// 每日: 0 {timeValue} * * *
		return fmt.Sprintf("0 %s * * *", timeValue)
	case FrequencyWeekly:
		// 每周: 0 {timeValue} * * 0 (周日)
		return fmt.Sprintf("0 %s * * 0", timeValue)
	case FrequencyMonthly:
		// 每月: 0 {timeValue} 1 * *
		return fmt.Sprintf("0 %s 1 * *", timeValue)
	default:
		return timeValue // 自定义模式直接返回用户输入
	}
}

// validateCronExpression 验证 Cron 表达式
func (t *TGBotHandler) validateCronExpression(cronExpr string) error {
	if cronExpr == "" {
		return fmt.Errorf("Cron 表达式不能为空")
	}
	
	// 基本的字段数量验证
	fields := strings.Fields(cronExpr)
	if len(fields) != 5 && len(fields) != 6 {
		return fmt.Errorf("Cron 表达式必须包含5个或6个字段")
	}
	
	// TODO: 可以使用更严格的验证，比如调用调度器的 validateCron 方法
	// 这里暂时使用基本的验证
	return nil
}