package e2e

import (
	"bytes"
	"fmt"
	"os"
	"runtime"
	"sync"
	"testing"
	"time"
	"vps-tg-bot/pkg/bot"
	"vps-tg-bot/pkg/config"
	"vps-tg-bot/pkg/scheduler"
	"vps-tg-bot/pkg/system"

	tgbotapi "github.com/go-telegram-bot-api/telegram-bot-api/v5"
)

// MockTelegramAPIWithMetrics 扩展 MockTelegramAPI，添加性能指标
type MockTelegramAPIWithMetrics struct {
	*MockTelegramAPI
	responseTimes []time.Duration
	mu            sync.Mutex
}

func NewMockTelegramAPIWithMetrics() *MockTelegramAPIWithMetrics {
	return &MockTelegramAPIWithMetrics{
		MockTelegramAPI: NewMockTelegramAPI(),
		responseTimes:   make([]time.Duration, 0),
	}
}

func (m *MockTelegramAPIWithMetrics) Send(c tgbotapi.Chattable) (tgbotapi.Message, error) {
	start := time.Now()
	msg, err := m.MockTelegramAPI.Send(c)
	duration := time.Since(start)
	
	m.mu.Lock()
	defer m.mu.Unlock()
	m.responseTimes = append(m.responseTimes, duration)
	
	return msg, err
}

func (m *MockTelegramAPIWithMetrics) GetAverageResponseTime() time.Duration {
	m.mu.Lock()
	defer m.mu.Unlock()
	if len(m.responseTimes) == 0 {
		return 0
	}
	
	var total time.Duration
	for _, duration := range m.responseTimes {
		total += duration
	}
	return total / time.Duration(len(m.responseTimes))
}

func (m *MockTelegramAPIWithMetrics) GetMaxResponseTime() time.Duration {
	m.mu.Lock()
	defer m.mu.Unlock()
	if len(m.responseTimes) == 0 {
		return 0
	}
	
	var max time.Duration
	for _, duration := range m.responseTimes {
		if duration > max {
			max = duration
		}
	}
	return max
}

// PerformanceTestSuite 性能测试套件
type PerformanceTestSuite struct {
	api        *MockTelegramAPIWithMetrics
	botHandler bot.BotHandler
	mockSys    *system.MockSystemExecutor
	jobManager scheduler.JobManager
	cfg        *config.Config
	adminChat  int64
	t          *testing.T
}

func NewPerformanceTestSuite(t *testing.T) *PerformanceTestSuite {
	stateFile := fmt.Sprintf("test_perf_state_%d.json", time.Now().UnixNano())
	historyFile := fmt.Sprintf("test_perf_history_%d.json", time.Now().UnixNano())

	// 创建测试脚本
	cwd, _ := os.Getwd()
	coreScript := cwd + "/test_core.sh"
	rulesScript := cwd + "/test_rules.sh"
	os.WriteFile(coreScript, []byte("#!/bin/bash\necho 'Core maintenance completed'"), 0755)
	os.WriteFile(rulesScript, []byte("#!/bin/bash\necho 'Rules update completed'"), 0755)

	// 设置环境变量
	SetTestEnv("TG_TOKEN", "123456789:ABCdefGHIjklMNOpqrsTUVwxyz1234567")
	SetTestEnv("TG_CHAT_ID", "123456789")
	SetTestEnv("STATE_FILE", stateFile)
	SetTestEnv("CORE_SCRIPT", coreScript)
	SetTestEnv("RULES_SCRIPT", rulesScript)
	SetTestEnv("HISTORY_FILE", historyFile)

	cfg, err := config.LoadConfig()
	if err != nil {
		t.Fatalf("加载配置失败: %v", err)
	}

	mockSys := system.NewMockSystemExecutor()
	setupMockSystemCommands(mockSys)

	jobManager := scheduler.NewCronJobManagerWithExecutor(stateFile, mockSys)
	jobManager.Start()

	api := NewMockTelegramAPIWithMetrics()
	botHandler := bot.NewTGBotHandler(api, cfg, mockSys, jobManager)

	return &PerformanceTestSuite{
		api:        api,
		botHandler: botHandler,
		mockSys:    mockSys,
		jobManager: jobManager,
		cfg:        cfg,
		adminChat:  cfg.AdminChatID,
		t:          t,
	}
}

func (s *PerformanceTestSuite) Cleanup() {
	s.jobManager.Stop()
	UnsetTestEnv("TG_TOKEN")
	UnsetTestEnv("TG_CHAT_ID")
	UnsetTestEnv("STATE_FILE")
	UnsetTestEnv("CORE_SCRIPT")
	UnsetTestEnv("RULES_SCRIPT")
	UnsetTestEnv("HISTORY_FILE")
	
	os.Remove("test_core.sh")
	os.Remove("test_rules.sh")
}

// 工具函数
func SetTestEnv(key, value string) {
	os.Setenv(key, value)
}

func UnsetTestEnv(key string) {
	os.Unsetenv(key)
}

// ===================== 性能测试用例 =====================

// TestPerformance_BasicResponseTime 测试基本响应时间
func TestPerformance_BasicResponseTime(t *testing.T) {
	suite := NewPerformanceTestSuite(t)
	defer suite.Cleanup()

	iterations := 100
	durations := make([]time.Duration, iterations)

	for i := 0; i < iterations; i++ {
		start := time.Now()
		
		update := tgbotapi.Update{
			CallbackQuery: &tgbotapi.CallbackQuery{
				ID: fmt.Sprintf("cb_perf_%d", i),
				From: &tgbotapi.User{ID: suite.adminChat},
				Message: &tgbotapi.Message{
					Chat:      &tgbotapi.Chat{ID: suite.adminChat},
					MessageID: 1,
				},
				Data: "status",
			},
		}
		
		err := suite.botHandler.HandleUpdate(update)
		if err != nil {
			t.Errorf("第 %d 次操作失败: %v", i+1, err)
		}
		
		durations[i] = time.Since(start)
	}

	// 计算统计信息
	var total time.Duration
	var max time.Duration
	for _, d := range durations {
		total += d
		if d > max {
			max = d
		}
	}

	avg := total / time.Duration(iterations)
	
	t.Logf("响应时间统计 (100 次操作):")
	t.Logf("  平均响应时间: %v", avg)
	t.Logf("  最大响应时间: %v", max)
	t.Logf("  最小响应时间: %v", Min(durations))
	
	// 性能断言：平均响应时间应小于 100ms
	if avg > 100*time.Millisecond {
		t.Errorf("平均响应时间过长: %v (期望 < 100ms)", avg)
	}
}

// TestPerformance_ConcurrentRequests 测试并发请求性能
func TestPerformance_ConcurrentRequests(t *testing.T) {
	suite := NewPerformanceTestSuite(t)
	defer suite.Cleanup()

	concurrency := 50
	iterationsPerGoroutine := 20
	var wg sync.WaitGroup
	errors := make(chan error, concurrency*iterationsPerGoroutine)
	start := time.Now()

	for i := 0; i < concurrency; i++ {
		wg.Add(1)
		go func(goroutineID int) {
			defer wg.Done()
			
			for j := 0; j < iterationsPerGoroutine; j++ {
				update := tgbotapi.Update{
					CallbackQuery: &tgbotapi.CallbackQuery{
						ID: fmt.Sprintf("cb_concurrent_%d_%d", goroutineID, j),
						From: &tgbotapi.User{ID: suite.adminChat},
						Message: &tgbotapi.Message{
							Chat:      &tgbotapi.Chat{ID: suite.adminChat},
							MessageID: 1,
						},
						Data: "status",
					},
				}
				
				err := suite.botHandler.HandleUpdate(update)
				if err != nil {
					errors <- fmt.Errorf("goroutine %d, iteration %d: %v", goroutineID, j, err)
				}
				
				// 短暂延迟模拟真实用户行为
				time.Sleep(10 * time.Millisecond)
			}
		}(i)
	}

	wg.Wait()
	close(errors)
	totalDuration := time.Since(start)

	// 检查错误
	errorCount := 0
	for err := range errors {
		t.Error(err)
		errorCount++
	}

	totalRequests := concurrency * iterationsPerGoroutine
	requestsPerSecond := float64(totalRequests) / totalDuration.Seconds()

	t.Logf("并发性能测试结果:")
	t.Logf("  并发数: %d", concurrency)
	t.Logf("  总请求数: %d", totalRequests)
	t.Logf("  错误数: %d", errorCount)
	t.Logf("  总耗时: %v", totalDuration)
	t.Logf("  平均请求/秒: %.2f", requestsPerSecond)
	t.Logf("  错误率: %.2f%%", float64(errorCount)/float64(totalRequests)*100)

	// 性能断言
	if errorCount > totalRequests/10 { // 错误率不应超过 10%
		t.Errorf("错误率过高: %d/%d (%.2f%%)", errorCount, totalRequests, float64(errorCount)/float64(totalRequests)*100)
	}
}

// TestPerformance_MemoryUsage 测试内存使用情况
func TestPerformance_MemoryUsage(t *testing.T) {
	suite := NewPerformanceTestSuite(t)
	defer suite.Cleanup()

	// 获取初始内存使用情况
	var m1 runtime.MemStats
	runtime.ReadMemStats(&m1)

	// 执行大量操作
	operations := 1000
	for i := 0; i < operations; i++ {
		update := tgbotapi.Update{
			CallbackQuery: &tgbotapi.CallbackQuery{
				ID: fmt.Sprintf("cb_mem_%d", i),
				From: &tgbotapi.User{ID: suite.adminChat},
				Message: &tgbotapi.Message{
					Chat:      &tgbotapi.Chat{ID: suite.adminChat},
					MessageID: 1,
				},
				Data: "status",
			},
		}
		
		err := suite.botHandler.HandleUpdate(update)
		if err != nil {
			t.Errorf("操作 %d 失败: %v", i, err)
		}
	}

	// 强制垃圾回收
	runtime.GC()
	
	// 获取操作后内存使用情况
	var m2 runtime.MemStats
	runtime.ReadMemStats(&m2)

	allocDiff := int64(m2.Alloc) - int64(m1.Alloc)
	sysDiff := int64(m2.Sys) - int64(m1.Sys)

	t.Logf("内存使用统计:")
	t.Logf("  操作前 Alloc: %d KB", m1.Alloc/1024)
	t.Logf("  操作后 Alloc: %d KB", m2.Alloc/1024)
	t.Logf("  Alloc 增长: %d KB", allocDiff/1024)
	t.Logf("  Sys 增长: %d KB", sysDiff/1024)
	t.Logf("  单次操作平均内存增长: %d bytes", allocDiff/int64(operations))

	// 内存增长检查：单次操作内存增长不应超过 1KB
	if allocDiff/int64(operations) > 1024 {
		t.Errorf("内存增长过快: 平均 %d bytes/操作", allocDiff/int64(operations))
	}
}

// TestPerformance_HighFrequencyClick 测试高频点击处理
func TestPerformance_HighFrequencyClick(t *testing.T) {
	suite := NewPerformanceTestSuite(t)
	defer suite.Cleanup()

	clickInterval := 10 * time.Millisecond // 100次/秒
	duration := 5 * time.Second
	operations := int(duration / clickInterval)
	
	var wg sync.WaitGroup
	errors := make(chan error, operations)
	
	start := time.Now()
	
	for i := 0; i < operations; i++ {
		wg.Add(1)
		go func(index int) {
			defer wg.Done()
			
			update := tgbotapi.Update{
				CallbackQuery: &tgbotapi.CallbackQuery{
					ID: fmt.Sprintf("cb_freq_%d", index),
					From: &tgbotapi.User{ID: suite.adminChat},
					Message: &tgbotapi.Message{
						Chat:      &tgbotapi.Chat{ID: suite.adminChat},
						MessageID: 1,
					},
					Data: "status",
				},
			}
			
			err := suite.botHandler.HandleUpdate(update)
			if err != nil {
				errors <- fmt.Errorf("高频点击 %d 失败: %v", index, err)
			}
		}(i)
		
		time.Sleep(clickInterval)
	}
	
	wg.Wait()
	close(errors)
	totalDuration := time.Since(start)

	// 统计错误
	errorCount := 0
	for err := range errors {
		t.Log(err)
		errorCount++
	}

	clicksPerSecond := float64(operations) / totalDuration.Seconds()

	t.Logf("高频点击测试结果:")
	t.Logf("  点击频率: %.2f 次/秒", clicksPerSecond)
	t.Logf("  总点击数: %d", operations)
	t.Logf("  错误数: %d", errorCount)
	t.Logf("  错误率: %.2f%%", float64(errorCount)/float64(operations)*100)

	// 断言：在高频点击下，系统应该保持稳定
	if errorCount > operations/20 { // 错误率不应超过 5%
		t.Errorf("高频点击下错误率过高: %d/%d (%.2f%%)", errorCount, operations, float64(errorCount)/float64(operations)*100)
	}
}

// TestPerformance_LargeMessageHandling 测试大消息处理性能
func TestPerformance_LargeMessageHandling(t *testing.T) {
	suite := NewPerformanceTestSuite(t)
	defer suite.Cleanup()

	// 生成大消息
	largeText := generateLargeText(10000) // 10KB 文本
	iterations := 50
	
	start := time.Now()
	
	for i := 0; i < iterations; i++ {
		update := tgbotapi.Update{
			Message: &tgbotapi.Message{
				Chat: &tgbotapi.Chat{ID: suite.adminChat},
				Text: largeText,
				Entities: []tgbotapi.MessageEntity{
					{Type: "bot_command", Offset: 0, Length: 6},
				},
			},
		}
		
		err := suite.botHandler.HandleUpdate(update)
		if err != nil {
			t.Errorf("大消息处理 %d 失败: %v", i+1, err)
		}
	}
	
	duration := time.Since(start)
	avgProcessingTime := duration / time.Duration(iterations)

	t.Logf("大消息处理性能:")
	t.Logf("  消息大小: %d 字符", len(largeText))
	t.Logf("  处理次数: %d", iterations)
	t.Logf("  总耗时: %v", duration)
	t.Logf("  平均处理时间: %v", avgProcessingTime)

	// 断言：大消息处理时间应合理
	if avgProcessingTime > 500*time.Millisecond {
		t.Errorf("大消息处理时间过长: %v", avgProcessingTime)
	}
}

// TestPerformance_StressTest 压力测试
func TestPerformance_StressTest(t *testing.T) {
	if testing.Short() {
		t.Skip("跳过压力测试 (使用 -short 标志)")
	}

	suite := NewPerformanceTestSuite(t)
	defer suite.Cleanup()

	// 压力测试参数
	workers := 10
	requestsPerWorker := 100
	
	var wg sync.WaitGroup
	errors := make(chan error, workers*requestsPerWorker)
	successCount := 0
	var successMu sync.Mutex
	
	start := time.Now()
	
	for w := 0; w < workers; w++ {
		wg.Add(1)
		go func(workerID int) {
			defer wg.Done()
			
			for i := 0; i < requestsPerWorker; i++ {
				// 随机选择操作类型
				operations := []string{"status", "maintain_now", "schedule_menu", "view_logs", "view_history"}
				operation := operations[(workerID+i)%len(operations)]
				
				update := tgbotapi.Update{
					CallbackQuery: &tgbotapi.CallbackQuery{
						ID: fmt.Sprintf("cb_stress_%d_%d", workerID, i),
						From: &tgbotapi.User{ID: suite.adminChat},
						Message: &tgbotapi.Message{
							Chat:      &tgbotapi.Chat{ID: suite.adminChat},
							MessageID: 1,
						},
						Data: operation,
					},
				}
				
				err := suite.botHandler.HandleUpdate(update)
				if err != nil {
					errors <- fmt.Errorf("压力测试 worker %d, request %d: %v", workerID, i, err)
				} else {
					successMu.Lock()
					successCount++
					successMu.Unlock()
				}
				
				// 随机延迟模拟真实用户行为
				time.Sleep(time.Duration(50+100*float64(i)/float64(requestsPerWorker)) * time.Millisecond)
			}
		}(w)
	}
	
	wg.Wait()
	close(errors)
	totalDuration := time.Since(start)

	// 统计结果
	errorCount := 0
	for range errors {
		errorCount++
	}

	totalRequests := workers * requestsPerWorker
	requestsPerSecond := float64(successCount) / totalDuration.Seconds()

	t.Logf("压力测试结果:")
	t.Logf("  测试时长: %v", totalDuration)
	t.Logf("  工作线程: %d", workers)
	t.Logf("  总请求数: %d", totalRequests)
	t.Logf("  成功请求: %d", successCount)
	t.Logf("  失败请求: %d", errorCount)
	t.Logf("  成功率: %.2f%%", float64(successCount)/float64(totalRequests)*100)
	t.Logf("  平均QPS: %.2f", requestsPerSecond)

	// 压力测试断言
	if errorCount > totalRequests/10 { // 错误率不应超过 10%
		t.Errorf("压力测试失败: 错误率 %.2f%%", float64(errorCount)/float64(totalRequests)*100)
	}
}

// 辅助函数

func Min(durations []time.Duration) time.Duration {
	if len(durations) == 0 {
		return 0
	}
	min := durations[0]
	for _, d := range durations {
		if d < min {
			min = d
		}
	}
	return min
}

func generateLargeText(size int) string {
	var buffer bytes.Buffer
	pattern := "这是一条测试消息，用于验证大文本处理性能。\n"
	
	for buffer.Len() < size {
		if buffer.Len()+len(pattern) > size {
			buffer.WriteString(pattern[:size-buffer.Len()])
		} else {
			buffer.WriteString(pattern)
		}
	}
	
	return buffer.String()
}