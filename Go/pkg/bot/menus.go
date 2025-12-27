package bot

import (
	"fmt"
	"strconv"
	"strings"

	tgbotapi "github.com/go-telegram-bot-api/telegram-bot-api/v5"
)

// TaskType 任务类型
type TaskType string

const (
	TaskTypeCore       TaskType = "core_maintain"        // 核心维护
	TaskTypeRules      TaskType = "rules_maintain"       // 规则维护
	TaskTypeUpdateXray TaskType = "update_xray"          // 更新 Xray
	TaskTypeUpdateSing TaskType = "update_singbox"       // 更新 Sing-box
)

// Frequency 频率类型
type Frequency string

const (
	FrequencyDaily   Frequency = "daily"   // 每日
	FrequencyWeekly  Frequency = "weekly"  // 每周
	FrequencyMonthly Frequency = "monthly" // 每月
	FrequencyCustom  Frequency = "custom"  // 自定义
)

// MenuState 菜单状态
type MenuState struct {
	CurrentStep   string    // 当前步骤
	TaskType      TaskType  // 选择的任务类型
	Frequency     Frequency // 选择的频率
	SelectedTime  string    // 选择的时间
	CustomCron    string    // 自定义 Cron 表达式
}

// NewMenuState 创建新的菜单状态
func NewMenuState() *MenuState {
	return &MenuState{
		CurrentStep: "task_type",
	}
}

// BuildTaskTypeMenu 构建任务类型选择菜单
func (t *TGBotHandler) BuildTaskTypeMenu(chatID int64) error {
	keyboard := [][]tgbotapi.InlineKeyboardButton{
		{
			tgbotapi.NewInlineKeyboardButtonData("🔄 核心维护", fmt.Sprintf("menu_task_%s", TaskTypeCore)),
			tgbotapi.NewInlineKeyboardButtonData("🌍 规则维护", fmt.Sprintf("menu_task_%s", TaskTypeRules)),
		},
		{
			tgbotapi.NewInlineKeyboardButtonData("🔧 更新 Xray", fmt.Sprintf("menu_task_%s", TaskTypeUpdateXray)),
			tgbotapi.NewInlineKeyboardButtonData("📦 更新 Sing-box", fmt.Sprintf("menu_task_%s", TaskTypeUpdateSing)),
		},
		{
			tgbotapi.NewInlineKeyboardButtonData("📋 查看任务列表", "menu_view_tasks"),
			tgbotapi.NewInlineKeyboardButtonData("🔙 返回主菜单", "back_main"),
		},
	}

	text := "⏰ *定时任务设置*\n\n📝 请选择任务类型："
	return t.SendInlineKeyboard(chatID, text, keyboard)
}

// BuildFrequencyMenu 构建频率选择菜单
func (t *TGBotHandler) BuildFrequencyMenu(chatID int64, taskType TaskType) error {
	keyboard := [][]tgbotapi.InlineKeyboardButton{
		{
			tgbotapi.NewInlineKeyboardButtonData("🗓️ 每日执行", fmt.Sprintf("menu_freq_%s_%s", taskType, FrequencyDaily)),
			tgbotapi.NewInlineKeyboardButtonData("📅 每周执行", fmt.Sprintf("menu_freq_%s_%s", taskType, FrequencyWeekly)),
		},
		{
			tgbotapi.NewInlineKeyboardButtonData("⚙️ 自定义 Cron", fmt.Sprintf("menu_freq_%s_%s", taskType, FrequencyCustom)),
			tgbotapi.NewInlineKeyboardButtonData("🔙 返回任务类型", "menu_back_task_types"),
		},
	}

	taskDisplayName := getTaskDisplayName(string(taskType))
	text := fmt.Sprintf("⏰ *%s 定时设置*\n\n📝 请选择执行频率：", taskDisplayName)
	return t.SendInlineKeyboard(chatID, text, keyboard)
}

// BuildTimeSelectionKeyboard 构建时间选择键盘网格
func (t *TGBotHandler) BuildTimeSelectionKeyboard(chatID int64, taskType TaskType, frequency Frequency) error {
	keyboard := [][]tgbotapi.InlineKeyboardButton{}

	// 生成时间选项
	timeOptions := t.generateTimeOptions(frequency)
	
	// 按行组织按钮（每行 3 个）
	for i := 0; i < len(timeOptions); i += 3 {
		row := []tgbotapi.InlineKeyboardButton{}
		for j := i; j < i+3 && j < len(timeOptions); j++ {
			option := timeOptions[j]
			callbackData := fmt.Sprintf("menu_time_%s_%s_%s", taskType, frequency, option.Value)
			row = append(row, tgbotapi.NewInlineKeyboardButtonData(option.Label, callbackData))
		}
		keyboard = append(keyboard, row)
	}

	// 添加返回按钮
	keyboard = append(keyboard, []tgbotapi.InlineKeyboardButton{
		tgbotapi.NewInlineKeyboardButtonData("🔙 返回频率选择", fmt.Sprintf("menu_freq_%s", taskType)),
	})

	taskDisplayName := getTaskDisplayName(string(taskType))
	frequencyDisplayName := getFrequencyDisplayName(frequency)
	text := fmt.Sprintf("⏰ *%s %s执行*\n\n🕒 请选择具体执行时间：", taskDisplayName, frequencyDisplayName)

	return t.SendInlineKeyboard(chatID, text, keyboard)
}

// generateTimeOptions 生成时间选项
func (t *TGBotHandler) generateTimeOptions(frequency Frequency) []TimeOption {
	var options []TimeOption

	switch frequency {
	case FrequencyDaily:
		// 每日：24小时选项
		for hour := 0; hour < 24; hour++ {
			label := formatHourLabel(hour)
			value := fmt.Sprintf("%d", hour)
			options = append(options, TimeOption{Label: label, Value: value})
		}
	case FrequencyWeekly:
		// 每周：每周日各小时选项
		for hour := 0; hour < 24; hour++ {
			label := fmt.Sprintf("周日 %s", formatHourLabel(hour))
			value := fmt.Sprintf("0 %d", hour) // 每周日 (Dow=0)
			options = append(options, TimeOption{Label: label, Value: value})
		}
	case FrequencyMonthly:
		// 每月：每月1号各小时选项
		for hour := 0; hour < 24; hour++ {
			label := fmt.Sprintf("1号 %s", formatHourLabel(hour))
			value := fmt.Sprintf("%d 1", hour) // 每月1号
			options = append(options, TimeOption{Label: label, Value: value})
		}
	}

	return options
}

// TimeOption 时间选项
type TimeOption struct {
	Label string
	Value string
}

// formatHourLabel 格式化小时标签
func formatHourLabel(hour int) string {
	switch hour {
	case 0:
		return "深夜0点"
	case 1:
		return "深夜1点"
	case 2, 3, 4, 5:
		return fmt.Sprintf("凌晨%d点", hour)
	case 6, 7, 8, 9, 10, 11:
		return fmt.Sprintf("上午%d点", hour)
	case 12:
		return "中午12点"
	case 13, 14, 15, 16, 17:
		return fmt.Sprintf("下午%d点", hour)
	case 18, 19, 20, 21, 22, 23:
		return fmt.Sprintf("晚上%d点", hour)
	default:
		return fmt.Sprintf("%d点", hour)
	}
}

// getTaskDisplayName 获取任务类型显示名称
func getTaskDisplayName(taskType string) string {
	switch TaskType(taskType) {
	case TaskTypeCore:
		return "🔄 核心维护"
	case TaskTypeRules:
		return "🌍 规则维护"
	case TaskTypeUpdateXray:
		return "🔧 更新 Xray"
	case TaskTypeUpdateSing:
		return "📦 更新 Sing-box"
	default:
		return "❓ 未知任务"
	}
}

// getFrequencyDisplayName 获取频率显示名称
func getFrequencyDisplayName(frequency Frequency) string {
	switch frequency {
	case FrequencyDaily:
		return "每日"
	case FrequencyWeekly:
		return "每周"
	case FrequencyCustom:
		return "自定义"
	default:
		return "未知"
	}
}

// HandleTaskTypeSelection 处理任务类型选择
func (t *TGBotHandler) HandleTaskTypeSelection(query *tgbotapi.CallbackQuery, taskType TaskType) error {
	// 更新消息
	taskDisplayName := getTaskDisplayName(string(taskType))
	text := fmt.Sprintf("⏰ *%s 定时设置*\n\n📝 请选择执行频率：", taskDisplayName)

	keyboard := [][]tgbotapi.InlineKeyboardButton{
		{
			tgbotapi.NewInlineKeyboardButtonData("🗓️ 每日执行", fmt.Sprintf("menu_freq_%s_%s", taskType, FrequencyDaily)),
			tgbotapi.NewInlineKeyboardButtonData("📅 每周执行", fmt.Sprintf("menu_freq_%s_%s", taskType, FrequencyWeekly)),
		},
		{
			tgbotapi.NewInlineKeyboardButtonData("⚙️ 自定义 Cron", fmt.Sprintf("menu_freq_%s_%s", taskType, FrequencyCustom)),
			tgbotapi.NewInlineKeyboardButtonData("🔙 返回任务类型", "menu_back_task_types"),
		},
	}

	msg := tgbotapi.NewEditMessageText(query.Message.Chat.ID, query.Message.MessageID, text)
	msg.ParseMode = tgbotapi.ModeMarkdown
	keyboardMarkup := tgbotapi.NewInlineKeyboardMarkup(keyboard...)
	msg.ReplyMarkup = &keyboardMarkup

	_, err := t.api.Send(msg)
	return err
}

// HandleFrequencySelection 处理频率选择
func (t *TGBotHandler) HandleFrequencySelection(query *tgbotapi.CallbackQuery, taskType TaskType, frequency Frequency) error {
	if frequency == FrequencyCustom {
		// 自定义 Cron 模式，提示用户输入
		taskDisplayName := getTaskDisplayName(string(taskType))
		text := fmt.Sprintf("⏰ *%s 自定义定时设置*\n\n📝 请发送 Cron 表达式：\n\n*示例：*\n• 每天凌晨4点: `0 4 * * *`\n• 每周日凌晨4点: `0 4 * * Sun`\n• 每月1号凌晨4点: `0 4 1 * *`", taskDisplayName)

		keyboard := [][]tgbotapi.InlineKeyboardButton{
			{
				tgbotapi.NewInlineKeyboardButtonData("🔙 返回频率选择", fmt.Sprintf("menu_freq_%s", taskType)),
				tgbotapi.NewInlineKeyboardButtonData("🔙 返回任务类型", "menu_back_task_types"),
			},
		}

		msg := tgbotapi.NewEditMessageText(query.Message.Chat.ID, query.Message.MessageID, text)
		msg.ParseMode = tgbotapi.ModeMarkdown
		keyboardMarkup := tgbotapi.NewInlineKeyboardMarkup(keyboard...)
		msg.ReplyMarkup = &keyboardMarkup

		_, err := t.api.Send(msg)
		if err != nil {
			return err
		}

		// 发送 ForceReply 消息，提示用户输入 Cron 表达式
		replyMsg := tgbotapi.NewMessage(query.Message.Chat.ID, "请输入 Cron 表达式：")
		replyMsg.ReplyMarkup = tgbotapi.ForceReply{}
		_, err = t.api.Send(replyMsg)
		return err
	}

	// 其他频率：显示时间选择界面
	return t.BuildTimeSelectionKeyboard(query.Message.Chat.ID, taskType, frequency)
}

// HandleTimeSelection 处理时间选择
func (t *TGBotHandler) HandleTimeSelection(query *tgbotapi.CallbackQuery, taskType TaskType, frequency Frequency, timeValue string) error {
	// 构建 Cron 表达式
	cronExpr := t.buildCronExpression(frequency, timeValue)

	// 生成任务名称
	taskDisplayName := getTaskDisplayName(string(taskType))
	frequencyDisplayName := getFrequencyDisplayName(frequency)
	timeDisplayName := t.formatTimeDisplay(frequency, timeValue)

	// 显示确认信息
	text := fmt.Sprintf("⏰ *任务设置确认*\n\n✅ 任务类型: %s\n✅ 执行频率: %s\n✅ 执行时间: %s\n✅ Cron 表达式: `%s`\n\n🔄 正在设置定时任务...", 
		taskDisplayName, frequencyDisplayName, timeDisplayName, cronExpr)

	msg := tgbotapi.NewEditMessageText(query.Message.Chat.ID, query.Message.MessageID, text)
	msg.ParseMode = tgbotapi.ModeMarkdown
	_, err := t.api.Send(msg)
	if err != nil {
		return err
	}

	// 在后台设置任务
	go func() {
		err := t.setScheduledTask(taskType, cronExpr)
		if err != nil {
			t.SendMessage(query.Message.Chat.ID, fmt.Sprintf("❌ 设置定时任务失败: %v", err))
			return
		}

		// 成功消息
		successText := fmt.Sprintf("✅ *定时任务设置成功*\n\n🔧 任务: %s\n⏰ 时间: %s %s\n🆔 Cron: `%s`", 
			taskDisplayName, frequencyDisplayName, timeDisplayName, cronExpr)

		t.SendMessage(query.Message.Chat.ID, successText)
	}()

	return nil
}

// buildCronExpression 构建 Cron 表达式
func (t *TGBotHandler) buildCronExpression(frequency Frequency, timeValue string) string {
	switch frequency {
	case FrequencyDaily:
		// 每日: "0 {hour} * * *"
		return fmt.Sprintf("0 %s * * *", timeValue)
	case FrequencyWeekly:
		// 每周: "{minute} {hour} * * 0"
		return fmt.Sprintf("%s * * 0", timeValue)
	case FrequencyMonthly:
		// 每月: "0 {hour} {day} * *"
		parts := strings.Split(timeValue, " ")
		if len(parts) == 2 {
			return fmt.Sprintf("0 %s %s * *", parts[0], parts[1])
		}
		return fmt.Sprintf("0 %s * * *", timeValue)
	default:
		return timeValue
	}
}

// formatTimeDisplay 格式化时间显示
func (t *TGBotHandler) formatTimeDisplay(frequency Frequency, timeValue string) string {
	switch frequency {
	case FrequencyDaily:
		hour, _ := strconv.Atoi(timeValue)
		return formatHourLabel(hour)
	case FrequencyWeekly:
		parts := strings.Split(timeValue, " ")
		if len(parts) == 2 {
			hour, _ := strconv.Atoi(parts[1])
			return fmt.Sprintf("周日 %s", formatHourLabel(hour))
		}
		return timeValue
	case FrequencyMonthly:
		parts := strings.Split(timeValue, " ")
		if len(parts) == 2 {
			hour, _ := strconv.Atoi(parts[0])
			return fmt.Sprintf("每月1号 %s", formatHourLabel(hour))
		}
		return timeValue
	default:
		return timeValue
	}
}

// setScheduledTask 设置定时任务
func (t *TGBotHandler) setScheduledTask(taskType TaskType, cronExpr string) error {
	// 生成任务名称
	taskDisplayName := getTaskDisplayName(string(taskType))
	taskName := fmt.Sprintf("%s %s", taskDisplayName, "定时任务")

	// 使用调度器的 AddJob 方法
	_, err := t.jobManager.AddJob(taskName, string(taskType), cronExpr)
	return err
}

// HandleCustomCronInput 处理自定义 Cron 输入
func (t *TGBotHandler) HandleCustomCronInput(message *tgbotapi.Message, taskType TaskType) error {
	cronExpr := strings.TrimSpace(message.Text)
	
	// 验证 Cron 表达式
	if err := t.validateCronExpression(cronExpr); err != nil {
		return t.SendMessage(message.Chat.ID, fmt.Sprintf("❌ Cron 表达式格式错误: %v\n\n请重新输入有效的 Cron 表达式。", err))
	}

	// 生成任务名称
	taskDisplayName := getTaskDisplayName(string(taskType))
	taskName := fmt.Sprintf("%s 自定义定时任务", taskDisplayName)

	// 显示设置进度
	progressText := fmt.Sprintf("⏰ *设置自定义定时任务*\n\n🔧 任务: %s\n🆔 Cron: `%s`\n\n🔄 正在设置...", 
		taskDisplayName, cronExpr)
	
	t.SendMessage(message.Chat.ID, progressText)

	// 在后台设置任务
	go func() {
		_, err := t.jobManager.AddJob(taskName, string(taskType), cronExpr)
		if err != nil {
			t.SendMessage(message.Chat.ID, fmt.Sprintf("❌ 设置定时任务失败: %v", err))
			return
		}

		// 成功消息
		successText := fmt.Sprintf("✅ *定时任务设置成功*\n\n🔧 任务: %s\n🆔 Cron: `%s`", 
			taskDisplayName, cronExpr)
		
		t.SendMessage(message.Chat.ID, successText)
	}()

	return nil
}

// validateCronExpression 验证 Cron 表达式
func (t *TGBotHandler) validateCronExpression(cronExpr string) error {
	if strings.TrimSpace(cronExpr) == "" {
		return fmt.Errorf("Cron 表达式不能为空")
	}

	// 基本的格式验证（5个或6个字段）
	fields := strings.Fields(cronExpr)
	if len(fields) != 5 && len(fields) != 6 {
		return fmt.Errorf("Cron 表达式必须包含5个或6个字段")
	}

	// TODO: 可以使用更严格的验证，比如调用调度器的 validateCron 方法
	// 这里暂时使用基本的验证
	return nil
}

// HandleViewTasks 处理查看任务列表
func (t *TGBotHandler) HandleViewTasks(query *tgbotapi.CallbackQuery) error {
	jobList := t.jobManager.GetJobList()
	
	if len(jobList) == 0 {
		text := "📋 *任务列表*\n\n📭 暂无定时任务"
		
		keyboard := [][]tgbotapi.InlineKeyboardButton{
			{
				tgbotapi.NewInlineKeyboardButtonData("➕ 添加任务", "menu_task_add"),
				tgbotapi.NewInlineKeyboardButtonData("🔙 返回", "menu_back_task_types"),
			},
		}
		
		return t.SendInlineKeyboardWithEdit(query.Message.Chat.ID, query.Message.MessageID, text, keyboard)
	}

	// 构建任务列表文本
	text := "📋 *任务列表*\n\n"
	for _, job := range jobList {
		statusIcon := "✅"
		if !job.Enabled {
			statusIcon = "⏸️"
		}
		
		taskDisplayName := getTaskDisplayName(job.Type)
		text += fmt.Sprintf("%s *%s*\n", statusIcon, job.Name)
		text += fmt.Sprintf("   任务: %s\n", taskDisplayName)
		text += fmt.Sprintf("   时间: `%s`\n", job.Spec)
		text += fmt.Sprintf("   ID: %d\n\n", job.ID)
	}

	// 构建键盘
	keyboard := [][]tgbotapi.InlineKeyboardButton{
		{
			tgbotapi.NewInlineKeyboardButtonData("➕ 添加任务", "menu_task_add"),
			tgbotapi.NewInlineKeyboardButtonData("🔙 返回", "menu_back_task_types"),
		},
	}

	return t.SendInlineKeyboardWithEdit(query.Message.Chat.ID, query.Message.MessageID, text, keyboard)
}

// SendInlineKeyboardWithEdit 发送内联键盘（编辑现有消息）
func (t *TGBotHandler) SendInlineKeyboardWithEdit(chatID int64, messageID int, text string, keyboard [][]tgbotapi.InlineKeyboardButton) error {
	msg := tgbotapi.NewEditMessageText(chatID, messageID, text)
	msg.ParseMode = tgbotapi.ModeMarkdown
	keyboardMarkup := tgbotapi.NewInlineKeyboardMarkup(keyboard...)
	msg.ReplyMarkup = &keyboardMarkup
	_, err := t.api.Send(msg)
	return err
}