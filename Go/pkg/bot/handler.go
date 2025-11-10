package bot

import (
	"fmt"
	"log"
	"time"
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
	return b.SendMessageToChat(b.config.AdminChatID, text)
}

// SendMessageToChat 发送消息到指定聊天
func (b *Bot) SendMessageToChat(chatID int64, text string) error {
	msg := tgbotapi.NewMessage(chatID, text)
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

	router := NewRouter(b)

	for update := range b.updates {
		if update.Message != nil {
			router.HandleMessage(update.Message)
		} else if update.CallbackQuery != nil {
			router.HandleCallback(update.CallbackQuery)
		}
	}
}

// ShowMainMenu 显示主菜单
func (b *Bot) ShowMainMenu(chatID int64) error {
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

	msg := tgbotapi.NewMessage(chatID, "🤖 *VPS 管理 Bot*\n\n请选择操作：")
	msg.ReplyMarkup = keyboard
	msg.ParseMode = tgbotapi.ModeMarkdown
	_, err := b.api.Send(msg)
	return err
}

// ExecuteMaintenance 执行维护
func (b *Bot) ExecuteMaintenance(chatID int64) error {
	msg := tgbotapi.NewMessage(chatID, "⏳ 正在执行维护，请稍候...")
	b.api.Send(msg)

	// 在goroutine中执行维护，避免阻塞Bot响应
	go func() {
		result, err := system.RunMaintenance(b.config.CoreScript)
		if err != nil {
			replyMsg := tgbotapi.NewMessage(chatID, fmt.Sprintf("❌ 维护失败: %v", err))
			b.api.Send(replyMsg)
			return
		}

		replyMsg := tgbotapi.NewMessage(chatID, fmt.Sprintf("✅ *维护完成*\n\n```\n%s\n```\n\n⚠️ 系统将在 5 秒后重启", result))
		replyMsg.ParseMode = tgbotapi.ModeMarkdown
		b.api.Send(replyMsg)

		// 延迟5秒后重启
		time.Sleep(5 * time.Second)
		if err := system.RebootVPS(); err != nil {
			log.Printf("重启失败: %v", err)
		}
	}()

	return nil
}

// ExecuteReboot 执行重启
func (b *Bot) ExecuteReboot(chatID int64) error {
	msg := tgbotapi.NewMessage(chatID, "⚠️ 系统将在 5 秒后重启...")
	b.api.Send(msg)

	go func() {
		if err := system.RebootVPS(); err != nil {
			log.Printf("重启失败: %v", err)
		}
	}()

	return nil
}

// GetAPI 获取 Bot API 实例（用于定时任务发送消息）
func (b *Bot) GetAPI() *tgbotapi.BotAPI {
	return b.api
}
