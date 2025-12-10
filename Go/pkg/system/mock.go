package system

import (
	"errors"
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
	// Simple key generation for mock map: just the command, or cmd+args?
	// For simplicity, let's use the command name, or we could join args.
	// But the test setup used "core_maintain" which is not a real command usually.
	// Let's check exact match first, then cmd only.
	
	key := cmd
	// In the test I wrote: m.RunCommand("core_maintain") -> keys "core_maintain"
	
	if val, ok := m.CommandError[key]; ok {
		return "", val
	}
	if val, ok := m.CommandOutput[key]; ok {
		return val, nil
	}
	return "", errors.New("command not mocked: " + key)
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
