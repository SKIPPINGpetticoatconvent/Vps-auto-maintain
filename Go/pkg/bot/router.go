package bot

import (
	"fmt"
	"log"
	"vps-tg-bot/pkg/system"

	tgbotapi "github.com/go-telegram-bot-api/telegram-bot-api/v5"
)

// CommandHandler 命令处理函数类型
type CommandHandler func(*tgbotapi.Message) error

// CallbackHandler 回调处理函数类型
type CallbackHandler func(*tgbotapi.CallbackQuery) error

// Router 命令路由器
type Router struct {
	bot          *Bot
	commands     map[string]CommandHandler
	callbacks    map[string]CallbackHandler
	errorHandler func(int64, error)
}

// NewRouter 创建新的路由器
func NewRouter(bot *Bot) *Router {
	r := &Router{
		bot:       bot,
		commands:  make(map[string]CommandHandler),
		callbacks: make(map[string]CallbackHandler),
		errorHandler: func(chatID int64, err error) {
			log.Printf("处理错误 (ChatID: %d): %v", chatID, err)
		},
	}

	// 注册命令处理器
	r.RegisterCommand("start", r.handleStartCommand)
	r.RegisterCommand("status", r.handleStatusCommand)
	r.RegisterCommand("maintain", r.handleMaintainCommand)
	r.RegisterCommand("reboot", r.handleRebootCommand)
	r.RegisterCommand("help", r.handleHelpCommand)

	// 注册回调处理器
	r.RegisterCallback("status", r.handleStatusCallback)
	r.RegisterCallback("status_detail", r.handleStatusDetailCallback)
	r.RegisterCallback("maintain_core", r.handleMaintainCallback)
	r.RegisterCallback("logs", r.handleLogsCallback)
	r.RegisterCallback("reboot", r.handleRebootCallback)
	r.RegisterCallback("back", r.handleBackCallback)

	return r
}

// RegisterCommand 注册命令处理器
func (r *Router) RegisterCommand(command string, handler CommandHandler) {
	r.commands[command] = handler
}

// RegisterCallback 注册回调处理器
func (r *Router) RegisterCallback(callback string, handler CallbackHandler) {
	r.callbacks[callback] = handler
}

// HandleMessage 处理消息
func (r *Router) HandleMessage(message *tgbotapi.Message) {
	if !r.bot.IsAdmin(message.Chat.ID) {
		r.bot.SendMessageToChat(message.Chat.ID, "❌ 无权限访问此 Bot")
		return
	}

	if message.IsCommand() {
		handler, exists := r.commands[message.Command()]
		if !exists {
			r.bot.SendMessageToChat(message.Chat.ID, "❌ 未知命令，使用 /help 查看帮助")
			return
		}

		if err := handler(message); err != nil {
			r.errorHandler(message.Chat.ID, err)
			r.bot.SendMessageToChat(message.Chat.ID, fmt.Sprintf("❌ 执行命令失败: %v", err))
		}
	}
}

// HandleCallback 处理回调
func (r *Router) HandleCallback(query *tgbotapi.CallbackQuery) {
	if !r.bot.IsAdmin(query.Message.Chat.ID) {
		callback := tgbotapi.NewCallback(query.ID, "❌ 无权限访问")
		r.bot.api.Request(callback)
		return
	}

	callback := tgbotapi.NewCallback(query.ID, "")
	r.bot.api.Request(callback)

	handler, exists := r.callbacks[query.Data]
	if !exists {
		log.Printf("未知回调: %s", query.Data)
		return
	}

	if err := handler(query); err != nil {
		r.errorHandler(query.Message.Chat.ID, err)
		msg := tgbotapi.NewEditMessageText(query.Message.Chat.ID, query.Message.MessageID, fmt.Sprintf("❌ 执行操作失败: %v", err))
		r.bot.api.Send(msg)
	}
}

// 命令处理器实现
func (r *Router) handleStartCommand(message *tgbotapi.Message) error {
	return r.bot.ShowMainMenu(message.Chat.ID)
}

func (r *Router) handleStatusCommand(message *tgbotapi.Message) error {
	info, err := system.CheckUptime()
	if err != nil {
		return err
	}

	text := fmt.Sprintf("📊 *系统状态*\n\n```\n%s\n```", info)
	return r.bot.SendMessageToChat(message.Chat.ID, text)
}

func (r *Router) handleMaintainCommand(message *tgbotapi.Message) error {
	return r.bot.ExecuteMaintenance(message.Chat.ID)
}

func (r *Router) handleRebootCommand(message *tgbotapi.Message) error {
	return r.bot.ExecuteReboot(message.Chat.ID)
}

func (r *Router) handleHelpCommand(message *tgbotapi.Message) error {
	helpText := `📖 *命令帮助*

/start - 显示主菜单
/status - 查看系统状态
/maintain - 执行系统维护
/reboot - 重启 VPS
/help - 显示此帮助信息

💡 提示：使用 /start 打开交互式菜单`
	return r.bot.SendMessageToChat(message.Chat.ID, helpText)
}

// 回调处理器实现
func (r *Router) handleStatusCallback(query *tgbotapi.CallbackQuery) error {
	info, err := system.CheckUptime()
	if err != nil {
		return err
	}

	keyboard := tgbotapi.NewInlineKeyboardMarkup(
		tgbotapi.NewInlineKeyboardRow(
			tgbotapi.NewInlineKeyboardButtonData("📊 详细状态", "status_detail"),
			tgbotapi.NewInlineKeyboardButtonData("🔙 返回", "back"),
		),
	)

	text := fmt.Sprintf("📊 *系统状态*\n\n```\n%s\n```", info)
	msg := tgbotapi.NewEditMessageText(query.Message.Chat.ID, query.Message.MessageID, text)
	msg.ReplyMarkup = &keyboard
	msg.ParseMode = tgbotapi.ModeMarkdown
	_, err = r.bot.api.Send(msg)
	return err
}

func (r *Router) handleStatusDetailCallback(query *tgbotapi.CallbackQuery) error {
	status, err := system.GetDetailedStatus()
	if err != nil {
		return err
	}

	keyboard := tgbotapi.NewInlineKeyboardMarkup(
		tgbotapi.NewInlineKeyboardRow(
			tgbotapi.NewInlineKeyboardButtonData("🔙 返回", "back"),
		),
	)

	msg := tgbotapi.NewEditMessageText(query.Message.Chat.ID, query.Message.MessageID, status)
	msg.ReplyMarkup = &keyboard
	msg.ParseMode = tgbotapi.ModeMarkdown
	_, err = r.bot.api.Send(msg)
	return err
}

func (r *Router) handleMaintainCallback(query *tgbotapi.CallbackQuery) error {
	return r.bot.ExecuteMaintenance(query.Message.Chat.ID)
}

func (r *Router) handleLogsCallback(query *tgbotapi.CallbackQuery) error {
	logs, err := system.GetLogs("vps-tg-bot", 20)
	if err != nil {
		return err
	}

	text := fmt.Sprintf("📋 *服务日志*\n\n```\n%s\n```", logs)
	msg := tgbotapi.NewEditMessageText(query.Message.Chat.ID, query.Message.MessageID, text)
	msg.ParseMode = tgbotapi.ModeMarkdown
	_, err = r.bot.api.Send(msg)
	return err
}

func (r *Router) handleRebootCallback(query *tgbotapi.CallbackQuery) error {
	return r.bot.ExecuteReboot(query.Message.Chat.ID)
}

func (r *Router) handleBackCallback(query *tgbotapi.CallbackQuery) error {
	return r.bot.ShowMainMenu(query.Message.Chat.ID)
}
