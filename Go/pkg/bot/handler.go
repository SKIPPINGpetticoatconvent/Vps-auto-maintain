package bot

import (
	"context"
	"fmt"
	"log"
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
func NewTGBotHandler(api TelegramAPI, systemExec system.SystemExecutor, jobManager scheduler.JobManager, adminChatID int64) BotHandler {
	return &TGBotHandler{
		api:         api,
		systemExec:  systemExec,
		jobManager:  jobManager,
		adminChatID: adminChatID,
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
	case "schedule_menu":
		return t.handleScheduleMenu(query)
	case "schedule_core":
		return t.handleSetCoreSchedule(query)
	case "schedule_rules":
		return t.handleSetRulesSchedule(query)
	case "schedule_clear":
		return t.handleClearSchedule(query)
	case "view_logs":
		return t.handleViewLogs(query)
	case "reboot_confirm":
		return t.handleRebootConfirm(query)
	case "back_main":
		return t.handleBackToMain(query)
	default:
		log.Printf("未知的回调数据: %s", query.Data)
	}
	
	return nil
}

// SendMessage 发送消息
func (t *TGBotHandler) SendMessage(chatID int64, text string) error {
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
		{tgbotapi.NewInlineKeyboardButtonData("📋 查看日志", "view_logs"), tgbotapi.NewInlineKeyboardButtonData("🔄 重启 VPS", "reboot_confirm")},
	}
	
	text := "🤖 *VPS 管理 Bot*\n\n请选择操作："
	return t.SendInlineKeyboard(chatID, text, keyboard)
}

// handleMaintainMenu 显示维护菜单
func (t *TGBotHandler) handleMaintainMenu(query *tgbotapi.CallbackQuery) error {
	keyboard := [][]tgbotapi.InlineKeyboardButton{
		{tgbotapi.NewInlineKeyboardButtonData("🔧 核心维护", "maintain_core"), tgbotapi.NewInlineKeyboardButtonData("📜 规则更新", "maintain_rules")},
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
	keyboard := [][]tgbotapi.InlineKeyboardButton{
		{tgbotapi.NewInlineKeyboardButtonData("⏰ 设置核心 (每日04:00)", "schedule_core")},
		{tgbotapi.NewInlineKeyboardButtonData("📅 设置规则 (周日07:00)", "schedule_rules")},
		{tgbotapi.NewInlineKeyboardButtonData("🗑️ 清除所有", "schedule_clear"), tgbotapi.NewInlineKeyboardButtonData("🔙 返回", "back_main")},
	}
	
	text := "⚙️ *调度菜单*\n\n配置定时维护任务："
	
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
	
	text := fmt.Sprintf("📊 *系统状态*\n\n时间: %s %s\n状态: 🟢 运行正常", 
		systemTime.Format("2006-01-02 15:04:05"), timezone)
	
	return t.SendMessage(query.Message.Chat.ID, text)
}

// handleCoreMaintain 处理核心维护
func (t *TGBotHandler) handleCoreMaintain(query *tgbotapi.CallbackQuery) error {
	// 在后台执行维护
	go func() {
		result, err := t.systemExec.RunCoreMaintain()
		if err != nil {
			t.SendMessage(query.Message.Chat.ID, fmt.Sprintf("❌ 核心维护失败: %v", err))
			return
		}
		
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
		result, err := t.systemExec.RunRulesMaintain()
		if err != nil {
			t.SendMessage(query.Message.Chat.ID, fmt.Sprintf("❌ 规则维护失败: %v", err))
			return
		}
		
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

		// 执行核心维护
		coreResult, err := t.runWithTimeout(ctx, func() (string, error) {
			return t.systemExec.RunCoreMaintain()
		})
		if err != nil {
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
			if ctx.Err() == context.DeadlineExceeded {
				t.SendMessage(query.Message.Chat.ID, "❌ 维护任务超时，已取消")
			} else {
				t.SendMessage(query.Message.Chat.ID, fmt.Sprintf("❌ 规则维护失败: %v", err))
			}
			return
		}

		result := fmt.Sprintf("核心维护:\n%s\n\n规则维护:\n%s", coreResult, rulesResult)
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