package scheduler

import (
	"encoding/json"
	"fmt"
	"log"
	"os"

	"vps-tg-bot/pkg/system"

	"github.com/robfig/cron/v3"
)

// JobManager 接口定义
type JobManager interface {
	Start()
	Stop()
	
	// 添加或更新作业
	SetJob(name string, cronExp string, task func()) error
	
	// 移除作业
	RemoveJob(name string)
	
	// 清除所有作业
	ClearAll()
	
	// 获取作业状态
	GetJobStatus(name string) string // 返回 "✅ Schedule" 或 "❌ Not Set"
	
	// 状态持久化
	SaveState() error
	LoadState() error
	
	// 通知回调设置
	SetNotificationCallback(adminChatID int64, callback func(int64, string))
}

// CronJobManager 实现 JobManager 接口
type CronJobManager struct {
	cron           *cron.Cron
	stateFile      string
	jobs           map[string]*JobEntry // 存储作业信息
	taskRegistry   map[string]func() // 任务函数映射，用于在 LoadState 时重新注册任务
	systemExec     system.SystemExecutor // 系统执行器实例
	notifyCallback func(int64, string) // 通知回调函数
	adminChatID    int64 // 管理员 Chat ID
}

// JobEntry 存储作业信息
type JobEntry struct {
	EntryID    cron.EntryID
	Expression string
}

// NewCronJobManager 创建新的 CronJobManager
func NewCronJobManager(stateFile string) JobManager {
	c := cron.New(cron.WithSeconds())
	
	manager := &CronJobManager{
		cron:         c,
		stateFile:    stateFile,
		jobs:         make(map[string]*JobEntry),
		taskRegistry: make(map[string]func()),
		systemExec:   system.NewRealSystemExecutor(),
	}
	
	// 注册默认任务
	manager.registerDefaultTasks()
	
	return manager
}

// notify 发送通知消息（如果回调已设置）
func (c *CronJobManager) notify(message string) {
	if c.notifyCallback != nil && c.adminChatID != 0 {
		c.notifyCallback(c.adminChatID, message)
	}
}

// SetNotificationCallback 设置通知回调函数
func (c *CronJobManager) SetNotificationCallback(adminChatID int64, callback func(int64, string)) {
	c.adminChatID = adminChatID
	c.notifyCallback = callback
	log.Printf("已设置通知回调，管理员 Chat ID: %d", adminChatID)
}

// registerDefaultTasks 注册默认任务
func (c *CronJobManager) registerDefaultTasks() {
	// 注册核心维护任务
	c.taskRegistry["core_maintain"] = func() {
		log.Println("开始执行定时核心维护任务...")
		result, err := c.systemExec.RunCoreMaintain()
		if err != nil {
			log.Printf("核心维护失败: %v", err)
			c.notify(fmt.Sprintf("❌ 定时核心维护失败: %v", err))
		} else {
			log.Printf("核心维护完成: %s", result)
			c.notify(fmt.Sprintf("✅ 定时核心维护完成\n\n```\n%s\n```", result))
		}
	}
	
	// 注册规则维护任务
	c.taskRegistry["rules_maintain"] = func() {
		log.Println("开始执行定时规则维护任务...")
		result, err := c.systemExec.RunRulesMaintain()
		if err != nil {
			log.Printf("规则维护失败: %v", err)
			c.notify(fmt.Sprintf("❌ 定时规则维护失败: %v", err))
		} else {
			log.Printf("规则维护完成: %s", result)
			c.notify(fmt.Sprintf("✅ 定时规则维护完成\n\n```\n%s\n```", result))
		}
	}
	
	// 注册 Xray 重启任务
	c.taskRegistry["restart_xray"] = func() {
		log.Println("开始执行定时 Xray 重启任务...")
		result, err := c.systemExec.RestartService("xray")
		if err != nil {
			log.Printf("Xray 重启失败: %v", err)
			c.notify(fmt.Sprintf("❌ 定时 Xray 重启失败: %v", err))
		} else {
			log.Printf("Xray 重启完成: %s", result)
			c.notify(fmt.Sprintf("✅ 定时 Xray 重启完成\n\n```\n%s\n```", result))
		}
	}
	
	// 注册 Sing-box 重启任务
	c.taskRegistry["restart_singbox"] = func() {
		log.Println("开始执行定时 Sing-box 重启任务...")
		result, err := c.systemExec.RestartService("sing-box")
		if err != nil {
			log.Printf("Sing-box 重启失败: %v", err)
			c.notify(fmt.Sprintf("❌ 定时 Sing-box 重启失败: %v", err))
		} else {
			log.Printf("Sing-box 重启完成: %s", result)
			c.notify(fmt.Sprintf("✅ 定时 Sing-box 重启完成\n\n```\n%s\n```", result))
		}
	}
}

// Start 启动调度器
func (c *CronJobManager) Start() {
	c.cron.Start()
	log.Printf("调度器已启动，状态文件: %s", c.stateFile)
}

// Stop 停止调度器
func (c *CronJobManager) Stop() {
	c.cron.Stop()
	log.Println("调度器已停止")
}

// SetJob 添加或更新作业
func (c *CronJobManager) SetJob(name string, cronExp string, task func()) error {
	// 如果任务不存在于注册表中，使用传入的任务函数
	if _, exists := c.taskRegistry[name]; !exists && task != nil {
		c.taskRegistry[name] = task
	}
	
	// 如果作业已存在，先移除
	if existingJob, exists := c.jobs[name]; exists {
		c.cron.Remove(existingJob.EntryID)
	}
	
	// 添加新作业
	taskFunc, ok := c.taskRegistry[name]
	if !ok {
		return fmt.Errorf("任务 '%s' 未在任务注册表中找到", name)
	}
	
	entryID, err := c.cron.AddFunc(cronExp, taskFunc)
	if err != nil {
		return fmt.Errorf("添加作业失败: %v", err)
	}
	
	// 保存作业信息
	c.jobs[name] = &JobEntry{
		EntryID:    entryID,
		Expression: cronExp,
	}
	
	// 自动保存状态
	return c.SaveState()
}

// RemoveJob 移除作业
func (c *CronJobManager) RemoveJob(name string) {
	if job, exists := c.jobs[name]; exists {
		c.cron.Remove(job.EntryID)
		delete(c.jobs, name)
	}
}

// ClearAll 清除所有作业
func (c *CronJobManager) ClearAll() {
	for name := range c.jobs {
		c.RemoveJob(name)
	}
}

// GetJobStatus 获取作业状态
func (c *CronJobManager) GetJobStatus(name string) string {
	if _, exists := c.jobs[name]; exists {
		return "✅ Schedule"
	}
	return "❌ Not Set"
}

// SaveState 保存状态到文件
func (c *CronJobManager) SaveState() error {
	state := make(map[string]string)
	for name, job := range c.jobs {
		state[name] = job.Expression
	}
	
	data, err := json.MarshalIndent(state, "", "  ")
	if err != nil {
		return fmt.Errorf("序列化状态失败: %v", err)
	}
	
	err = os.WriteFile(c.stateFile, data, 0600)
	if err != nil {
		return fmt.Errorf("保存状态文件失败: %v", err)
	}
	
	return nil
}

// LoadState 从文件加载状态
func (c *CronJobManager) LoadState() error {
	data, err := os.ReadFile(c.stateFile)
	if err != nil {
		if os.IsNotExist(err) {
			// 文件不存在，不是错误
			return nil
		}
		return fmt.Errorf("读取状态文件失败: %v", err)
	}
	
	var state map[string]string
	err = json.Unmarshal(data, &state)
	if err != nil {
		return fmt.Errorf("反序列化状态失败: %v", err)
	}
	
	// 重新注册所有作业
	for name, cronExp := range state {
		if _, exists := c.taskRegistry[name]; exists {
			err = c.SetJob(name, cronExp, nil)
			if err != nil {
				log.Printf("重新注册作业 '%s' 失败: %v", name, err)
			}
		} else {
			log.Printf("跳过未知作业: %s", name)
		}
	}
	
	return nil
}