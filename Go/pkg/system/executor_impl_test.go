package system

import (
	"testing"
)

func TestRealSystemExecutor_IsInstalled(t *testing.T) {
	// Skip this test in short mode if needed, but "ls" should be present on most systems
	executor := NewRealSystemExecutor()
	
	if !executor.IsInstalled("ls") {
		t.Error("Expected 'ls' to be installed")
	}

	if executor.IsInstalled("non_existent_program_xyz_123") {
		t.Error("Expected 'non_existent_program_xyz_123' not to be installed")
	}
}
