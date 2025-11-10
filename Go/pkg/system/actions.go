package system

import (
	"context"
	"fmt"
	"os"
	"os/exec"
	"strings"
	"time"
)

var defaultExecutor = NewExecutor(30 * time.Second)

// CheckUptime 检查系统运行时间和当前时间
func CheckUptime() (string, error) {
	ctx := context.Background()
	output, err := defaultExecutor.ExecuteShell(ctx, "uptime && date")
	if err != nil {
		return "", fmt.Errorf("执行 uptime 失败: %v", err)
	}
	return output, nil
}

// GetDetailedStatus 获取详细系统状态
func GetDetailedStatus() (string, error) {
	info, err := GetSystemInfo()
	if err != nil {
		return "", err
	}

	var result strings.Builder
	result.WriteString("📊 *系统状态详情*\n\n")

	if uptime, ok := info["uptime"]; ok {
		result.WriteString(fmt.Sprintf("⏱ *运行时间*\n```\n%s\n```\n\n", uptime))
	}

	if date, ok := info["date"]; ok {
		result.WriteString(fmt.Sprintf("🕐 *当前时间*\n```\n%s\n```\n\n", date))
	}

	if mem, ok := info["memory"]; ok {
		result.WriteString(fmt.Sprintf("💾 *内存使用*\n```\n%s\n```\n\n", mem))
	}

	if disk, ok := info["disk"]; ok {
		result.WriteString(fmt.Sprintf("💿 *磁盘使用*\n```\n%s\n```\n\n", disk))
	}

	if cpu, ok := info["cpu"]; ok {
		result.WriteString(fmt.Sprintf("⚡ *CPU 使用*\n```\n%s\n```\n", cpu))
	}

	return result.String(), nil
}

// RunMaintenance 执行系统维护脚本
func RunMaintenance(scriptPath string) (string, error) {
	if _, err := os.Stat(scriptPath); os.IsNotExist(err) {
		return "", fmt.Errorf("维护脚本不存在: %s", scriptPath)
	}

	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Minute)
	defer cancel()

	output, err := defaultExecutor.ExecuteBash(ctx, scriptPath)
	if err != nil {
		// 即使命令失败，也尝试读取结果文件
		resultFile := "/tmp/vps_maintain_result.txt"
		if result, readErr := os.ReadFile(resultFile); readErr == nil {
			return string(result), fmt.Errorf("执行维护脚本失败: %v", err)
		}
		return output, fmt.Errorf("执行维护脚本失败: %v", err)
	}

	// 读取结果文件
	resultFile := "/tmp/vps_maintain_result.txt"
	if result, readErr := os.ReadFile(resultFile); readErr == nil {
		return string(result), nil
	}

	return output, nil
}

// RunRulesMaintenance 执行规则更新脚本
func RunRulesMaintenance(scriptPath string) (string, error) {
	if _, err := os.Stat(scriptPath); os.IsNotExist(err) {
		return "", fmt.Errorf("规则更新脚本不存在: %s", scriptPath)
	}

	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Minute)
	defer cancel()

	output, err := defaultExecutor.ExecuteBash(ctx, scriptPath)
	if err != nil {
		// 即使命令失败，也尝试读取结果文件
		resultFile := "/tmp/vps_rules_result.txt"
		if result, readErr := os.ReadFile(resultFile); readErr == nil {
			return string(result), fmt.Errorf("执行规则更新脚本失败: %v", err)
		}
		return output, fmt.Errorf("执行规则更新脚本失败: %v", err)
	}

	// 读取结果文件
	resultFile := "/tmp/vps_rules_result.txt"
	if result, readErr := os.ReadFile(resultFile); readErr == nil {
		return string(result), nil
	}

	return output, nil
}

// RebootVPS 重启 VPS (延迟5秒)
func RebootVPS() error {
	time.Sleep(5 * time.Second)
	cmd := exec.Command("/sbin/reboot")
	return cmd.Run()
}

// ShutdownVPS 关闭 VPS (延迟5秒)
func ShutdownVPS() error {
	time.Sleep(5 * time.Second)
	cmd := exec.Command("/sbin/shutdown", "-h", "now")
	return cmd.Run()
}

// GetLogs 获取 systemd 服务日志
func GetLogs(serviceName string, lines int) (string, error) {
	ctx := context.Background()
	output, err := defaultExecutor.Execute(ctx, "journalctl", "-u", serviceName, "-n", fmt.Sprintf("%d", lines), "--no-pager")
	if err != nil {
		return "", fmt.Errorf("获取日志失败: %v", err)
	}

	logs := output
	// 限制日志长度（最多2000字符）
	if len(logs) > 2000 {
		logs = logs[len(logs)-2000:]
	}
	return logs, nil
}
