package scheduler

import (
	"fmt"
	"log"
	"vps-tg-bot/pkg/config"
	"vps-tg-bot/pkg/system"

	tgbotapi "github.com/go-telegram-bot-api/telegram-bot-api/v5"
	"github.com/robfig/cron/v3"
)

// Scheduler 定时任务调度器
type Scheduler struct {
	cron   *cron.Cron
	config *config.Config
	botAPI *tgbotapi.BotAPI
}

// NewScheduler 创建新的调度器
func NewScheduler(cfg *config.Config, botAPI *tgbotapi.BotAPI) *Scheduler {
	// 使用秒级精度
	c := cron.New(cron.WithSeconds())
	return &Scheduler{
		cron:   c,
		config: cfg,
		botAPI: botAPI,
	}
}

// Start 启动调度器
func (s *Scheduler) Start() {
	// 每周日 04:00 执行维护任务
	// Cron 表达式: 秒 分 时 日 月 星期
	// 0 0 4 * * 0 表示每周日 04:00:00
	_, err := s.cron.AddFunc("0 0 4 * * 0", s.scheduledTask)
	if err != nil {
		log.Printf("添加定时任务失败: %v", err)
		return
	}

	s.cron.Start()
	log.Println("定时任务调度器已启动 (每周日 04:00 执行维护)")
}

// AddTask 添加自定义定时任务
func (s *Scheduler) AddTask(cronExpr string, task func()) error {
	_, err := s.cron.AddFunc(cronExpr, task)
	if err != nil {
		return fmt.Errorf("添加定时任务失败: %v", err)
	}
	return nil
}

// GetTasks 获取所有任务列表
func (s *Scheduler) GetTasks() []cron.Entry {
	return s.cron.Entries()
}

// Stop 停止调度器
func (s *Scheduler) Stop() {
	s.cron.Stop()
	log.Println("定时任务调度器已停止")
}

// scheduledTask 定时执行的任务
func (s *Scheduler) scheduledTask() {
	log.Println("开始执行定时维护任务...")

	// 执行规则更新
	_, err := system.RunRulesMaintenance(s.config.RulesScript)
	if err != nil {
		log.Printf("规则更新失败: %v", err)
	}

	// 执行系统维护
	result, err := system.RunMaintenance(s.config.CoreScript)
	if err != nil {
		log.Printf("系统维护失败: %v", err)
		s.sendNotification("❌ 定时维护执行失败: " + err.Error())
		return
	}

	// 发送通知
	message := "🕒 定时维护已执行，系统将在 5 秒后自动重启\n\n```\n" + result + "\n```"
	s.sendNotification(message)

	// 延迟5秒后重启
	go func() {
		if err := system.RebootVPS(); err != nil {
			log.Printf("重启失败: %v", err)
		}
	}()
}

// sendNotification 发送通知消息
func (s *Scheduler) sendNotification(text string) {
	if s.botAPI == nil {
		log.Println("Bot API 未初始化，无法发送通知")
		return
	}

	msg := tgbotapi.NewMessage(s.config.AdminChatID, text)
	msg.ParseMode = tgbotapi.ModeMarkdown
	_, err := s.botAPI.Send(msg)
	if err != nil {
		log.Printf("发送通知失败: %v", err)
	}
}
