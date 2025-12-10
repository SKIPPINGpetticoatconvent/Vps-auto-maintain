package system

import (
	"errors"
	"testing"
	"time"
)

func TestSystemExecutor_Mock(t *testing.T) {
	mock := NewMockSystemExecutor()
	mock.InstalledPrograms["git"] = true
	mock.SystemTime = time.Date(2023, 10, 27, 10, 0, 0, 0, time.UTC)
	mock.Timezone = "UTC"
	mock.CommandOutput["core_maintain"] = "Core maintenance complete"
	mock.CommandError["reboot"] = errors.New("reboot failed")

	if !mock.IsInstalled("git") {
		t.Error("Expected git to be installed")
	}

	if mock.IsInstalled("docker") {
		t.Error("Expected docker not to be installed")
	}

	tm, tz := mock.GetSystemTime()
	if !tm.Equal(mock.SystemTime) || tz != "UTC" {
		t.Error("System time or timezone mismatch")
	}

	out, err := mock.RunCoreMaintain()
	if err != nil || out != "Core maintenance complete" {
		t.Errorf("RunCoreMaintain failed: %v, %s", err, out)
	}

	err = mock.Reboot()
	if err == nil || err.Error() != "reboot failed" {
		t.Error("Expected reboot error")
	}
}
