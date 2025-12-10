package system

import (
	"errors"
	"strings"
	"time"
)

// MockSystemExecutor is a mock implementation of SystemExecutor for testing
type MockSystemExecutor struct {
	InstalledPrograms map[string]bool
	SystemTime        time.Time
	Timezone          string
	CommandOutput     map[string]string
	CommandError      map[string]error
}

func NewMockSystemExecutor() *MockSystemExecutor {
	return &MockSystemExecutor{
		InstalledPrograms: make(map[string]bool),
		CommandOutput:     make(map[string]string),
		CommandError:      make(map[string]error),
	}
}

func (m *MockSystemExecutor) IsInstalled(program string) bool {
	return m.InstalledPrograms[program]
}

func (m *MockSystemExecutor) GetSystemTime() (time.Time, string) {
	return m.SystemTime, m.Timezone
}

func (m *MockSystemExecutor) RunCommand(cmd string, args ...string) (string, error) {
	// Try exact match with args first
	fullCmd := cmd
	if len(args) > 0 {
		fullCmd = cmd + " " + strings.Join(args, " ") // Simple join, not shell escaping
	}

	if val, ok := m.CommandError[fullCmd]; ok {
		return "", val
	}
	if val, ok := m.CommandOutput[fullCmd]; ok {
		return val, nil
	}

	// Fallback to just command name
	if val, ok := m.CommandError[cmd]; ok {
		return "", val
	}
	if val, ok := m.CommandOutput[cmd]; ok {
		return val, nil
	}

	return "", errors.New("command not mocked: " + fullCmd)
}

func (m *MockSystemExecutor) RunCoreMaintain() (string, error) {
	return m.RunCommand("core_maintain")
}

func (m *MockSystemExecutor) RunRulesMaintain() (string, error) {
	return m.RunCommand("rules_maintain")
}

func (m *MockSystemExecutor) Reboot() error {
	_, err := m.RunCommand("reboot")
	return err
}

func (m *MockSystemExecutor) GetLogs(lines int) (string, error) {
	return m.RunCommand("journalctl")
}

func (m *MockSystemExecutor) GetSystemStatus() (*SystemStatus, error) {
	return &SystemStatus{
		Uptime:       "1 day",
		LoadAverage:  "0.5 0.3 0.1",
		MemoryUsage:  "1024MB/2048MB",
		DiskUsage:    "10GB/20GB (50%)",
		CPUUsage:     "10%",
		ProcessCount: 100,
	}, nil
}

func (m *MockSystemExecutor) GetServiceStatus(service string) (string, error) {
	return "active", nil
}

func (m *MockSystemExecutor) GetResourceUsage() (*ResourceUsage, error) {
	return &ResourceUsage{
		MemoryTotal:   "2048MB",
		MemoryUsed:    "1024MB",
		MemoryFree:    "1024MB",
		MemoryPercent: 50.0,
		DiskTotal:     "20GB",
		DiskUsed:      "10GB",
		DiskFree:      "10GB",
		DiskPercent:   50.0,
		CPUPercent:    10.0,
		ProcessCount:  100,
	}, nil
}

func (m *MockSystemExecutor) GetNetworkStatus() (*NetworkStatus, error) {
	return &NetworkStatus{
		Interfaces: []NetworkInterface{
			{Name: "eth0", IPAddress: "192.168.1.1", Status: "UP"},
		},
		Connections:    10,
		ActiveServices: []string{"ssh", "nginx"},
	}, nil
}
