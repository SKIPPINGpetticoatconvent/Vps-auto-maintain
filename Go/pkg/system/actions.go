package system

import (
	"fmt"
	"os"
	"os/exec"
	"strings"
	"time"
)

// CheckUptime 检查系统运行时间和当前时间
func CheckUptime() (string, error) {
	cmd := exec.Command("sh", "-c", "uptime && date")
	output, err := cmd.CombinedOutput()
	if err != nil {
		return "", fmt.Errorf("执行 uptime 失败: %v", err)
	}
	return strings.TrimSpace(string(output)), nil
}

// RunMaintenance 执行系统维护脚本
func RunMaintenance(scriptPath string) (string, error) {
	if _, err := os.Stat(scriptPath); os.IsNotExist(err) {
		return "", fmt.Errorf("维护脚本不存在: %s", scriptPath)
	}

	cmd := exec.Command("bash", scriptPath)
	output, err := cmd.CombinedOutput()
	if err != nil {
		return string(output), fmt.Errorf("执行维护脚本失败: %v", err)
	}

	// 读取结果文件
	resultFile := "/tmp/vps_maintain_result.txt"
	result, readErr := os.ReadFile(resultFile)
	if readErr == nil {
		return string(result), nil
	}

	return string(output), nil
}

// RunRulesMaintenance 执行规则更新脚本
func RunRulesMaintenance(scriptPath string) (string, error) {
	if _, err := os.Stat(scriptPath); os.IsNotExist(err) {
		return "", fmt.Errorf("规则更新脚本不存在: %s", scriptPath)
	}

	cmd := exec.Command("bash", scriptPath)
	output, err := cmd.CombinedOutput()
	if err != nil {
		return string(output), fmt.Errorf("执行规则更新脚本失败: %v", err)
	}

	// 读取结果文件
	resultFile := "/tmp/vps_rules_result.txt"
	result, readErr := os.ReadFile(resultFile)
	if readErr == nil {
		return string(result), nil
	}

	return string(output), nil
}

// RebootVPS 重启 VPS (延迟5秒)
func RebootVPS() error {
	time.Sleep(5 * time.Second)
	cmd := exec.Command("/sbin/reboot")
	return cmd.Run()
}

// GetLogs 获取 systemd 服务日志
func GetLogs(serviceName string, lines int) (string, error) {
	cmd := exec.Command("journalctl", "-u", serviceName, "-n", fmt.Sprintf("%d", lines), "--no-pager")
	output, err := cmd.CombinedOutput()
	if err != nil {
		return "", fmt.Errorf("获取日志失败: %v", err)
	}
	
	logs := string(output)
	// 限制日志长度（最多2000字符）
	if len(logs) > 2000 {
		logs = logs[len(logs)-2000:]
	}
	return logs, nil
}
