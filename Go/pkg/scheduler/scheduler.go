package scheduler

import (
	"encoding/json"
	"fmt"
	"log"
	"os"
	"strings"

	"vps-tg-bot/pkg/system"

	"github.com/robfig/cron/v3"
)

// JobManager 接口定义
type JobManager interface {
	Start()
	Stop()
	
	// 添加或更新作业 (向后兼容)
	SetJob(name string, cronExp string, task func()) error
	
	// 移除作业 (向后兼容)
	RemoveJob(name string)
	
	// 清除所有作业
	ClearAll()
	
	// 获取作业状态
	GetJobStatus(name string) string // 返回 "✅ Schedule" 或 "❌ Not Set"
	
	// 动态任务管理 (新方法)
	AddJob(name, jobType, spec string) (int, error) // 返回任务 ID
	RemoveJobByID(id int) error
	GetJobList() []JobEntry
	UpdateJobByID(id int, spec string) error
	
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
	jobs           map[string]*JobEntry // 存储作业信息（使用内部名称作为键）
	jobEntries     []JobEntry // 存储所有作业条目（用于 GetJobList）
	nextID         int // 下一个任务的 ID
	taskRegistry   map[string]func() // 任务函数映射，用于在 LoadState 时重新注册任务
	systemExec     system.SystemExecutor // 系统执行器实例
	notifyCallback func(int64, string) // 通知回调函数
	adminChatID    int64 // 管理员 Chat ID
}

// JobEntry 存储作业信息
type JobEntry struct {
	ID       int    `json:"id"`
	Name     string `json:"name"`
	Type     string `json:"type"`
	Spec     string `json:"spec"`
	Enabled  bool   `json:"enabled"`
	EntryID  cron.EntryID `json:"-"`
	InternalName string `json:"-"` // 用于内部映射的任务名称
}

// NewCronJobManager 创建新的 CronJobManager
func NewCronJobManager(stateFile string) JobManager {
	return NewCronJobManagerWithExecutor(stateFile, system.NewRealSystemExecutor())
}

// NewCronJobManagerWithExecutor 创建带有自定义执行器的 CronJobManager (用于测试)
func NewCronJobManagerWithExecutor(stateFile string, executor system.SystemExecutor) JobManager {
	c := cron.New(cron.WithSeconds())
	
	manager := &CronJobManager{
		cron:         c,
		stateFile:    stateFile,
		jobs:         make(map[string]*JobEntry),
		jobEntries:   make([]JobEntry, 0),
		nextID:       1,
		taskRegistry: make(map[string]func()),
		systemExec:   executor,
	}
	
	// 仅注册默认任务函数，不自动添加
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

// ensureDefaultJobs 确保默认任务存在（仅在首次启动或没有任务时）
func (c *CronJobManager) ensureDefaultJobs() {
	// 如果没有任何任务，则添加默认任务
	if len(c.jobs) == 0 {
		log.Println("检测到无现有任务，添加默认任务...")
		
		// 添加默认的每日核心维护任务
		if _, err := c.AddJob("每日核心维护", "core_maintain", "0 0 4 * * *"); err != nil {
			log.Printf("添加默认核心维护任务失败: %v", err)
		}
		
		// 添加默认的每周规则维护任务
		if _, err := c.AddJob("每周规则维护", "rules_maintain", "0 0 7 * * 0"); err != nil {
			log.Printf("添加默认规则维护任务失败: %v", err)
		}
		
		log.Println("默认任务添加完成")
	}
}

// Start 启动调度器
func (c *CronJobManager) Start() {
	c.cron.Start()
	
	// 加载状态后检查是否需要添加默认任务
	c.ensureDefaultJobs()
	
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
		ID:           c.nextID,
		Name:         name,
		Type:         name, // 使用 name 作为 type
		Spec:         cronExp,
		Enabled:      true,
		EntryID:      entryID,
		InternalName: name,
	}
	
	// 添加到 jobEntries
	jobEntry := *c.jobs[name]
	c.jobEntries = append(c.jobEntries, jobEntry)
	c.nextID++
	
	// 自动保存状态
	return c.SaveState()
}

// RemoveJob 移除作业
func (c *CronJobManager) RemoveJob(name string) {
	if job, exists := c.jobs[name]; exists {
		c.cron.Remove(job.EntryID)
		delete(c.jobs, name)
		
		// 从 jobEntries 中删除
		for i, entry := range c.jobEntries {
			if entry.ID == job.ID {
				c.jobEntries = append(c.jobEntries[:i], c.jobEntries[i+1:]...)
				break
			}
		}
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
	state := make(map[string]interface{})
	for name, job := range c.jobs {
		state[name] = map[string]interface{}{
			"id": job.ID,
			"name": job.Name,
			"type": job.Type,
			"spec": job.Spec,
			"enabled": job.Enabled,
		}
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
	
	var state map[string]interface{}
	err = json.Unmarshal(data, &state)
	if err != nil {
		return fmt.Errorf("反序列化状态失败: %v", err)
	}
	
	// 重新注册所有作业
	for name, jobData := range state {
		jobMap, ok := jobData.(map[string]interface{})
		if !ok {
			log.Printf("跳过无效的作业数据: %s", name)
			continue
		}
		
		// 提取作业信息
		jobType, _ := jobMap["type"].(string)
		spec, _ := jobMap["spec"].(string)
		
		// 如果任务类型在注册表中，直接添加任务
		if taskFunc, exists := c.taskRegistry[jobType]; exists {
			// 验证并转换 Cron 表达式
			convertedSpec, err := c.validateCron(spec)
			if err != nil {
				log.Printf("任务 '%s' 的 Cron 表达式无效: %v", name, err)
				continue
			}
			
			// 添加 cron 任务
			entryID, err := c.cron.AddFunc(convertedSpec, taskFunc)
			if err != nil {
				log.Printf("重新注册作业 '%s' 失败: %v", name, err)
				continue
			}
			
			// 更新为转换后的表达式
			spec = convertedSpec
			
			// 创建任务条目
			jobEntry := JobEntry{
				ID:           c.nextID,
				Name:         name,
				Type:         jobType,
				Spec:         spec,
				Enabled:      true,
				EntryID:      entryID,
				InternalName: jobType,
			}
			
			// 保存任务
			c.jobs[name] = &jobEntry
			c.jobEntries = append(c.jobEntries, jobEntry)
			c.nextID++
			
			log.Printf("已恢复任务: %s (类型: %s, ID: %d)", name, jobType, jobEntry.ID)
		} else {
			log.Printf("跳过未知任务类型: %s (作业: %s)", jobType, name)
		}
	}
	
	return nil
}

// validateCron 验证 Cron 表达式，返回转换后的表达式
func (c *CronJobManager) validateCron(spec string) (string, error) {
	if spec == "" {
		return "", fmt.Errorf("Cron 表达式不能为空")
	}
	
	// 清理空格
	spec = strings.TrimSpace(spec)
	
	// 自动转换5字段格式为6字段格式（向后兼容）
	// 如果是5字段格式，在前面添加秒字段"0 "
	fields := strings.Fields(spec)
	if len(fields) == 5 {
		spec = "0 " + spec
		log.Printf("自动转换5字段格式为6字段: %s -> %s", strings.TrimSpace(spec), spec)
	} else if len(fields) != 6 {
		return "", fmt.Errorf("Cron 表达式必须包含5或6个字段: 秒 分 时 日 月 星期或 分 时 日 月 星期")
	}
	
	// 使用 cron 库的解析器来验证表达式
	// 创建一个支持秒的解析器
	parser := cron.NewParser(cron.Second | cron.Minute | cron.Hour | cron.Dom | cron.Month | cron.Dow)
	_, err := parser.Parse(spec)
	if err != nil {
		return "", fmt.Errorf("无效的 Cron 表达式 '%s': %v", spec, err)
	}
	
	return spec, nil
}

// AddJob 动态添加任务
func (c *CronJobManager) AddJob(name, jobType, spec string) (int, error) {
	// 验证并转换 Cron 表达式
	convertedSpec, err := c.validateCron(spec)
	if err != nil {
		return 0, err
	}
	
	// 检查任务是否已存在
	if _, exists := c.jobs[name]; exists {
		return 0, fmt.Errorf("任务 '%s' 已存在", name)
	}
	
	// 检查任务类型是否在注册表中
	if _, exists := c.taskRegistry[jobType]; !exists {
		return 0, fmt.Errorf("未知的任务类型 '%s'", jobType)
	}
	
	// 获取任务函数
	taskFunc, ok := c.taskRegistry[jobType]
	if !ok {
		return 0, fmt.Errorf("任务类型 '%s' 的函数未找到", jobType)
	}
	
	// 添加 cron 任务（使用转换后的表达式）
	entryID, err := c.cron.AddFunc(convertedSpec, taskFunc)
	if err != nil {
		return 0, fmt.Errorf("添加 Cron 任务失败: %v", err)
	}
	
	// 更新为转换后的表达式
	spec = convertedSpec
	
	// 创建任务条目
	jobEntry := JobEntry{
		ID:           c.nextID,
		Name:         name,
		Type:         jobType,
		Spec:         spec,
		Enabled:      true,
		EntryID:      entryID,
		InternalName: jobType, // 使用任务类型作为内部名称
	}
	
	// 保存任务
	c.jobs[name] = &jobEntry
	c.jobEntries = append(c.jobEntries, jobEntry)
	c.nextID++
	
	// 自动保存状态
	if err := c.SaveState(); err != nil {
		log.Printf("保存状态失败: %v", err)
	}
	
	log.Printf("已添加任务: %s (类型: %s, ID: %d)", name, jobType, jobEntry.ID)
	return jobEntry.ID, nil
}

// RemoveJobByID 根据 ID 移除任务
func (c *CronJobManager) RemoveJobByID(id int) error {
	// 查找任务
	var targetJob *JobEntry
	var targetName string
	
	for name, job := range c.jobs {
		if job.ID == id {
			targetJob = job
			targetName = name
			break
		}
	}
	
	if targetJob == nil {
		return fmt.Errorf("未找到 ID 为 %d 的任务", id)
	}
	
	// 从 cron 中移除
	c.cron.Remove(targetJob.EntryID)
	
	// 从映射中删除
	delete(c.jobs, targetName)
	
	// 从切片中删除
	for i, entry := range c.jobEntries {
		if entry.ID == id {
			c.jobEntries = append(c.jobEntries[:i], c.jobEntries[i+1:]...)
			break
		}
	}
	
	// 自动保存状态
	if err := c.SaveState(); err != nil {
		log.Printf("保存状态失败: %v", err)
	}
	
	log.Printf("已移除任务: %s (ID: %d)", targetName, id)
	return nil
}

// GetJobList 获取所有任务列表
func (c *CronJobManager) GetJobList() []JobEntry {
	// 返回副本以防止外部修改
	result := make([]JobEntry, len(c.jobEntries))
	copy(result, c.jobEntries)
	return result
}

// UpdateJobByID 根据 ID 更新任务时间
func (c *CronJobManager) UpdateJobByID(id int, spec string) error {
	// 验证并转换 Cron 表达式
	convertedSpec, err := c.validateCron(spec)
	if err != nil {
		return err
	}
	
	// 查找任务
	var targetJob *JobEntry
	var targetName string
	
	for name, job := range c.jobs {
		if job.ID == id {
			targetJob = job
			targetName = name
			break
		}
	}
	
	if targetJob == nil {
		return fmt.Errorf("未找到 ID 为 %d 的任务", id)
	}
	
	// 从 cron 中移除旧任务
	c.cron.Remove(targetJob.EntryID)
	
	// 获取任务函数
	taskFunc, ok := c.taskRegistry[targetJob.InternalName]
	if !ok {
		return fmt.Errorf("任务类型 '%s' 的函数未找到", targetJob.InternalName)
	}
	
	// 添加新的 cron 任务（使用转换后的表达式）
	newEntryID, err := c.cron.AddFunc(convertedSpec, taskFunc)
	if err != nil {
		return fmt.Errorf("更新 Cron 任务失败: %v", err)
	}
	
	// 更新任务信息（使用转换后的表达式）
	targetJob.Spec = convertedSpec
	targetJob.EntryID = newEntryID
	
	// 更新切片中的副本
	for i, entry := range c.jobEntries {
		if entry.ID == id {
			c.jobEntries[i].Spec = convertedSpec
			c.jobEntries[i].EntryID = newEntryID
			break
		}
	}
	
	// 更新输入参数为转换后的表达式
	spec = convertedSpec
	
	// 自动保存状态
	if err := c.SaveState(); err != nil {
		log.Printf("保存状态失败: %v", err)
	}
	
	log.Printf("已更新任务: %s (ID: %d, 新时间: %s)", targetName, id, spec)
	return nil
}