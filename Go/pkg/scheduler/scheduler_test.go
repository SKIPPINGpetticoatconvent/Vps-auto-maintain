package scheduler

import (
	"os"
	"testing"
)

func TestSetJob_AddsEntry(t *testing.T) {
	// 测试 SetJob 方法是否能正确添加任务
	manager := NewCronJobManager("test_state.json")
	
	// 创建测试任务
	testTask := func() {
		// 测试任务
	}
	
	// 添加任务
	err := manager.SetJob("test_job", "0 0 4 * * *", testTask)
	if err != nil {
		t.Fatalf("SetJob 失败: %v", err)
	}
	
	// 验证任务状态
	status := manager.GetJobStatus("test_job")
	if status != "✅ Schedule" {
		t.Errorf("期望任务状态为 '✅ Schedule'，实际为 '%s'", status)
	}
	
	// 清理
	manager.ClearAll()
	os.Remove("test_state.json")
}

func TestSaveLoadState(t *testing.T) {
	// 测试状态持久化功能
	stateFile := "test_save_load.json"
	
	// 创建第一个管理器并添加任务
	manager1 := NewCronJobManager(stateFile)
	task1 := func() { /* 空任务 */ }
	err := manager1.SetJob("core_maintain", "0 0 4 * * *", task1)
	if err != nil {
		t.Fatalf("第一个管理器 SetJob 失败: %v", err)
	}
	
	err = manager1.SetJob("rules_maintain", "0 0 7 * * 0", task1)
	if err != nil {
		t.Fatalf("第一个管理器 SetJob 失败: %v", err)
	}
	
	// 验证任务已设置
	if status := manager1.GetJobStatus("core_maintain"); status != "✅ Schedule" {
		t.Errorf("core_maintain 任务未正确设置")
	}
	
	if status := manager1.GetJobStatus("rules_maintain"); status != "✅ Schedule" {
		t.Errorf("rules_maintain 任务未正确设置")
	}
	
	// 保存状态
	err = manager1.SaveState()
	if err != nil {
		t.Fatalf("SaveState 失败: %v", err)
	}
	
	// 创建第二个管理器并加载状态
	manager2 := NewCronJobManager(stateFile)
	err = manager2.LoadState()
	if err != nil {
		t.Fatalf("LoadState 失败: %v", err)
	}
	
	// 验证任务已正确恢复
	if status := manager2.GetJobStatus("core_maintain"); status != "✅ Schedule" {
		t.Error("core_maintain 任务未恢复")
	}
	
	if status := manager2.GetJobStatus("rules_maintain"); status != "✅ Schedule" {
		t.Error("rules_maintain 任务未恢复")
	}
	
	// 清理
	manager1.ClearAll()
	manager2.ClearAll()
	os.Remove(stateFile)
}

func TestRemoveJob(t *testing.T) {
	// 测试任务移除功能
	manager := NewCronJobManager("test_remove.json")
	
	task := func() { /* 空任务 */ }
	
	// 添加任务
	err := manager.SetJob("test_job", "0 0 4 * * *", task)
	if err != nil {
		t.Fatalf("SetJob 失败: %v", err)
	}
	
	// 验证任务存在
	if status := manager.GetJobStatus("test_job"); status != "✅ Schedule" {
		t.Fatal("任务未正确添加")
	}
	
	// 移除任务
	manager.RemoveJob("test_job")
	
	// 验证任务已移除
	if status := manager.GetJobStatus("test_job"); status != "❌ Not Set" {
		t.Error("任务未正确移除")
	}
	
	// 清理
	manager.ClearAll()
	os.Remove("test_remove.json")
}

func TestClearAll(t *testing.T) {
	// 测试清除所有任务功能
	manager := NewCronJobManager("test_clear.json")
	
	task := func() { /* 空任务 */ }
	
	// 添加多个任务
	manager.SetJob("job1", "0 0 4 * * *", task)
	manager.SetJob("job2", "0 0 5 * * *", task)
	manager.SetJob("job3", "0 0 6 * * *", task)
	
	// 验证任务已添加
	if status := manager.GetJobStatus("job1"); status != "✅ Schedule" {
		t.Errorf("job1 任务未正确添加")
	}
	if status := manager.GetJobStatus("job2"); status != "✅ Schedule" {
		t.Errorf("job2 任务未正确添加")
	}
	if status := manager.GetJobStatus("job3"); status != "✅ Schedule" {
		t.Errorf("job3 任务未正确添加")
	}
	
	// 清除所有任务
	manager.ClearAll()
	
	// 验证所有任务已清除
	if status := manager.GetJobStatus("job1"); status != "❌ Not Set" {
		t.Errorf("job1 任务未清除")
	}
	if status := manager.GetJobStatus("job2"); status != "❌ Not Set" {
		t.Errorf("job2 任务未清除")
	}
	if status := manager.GetJobStatus("job3"); status != "❌ Not Set" {
		t.Errorf("job3 任务未清除")
	}
	
	// 清理
	os.Remove("test_clear.json")
}

func TestGetJobStatus(t *testing.T) {
	// 测试获取作业状态功能
	manager := NewCronJobManager("test_status.json")
	
	task := func() { /* 空任务 */ }
	
	// 测试未设置任务的状态
	status := manager.GetJobStatus("nonexistent_job")
	if status != "❌ Not Set" {
		t.Errorf("期望 '❌ Not Set'，实际 '%s'", status)
	}
	
	// 添加任务
	err := manager.SetJob("test_job", "0 0 4 * * *", task)
	if err != nil {
		t.Fatalf("SetJob 失败: %v", err)
	}
	
	// 测试已设置任务的状态
	status = manager.GetJobStatus("test_job")
	if status != "✅ Schedule" {
		t.Errorf("期望 '✅ Schedule'，实际 '%s'", status)
	}
	
	// 清理
	manager.ClearAll()
	os.Remove("test_status.json")
}

func TestUpdateExistingJob(t *testing.T) {
	// 测试更新现有任务功能
	manager := NewCronJobManager("test_update.json")
	
	task1 := func() { /* 任务1 */ }
	task2 := func() { /* 任务2 */ }
	
	// 添加第一个任务
	err := manager.SetJob("test_job", "0 0 4 * * *", task1)
	if err != nil {
		t.Fatalf("第一次 SetJob 失败: %v", err)
	}
	
	// 验证任务已设置
	if status := manager.GetJobStatus("test_job"); status != "✅ Schedule" {
		t.Fatal("任务未正确添加")
	}
	
	// 更新同一任务的不同 cron 表达式
	err = manager.SetJob("test_job", "0 0 5 * * *", task2)
	if err != nil {
		t.Fatalf("第二次 SetJob 失败: %v", err)
	}
	
	// 验证任务仍然存在
	if status := manager.GetJobStatus("test_job"); status != "✅ Schedule" {
		t.Error("更新后任务状态不正确")
	}
	
	// 清理
	manager.ClearAll()
	os.Remove("test_update.json")
}