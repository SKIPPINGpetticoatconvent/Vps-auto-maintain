package scheduler

import (
	"os"
	"testing"
	"time"
	"vps-tg-bot/pkg/system"
)

func TestNotificationCallback(t *testing.T) {
	// Setup
	tmpFile := "test_notification.json"
	defer os.Remove(tmpFile)

	mockExecutor := system.NewMockSystemExecutor()
	// Mock successful execution
	mockExecutor.CommandOutput["core_maintain"] = "Maintenance successful"

	// Type assertion to access private fields if needed, though we are in the same package
	manager := NewCronJobManagerWithExecutor(tmpFile, mockExecutor).(*CronJobManager)

	// Channel to receive notification
	notifyChan := make(chan string, 1)
	
	// Set callback
	manager.SetNotificationCallback(12345, func(chatID int64, msg string) {
		if chatID != 12345 {
			t.Errorf("Expected chatID 12345, got %d", chatID)
		}
		notifyChan <- msg
	})

	// Manually trigger the task function
	taskFunc, ok := manager.taskRegistry["core_maintain"]
	if !ok {
		t.Fatal("core_maintain task not found in registry")
	}

	// Execute task
	taskFunc()

	// Check notification
	select {
	case msg := <-notifyChan:
		// Expected message format from registerDefaultTasks
		if msg == "" {
			t.Error("Received empty notification")
		}
	case <-time.After(1 * time.Second):
		t.Error("Timeout waiting for notification")
	}
}

func TestStopScheduler(t *testing.T) {
	tmpFile := "test_stop.json"
	defer os.Remove(tmpFile)

	manager := NewCronJobManager(tmpFile)
	manager.Start()
	
	// Just ensure it doesn't panic
	manager.Stop()
}

func TestAllDefaultTasks(t *testing.T) {
	tmpFile := "test_all_tasks.json"
	defer os.Remove(tmpFile)

	mockExecutor := system.NewMockSystemExecutor()
	// Mock successful execution
	mockExecutor.CommandOutput["core_maintain"] = "Core OK"
	mockExecutor.CommandOutput["rules_maintain"] = "Rules OK"
	// Note: MockSystemExecutor.RestartService checks for "xray" and "sing-box" specifically
	// We don't need to mock CommandOutput for them because RestartService in MockSystemExecutor handles them directly
	// based on hardcoded logic, but we can verify the output.

	manager := NewCronJobManagerWithExecutor(tmpFile, mockExecutor).(*CronJobManager)

	notifyChan := make(chan string, 10)
	manager.SetNotificationCallback(12345, func(chatID int64, msg string) {
		notifyChan <- msg
	})

	tasks := []string{"core_maintain", "rules_maintain", "restart_xray", "restart_singbox"}

	for _, taskName := range tasks {
		taskFunc, ok := manager.taskRegistry[taskName]
		if !ok {
			t.Fatalf("%s task not found", taskName)
		}
		taskFunc()
		
		select {
		case msg := <-notifyChan:
			if msg == "" {
				t.Errorf("Task %s produced empty notification", taskName)
			}
		case <-time.After(100 * time.Millisecond):
			t.Errorf("Timeout waiting for notification for %s", taskName)
		}
	}
}

func TestTaskFailures(t *testing.T) {
	tmpFile := "test_task_failures.json"
	defer os.Remove(tmpFile)

	mockExecutor := system.NewMockSystemExecutor()
	// Mock failure
	mockExecutor.CommandError["core_maintain"] = os.ErrPermission

	manager := NewCronJobManagerWithExecutor(tmpFile, mockExecutor).(*CronJobManager)

	notifyChan := make(chan string, 1)
	manager.SetNotificationCallback(12345, func(chatID int64, msg string) {
		notifyChan <- msg
	})

	taskFunc := manager.taskRegistry["core_maintain"]
	taskFunc()

	select {
	case msg := <-notifyChan:
		if msg == "" {
			t.Error("Received empty notification")
		}
	case <-time.After(100 * time.Millisecond):
		t.Error("Timeout waiting for failure notification")
	}
}
