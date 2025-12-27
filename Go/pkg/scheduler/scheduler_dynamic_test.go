package scheduler

import (
	"os"
	"testing"
)

func TestDynamicJobManagement(t *testing.T) {
	// 测试动态任务管理功能
	stateFile := "test_dynamic_jobs.json"
	
	// 清理测试文件
	defer func() {
		os.Remove(stateFile)
	}()
	
	manager := NewCronJobManager(stateFile)
	
	// 测试添加任务
	jobID1, err := manager.AddJob("测试任务1", "core_maintain", "0 0 4 * * *")
	if err != nil {
		t.Fatalf("添加第一个任务失败: %v", err)
	}
	
	jobID2, err := manager.AddJob("测试任务2", "rules_maintain", "0 0 5 * * *")
	if err != nil {
		t.Fatalf("添加第二个任务失败: %v", err)
	}
	
	// 验证任务列表
	jobList := manager.GetJobList()
	if len(jobList) != 2 {
		t.Errorf("期望 2 个任务，实际 %d 个", len(jobList))
	}
	
	// 验证任务信息
	var foundJob1, foundJob2 bool
	for _, job := range jobList {
		switch job.ID {
		case jobID1:
			foundJob1 = true
			if job.Name != "测试任务1" || job.Type != "core_maintain" || job.Spec != "0 0 4 * * *" {
				t.Errorf("任务1信息不匹配: %+v", job)
			}
		case jobID2:
			foundJob2 = true
			if job.Name != "测试任务2" || job.Type != "rules_maintain" || job.Spec != "0 0 5 * * *" {
				t.Errorf("任务2信息不匹配: %+v", job)
			}
		}
	}
	
	if !foundJob1 || !foundJob2 {
		t.Error("未找到预期的任务")
	}
	
	// 测试更新任务
	err = manager.UpdateJobByID(jobID1, "0 0 6 * * *")
	if err != nil {
		t.Fatalf("更新任务失败: %v", err)
	}
	
	// 验证更新结果
	updatedList := manager.GetJobList()
	for _, job := range updatedList {
		if job.ID == jobID1 && job.Spec != "0 0 6 * * *" {
			t.Errorf("任务更新失败，期望 '0 0 6 * * *'，实际 '%s'", job.Spec)
		}
	}
	
	// 测试通过 ID 移除任务
	err = manager.RemoveJobByID(jobID2)
	if err != nil {
		t.Fatalf("移除任务失败: %v", err)
	}
	
	// 验证移除结果
	finalList := manager.GetJobList()
	if len(finalList) != 1 {
		t.Errorf("期望 1 个任务，实际 %d 个", len(finalList))
	}
	
	if finalList[0].ID != jobID1 {
		t.Error("移除错误的任务")
	}
}

func TestCronValidation(t *testing.T) {
	manager := NewCronJobManager("test_validation.json")
	defer os.Remove("test_validation.json")
	
	// 测试有效的 Cron 表达式
	_, err := manager.AddJob("有效任务", "core_maintain", "0 0 4 * * *")
	if err != nil {
		t.Errorf("有效的 Cron 表达式应该成功: %v", err)
	}
	
	// 测试无效的 Cron 表达式
	_, err = manager.AddJob("无效任务", "core_maintain", "invalid cron")
	if err == nil {
		t.Error("无效的 Cron 表达式应该返回错误")
	}
	
	// 测试空的 Cron 表达式
	_, err = manager.AddJob("空表达式任务", "core_maintain", "")
	if err == nil {
		t.Error("空的 Cron 表达式应该返回错误")
	}
}

func TestPersistenceWithNewFormat(t *testing.T) {
	// 测试新格式的持久化
	stateFile := "test_new_format.json"
	
	// 清理测试文件
	defer func() {
		os.Remove(stateFile)
	}()
	
	// 创建第一个管理器并添加任务
	manager1 := NewCronJobManager(stateFile)
	
	// 添加不同类型的任务
	_, err := manager1.AddJob("核心维护", "core_maintain", "0 0 4 * * *")
	if err != nil {
		t.Fatalf("添加核心维护任务失败: %v", err)
	}
	
	_, err = manager1.AddJob("规则维护", "rules_maintain", "0 0 7 * * 0")
	if err != nil {
		t.Fatalf("添加规则维护任务失败: %v", err)
	}
	
	// 保存状态
	err = manager1.SaveState()
	if err != nil {
		t.Fatalf("保存状态失败: %v", err)
	}
	
	// 创建第二个管理器并加载状态
	manager2 := NewCronJobManager(stateFile)
	err = manager2.LoadState()
	if err != nil {
		t.Fatalf("加载状态失败: %v", err)
	}
	
	// 验证任务已正确恢复
	jobList := manager2.GetJobList()
	if len(jobList) != 2 {
		t.Errorf("期望恢复 2 个任务，实际 %d 个", len(jobList))
	}
	
	// 验证任务详情
	for _, job := range jobList {
		switch job.Name {
		case "核心维护":
			if job.Type != "core_maintain" || job.Spec != "0 0 4 * * *" {
				t.Errorf("核心维护任务信息不匹配: %+v", job)
			}
		case "规则维护":
			if job.Type != "rules_maintain" || job.Spec != "0 0 7 * * 0" {
				t.Errorf("规则维护任务信息不匹配: %+v", job)
			}
		default:
			t.Errorf("未知任务: %s", job.Name)
		}
	}
}

func TestDefaultJobInitialization(t *testing.T) {
	// 测试默认任务初始化
	stateFile := "test_default_init.json"
	
	// 清理测试文件
	defer func() {
		os.Remove(stateFile)
	}()
	
	// 确保文件不存在
	os.Remove(stateFile)
	
	// 创建管理器（应该自动添加默认任务）
	manager := NewCronJobManager(stateFile)
	manager.Start() // Start 方法会调用 ensureDefaultJobs
	
	// 验证默认任务已添加
	jobList := manager.GetJobList()
	if len(jobList) == 0 {
		t.Error("应该自动添加默认任务")
	}
	
	// 验证至少包含核心维护和规则维护任务
	hasCoreMaintain := false
	hasRulesMaintain := false
	
	for _, job := range jobList {
		if job.Type == "core_maintain" {
			hasCoreMaintain = true
		}
		if job.Type == "rules_maintain" {
			hasRulesMaintain = true
		}
	}
	
	if !hasCoreMaintain || !hasRulesMaintain {
		t.Errorf("缺少默认任务 - 核心维护: %v, 规则维护: %v", hasCoreMaintain, hasRulesMaintain)
	}
}

func TestBackwardCompatibility(t *testing.T) {
	// 测试向后兼容性
	stateFile := "test_compat.json"
	
	// 清理测试文件
	defer func() {
		os.Remove(stateFile)
	}()
	
	manager := NewCronJobManager(stateFile)
	
	// 使用旧的 SetJob 方法
	taskFunc := func() {
		// 测试任务函数
	}
	
	err := manager.SetJob("compat_test", "0 0 4 * * *", taskFunc)
	if err != nil {
		t.Fatalf("SetJob 失败: %v", err)
	}
	
	// 验证任务已添加
	if status := manager.GetJobStatus("compat_test"); status != "✅ Schedule" {
		t.Error("SetJob 添加的任务状态不正确")
	}
	
	// 验证任务在列表中
	jobList := manager.GetJobList()
	if len(jobList) == 0 {
		t.Error("SetJob 添加的任务未出现在列表中")
	}
}