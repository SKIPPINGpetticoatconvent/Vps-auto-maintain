package bot

import (
	"fmt"
	"log"
	"vps-tg-bot/pkg/config"
	"vps-tg-bot/pkg/system"

	tgbotapi "github.com/go-telegram-bot-api/telegram-bot-api/v5"
)

// Bot 结构体
type Bot struct {
	api     *tgbotapi.BotAPI
	config  *config.Config
	updates tgbotapi.UpdatesChannel
}

// NewBot 创建新的 Bot 实例
func NewBot(cfg *config.Config) (*Bot, error) {
	api, err := tgbotapi.NewBotAPI(cfg.Token)
	if err != nil {
		return nil, fmt.Errorf("创建 Bot API 失败: %v", err)
	}

	api.Debug = false
	log.Printf("已授权为: %s", api.Self.UserName)

	u := tgbotapi.NewUpdate(0)
	u.Timeout = 60
	updates := api.GetUpdatesChan(u)

	return &Bot{
		api:     api,
		config:  cfg,
		updates: updates,
	}, nil
}

// SendMessage 发送消息给管理员
func (b *Bot) SendMessage(text string) error {
	msg := tgbotapi.NewMessage(b.config.AdminChatID, text)
	msg.ParseMode = tgbotapi.ModeMarkdown
	_, err := b.api.Send(msg)
	return err
}

// IsAdmin 检查用户是否为管理员
func (b *Bot) IsAdmin(chatID int64) bool {
	return chatID == b.config.AdminChatID
}

// Start 启动 Bot 并处理消息
func (b *Bot) Start() {
	log.Println("Bot 开始运行...")

	for update := range b.updates {
		if update.Message != nil {
			b.handleMessage(update.Message)
		} else if update.CallbackQuery != nil {
			b.handleCallback(update.CallbackQuery)
		}
	}
}

// handleMessage 处理文本消息
func (b *Bot) handleMessage(message *tgbotapi.Message) {
	if !b.IsAdmin(message.Chat.ID) {
		msg := tgbotapi.NewMessage(message.Chat.ID, "❌ 无权限访问此 Bot")
		b.api.Send(msg)
		return
	}

	if message.IsCommand() {
		switch message.Command() {
		case "start":
			b.handleStart(message)
		case "status":
			b.handleStatus(message)
		case "maintain":
			b.handleMaintain(message)
		case "reboot":
			b.handleReboot(message)
		}
	}
}

// handleStart 处理 /start 命令
func (b *Bot) handleStart(message *tgbotapi.Message) {
	keyboard := tgbotapi.NewInlineKeyboardMarkup(
		tgbotapi.NewInlineKeyboardRow(
			tgbotapi.NewInlineKeyboardButtonData("📊 系统状态", "status"),
		),
		tgbotapi.NewInlineKeyboardRow(
			tgbotapi.NewInlineKeyboardButtonData("🔧 立即维护", "maintain_core"),
		),
		tgbotapi.NewInlineKeyboardRow(
			tgbotapi.NewInlineKeyboardButtonData("📋 查看日志", "logs"),
		),
		tgbotapi.NewInlineKeyboardRow(
			tgbotapi.NewInlineKeyboardButtonData("♻️ 重启 VPS", "reboot"),
		),
	)

	msg := tgbotapi.NewMessage(message.Chat.ID, "🤖 *VPS 管理 Bot*\n\n请选择操作：")
	msg.ReplyMarkup = keyboard
	msg.ParseMode = tgbotapi.ModeMarkdown
	b.api.Send(msg)
}

// handleStatus 处理 /status 命令
func (b *Bot) handleStatus(message *tgbotapi.Message) {
	info, err := system.CheckUptime()
	if err != nil {
		msg := tgbotapi.NewMessage(message.Chat.ID, fmt.Sprintf("❌ 获取系统状态失败: %v", err))
		b.api.Send(msg)
		return
	}

	msg := tgbotapi.NewMessage(message.Chat.ID, fmt.Sprintf("📊 *系统状态*\n\n```\n%s\n```", info))
	msg.ParseMode = tgbotapi.ModeMarkdown
	b.api.Send(msg)
}

// handleMaintain 处理 /maintain 命令
func (b *Bot) handleMaintain(message *tgbotapi.Message) {
	msg := tgbotapi.NewMessage(message.Chat.ID, "⏳ 正在执行维护，请稍候...")
	b.api.Send(msg)

	result, err := system.RunMaintenance(b.config.CoreScript)
	if err != nil {
		msg := tgbotapi.NewMessage(message.Chat.ID, fmt.Sprintf("❌ 维护失败: %v", err))
		b.api.Send(msg)
		return
	}

	replyMsg := tgbotapi.NewMessage(message.Chat.ID, fmt.Sprintf("✅ *维护完成*\n\n```\n%s\n```\n\n⚠️ 系统将在 5 秒后重启", result))
	replyMsg.ParseMode = tgbotapi.ModeMarkdown
	b.api.Send(replyMsg)

	// 延迟5秒后重启
	go func() {
		if err := system.RebootVPS(); err != nil {
			log.Printf("重启失败: %v", err)
		}
	}()
}

// handleReboot 处理 /reboot 命令
func (b *Bot) handleReboot(message *tgbotapi.Message) {
	msg := tgbotapi.NewMessage(message.Chat.ID, "⚠️ 系统将在 5 秒后重启...")
	b.api.Send(msg)

	go func() {
		if err := system.RebootVPS(); err != nil {
			log.Printf("重启失败: %v", err)
		}
	}()
}

// handleCallback 处理回调查询（按钮点击）
func (b *Bot) handleCallback(query *tgbotapi.CallbackQuery) {
	if !b.IsAdmin(query.Message.Chat.ID) {
		callback := tgbotapi.NewCallback(query.ID, "❌ 无权限访问")
		b.api.Request(callback)
		return
	}

	callback := tgbotapi.NewCallback(query.ID, "")
	b.api.Request(callback)

	switch query.Data {
	case "status":
		b.handleStatusCallback(query)
	case "maintain_core":
		b.handleMaintainCallback(query)
	case "logs":
		b.handleLogsCallback(query)
	case "reboot":
		b.handleRebootCallback(query)
	}
}

// handleStatusCallback 处理状态查询回调
func (b *Bot) handleStatusCallback(query *tgbotapi.CallbackQuery) {
	info, err := system.CheckUptime()
	if err != nil {
		msg := tgbotapi.NewEditMessageText(query.Message.Chat.ID, query.Message.MessageID, fmt.Sprintf("❌ 获取系统状态失败: %v", err))
		b.api.Send(msg)
		return
	}

	msg := tgbotapi.NewEditMessageText(query.Message.Chat.ID, query.Message.MessageID, fmt.Sprintf("📊 *系统状态*\n\n```\n%s\n```", info))
	msg.ParseMode = tgbotapi.ModeMarkdown
	b.api.Send(msg)
}

// handleMaintainCallback 处理维护回调
func (b *Bot) handleMaintainCallback(query *tgbotapi.CallbackQuery) {
	msg := tgbotapi.NewEditMessageText(query.Message.Chat.ID, query.Message.MessageID, "⏳ 正在执行维护，请稍候...")
	b.api.Send(msg)

	result, err := system.RunMaintenance(b.config.CoreScript)
	if err != nil {
		msg := tgbotapi.NewEditMessageText(query.Message.Chat.ID, query.Message.MessageID, fmt.Sprintf("❌ 维护失败: %v", err))
		b.api.Send(msg)
		return
	}

	replyMsg := tgbotapi.NewEditMessageText(query.Message.Chat.ID, query.Message.MessageID, fmt.Sprintf("✅ *维护完成*\n\n```\n%s\n```\n\n⚠️ 系统将在 5 秒后重启", result))
	replyMsg.ParseMode = tgbotapi.ModeMarkdown
	b.api.Send(replyMsg)

	// 延迟5秒后重启
	go func() {
		if err := system.RebootVPS(); err != nil {
			log.Printf("重启失败: %v", err)
		}
	}()
}

// handleLogsCallback 处理日志查询回调
func (b *Bot) handleLogsCallback(query *tgbotapi.CallbackQuery) {
	logs, err := system.GetLogs("vps-tg-bot", 20)
	if err != nil {
		msg := tgbotapi.NewEditMessageText(query.Message.Chat.ID, query.Message.MessageID, fmt.Sprintf("❌ 获取日志失败: %v", err))
		b.api.Send(msg)
		return
	}

	msg := tgbotapi.NewEditMessageText(query.Message.Chat.ID, query.Message.MessageID, fmt.Sprintf("📋 *日志*\n\n```\n%s\n```", logs))
	msg.ParseMode = tgbotapi.ModeMarkdown
	b.api.Send(msg)
}

// handleRebootCallback 处理重启回调
func (b *Bot) handleRebootCallback(query *tgbotapi.CallbackQuery) {
	msg := tgbotapi.NewEditMessageText(query.Message.Chat.ID, query.Message.MessageID, "⚠️ 系统将在 5 秒后重启...")
	b.api.Send(msg)

	go func() {
		if err := system.RebootVPS(); err != nil {
			log.Printf("重启失败: %v", err)
		}
	}()
}

// GetAPI 获取 Bot API 实例（用于定时任务发送消息）
func (b *Bot) GetAPI() *tgbotapi.BotAPI {
	return b.api
}
