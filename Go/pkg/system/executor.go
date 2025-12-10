package system

import "time"

type SystemExecutor interface {
	IsInstalled(program string) bool
	GetSystemTime() (time.Time, string)
	RunCommand(cmd string, args ...string) (string, error)
	RunCoreMaintain() (string, error)
	RunRulesMaintain() (string, error)
	Reboot() error
	GetLogs(lines int) (string, error)
	
	// 新增系统状态检查功能
	GetSystemStatus() (*SystemStatus, error)
	GetServiceStatus(service string) (string, error)
	GetResourceUsage() (*ResourceUsage, error)
	GetNetworkStatus() (*NetworkStatus, error)
}

// SystemStatus 系统状态信息
type SystemStatus struct {
	Uptime       string
	LoadAverage  string
	MemoryUsage  string
	DiskUsage    string
	CPUUsage     string
	ProcessCount int
}

// ResourceUsage 资源使用情况
type ResourceUsage struct {
	MemoryTotal     string
	MemoryUsed      string
	MemoryFree      string
	MemoryPercent   float64
	DiskTotal       string
	DiskUsed        string
	DiskFree        string
	DiskPercent     float64
	CPUPercent      float64
	ProcessCount    int
}

// NetworkStatus 网络状态信息
type NetworkStatus struct {
	Interfaces     []NetworkInterface
	Connections    int
	ActiveServices []string
}

// NetworkInterface 网络接口信息
type NetworkInterface struct {
	Name      string
	IPAddress string
	Status    string
	RXBytes   uint64
	TXBytes   uint64
}
