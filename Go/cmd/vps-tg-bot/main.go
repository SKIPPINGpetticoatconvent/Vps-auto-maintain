package main

import (
	"log"
	"os"
	"os/signal"
	"syscall"
	"vps-tg-bot/pkg/bot"
	"vps-tg-bot/pkg/config"
	"vps-tg-bot/pkg/scheduler"
)

func main() {
	log.Println("正在启动 VPS Telegram Bot...")

	// 加载配置
	cfg, err := config.Load()
	if err != nil {
		log.Fatalf("加载配置失败: %v", err)
	}

	// 创建 Bot 实例
	botInstance, err := bot.NewBot(cfg)
	if err != nil {
		log.Fatalf("创建 Bot 失败: %v", err)
	}

	// 创建调度器
	sched := scheduler.NewScheduler(cfg, botInstance.GetAPI())
	sched.Start()

	// 发送启动通知
	if err := botInstance.SendMessage("🤖 *VPS 管理 Bot 已启动*\n\n使用 /start 打开管理面板"); err != nil {
		log.Printf("发送启动通知失败: %v", err)
	}

	// 处理系统信号
	sigChan := make(chan os.Signal, 1)
	signal.Notify(sigChan, os.Interrupt, syscall.SIGTERM)

	// 在 goroutine 中启动 Bot
	go func() {
		botInstance.Start()
	}()

	// 等待中断信号
	<-sigChan
	log.Println("收到停止信号，正在关闭...")

	// 停止调度器
	sched.Stop()

	// 发送关闭通知
	if err := botInstance.SendMessage("⚠️ *VPS 管理 Bot 已停止*"); err != nil {
		log.Printf("发送关闭通知失败: %v", err)
	}

	log.Println("Bot 已关闭")
}
