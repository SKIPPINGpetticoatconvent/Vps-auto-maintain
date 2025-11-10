package system

import (
	"context"
	"fmt"
	"os/exec"
	"strings"
	"time"
)

// CommandExecutor 命令执行器
type CommandExecutor struct {
	Timeout time.Duration
}

// NewExecutor 创建新的命令执行器
func NewExecutor(timeout time.Duration) *CommandExecutor {
	if timeout == 0 {
		timeout = 30 * time.Second // 默认30秒超时
	}
	return &CommandExecutor{
		Timeout: timeout,
	}
}

// Execute 执行命令（带超时）
func (e *CommandExecutor) Execute(ctx context.Context, name string, args ...string) (string, error) {
	ctx, cancel := context.WithTimeout(ctx, e.Timeout)
	defer cancel()

	cmd := exec.CommandContext(ctx, name, args...)
	output, err := cmd.CombinedOutput()
	if err != nil {
		if ctx.Err() == context.DeadlineExceeded {
			return "", fmt.Errorf("命令执行超时: %s %s", name, strings.Join(args, " "))
		}
		return string(output), fmt.Errorf("执行失败: %v, 输出: %s", err, string(output))
	}

	return strings.TrimSpace(string(output)), nil
}

// ExecuteShell 执行 shell 命令
func (e *CommandExecutor) ExecuteShell(ctx context.Context, command string) (string, error) {
	return e.Execute(ctx, "sh", "-c", command)
}

// ExecuteBash 执行 bash 命令
func (e *CommandExecutor) ExecuteBash(ctx context.Context, command string) (string, error) {
	return e.Execute(ctx, "bash", "-c", command)
}

// CheckCommandExists 检查命令是否存在
func CheckCommandExists(command string) bool {
	_, err := exec.LookPath(command)
	return err == nil
}

// GetSystemInfo 获取系统信息
func GetSystemInfo() (map[string]string, error) {
	executor := NewExecutor(10 * time.Second)
	ctx := context.Background()

	info := make(map[string]string)

	// 系统负载
	if uptime, err := executor.ExecuteShell(ctx, "uptime"); err == nil {
		info["uptime"] = uptime
	}

	// 当前时间
	if date, err := executor.ExecuteShell(ctx, "date"); err == nil {
		info["date"] = date
	}

	// 内存使用
	if mem, err := executor.ExecuteShell(ctx, "free -h | grep Mem"); err == nil {
		info["memory"] = mem
	}

	// 磁盘使用
	if disk, err := executor.ExecuteShell(ctx, "df -h / | tail -1"); err == nil {
		info["disk"] = disk
	}

	// CPU 信息
	if cpu, err := executor.ExecuteShell(ctx, "top -bn1 | grep 'Cpu(s)' | head -1"); err == nil {
		info["cpu"] = cpu
	}

	return info, nil
}

// GetServiceStatus 获取服务状态
func GetServiceStatus(serviceName string) (string, error) {
	executor := NewExecutor(5 * time.Second)
	ctx := context.Background()

	if !CheckCommandExists("systemctl") {
		return "", fmt.Errorf("systemctl 命令不存在")
	}

	status, err := executor.Execute(ctx, "systemctl", "is-active", serviceName)
	if err != nil {
		return "inactive", nil
	}

	return status, nil
}
