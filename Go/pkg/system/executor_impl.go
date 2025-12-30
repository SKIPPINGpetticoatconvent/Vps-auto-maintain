package system

import (
	"context"
	"fmt"
	"log"
	"os"
	"os/exec"
	"strconv"
	"strings"
	"time"
	"vps-tg-bot/pkg/config"
)

// serviceRestartCommand 服务重启命令配置
type serviceRestartCommand struct {
	command string
	args    []string
}

// serviceRestartCommands 白名单服务的重启命令映射
var serviceRestartCommands = map[string]serviceRestartCommand{
	"xray":     {command: "x-ui", args: []string{"restart"}},
	"sing-box": {command: "sb", args: []string{"restart"}},
}

// RealSystemExecutor 系统执行器的实际实现
type RealSystemExecutor struct {
	config  *config.Config
	checker SystemStatusChecker
}

// NewRealSystemExecutor 创建新的系统执行器
func NewRealSystemExecutor() *RealSystemExecutor {
	// 使用默认配置
	defaultConfig := config.GetDefaultConfig()
	executor := &RealSystemExecutor{
		config: defaultConfig,
	}
	executor.checker = NewRealSystemStatusChecker(executor)
	return executor
}

// NewRealSystemExecutorWithConfig 使用指定配置创建系统执行器
func NewRealSystemExecutorWithConfig(cfg *config.Config) *RealSystemExecutor {
	executor := &RealSystemExecutor{
		config: cfg,
	}
	executor.checker = NewRealSystemStatusChecker(executor)
	return executor
}

func (e *RealSystemExecutor) IsInstalled(program string) bool {
	// Check typical paths
	paths := []string{
		"/usr/local/bin/" + program,
		"/usr/bin/" + program,
		"/bin/" + program,
	}

	for _, p := range paths {
		if _, err := exec.LookPath(p); err == nil {
			return true
		}
	}

	// Fallback to searching in PATH
	if _, err := exec.LookPath(program); err == nil {
		return true
	}

	return false
}

func (e *RealSystemExecutor) GetSystemTime() (time.Time, string) {
	now := time.Now()
	name, _ := now.Zone()
	return now, name
}

func (e *RealSystemExecutor) RunCommand(cmd string, args ...string) (string, error) {
	out, err := exec.Command(cmd, args...).CombinedOutput()
	output := strings.TrimSpace(string(out))
	if err != nil {
		return output, err
	}
	return output, nil
}

func (e *RealSystemExecutor) RunCoreMaintain() (string, error) {
	scriptPath := e.config.CoreScript
	if scriptPath == "" {
		scriptPath = "/usr/local/bin/vps-maintain-core.sh"
	}
	if err := e.checkScriptSecurity(scriptPath); err != nil {
		return "", fmt.Errorf("security check failed: %v", err)
	}
	return e.runCommandWithTimeout(scriptPath)
}

func (e *RealSystemExecutor) RunRulesMaintain() (string, error) {
	scriptPath := e.config.RulesScript
	if scriptPath == "" {
		scriptPath = "/usr/local/bin/vps-maintain-rules.sh"
	}
	if err := e.checkScriptSecurity(scriptPath); err != nil {
		return "", fmt.Errorf("security check failed: %v", err)
	}
	return e.runCommandWithTimeout(scriptPath)
}

// checkScriptSecurity 检查脚本文件的安全性
func (e *RealSystemExecutor) checkScriptSecurity(path string) error {
	info, err := os.Stat(path)
	if err != nil {
		return fmt.Errorf("failed to stat script: %v", err)
	}

	// 检查文件权限，确保只有所有者可写 (例如 0755 或 0700)
	// 这里的检查比较宽松，只要其他用户不可写即可
	mode := info.Mode()
	if mode&0002 != 0 {
		return fmt.Errorf("script is world-writable, which is insecure")
	}

	// 在 Linux 上，我们还可以检查所有者是否为 root (UID 0)
	// 但由于 Go 的 os.FileInfo 不直接提供 UID，这需要 syscall，为了跨平台兼容性（虽然主要是 Linux），
	// 我们暂时只检查权限。在生产环境中，应该强制要求 root 所有权。
	
	return nil
}

// runCommandWithTimeout 使用配置的超时时间执行命令
func (e *RealSystemExecutor) runCommandWithTimeout(cmd string, args ...string) (string, error) {
	ctx, cancel := context.WithTimeout(context.Background(), time.Duration(e.config.CommandTimeout)*time.Second)
	defer cancel()

	// 创建命令
	c := exec.CommandContext(ctx, cmd, args...)
	
	// 设置环境变量
	c.Env = append(c.Env, os.Environ()...)
	
	// 执行命令
	out, err := c.CombinedOutput()
	output := strings.TrimSpace(string(out))
	
	// 检查超时
	if ctx.Err() == context.DeadlineExceeded {
		return output, fmt.Errorf("命令执行超时 (%d 秒)", e.config.CommandTimeout)
	}
	
	if err != nil {
		return output, err
	}
	
	return output, nil
}

func (e *RealSystemExecutor) Reboot() error {
	_, err := e.RunCommand("reboot")
	return err
}

func (e *RealSystemExecutor) GetLogs(lines int) (string, error) {
	return e.RunCommand("journalctl", "-u", "vps-tg-bot", "-n", strconv.Itoa(lines), "--no-pager")
}

// GetSystemStatus 获取系统状态信息
func (e *RealSystemExecutor) GetSystemStatus() (*SystemStatus, error) {
	return e.checker.GetSystemStatus()
}

// GetServiceStatus 获取服务状态
func (e *RealSystemExecutor) GetServiceStatus(service string) (string, error) {
	return e.checker.GetServiceStatus(service)
}

// GetResourceUsage 获取资源使用情况
func (e *RealSystemExecutor) GetResourceUsage() (*ResourceUsage, error) {
	return e.checker.GetResourceUsage()
}

// GetNetworkStatus 获取网络状态信息
func (e *RealSystemExecutor) GetNetworkStatus() (*NetworkStatus, error) {
	return e.checker.GetNetworkStatus()
}

// RestartService 重启白名单中的服务
// 支持的服务: "xray" (x-ui restart), "sing-box" (sb restart)
func (e *RealSystemExecutor) RestartService(service string) (string, error) {
	cmdInfo, ok := serviceRestartCommands[service]
	if !ok {
		allowedList := make([]string, 0, len(serviceRestartCommands))
		for k := range serviceRestartCommands {
			allowedList = append(allowedList, k)
		}
		return "", fmt.Errorf("服务 '%s' 不在允许列表中，允许的服务: %v", service, allowedList)
	}
	
	log.Printf("正在重启服务: %s (命令: %s %v)", service, cmdInfo.command, cmdInfo.args)
	return e.runCommandWithTimeout(cmdInfo.command, cmdInfo.args...)
}

// UpdateXray 更新 Xray 核心
func (e *RealSystemExecutor) UpdateXray() (string, error) {
	log.Println("正在更新 Xray 核心...")
	// 执行 Xray 更新命令
	return e.runCommandWithTimeout("bash", "-c", "curl -Ls https://raw.githubusercontent.com/mhsanaei/3x-ui/master/install.sh | bash -s install")
}

// UpdateSingbox 更新 Sing-box 核心
func (e *RealSystemExecutor) UpdateSingbox() (string, error) {
	log.Println("正在更新 Sing-box 核心...")
	// 执行 Sing-box 更新命令
	return e.runCommandWithTimeout("sb", "up")
}
