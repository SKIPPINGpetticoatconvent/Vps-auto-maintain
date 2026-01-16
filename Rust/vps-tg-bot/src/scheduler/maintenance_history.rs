use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use anyhow::Result;
use std::collections::VecDeque;

/// 维护结果状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MaintenanceResult {
    Success,
    Failed,
    Partial,
}

/// 维护历史记录结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaintenanceRecord {
    pub id: u64,
    pub timestamp: DateTime<Utc>,
    pub task_type: String,
    pub result: MaintenanceResult,
    pub output: String,
    pub error_message: Option<String>,
}

impl MaintenanceRecord {
    pub fn new(
        task_type: String,
        result: MaintenanceResult,
        output: String,
        error_message: Option<String>,
    ) -> Self {
        Self {
            id: Utc::now().timestamp() as u64,
            timestamp: Utc::now(),
            task_type,
            result,
            output,
            error_message,
        }
    }
}

/// 维护历史管理器
#[derive(Debug)]
pub struct MaintenanceHistory {
    records: VecDeque<MaintenanceRecord>,
    max_records: usize,
    history_file: String,
}

impl MaintenanceHistory {
    pub fn new(max_records: usize) -> Self {
        Self::new_with_path(max_records, "maintenance_history.json".to_string())
    }

    pub fn new_with_path(max_records: usize, history_file: String) -> Self {
        let mut history = Self {
            records: VecDeque::with_capacity(max_records),
            max_records,
            history_file,
        };
        
        // 加载历史记录
        let _ = history.load_from_file();
        
        history
    }

    /// 添加新的维护记录
    pub fn add_record(&mut self, record: MaintenanceRecord) {
        // 如果达到最大记录数，删除最旧的记录
        if self.records.len() >= self.max_records {
            self.records.pop_front();
        }
        
        self.records.push_back(record);
        
        // 保存到文件
        let _ = self.save_to_file();
    }

    /// 获取所有记录
    pub fn get_all_records(&self) -> Vec<&MaintenanceRecord> {
        self.records.iter().rev().collect()
    }

    /// 获取最近N条记录
    pub fn get_recent_records(&self, count: usize) -> Vec<&MaintenanceRecord> {
        self.records.iter().rev().take(count).collect()
    }

    /// 获取特定任务类型的记录
    #[allow(dead_code)]
    pub fn get_records_by_task_type(&self, task_type: &str) -> Vec<&MaintenanceRecord> {
        self.records
            .iter()
            .filter(|record| record.task_type.contains(task_type))
            .rev()
            .collect()
    }

    /// 获取成功/失败的记录统计
    pub fn get_statistics(&self) -> (usize, usize, usize) {
        let mut success_count = 0;
        let mut failed_count = 0;
        let mut partial_count = 0;
        
        for record in &self.records {
            match record.result {
                MaintenanceResult::Success => success_count += 1,
                MaintenanceResult::Failed => failed_count += 1,
                MaintenanceResult::Partial => partial_count += 1,
            }
        }
        
        (success_count, failed_count, partial_count)
    }

    /// 清除所有记录
    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.records.clear();
        let _ = self.save_to_file();
    }

    /// 保存历史记录到文件
    fn save_to_file(&self) -> Result<()> {
        let json = serde_json::to_string_pretty(&self.records.iter().rev().collect::<Vec<_>>())?;
        fs::write(&self.history_file, json)?;
        Ok(())
    }

    /// 从文件加载历史记录
    fn load_from_file(&mut self) -> Result<()> {
        if !Path::new(&self.history_file).exists() {
            return Ok(());
        }
        
        let content = fs::read_to_string(&self.history_file)?;
        let records: Vec<MaintenanceRecord> = serde_json::from_str(&content)?;
        
        // 限制记录数量
        // JSON 是最新的在前 [3, 2, 1]
        let limited_records: Vec<_> = if records.len() > self.max_records {
            records.into_iter().take(self.max_records).collect()
        } else {
            records
        };
        
        // 存入 Deque 需要反转为 [1, 2, 3] (由于 push_back)
        self.records = limited_records.into_iter().rev().collect();
        Ok(())
    }

    /// 格式化记录为可读文本
    pub fn format_record(&self, record: &MaintenanceRecord) -> String {
        let result_icon = match record.result {
            MaintenanceResult::Success => "✅",
            MaintenanceResult::Failed => "❌", 
            MaintenanceResult::Partial => "⚠️",
        };
        
        let timestamp = record.timestamp.format("%Y-%m-%d %H:%M:%S UTC");
        let mut text = format!("{} [{}] {}\n📅 时间: {}\n📝 输出:\n{}", 
            result_icon, 
            record.task_type,
            match record.result {
                MaintenanceResult::Success => "成功",
                MaintenanceResult::Failed => "失败",
                MaintenanceResult::Partial => "部分成功",
            },
            timestamp,
            record.output
        );
        
        if let Some(ref error) = record.error_message {
            text.push_str(&format!("\n❌ 错误: {}", error));
        }
        
        text
    }

    /// 生成历史记录摘要
    pub fn generate_summary(&self) -> String {
        if self.records.is_empty() {
            return "📋 暂无维护历史记录".to_string();
        }

        let (success_count, failed_count, partial_count) = self.get_statistics();
        let total_records = self.records.len();
        let success_rate = if total_records > 0 {
            format!("{:.1}%", (success_count as f64 / total_records as f64) * 100.0)
        } else {
            "0%".to_string()
        };

        let mut summary = String::new();
        summary.push_str("📜 维护历史摘要\n\n");
        summary.push_str(&format!("📊 总记录数: {}\n", total_records));
        summary.push_str(&format!("✅ 成功: {} ({})\n", success_count, success_rate));
        summary.push_str(&format!("❌ 失败: {}\n", failed_count));
        summary.push_str(&format!("⚠️ 部分成功: {}\n\n", partial_count));

        // 显示最近5条记录
        let recent_records = self.get_recent_records(5);
        if !recent_records.is_empty() {
            summary.push_str("📋 最近记录:\n\n");
            for (i, record) in recent_records.iter().enumerate() {
                let result_icon = match record.result {
                    MaintenanceResult::Success => "✅",
                    MaintenanceResult::Failed => "❌",
                    MaintenanceResult::Partial => "⚠️",
                };
                let timestamp = record.timestamp.format("%m-%d %H:%M");
                summary.push_str(&format!("{}. {} [{}] {}\n", 
                    i + 1, 
                    result_icon, 
                    record.task_type,
                    timestamp
                ));
            }
        }

        summary
    }
}

// 全局维护历史管理器实例
use once_cell::sync::Lazy;
use std::sync::Arc;
use tokio::sync::Mutex;

pub static MAINTENANCE_HISTORY: Lazy<Arc<Mutex<MaintenanceHistory>>> = Lazy::new(|| {
    Arc::new(Mutex::new(MaintenanceHistory::new(100))) // 保存最近100条记录
});

/// 初始化维护历史管理器
pub async fn init_maintenance_history() -> Result<()> {
    let history = MaintenanceHistory::new(100);
    let mut history_guard = MAINTENANCE_HISTORY.lock().await;
    *history_guard = history;
    Ok(())
}

/// 记录维护操作
pub async fn record_maintenance(
    task_type: &str,
    result: MaintenanceResult,
    output: &str,
    error_message: Option<&str>,
) {
    let mut history_guard = MAINTENANCE_HISTORY.lock().await;
    let record = MaintenanceRecord::new(
        task_type.to_string(),
        result,
        output.to_string(),
        error_message.map(|s| s.to_string()),
    );
    history_guard.add_record(record);
}

/// 获取维护历史摘要
pub async fn get_maintenance_summary() -> String {
    let history_guard = MAINTENANCE_HISTORY.lock().await;
    history_guard.generate_summary()
}

/// 获取维护历史详细记录
pub async fn get_maintenance_history_details(page: usize, page_size: usize) -> (String, usize) {
    let history_guard = MAINTENANCE_HISTORY.lock().await;
    let all_records = history_guard.get_all_records();
    let total_records = all_records.len();
    
    if total_records == 0 {
        return ("📋 暂无维护历史记录".to_string(), 0);
    }
    
    // 计算分页
    let start_idx = page * page_size;
    let end_idx = std::cmp::min(start_idx + page_size, total_records);
    
    if start_idx >= total_records {
        return ("📋 没有更多记录".to_string(), total_records);
    }
    
    let page_records = &all_records[start_idx..end_idx];
    
    let mut text = format!("📜 维护历史记录 (第{}页/共{}页)\n\n", page + 1, total_records.div_ceil(page_size));
    
    for (i, record) in page_records.iter().enumerate() {
        text.push_str(&format!("{}. {}\n\n", start_idx + i + 1, history_guard.format_record(record)));
        
        // 防止消息过长，限制每页最多3条记录
        if i >= 2 && start_idx + i + 1 < end_idx {
            text.push_str("... (记录过多，显示部分内容) ...");
            break;
        }
    }
    
    (text, total_records)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, Utc, Duration};
    use tempfile::{NamedTempFile};
    use std::fs;
    use tempfile::TempDir;

    fn create_history_with_temp(max_records: usize) -> (MaintenanceHistory, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("history.json").to_str().unwrap().to_string();
        let history = MaintenanceHistory::new_with_path(max_records, path);
        (history, temp_dir)
    }


    #[test]
    fn test_maintenance_result_variants() {
        // 测试维护结果枚举
        assert_eq!(format!("{:?}", MaintenanceResult::Success), "Success");
        assert_eq!(format!("{:?}", MaintenanceResult::Failed), "Failed");
        assert_eq!(format!("{:?}", MaintenanceResult::Partial), "Partial");
        
        // 测试相等性
        assert_eq!(MaintenanceResult::Success, MaintenanceResult::Success);
        assert_ne!(MaintenanceResult::Success, MaintenanceResult::Failed);
        assert_ne!(MaintenanceResult::Failed, MaintenanceResult::Partial);
    }

    #[test]
    fn test_maintenance_record_new() {
        let record = MaintenanceRecord::new(
            "测试任务".to_string(),
            MaintenanceResult::Success,
            "测试输出".to_string(),
            None,
        );
        
        assert_eq!(record.task_type, "测试任务");
        assert_eq!(record.result, MaintenanceResult::Success);
        assert_eq!(record.output, "测试输出");
        assert_eq!(record.error_message, None);
        assert!(record.id > 0);
        assert!(record.timestamp <= Utc::now());
    }

    #[test]
    fn test_maintenance_record_with_error() {
        let error_msg = Some("测试错误信息".to_string());
        let record = MaintenanceRecord::new(
            "错误任务".to_string(),
            MaintenanceResult::Failed,
            "错误输出".to_string(),
            error_msg.clone(),
        );
        
        assert_eq!(record.error_message, error_msg);
    }

    #[test]
    fn test_maintenance_history_new() {
        let history = MaintenanceHistory::new(50);
        assert_eq!(history.max_records, 50);
        assert!(history.records.is_empty());
        assert_eq!(history.history_file, "maintenance_history.json");
    }

    #[test]
    fn test_maintenance_history_add_record() {
        let (mut history, _temp) = create_history_with_temp(10);
        
        let record = MaintenanceRecord::new(
            "测试记录".to_string(),
            MaintenanceResult::Success,
            "测试输出".to_string(),
            None,
        );
        
        history.add_record(record);
        
        assert_eq!(history.records.len(), 1);
        assert_eq!(history.records.back().unwrap().task_type, "测试记录");
    }

    #[test]
    fn test_maintenance_history_max_records_limit() {
        let (mut history, _temp) = create_history_with_temp(3);
        
        // 添加4条记录，应该只保留最新的3条
        for i in 1..=4 {
            let record = MaintenanceRecord::new(
                format!("任务{}", i),
                MaintenanceResult::Success,
                format!("输出{}", i),
                None,
            );
            history.add_record(record);
        }
        
        assert_eq!(history.records.len(), 3);
        assert_eq!(history.records.back().unwrap().task_type, "任务4");
        assert_eq!(history.records.front().unwrap().task_type, "任务2");
    }

    #[test]
    fn test_maintenance_history_get_all_records() {
        let (mut history, _temp) = create_history_with_temp(10);
        
        // 添加几条记录
        for i in 1..=3 {
            let record = MaintenanceRecord::new(
                format!("任务{}", i),
                MaintenanceResult::Success,
                format!("输出{}", i),
                None,
            );
            history.add_record(record);
        }
        
        let records = history.get_all_records();
        assert_eq!(records.len(), 3);
        
        // 应该是倒序（最新的在前）
        assert_eq!(records[0].task_type, "任务3");
        assert_eq!(records[1].task_type, "任务2");
        assert_eq!(records[2].task_type, "任务1");
    }

    #[test]
    fn test_maintenance_history_get_recent_records() {
        let (mut history, _temp) = create_history_with_temp(10);
        
        // 添加5条记录
        for i in 1..=5 {
            let record = MaintenanceRecord::new(
                format!("任务{}", i),
                MaintenanceResult::Success,
                format!("输出{}", i),
                None,
            );
            history.add_record(record);
        }
        
        // 获取最近2条
        let recent = history.get_recent_records(2);
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].task_type, "任务5");
        assert_eq!(recent[1].task_type, "任务4");
        
        // 获取最近10条（超过总数）
        let all_recent = history.get_recent_records(10);
        assert_eq!(all_recent.len(), 5);
    }

    #[test]
    fn test_maintenance_history_get_records_by_task_type() {
        let (mut history, _temp) = create_history_with_temp(10);
        
        // 添加不同类型的记录
        let records_data = vec![
            ("系统维护", MaintenanceResult::Success),
            ("系统维护", MaintenanceResult::Failed),
            ("核心维护", MaintenanceResult::Success),
            ("系统维护", MaintenanceResult::Partial),
            ("规则维护", MaintenanceResult::Success),
        ];
        
        for (task_type, result) in records_data {
            let record = MaintenanceRecord::new(
                task_type.to_string(),
                result,
                "测试输出".to_string(),
                None,
            );
            history.add_record(record);
        }
        
        let system_records = history.get_records_by_task_type("系统维护");
        assert_eq!(system_records.len(), 3);
        
        let core_records = history.get_records_by_task_type("核心维护");
        assert_eq!(core_records.len(), 1);
        
        let rules_records = history.get_records_by_task_type("规则维护");
        assert_eq!(rules_records.len(), 1);
        
        let nonexistent_records = history.get_records_by_task_type("不存在的任务");
        assert_eq!(nonexistent_records.len(), 0);
    }

    #[test]
    fn test_maintenance_history_get_statistics() {
        let (mut history, _temp) = create_history_with_temp(10);
        
        // 添加不同结果的记录
        let records_data = vec![
            MaintenanceResult::Success,
            MaintenanceResult::Success,
            MaintenanceResult::Failed,
            MaintenanceResult::Partial,
            MaintenanceResult::Success,
            MaintenanceResult::Failed,
        ];
        
        for result in records_data {
            let record = MaintenanceRecord::new(
                "测试任务".to_string(),
                result,
                "测试输出".to_string(),
                None,
            );
            history.add_record(record);
        }
        
        let (success_count, failed_count, partial_count) = history.get_statistics();
        assert_eq!(success_count, 3);
        assert_eq!(failed_count, 2);
        assert_eq!(partial_count, 1);
    }

    #[test]
    fn test_maintenance_history_clear() {
        let (mut history, _temp) = create_history_with_temp(10);
        
        // 添加一些记录
        for i in 1..=3 {
            let record = MaintenanceRecord::new(
                format!("任务{}", i),
                MaintenanceResult::Success,
                format!("输出{}", i),
                None,
            );
            history.add_record(record);
        }
        
        assert_eq!(history.records.len(), 3);
        
        // 清空记录
        history.clear();
        assert_eq!(history.records.len(), 0);
    }

    #[test]
    fn test_maintenance_history_format_record() {
        let (history, _temp) = create_history_with_temp(10);
        let timestamp = Utc::now();
        
        let record = MaintenanceRecord {
            id: 123456,
            timestamp,
            task_type: "测试任务".to_string(),
            result: MaintenanceResult::Success,
            output: "测试输出内容".to_string(),
            error_message: None,
        };
        
        let formatted = history.format_record(&record);
        
        assert!(formatted.contains("✅"));
        assert!(formatted.contains("[测试任务]"));
        assert!(formatted.contains("成功"));
        assert!(formatted.contains("测试输出内容"));
        assert!(formatted.contains(&timestamp.format("%Y-%m-%d %H:%M:%S UTC").to_string()));
        
        // 测试带错误的记录
        let record_with_error = MaintenanceRecord {
            id: 123457,
            timestamp,
            task_type: "错误任务".to_string(),
            result: MaintenanceResult::Failed,
            output: "错误输出".to_string(),
            error_message: Some("具体错误信息".to_string()),
        };
        
        let formatted_error = history.format_record(&record_with_error);
        assert!(formatted_error.contains("❌"));
        assert!(formatted_error.contains("失败"));
        assert!(formatted_error.contains("❌ 错误:"));
        assert!(formatted_error.contains("具体错误信息"));
    }

    #[test]
    fn test_maintenance_history_generate_summary_empty() {
        let (history, _temp) = create_history_with_temp(10);
        let summary = history.generate_summary();
        
        assert_eq!(summary, "📋 暂无维护历史记录");
    }

    #[test]
    fn test_maintenance_history_generate_summary_with_records() {
        let (mut history, _temp) = create_history_with_temp(10);
        
        // 添加一些记录
        let records_data = vec![
            ("系统维护", MaintenanceResult::Success),
            ("系统维护", MaintenanceResult::Success),
            ("核心维护", MaintenanceResult::Failed),
        ];
        
        for (task_type, result) in records_data {
            let record = MaintenanceRecord::new(
                task_type.to_string(),
                result,
                "测试输出".to_string(),
                None,
            );
            history.add_record(record);
        }
        
        let summary = history.generate_summary();
        
        assert!(summary.contains("📜 维护历史摘要"));
        assert!(summary.contains("📊 总记录数: 3"));
        assert!(summary.contains("✅ 成功: 2"));
        assert!(summary.contains("❌ 失败: 1"));
        assert!(summary.contains("66.7%")); // 成功率
        assert!(summary.contains("📋 最近记录:"));
        assert!(summary.contains("核心维护"));
        assert!(summary.contains("系统维护"));
    }

    #[test]
    fn test_maintenance_history_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("history.json").to_str().unwrap().to_string();
        
        {
            let mut history = MaintenanceHistory::new_with_path(10, path.clone());
            
            // 添加一些记录
            for i in 1..=3 {
                let record = MaintenanceRecord::new(
                    format!("任务{}", i),
                    MaintenanceResult::Success,
                    format!("输出{}", i),
                    None,
                );
                history.add_record(record);
            }
        } // Drop to ensure flush (though changes are immediate on add_record)
        
        // 创建新的历史实例并加载
        let mut loaded_history = MaintenanceHistory::new_with_path(10, path.clone());
        let result = loaded_history.load_from_file();
        
        assert!(result.is_ok());
        assert_eq!(loaded_history.records.len(), 3);
        // Correct expectation: "任务3" is the last added, so it should be back() (newest)
        // If save reversed it (3,2,1), load reversed it back (1,2,3).
        // So back() should be 3.
        assert_eq!(loaded_history.records.back().unwrap().task_type, "任务3");
    }

    #[test]
    fn test_maintenance_history_load_nonexistent_file() {
        let (mut history, _temp) = create_history_with_temp(10);
        
        // The file is created by create_history_with_temp (path generation), but not written until saved?
        // Actually new_with_path calls load_from_file.
        // If file doesn't exist, it handles it.
        // We want to ensure it DOES NOT exist.
        // existing helper creates a path in a temp dir. File doesn't exist yet unless created.
        // So just calling new_with_path works.
        
        // Use manual setup to be explicit about "load_from_file" call if we want to test RE-load
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("nonexistent.json").to_str().unwrap().to_string();
        
        let mut history = MaintenanceHistory::new_with_path(10, path); // calls load internally
        assert_eq!(history.records.len(), 0);
    }

    #[test]
    fn test_maintenance_history_save_preserves_order() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("history_order.json").to_str().unwrap().to_string();
        
        {
            let mut history = MaintenanceHistory::new_with_path(10, path.clone());
            
            // 按顺序添加记录
            for i in 1..=5 {
                let record = MaintenanceRecord::new(
                    format!("任务{}", i),
                    MaintenanceResult::Success,
                    format!("输出{}", i),
                    None,
                );
                history.add_record(record);
            }
        }
        
        // 重新加载
        let mut loaded_history = MaintenanceHistory::new_with_path(10, path);
        // load_from_file called in new
        
        // 验证顺序保持不变
        // records: [1, 2, 3, 4, 5] (queue order)
        let loaded_records = loaded_history.get_all_records(); 
        // get_all_records returns reversed iterator: 5, 4, 3, 2, 1?
        // Let's check get_all_records impl: iter().rev().collect().
        // If queue is [1, 2, 3, 4, 5]. rev is 5, 4, 3, 2, 1.
        assert_eq!(loaded_records.len(), 5);
        
        // The test originally asserted:
        // for (i, record) in loaded_records.iter().enumerate() {
        //    assert_eq!(record.task_type, format!("任务{}", 5 - i));
        // }
        // If i=0, 5-0=5. "任务5". Correct.
        for (i, record) in loaded_records.iter().enumerate() {
            assert_eq!(record.task_type, format!("任务{}", 5 - i));
        }
    }

    #[test]
    fn test_maintenance_history_max_records_on_load() {
        let temp_file = NamedTempFile::new().unwrap();
        let temp_path = temp_file.path().to_str().unwrap();
        
        // 创建包含5条记录的文件
        let records = vec![
            MaintenanceRecord {
                id: 1,
                timestamp: Utc::now(),
                task_type: "旧任务1".to_string(),
                result: MaintenanceResult::Success,
                output: "输出1".to_string(),
                error_message: None,
            },
            MaintenanceRecord {
                id: 2,
                timestamp: Utc::now(),
                task_type: "旧任务2".to_string(),
                result: MaintenanceResult::Success,
                output: "输出2".to_string(),
                error_message: None,
            },
            MaintenanceRecord {
                id: 3,
                timestamp: Utc::now(),
                task_type: "旧任务3".to_string(),
                result: MaintenanceResult::Success,
                output: "输出3".to_string(),
                error_message: None,
            },
            MaintenanceRecord {
                id: 4,
                timestamp: Utc::now(),
                task_type: "旧任务4".to_string(),
                result: MaintenanceResult::Success,
                output: "输出4".to_string(),
                error_message: None,
            },
            MaintenanceRecord {
                id: 5,
                timestamp: Utc::now(),
                task_type: "旧任务5".to_string(),
                result: MaintenanceResult::Success,
                output: "输出5".to_string(),
                error_message: None,
            },
        ];
        
        let json_content = serde_json::to_string_pretty(&records).unwrap();
        fs::write(temp_path, json_content).unwrap();
        
        // 加载时限制为3条记录
        let mut history = MaintenanceHistory::new(3);
        history.history_file = temp_path.to_string();
        let _ = history.load_from_file();
        
        assert_eq!(history.records.len(), 3);
        
        // 清理
        let _ = fs::remove_file(temp_path);
    }
}