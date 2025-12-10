package main

import (
	"log"
	"os"
	"os/signal"
	"syscall"
	"vps-tg-bot/pkg/bot"
	"vps-tg-bot/pkg/config"
	"vps-tg-bot/pkg/scheduler"
	"vps-tg-bot/pkg/system"

	tgbotapi "github.com/go-telegram-bot-api/telegram-bot-api/v5"
)

func main() {
	log.Println("正在启动 VPS Telegram Bot...")

	// 加载配置
	cfg, err := config.LoadConfig()
	if err != nil {
		log.Fatalf("加载配置失败: %v", err)
	}

	// 创建系统执行器
	systemExec := system.NewRealSystemExecutor()

	// 创建调度器管理器
	jobManager := scheduler.NewCronJobManager(cfg.StateFile)

	// 创建 Telegram Bot API
	api, err := tgbotapi.NewBotAPI(cfg.TelegramToken)
	if err != nil {
		log.Fatalf("创建 Bot API 失败: %v", err)
	}

	// 创建 Bot 处理器
	botHandler := bot.NewTGBotHandler(api, systemExec, jobManager, cfg.AdminChatID)

	// 启动调度器
	jobManager.Start()

	// 发送启动通知
	if err := botHandler.SendMessage(cfg.AdminChatID, "🤖 *VPS 管理 Bot 已启动*\n\n使用 /start 打开管理面板"); err != nil {
		log.Printf("发送启动通知失败: %v", err)
	}

	// 处理系统信号
	sigChan := make(chan os.Signal, 1)
	signal.Notify(sigChan, os.Interrupt, syscall.SIGTERM)

	// 在 goroutine 中处理 Telegram 更新
	go func() {
		u := tgbotapi.NewUpdate(0)
		u.Timeout = 60
		updates := api.GetUpdatesChan(u)

		for update := range updates {
			if err := botHandler.HandleUpdate(update); err != nil {
				log.Printf("处理更新失败: %v", err)
			}
		}
	}()

	// 等待中断信号
	<-sigChan
	log.Println("收到停止信号，正在关闭...")

	// 停止调度器
	jobManager.Stop()

	// 发送关闭通知
	if err := botHandler.SendMessage(cfg.AdminChatID, "⚠️ *VPS 管理 Bot 已停止*"); err != nil {
		log.Printf("发送关闭通知失败: %v", err)
	}

	log.Println("Bot 已关闭")
}