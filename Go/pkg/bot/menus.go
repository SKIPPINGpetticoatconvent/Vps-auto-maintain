package bot

import (
	"fmt"
	"log"
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
			tgbotapi.NewInlineKeyboardButtonData("🗓️ 每日执行", fmt.Sprintf("menu_freq_%s_daily", taskType)),
			tgbotapi.NewInlineKeyboardButtonData("📅 每周执行", fmt.Sprintf("menu_freq_%s_weekly", taskType)),
		},
		{
			tgbotapi.NewInlineKeyboardButtonData("⚙️ 自定义 Cron", fmt.Sprintf("menu_freq_%s_custom", taskType)),
			tgbotapi.NewInlineKeyboardButtonData("🔙 返回任务类型", "menu_back_task_types"),
		},
	}

	taskDisplayName := getTaskDisplayName(string(taskType))
	log.Printf("构建频率菜单，任务类型: %s, 显示名称: %s", taskType, taskDisplayName)
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
			log.Printf("生成时间选项回调数据: %s", callbackData)
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
	log.Printf("显示名称 - 任务: %s, 频率: %s", taskDisplayName, frequencyDisplayName)
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
	// 首先尝试直接匹配
	switch TaskType(taskType) {
	case TaskTypeCore:
		return "🔄 核心维护"
	case TaskTypeRules:
		return "🌍 规则维护"
	case TaskTypeUpdateXray:
		return "🔧 更新 Xray"
	case TaskTypeUpdateSing:
		return "📦 更新 Sing-box"
	}
	
	// 如果直接匹配失败，尝试清理和重新匹配
	taskType = strings.TrimSpace(taskType)
	switch TaskType(taskType) {
	case TaskTypeCore:
		return "🔄 核心维护"
	case TaskTypeRules:
		return "🌍 规则维护"
	case TaskTypeUpdateXray:
		return "🔧 更新 Xray"
	case TaskTypeUpdateSing:
		return "📦 更新 Sing-box"
	}
	
	// 记录调试信息
	log.Printf("无法识别的任务类型: %s", taskType)
	return "🔄 维护任务" // 使用通用名称而不是"未知任务"
}

// getFrequencyDisplayName 获取频率显示名称
func getFrequencyDisplayName(frequency Frequency) string {
	// 首先尝试直接匹配
	switch frequency {
	case FrequencyDaily:
		return "每日"
	case FrequencyWeekly:
		return "每周"
	case FrequencyMonthly:
		return "每月"
	case FrequencyCustom:
		return "自定义"
	}
	
	// 如果直接匹配失败，尝试清理和重新匹配
	frequency = Frequency(strings.TrimSpace(string(frequency)))
	switch frequency {
	case FrequencyDaily:
		return "每日"
	case FrequencyWeekly:
		return "每周"
	case FrequencyMonthly:
		return "每月"
	case FrequencyCustom:
		return "自定义"
	}
	
	// 记录调试信息
	log.Printf("无法识别的频率类型: %s", frequency)
	return "定时" // 使用通用名称而不是"未知"
}

// HandleTaskTypeSelection 处理任务类型选择
func (t *TGBotHandler) HandleTaskTypeSelection(query *tgbotapi.CallbackQuery, taskType TaskType) error {
	// 更新消息
	taskDisplayName := getTaskDisplayName(string(taskType))
	log.Printf("处理任务类型选择: %s -> %s", taskType, taskDisplayName)
	text := fmt.Sprintf("⏰ *%s 定时设置*\n\n📝 请选择执行频率：", taskDisplayName)

	keyboard := [][]tgbotapi.InlineKeyboardButton{
		{
			tgbotapi.NewInlineKeyboardButtonData("🗓️ 每日执行", fmt.Sprintf("menu_freq_%s_daily", taskType)),
			tgbotapi.NewInlineKeyboardButtonData("📅 每周执行", fmt.Sprintf("menu_freq_%s_weekly", taskType)),
		},
		{
			tgbotapi.NewInlineKeyboardButtonData("⚙️ 自定义 Cron", fmt.Sprintf("menu_freq_%s_custom", taskType)),
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
	log.Printf("处理频率选择: 任务类型=%s, 频率=%s", taskType, frequency)
	
	if frequency == FrequencyCustom {
		// 自定义 Cron 模式，提示用户输入
		taskDisplayName := getTaskDisplayName(string(taskType))
		log.Printf("自定义 Cron 模式: %s", taskDisplayName)
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
			log.Printf("发送自定义 Cron 消息失败: %v", err)
			return err
		}

		// 发送 ForceReply 消息，提示用户输入 Cron 表达式
		replyMsg := tgbotapi.NewMessage(query.Message.Chat.ID, "请输入 Cron 表达式：")
		replyMsg.ReplyMarkup = tgbotapi.ForceReply{}
		_, err = t.api.Send(replyMsg)
		if err != nil {
			log.Printf("发送 ForceReply 消息失败: %v", err)
			return err
		}
		
		return nil
	}

	// 非自定义模式，直接进入时间选择
	return t.BuildTimeSelectionKeyboard(query.Message.Chat.ID, taskType, frequency)
}

// HandleViewTasks 处理查看任务列表
func (t *TGBotHandler) HandleViewTasks(query *tgbotapi.CallbackQuery) error {
	jobList := t.jobManager.GetJobList()
	
	if len(jobList) == 0 {
		text := "📋 *任务列表*\n\n📭 暂无定时任务"
		
		keyboard := [][]tgbotapi.InlineKeyboardButton{
			{
				tgbotapi.NewInlineKeyboardButtonData("➕ 添加任务", "menu_task_add"),
				tgbotapi.NewInlineKeyboardButtonData("🗑️ 清除所有", "menu_task_clear_all"),
			},
			{
				tgbotapi.NewInlineKeyboardButtonData("🔙 返回", "menu_back_task_types"),
			},
		}
		
		return t.SendInlineKeyboardWithEdit(query.Message.Chat.ID, query.Message.MessageID, text, keyboard)
	}

	// 构建任务列表文本和操作按钮
	text := "📋 *任务列表*\n\n"
	var keyboard [][]tgbotapi.InlineKeyboardButton
	
	// 为每个任务显示详细信息和操作按钮
	for i, job := range jobList {
		statusIcon := "✅"
		if !job.Enabled {
			statusIcon = "⏸️"
		}
		
		taskDisplayName := getTaskDisplayName(job.Type)
		text += fmt.Sprintf("%s *%s* (ID: %d)\n", statusIcon, job.Name, job.ID)
		text += fmt.Sprintf("   任务: %s\n", taskDisplayName)
		text += fmt.Sprintf("   时间: `%s`\n", job.Spec)
		
		// 如果任务数量不多，为每个任务添加操作按钮
		if len(jobList) <= 5 {
			// 第一行：启用/禁用按钮
			if i == 0 || len(keyboard) == 0 || len(keyboard[len(keyboard)-1]) >= 2 {
				keyboard = append(keyboard, []tgbotapi.InlineKeyboardButton{})
			}
			
			toggleText := "⏸️ 禁用" 
			toggleAction := "menu_task_disable"
			if !job.Enabled {
				toggleText = "▶️ 启用"
				toggleAction = "menu_task_enable"
			}
			
			lastRow := &keyboard[len(keyboard)-1]
			*lastRow = append(*lastRow, tgbotapi.NewInlineKeyboardButtonData(toggleText, fmt.Sprintf("%s_%d", toggleAction, job.ID)))
			
			// 第二行：编辑和删除按钮
			if i == 0 || len(keyboard) == 1 || len(keyboard[len(keyboard)-1]) >= 2 {
				keyboard = append(keyboard, []tgbotapi.InlineKeyboardButton{})
			}
			
			editRow := &keyboard[len(keyboard)-1]
			*editRow = append(*editRow, tgbotapi.NewInlineKeyboardButtonData("✏️ 编辑", fmt.Sprintf("menu_task_edit_%d", job.ID)))
			*editRow = append(*editRow, tgbotapi.NewInlineKeyboardButtonData("🗑️ 删除", fmt.Sprintf("menu_task_delete_%d", job.ID)))
		}
		
		text += "\n"
	}

	// 添加全局操作按钮
	if len(jobList) <= 5 {
		// 如果任务较少，添加一个空行分隔
		keyboard = append(keyboard, []tgbotapi.InlineKeyboardButton{})
	}
	
	// 添加操作按钮行
	keyboard = append(keyboard, []tgbotapi.InlineKeyboardButton{
		tgbotapi.NewInlineKeyboardButtonData("➕ 添加任务", "menu_task_add"),
		tgbotapi.NewInlineKeyboardButtonData("🗑️ 清除所有", "menu_task_clear_all"),
	})
	
	// 添加返回按钮
	keyboard = append(keyboard, []tgbotapi.InlineKeyboardButton{
		tgbotapi.NewInlineKeyboardButtonData("🔙 返回", "menu_back_task_types"),
	})

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