package system

import (
	"runtime"
	"strings"
	"testing"
)

func TestRealSystemExecutor_IsInstalled(t *testing.T) {
	// Skip this test in short mode if needed, but "ls" should be present on most systems
	executor := NewRealSystemExecutor()
	
	cmdToCheck := "ls"
	if runtime.GOOS == "windows" {
		cmdToCheck = "cmd"
	}

	if !executor.IsInstalled(cmdToCheck) {
		t.Errorf("Expected '%s' to be installed", cmdToCheck)
	}

	if executor.IsInstalled("non_existent_program_xyz_123") {
		t.Error("Expected 'non_existent_program_xyz_123' not to be installed")
	}
}

func TestRestartService_DisallowedService(t *testing.T) {
	executor := NewRealSystemExecutor()
	
	// Test that SSH service is not in whitelist (security test)
	_, err := executor.RestartService("ssh")
	if err == nil {
		t.Error("Expected error for disallowed service 'ssh'")
	}
	if !strings.Contains(err.Error(), "不在允许列表中") {
		t.Errorf("Expected whitelist error message, got: %v", err)
	}
}

func TestRestartService_UnknownService(t *testing.T) {
	executor := NewRealSystemExecutor()
	
	// Test that unknown service is not in whitelist
	_, err := executor.RestartService("unknown-service-xyz")
	if err == nil {
		t.Error("Expected error for unknown service")
	}
	if !strings.Contains(err.Error(), "不在允许列表中") {
		t.Errorf("Expected whitelist error message, got: %v", err)
	}
}

func TestRestartService_WhitelistContainsExpectedServices(t *testing.T) {
	// Verify that the whitelist contains expected services
	expectedServices := []string{"xray", "sing-box"}
	
	for _, service := range expectedServices {
		if _, exists := serviceRestartCommands[service]; !exists {
			t.Errorf("Expected service '%s' to be in whitelist", service)
		}
	}
}

func TestRestartService_CommandMapping(t *testing.T) {
	// Verify the command mappings are correct
	tests := []struct {
		service      string
		expectedCmd  string
		expectedArgs []string
	}{
		{"xray", "x-ui", []string{"restart"}},
		{"sing-box", "sb", []string{"restart"}},
	}
	
	for _, tt := range tests {
		cmdInfo, exists := serviceRestartCommands[tt.service]
		if !exists {
			t.Errorf("Service '%s' not found in command mapping", tt.service)
			continue
		}
		if cmdInfo.command != tt.expectedCmd {
			t.Errorf("Service '%s': expected command '%s', got '%s'",
				tt.service, tt.expectedCmd, cmdInfo.command)
		}
		if len(cmdInfo.args) != len(tt.expectedArgs) {
			t.Errorf("Service '%s': expected %d args, got %d",
				tt.service, len(tt.expectedArgs), len(cmdInfo.args))
			continue
		}
		for i, arg := range tt.expectedArgs {
			if cmdInfo.args[i] != arg {
				t.Errorf("Service '%s': expected arg[%d]='%s', got '%s'",
					tt.service, i, arg, cmdInfo.args[i])
			}
		}
	}
}

func TestMockRestartService_AllowedServices(t *testing.T) {
	mock := NewMockSystemExecutor()
	
	// Test xray (allowed)
	result, err := mock.RestartService("xray")
	if err != nil {
		t.Errorf("Expected no error for xray, got: %v", err)
	}
	if !strings.Contains(result, "重启成功") {
		t.Errorf("Expected success message, got: %s", result)
	}
	
	// Test sing-box (allowed)
	result, err = mock.RestartService("sing-box")
	if err != nil {
		t.Errorf("Expected no error for sing-box, got: %v", err)
	}
	if !strings.Contains(result, "重启成功") {
		t.Errorf("Expected success message, got: %s", result)
	}
}

func TestMockRestartService_DisallowedService(t *testing.T) {
	mock := NewMockSystemExecutor()
	
	// Test ssh (not allowed)
	_, err := mock.RestartService("ssh")
	if err == nil {
		t.Error("Expected error for ssh service")
	}
	if !strings.Contains(err.Error(), "不在允许列表中") {
		t.Errorf("Expected whitelist error, got: %v", err)
	}
}
