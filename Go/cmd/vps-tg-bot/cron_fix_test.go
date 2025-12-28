package main

import (
	"log"
	"testing"
	"vps-tg-bot/pkg/scheduler"
)

// TestCronExpressionFix 测试 Cron 表达式修复
func TestCronExpressionFix(t *testing.T) {
	log.Println("开始测试 Cron 表达式修复...")

	// 创建调度器管理器
	jobManager := scheduler.NewCronJobManager("test_state.json")
	
	// 测试用例：5字段格式
	testCases := []struct {
		name        string
		cronExpr    string
		expectedErr bool
	}{
		{
			name:        "5字段格式 - 每日凌晨4点",
			cronExpr:    "0 4 * * *",
			expectedErr: false,
		},
		{
			name:        "5字段格式 - 每周日凌晨4点",
			cronExpr:    "0 4 * * Sun",
			expectedErr: false,
		},
		{
			name:        "5字段格式 - 每月1号凌晨4点",
			cronExpr:    "0 4 1 * *",
			expectedErr: false,
		},
		{
			name:        "6字段格式 - 每日凌晨4点",
			cronExpr:    "0 0 4 * * *",
			expectedErr: false,
		},
		{
			name:        "6字段格式 - 每周日凌晨4点",
			cronExpr:    "0 0 4 * * 0",
			expectedErr: false,
		},
		{
			name:        "6字段格式 - 每月1号凌晨4点",
			cronExpr:    "0 0 4 1 * *",
			expectedErr: false,
		},
		{
			name:        "不完整格式 - 缺少字段",
			cronExpr:    "0 4 * *",
			expectedErr: true,
		},
	}

	for _, tc := range testCases {
		t.Run(tc.name, func(t *testing.T) {
			log.Printf("测试: %s, Cron: %s", tc.name, tc.cronExpr)
			
			// 尝试添加任务来测试验证逻辑
			taskName := "测试任务 " + tc.name
			_, addErr := jobManager.AddJob(taskName, "core_maintain", tc.cronExpr)
			
			if tc.expectedErr && addErr == nil {
				t.Errorf("期望错误但没有错误: %s", tc.cronExpr)
			} else if !tc.expectedErr && addErr != nil {
				t.Errorf("不期望错误但有错误: %v, Cron: %s", addErr, tc.cronExpr)
			} else if !tc.expectedErr && addErr == nil {
				log.Printf("✅ 任务添加成功: %s (%s)", taskName, tc.cronExpr)
			}
		})
	}
	
	// 清理测试文件
	defer func() {
		jobManager.ClearAll()
		log.Println("清理测试完成")
	}()
}

// TestValidateCron 测试验证方法是否在调度器中可用
func TestValidateCron(t *testing.T) {
	jobManager := scheduler.NewCronJobManager("test_validate.json")
	
	// 测试5字段和6字段格式（使用不同的任务名称）
	validExprs := []struct {
		name string
		expr string
	}{
		{"5字段每日", "0 4 * * *"},
		{"6字段每日", "0 0 4 * * *"},
		{"5字段每周", "0 4 * * Sun"},
		{"6字段每周", "0 0 4 * * 0"},
	}
	
	for _, testCase := range validExprs {
		t.Run("Valid_"+testCase.name, func(t *testing.T) {
			_, err := jobManager.AddJob("验证测试_"+testCase.name, "core_maintain", testCase.expr)
			if err != nil {
				t.Errorf("有效表达式应该成功: %s, 错误: %v", testCase.expr, err)
			}
		})
	}
	
	// 测试无效表达式
	invalidExprs := []string{
		"0 4 * *",     // 缺少字段
		"0 4 * * * * *", // 字段过多
	}
	
	for _, expr := range invalidExprs {
		t.Run("Invalid_"+expr, func(t *testing.T) {
			_, err := jobManager.AddJob("验证测试_无效", "core_maintain", expr)
			if err == nil {
				t.Errorf("无效表达式应该失败: %s", expr)
			}
		})
	}
	
	log.Println("✅ 验证测试完成")
}