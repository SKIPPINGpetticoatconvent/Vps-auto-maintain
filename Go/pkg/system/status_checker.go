
package system

import (
	"fmt"
	"strconv"
	"strings"
	"time"
)

// SystemStatusChecker 系统状态检查器接口
type SystemStatusChecker interface {
	GetSystemStatus() (*SystemStatus, error)
	GetServiceStatus(service string) (string, error)
	GetResourceUsage() (*ResourceUsage, error)
	GetNetworkStatus() (*NetworkStatus, error)
}

// RealSystemStatusChecker 系统状态检查器的实际实现
type RealSystemStatusChecker struct {
	executor *RealSystemExecutor
}

// NewRealSystemStatusChecker 创建新的系统状态检查器
func NewRealSystemStatusChecker(executor *RealSystemExecutor) SystemStatusChecker {
	return &RealSystemStatusChecker{
		executor: executor,
	}
}

// GetSystemStatus 获取系统状态信息
func (c *RealSystemStatusChecker) GetSystemStatus() (*SystemStatus, error) {
	// 获取系统运行时间
	uptime, err := c.getUptime()
	if err != nil {
		return nil, err
	}

	// 获取系统负载
	loadAvg, err := c.getLoadAverage()
	if err != nil {
		return nil, err
	}

	// 获取内存使用情况
	memoryUsage, err := c.getMemoryUsage()
	if err != nil {
		return nil, err
	}

	// 获取磁盘使用情况
	diskUsage, err := c.getDiskUsage()
	if err != nil {
		return nil, err
	}

	// 获取CPU使用率
	cpuUsage, err := c.getCPUUsage()
	if err != nil {
		return nil, err
	}

	// 获取进程数量
	processCount, err := c.getProcessCount()
	if err != nil {
		return nil, err
	}

	return &SystemStatus{
		Uptime:       uptime,
		LoadAverage:  loadAvg,
		MemoryUsage:  memoryUsage,
		DiskUsage:    diskUsage,
		CPUUsage:     cpuUsage,
		ProcessCount: processCount,
	}, nil
}

// GetServiceStatus 获取服务状态
func (c *RealSystemStatusChecker) GetServiceStatus(service string) (string, error) {
	output, err := c.executor.RunCommand("systemctl", "is-active", service)
	if err != nil {
		return "inactive", nil
	}
	return strings.TrimSpace(output), nil
}

// GetResourceUsage 获取资源使用情况
func (c *RealSystemStatusChecker) GetResourceUsage() (*ResourceUsage, error) {
	// 获取内存信息
	memInfo, err := c.getMemoryInfo()
	if err != nil {
		return nil, err
	}

	// 获取磁盘信息
	diskInfo, err := c.getDiskInfo()
	if err != nil {
		return nil, err
	}

	// 获取CPU使用率
	cpuPercent, err := c.getCPUPercent()
	if err != nil {
		return nil, err
	}

	// 获取进程数量
	processCount, err := c.getProcessCount()
	if err != nil {
		return nil, err
	}

	return &ResourceUsage{
		MemoryTotal:     memInfo.Total,
		MemoryUsed:      memInfo.Used,
		MemoryFree:      memInfo.Free,
		MemoryPercent:   memInfo.Percent,
		DiskTotal:       diskInfo.Total,
		DiskUsed:        diskInfo.Used,
		DiskFree:        diskInfo.Free,
		DiskPercent:     diskInfo.Percent,
		CPUPercent:      cpuPercent,
		ProcessCount:    processCount,
	}, nil
}

// GetNetworkStatus 获取网络状态信息
func (c *RealSystemStatusChecker) GetNetworkStatus() (*NetworkStatus, error) {
	// 获取网络接口信息
	interfaces, err := c.getNetworkInterfaces()
	if err != nil {
		return nil, err
	}

	// 获取网络连接数
	connections, err := c.getNetworkConnections()
	if err != nil {
		return nil, err
	}

	// 获取活跃服务
	activeServices, err := c.getActiveServices()
	if err != nil {
		return nil, err
	}

	return &NetworkStatus{
		Interfaces:     interfaces,
		Connections:    connections,
		ActiveServices: activeServices,
	}, nil
}

// 辅助方法实现
func (c *RealSystemStatusChecker) getUptime() (string, error) {
	output, err := c.executor.RunCommand("uptime", "-p")
	if err != nil {
		// 尝试另一种格式
		output, err = c.executor.RunCommand("cat", "/proc/uptime")
		if err != nil {
			return "unknown", err
		}
		parts := strings.Fields(output)
		if len(parts) > 0 {
			seconds, _ := strconv.ParseFloat(parts[0], 64)
			uptime := time.Duration(seconds) * time.Second
			return uptime.String(), nil
		}
		return "unknown", nil
	}
	return strings.TrimSpace(output), nil
}

func (c *RealSystemStatusChecker) getLoadAverage() (string, error) {
	output, err := c.executor.RunCommand("cat", "/proc/loadavg")
	if err != nil {
		return "unknown", err
	}
	parts := strings.Fields(output)
	if len(parts) >= 3 {
		return fmt.Sprintf("%s %s %s", parts[0], parts[1], parts[2]), nil
	}
	return "unknown", nil
}

func (c *RealSystemStatusChecker) getMemoryUsage() (string, error) {
	output, err := c.executor.RunCommand("free", "-h")
	if err != nil {
		return "unknown", err
	}
	lines := strings.Split(output, "\n")
	if len(lines) >= 2 {
		parts := strings.Fields(lines[1])
		if len(parts) >= 7 {
			return fmt.Sprintf("%s/%s", parts[2], parts[1]), nil
		}
	}
	return "unknown", nil
}

func (c *RealSystemStatusChecker) getDiskUsage() (string, error) {
	output, err := c.executor.RunCommand("df", "-h", "/")
	if err != nil {
		return "unknown", err
	}
	lines := strings.Split(output, "\n")
	if len(lines) >= 2 {
		parts := strings.Fields(lines[1])
		if len(parts) >= 5 {
			return fmt.Sprintf("%s/%s (%s)", parts[2], parts[1], parts[4]), nil
		}
	}
	return "unknown", nil
}

func (c *RealSystemStatusChecker) getCPUUsage() (string, error) {
	// 简化实现，返回最近1分钟的负载
	output, err := c.executor.RunCommand("cat", "/proc/loadavg")
	if err != nil {
		return "unknown", err
	}
	parts := strings.Fields(output)
	if len(parts) > 0 {
		return parts[0], nil
	}
	return "unknown", nil
}

func (c *RealSystemStatusChecker) getProcessCount() (int, error) {
	output, err := c.executor.RunCommand("ps", "-e", "--no-headers")
	if err != nil {
		return 0, err
	}
	lines := strings.Split(output, "\n")
	count := 0
	for _, line := range lines {
		if strings.TrimSpace(line) != "" {
			count++
		}
	}
	return count, nil
}

// 内存信息结构
type memoryInfo struct {
	Total   string
	Used    string
	Free    string
	Percent float64
}

func (c *RealSystemStatusChecker) getMemoryInfo() (memoryInfo, error) {
	output, err := c.executor.RunCommand("free")
	if err != nil {
		return memoryInfo{}, err
	}

	lines := strings.Split(output, "\n")
	if len(lines) >= 2 {
		parts := strings.Fields(lines[1])
		if len(parts) >= 7 {
			totalKB, _ := strconv.ParseInt(parts[1], 10, 64)
			usedKB, _ := strconv.ParseInt(parts[2], 10, 64)
			freeKB, _ := strconv.ParseInt(parts[3], 10, 64)
			
			totalMB := totalKB / 1024
			usedMB := usedKB / 1024
			freeMB := freeKB / 1024
			
			percent := float64(usedKB) / float64(totalKB) * 100
			
			return memoryInfo{
				Total:   fmt.Sprintf("%dMB", totalMB),
				Used:    fmt.Sprintf("%dMB", usedMB),
				Free:    fmt.Sprintf("%dMB", freeMB),
				Percent: percent,
			}, nil
		}
	}
	return memoryInfo{}, fmt.Errorf("无法解析内存信息")
}

// 磁盘信息结构
type diskInfo struct {
	Total   string
	Used    string
	Free    string
	Percent float64
}

func (c *RealSystemStatusChecker) getDiskInfo() (diskInfo, error) {
	output, err := c.executor.RunCommand("df", "/")
	if err != nil {
		return diskInfo{}, err
	}

	lines := strings.Split(output, "\n")
	if len(lines) >= 2 {
		parts := strings.Fields(lines[1])
		if len(parts) >= 5 {
			totalKB, _ := strconv.ParseInt(parts[1], 10, 64)
			usedKB, _ := strconv.ParseInt(parts[2], 10, 64)
			freeKB, _ := strconv.ParseInt(parts[3], 10, 64)
			
			totalMB := totalKB / 1024
			usedMB := usedKB / 1024
			freeMB := freeKB / 1024
			
			percent := float64(usedKB) / float64(totalKB) * 100
			
			return diskInfo{
				Total:   fmt.Sprintf("%dMB", totalMB),
				Used:    fmt.Sprintf("%dMB", usedMB),
				Free:    fmt.Sprintf("%dMB", freeMB),
				Percent: percent,
			}, nil
		}
	}
	return diskInfo{}, fmt.Errorf("无法解析磁盘信息")
}

func (c *RealSystemStatusChecker) getCPUPercent() (float64, error) {
	// 简化实现，返回最近1分钟的负载
	output, err := c.executor.RunCommand("cat", "/proc/loadavg")
	if err != nil {
		return 0.0, err
	}
	parts := strings.Fields(output)
	if len(parts) > 0 {
		load, _ := strconv.ParseFloat(parts[0], 64)
		// 假设每个核心的负载为1.0，计算百分比
		cpuCount, _ := c.getCPUCount()
		if cpuCount == 0 {
			cpuCount = 1
		}
		percent := (load / float64(cpuCount)) * 100
		if percent > 100 {
			percent = 100
		}
		return percent, nil
	}
	return 0.0, nil
}

func (c *RealSystemStatusChecker) getCPUCount() (int, error) {
	output, err := c.executor.RunCommand("nproc")
	if err != nil {
		// Fallback
		return 1, nil
	}
	count, err := strconv.Atoi(strings.TrimSpace(output))
	if err != nil {
		return 1, nil
	}
	return count, nil
}

func (c *RealSystemStatusChecker) getNetworkInterfaces() ([]NetworkInterface, error) {
	output, err := c.executor.RunCommand("ip", "-o", "addr", "show")
	if err != nil {
		return nil, err
	}

	var interfaces []NetworkInterface
	lines := strings.Split(output, "\n")
	for _, line := range lines {
		parts := strings.Fields(line)
		if len(parts) >= 4 {
			name := parts[1]
			ip := parts[3]
			// 简单的去重和过滤
			if !strings.HasPrefix(name, "lo") && strings.Contains(ip, "/") {
				interfaces = append(interfaces, NetworkInterface{
					Name:      name,
					IPAddress: ip,
					Status:    "UP", // 简化假设
				})
			}
		}
	}
	return interfaces, nil
}

func (c *RealSystemStatusChecker) getNetworkConnections() (int, error) {
	// 使用 ss 命令获取连接数
	output, err := c.executor.RunCommand("ss", "-s")
	if err != nil {
		return 0, err
	}
	
	// 解析输出，寻找 TCP 连接数
	// Total: 196 (kernel 0)
	// TCP:   12 (estab 2, closed 0, orphaned 0, synrecv 0, timewait 0/0), ports 0
	
	lines := strings.Split(output, "\n")
	for _, line := range lines {
		if strings.Contains(line, "TCP:") {
			// 提取 estab 数量
			// 这里简化处理，直接返回总连接数或者解析 estab
			// 简单起见，我们尝试解析 "estab" 后面的数字
			parts := strings.Fields(line)
			for i, part := range parts {
				if part == "estab" && i+1 < len(parts) {
					countStr := strings.Trim(parts[i+1], ",")
					count, _ := strconv.Atoi(countStr)
					return count, nil
				}
			}
		}
	}
	
	return 0, nil
}

func (c *RealSystemStatusChecker) getActiveServices() ([]string, error) {
	// 检查常见服务状态
	services := []string{"ssh", "nginx", "docker", "xray", "sing-box"}
	var active []string
	
	for _, service := range services {
		status, _ := c.GetServiceStatus(service)
		if status == "active" {
			active = append(active, service)
		}
	}
	
	return active, nil
}