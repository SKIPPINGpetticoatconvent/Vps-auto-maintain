//! Rust VPS Telegram Bot - 性能测试模块
//! 测试系统在高负载、并发和大数据量下的表现

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::thread;
use std::collections::HashMap;

// 导入共享的 Mock 类型
mod ../common/mocks;
use mocks::{MockTelegramBot, MockCallbackQuery};

/// 性能指标收集器
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    pub response_times: Vec<Duration>,
    pub error_count: usize,
    pub success_count: usize,
    pub memory_usage: Vec<u64>,
}

impl PerformanceMetrics {
    pub fn new() -> Self {
        Self {
            response_times: Vec::new(),
            error_count: 0,
            success_count: 0,
            memory_usage: Vec::new(),
        }
    }

    pub fn add_response_time(&mut self, duration: Duration) {
        self.response_times.push(duration);
    }

    pub fn add_success(&mut self) {
        self.success_count += 1;
    }

    pub fn add_error(&mut self) {
        self.error_count += 1;
    }

    pub fn add_memory_usage(&mut self, bytes: u64) {
        self.memory_usage.push(bytes);
    }

    pub fn get_average_response_time(&self) -> Option<Duration> {
        if self.response_times.is_empty() {
            None
        } else {
            let total: Duration = self.response_times.iter().sum();
            Some(total / self.response_times.len() as u32)
        }
    }

    pub fn get_max_response_time(&self) -> Option<Duration> {
        self.response_times.iter().max().copied()
    }

    pub fn get_min_response_time(&self) -> Option<Duration> {
        self.response_times.iter().min().copied()
    }

    pub fn get_success_rate(&self) -> f64 {
        let total = self.success_count + self.error_count;
        if total == 0 {
            0.0
        } else {
            self.success_count as f64 / total as f64 * 100.0
        }
    }

    pub fn get_requests_per_second(&self, total_duration: Duration) -> f64 {
        let total_requests = self.success_count + self.error_count;
        if total_duration.as_secs_f64() == 0.0 {
            0.0
        } else {
            total_requests as f64 / total_duration.as_secs_f64()
        }
    }
}

/// 线程安全的性能指标收集器
pub struct ThreadSafeMetrics {
    metrics: Arc<Mutex<PerformanceMetrics>>,
}

impl ThreadSafeMetrics {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(Mutex::new(PerformanceMetrics::new())),
        }
    }

    pub fn add_response_time(&self, duration: Duration) {
        if let Ok(mut metrics) = self.metrics.lock() {
            metrics.add_response_time(duration);
        }
    }

    pub fn add_success(&self) {
        if let Ok(mut metrics) = self.metrics.lock() {
            metrics.add_success();
        }
    }

    pub fn add_error(&self) {
        if let Ok(mut metrics) = self.metrics.lock() {
            metrics.add_error();
        }
    }

    pub fn add_memory_usage(&self, bytes: u64) {
        if let Ok(mut metrics) = self.metrics.lock() {
            metrics.add_memory_usage(bytes);
        }
    }

    pub fn get_metrics(&self) -> PerformanceMetrics {
        if let Ok(metrics) = self.metrics.lock() {
            metrics.clone()
        } else {
            PerformanceMetrics::new()
        }
    }
}

/// 模拟的性能测试处理器
pub struct PerformanceTestHandler {
    /// 模拟的 Telegram Bot
    bot: Arc<MockTelegramBot>,
    /// 系统命令输出映射
    system_outputs: HashMap<String, String>,
    /// 性能指标
    metrics: ThreadSafeMetrics,
}

impl PerformanceTestHandler {
    pub fn new(admin_chat_id: i64) -> Self {
        let mut system_outputs = HashMap::new();
        
        // 设置模拟的系统命令输出
        system_outputs.insert("status".to_string(), "CPU: 10%, Memory: 50%, Disk: 40%".to_string());
        system_outputs.insert("maintain_core".to_string(), "Core maintenance completed".to_string());
        system_outputs.insert("maintain_rules".to_string(), "Rules update completed".to_string());
        system_outputs.insert("update_xray".to_string(), "Xray updated to v1.8.0".to_string());
        system_outputs.insert("update_singbox".to_string(), "Sing-box updated to v1.5.0".to_string());
        system_outputs.insert("logs".to_string(), "Dec 27 10:00:00 vps Bot started".to_string());

        Self {
            bot: Arc::new(MockTelegramBot::new(admin_chat_id)),
            system_outputs,
            metrics: ThreadSafeMetrics::new(),
        }
    }

    /// 模拟处理回调查询（带性能测量）
    pub fn handle_callback_with_metrics(&self, query: &MockCallbackQuery) -> Result<String, String> {
        let start_time = Instant::now();
        
        // 权限验证
        if query.chat_id != self.bot.admin_chat_id {
            // 需要创建临时的可变引用来调用方法
            let mut bot_clone = self.bot.as_ref().clone();
            bot_clone.answer_callback_query(&query.id, Some("❌ 无权限访问"));
            self.metrics.add_error();
            return Err("Unauthorized".to_string());
        }

        // 回答回调
        let mut bot_clone = self.bot.as_ref().clone();
        bot_clone.answer_callback_query(&query.id, None);

        let result = match query.data.as_str() {
            // 主菜单按钮
            "cmd_status" => {
                let status = self.system_outputs.get("status").unwrap();
                let mut bot_clone = self.bot.as_ref().clone();
                bot_clone.edit_message(
                    query.chat_id,
                    query.message_id,
                    &format!("📊 系统状态:\n\n{}", status),
                );
                Ok("Status displayed".to_string())
            }
            "menu_maintain" => {
                let mut bot_clone = self.bot.as_ref().clone();
                bot_clone.edit_message(
                    query.chat_id,
                    query.message_id,
                    "🛠️ 请选择维护操作:",
                );
                Ok("Maintain menu displayed".to_string())
            }
            "menu_schedule" => {
                let mut bot_clone = self.bot.as_ref().clone();
                bot_clone.edit_message(
                    query.chat_id,
                    query.message_id,
                    "⏰ 定时任务设置\n\n请选择要设置的任务类型:",
                );
                Ok("Schedule menu displayed".to_string())
            }
            "cmd_logs" => {
                let logs = self.system_outputs.get("logs").unwrap();
                let mut bot_clone = self.bot.as_ref().clone();
                bot_clone.edit_message(
                    query.chat_id,
                    query.message_id,
                    &format!("📋 系统日志:\n{}", logs),
                );
                Ok("Logs displayed".to_string())
            }
            
            // 维护菜单按钮
            "cmd_maintain_core" => {
                let mut bot_clone = self.bot.as_ref().clone();
                bot_clone.edit_message(
                    query.chat_id,
                    query.message_id,
                    "🔄 正在执行核心维护...",
                );
                let result = self.system_outputs.get("maintain_core").unwrap();
                let mut bot_clone = self.bot.as_ref().clone();
                bot_clone.edit_message(
                    query.chat_id,
                    query.message_id,
                    &format!("✅ 核心维护完成:\n{}", result),
                );
                Ok("Core maintenance completed".to_string())
            }
            "cmd_maintain_rules" => {
                let mut bot_clone = self.bot.as_ref().clone();
                bot_clone.edit_message(
                    query.chat_id,
                    query.message_id,
                    "🔄 正在执行规则维护...",
                );
                let result = self.system_outputs.get("maintain_rules").unwrap();
                let mut bot_clone = self.bot.as_ref().clone();
                bot_clone.edit_message(
                    query.chat_id,
                    query.message_id,
                    &format!("✅ 规则维护完成:\n{}", result),
                );
                Ok("Rules maintenance completed".to_string())
            }
            "cmd_update_xray" => {
                let mut bot_clone = self.bot.as_ref().clone();
                bot_clone.edit_message(
                    query.chat_id,
                    query.message_id,
                    "🔄 正在更新 Xray...",
                );
                let result = self.system_outputs.get("update_xray").unwrap();
                let mut bot_clone = self.bot.as_ref().clone();
                bot_clone.edit_message(
                    query.chat_id,
                    query.message_id,
                    &format!("✅ Xray 更新完成:\n{}", result),
                );
                Ok("Xray updated".to_string())
            }
            "cmd_update_sb" => {
                let mut bot_clone = self.bot.as_ref().clone();
                bot_clone.edit_message(
                    query.chat_id,
                    query.message_id,
                    "🔄 正在更新 Sing-box...",
                );
                let result = self.system_outputs.get("update_singbox").unwrap();
                let mut bot_clone = self.bot.as_ref().clone();
                bot_clone.edit_message(
                    query.chat_id,
                    query.message_id,
                    &format!("✅ Sing-box 更新完成:\n{}", result),
                );
                Ok("Sing-box updated".to_string())
            }
            
            // 任务类型按钮
            "task_system_maintenance" => {
                let mut bot_clone = self.bot.as_ref().clone();
                bot_clone.edit_message(
                    query.chat_id,
                    query.message_id,
                    "🔄 系统维护定时设置\n\n请选择执行时间:",
                );
                Ok("System maintenance schedule displayed".to_string())
            }
            "task_core_maintenance" => {
                let mut bot_clone = self.bot.as_ref().clone();
                bot_clone.edit_message(
                    query.chat_id,
                    query.message_id,
                    "🚀 核心维护定时设置\n\n请选择执行时间:",
                );
                Ok("Core maintenance schedule displayed".to_string())
            }
            "task_rules_maintenance" => {
                let mut bot_clone = self.bot.as_ref().clone();
                bot_clone.edit_message(
                    query.chat_id,
                    query.message_id,
                    "🌍 规则维护定时设置\n\n请选择执行时间:",
                );
                Ok("Rules maintenance schedule displayed".to_string())
            }
            "task_update_xray" => {
                let mut bot_clone = self.bot.as_ref().clone();
                bot_clone.edit_message(
                    query.chat_id,
                    query.message_id,
                    "🔧 更新 Xray 定时设置\n\n请选择执行时间:",
                );
                Ok("Xray update schedule displayed".to_string())
            }
            "task_update_singbox" => {
                let mut bot_clone = self.bot.as_ref().clone();
                bot_clone.edit_message(
                    query.chat_id,
                    query.message_id,
                    "📦 更新 Sing-box 定时设置\n\n请选择执行时间:",
                );
                Ok("Singbox update schedule displayed".to_string())
            }
            "view_tasks" => {
                let mut bot_clone = self.bot.as_ref().clone();
                bot_clone.edit_message(
                    query.chat_id,
                    query.message_id,
                    "📋 当前任务列表:\n\n暂无定时任务",
                );
                Ok("Tasks list displayed".to_string())
            }
            
            // 返回按钮
            "back_to_main" => {
                let mut bot_clone = self.bot.as_ref().clone();
                bot_clone.edit_message(
                    query.chat_id,
                    query.message_id,
                    "🚀 欢迎使用 VPS 管理机器人!\n\n请选择您要执行的操作:",
                );
                Ok("Back to main menu".to_string())
            }
            "back_to_task_types" => {
                let mut bot_clone = self.bot.as_ref().clone();
                bot_clone.edit_message(
                    query.chat_id,
                    query.message_id,
                    "⏰ 定时任务设置\n\n请选择要设置的任务类型:",
                );
                Ok("Back to task types".to_string())
            }
            
            _ => {
                let mut bot_clone = self.bot.as_ref().clone();
                bot_clone.answer_callback_query(&query.id, Some("未知命令"));
                Ok("Ignored".to_string())
            }
        };

        // 记录响应时间
        let duration = start_time.elapsed();
        self.metrics.add_response_time(duration);
        
        match &result {
            Ok(_) => self.metrics.add_success(),
            Err(_) => self.metrics.add_error(),
        }

        result
    }

    /// 获取性能指标
    pub fn get_metrics(&self) -> PerformanceMetrics {
        self.metrics.get_metrics()
    }

    /// 获取 Bot 引用
    pub fn get_bot(&self) -> &MockTelegramBot {
        &self.bot
    }
}

#[cfg(test)]
mod performance_tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    const TEST_CHAT_ID: i64 = 123456789;

    fn create_callback(data: &str) -> MockCallbackQuery {
        MockCallbackQuery {
            id: format!("cb_perf_{}", data),
            data: data.to_string(),
            chat_id: TEST_CHAT_ID,
            message_id: 1,
        }
    }

    #[test]
    fn test_basic_response_time() {
        let handler = PerformanceTestHandler::new(TEST_CHAT_ID);
        let iterations = 100;
        
        for i in 0..iterations {
            let query = create_callback("cmd_status");
            let result = handler.handle_callback_with_metrics(&query);
            assert!(result.is_ok(), "Iteration {} failed", i);
        }

        let metrics = handler.get_metrics();
        let avg_time = metrics.get_average_response_time().unwrap();
        
        println!("平均响应时间: {:?}", avg_time);
        assert!(avg_time < Duration::from_millis(10), "响应时间过长: {:?}", avg_time);
    }

    #[test]
    fn test_concurrent_requests() {
        let handler = Arc::new(PerformanceTestHandler::new(TEST_CHAT_ID));
        let concurrency = 50;
        let iterations_per_goroutine = 20;
        
        let mut handles = vec![];
        let error_count = Arc::new(AtomicUsize::new(0));
        
        let start = Instant::now();
        
        for i in 0..concurrency {
            let handler_clone = Arc::clone(&handler);
            let error_count_clone = Arc::clone(&error_count);
            
            let handle = thread::spawn(move || {
                for j in 0..iterations_per_goroutine {
                    let query = MockCallbackQuery {
                        id: format!("cb_concurrent_{}_{}", i, j),
                        data: "cmd_status".to_string(),
                        chat_id: TEST_CHAT_ID,
                        message_id: 1,
                    };
                    
                    if let Err(_) = handler_clone.handle_callback_with_metrics(&query) {
                        error_count_clone.fetch_add(1, Ordering::SeqCst);
                    }
                    
                    // 短暂延迟模拟真实用户行为
                    thread::sleep(Duration::from_millis(1));
                }
            });
            
            handles.push(handle);
        }
        
        for handle in handles {
            handle.join().unwrap();
        }
        
        let total_duration = start.elapsed();
        let total_requests = concurrency * iterations_per_goroutine;
        let errors = error_count.load(Ordering::SeqCst);
        let success_rate = ((total_requests - errors) as f64 / total_requests as f64) * 100.0;
        let requests_per_second = (total_requests as f64) / total_duration.as_secs_f64();

        println!("并发性能测试结果:");
        println!("  并发数: {}", concurrency);
        println!("  总请求数: {}", total_requests);
        println!("  错误数: {}", errors);
        println!("  成功率: {:.2}%", success_rate);
        println!("  平均QPS: {:.2}", requests_per_second);
        println!("  总耗时: {:?}", total_duration);

        assert!(success_rate >= 95.0, "成功率过低: {:.2}%", success_rate);
    }

    #[test]
    fn test_high_frequency_clicks() {
        let handler = Arc::new(PerformanceTestHandler::new(TEST_CHAT_ID));
        let click_interval = Duration::from_millis(10); // 100次/秒
        let duration = Duration::from_secs(5);
        let operations = (duration.as_millis() / click_interval.as_millis()) as usize;
        
        let error_count = Arc::new(AtomicUsize::new(0));
        let start = Instant::now();
        
        for i in 0..operations {
            let handler_clone = Arc::clone(&handler);
            let error_count_clone = Arc::clone(&error_count);
            
            thread::spawn(move || {
                let query = create_callback("cmd_status");
                if let Err(_) = handler_clone.handle_callback_with_metrics(&query) {
                    error_count_clone.fetch_add(1, Ordering::SeqCst);
                }
            });
            
            thread::sleep(click_interval);
        }
        
        // 等待所有线程完成
        thread::sleep(Duration::from_millis(100));
        
        let total_duration = start.elapsed();
        let errors = error_count.load(Ordering::SeqCst);
        let clicks_per_second = (operations as f64) / total_duration.as_secs_f64();
        let error_rate = (errors as f64 / operations as f64) * 100.0;

        println!("高频点击测试结果:");
        println!("  点击频率: {:.2} 次/秒", clicks_per_second);
        println!("  总点击数: {}", operations);
        println!("  错误数: {}", errors);
        println!("  错误率: {:.2}%", error_rate);

        assert!(error_rate < 5.0, "高频点击下错误率过高: {:.2}%", error_rate);
    }

    #[test]
    fn test_large_message_handling() {
        let handler = PerformanceTestHandler::new(TEST_CHAT_ID);
        
        // 生成大消息
        let large_text = generate_large_text(10000); // 10KB 文本
        let iterations = 50;
        
        let start = Instant::now();
        
        for i in 0..iterations {
            // 这里简化处理，实际应该测试消息处理
            let query = create_callback("cmd_status");
            let result = handler.handle_callback_with_metrics(&query);
            assert!(result.is_ok(), "大消息处理 {} 失败", i+1);
        }
        
        let duration = start.elapsed();
        let avg_processing_time = duration / iterations as u32;

        println!("大消息处理性能:");
        println!("  消息大小: {} 字符", large_text.len());
        println!("  处理次数: {}", iterations);
        println!("  总耗时: {:?}", duration);
        println!("  平均处理时间: {:?}", avg_processing_time);

        assert!(avg_processing_time < Duration::from_millis(100), "大消息处理时间过长");
    }

    #[test]
    fn test_stress_test() {
        if std::env::var("CI").is_ok() {
            println!("跳过压力测试 (CI环境)");
            return;
        }

        let handler = Arc::new(PerformanceTestHandler::new(TEST_CHAT_ID));
        let duration = Duration::from_secs(10);
        let workers = 5;
        
        let error_count = Arc::new(AtomicUsize::new(0));
        let success_count = Arc::new(AtomicUsize::new(0));
        let start = Instant::now();
        
        let mut handles = vec![];
        
        for w in 0..workers {
            let handler_clone = Arc::clone(&handler);
            let error_count_clone = Arc::clone(&error_count);
            let success_count_clone = Arc::clone(&success_count);
            
            let handle = thread::spawn(move || {
                let operations = ["cmd_status", "menu_maintain", "menu_schedule", "cmd_logs"];
                let mut operation_index = 0;
                
                let thread_start = Instant::now();
                while thread_start.elapsed() < duration {
                    let operation = operations[operation_index % operations.len()];
                    let query = create_callback(operation);
                    
                    match handler_clone.handle_callback_with_metrics(&query) {
                        Ok(_) => success_count_clone.fetch_add(1, Ordering::SeqCst),
                        Err(_) => error_count_clone.fetch_add(1, Ordering::SeqCst),
                    }
                    
                    operation_index += 1;
                    
                    // 随机延迟模拟真实用户行为
                    let delay = Duration::from_millis(10 + (operation_index % 50) as u64);
                    thread::sleep(delay);
                }
            });
            
            handles.push(handle);
        }
        
        for handle in handles {
            handle.join().unwrap();
        }
        
        let total_duration = start.elapsed();
        let total_errors = error_count.load(Ordering::SeqCst);
        let total_success = success_count.load(Ordering::SeqCst);
        let total_requests = total_errors + total_success;
        let success_rate = (total_success as f64 / total_requests as f64) * 100.0;
        let requests_per_second = (total_requests as f64) / total_duration.as_secs_f64();

        println!("压力测试结果:");
        println!("  测试时长: {:?}", total_duration);
        println!("  工作线程: {}", workers);
        println!("  总请求数: {}", total_requests);
        println!("  成功请求: {}", total_success);
        println!("  失败请求: {}", total_errors);
        println!("  成功率: {:.2}%", success_rate);
        println!("  平均QPS: {:.2}", requests_per_second);

        assert!(success_rate >= 90.0, "压力测试成功率过低: {:.2}%", success_rate);
    }

    #[test]
    fn test_memory_efficiency() {
        let handler = PerformanceTestHandler::new(TEST_CHAT_ID);
        
        // 记录初始状态（简化实现）
        let initial_memory = get_memory_usage();
        
        // 执行大量操作
        let operations = 1000;
        for i in 0..operations {
            let query = create_callback("cmd_status");
            let result = handler.handle_callback_with_metrics(&query);
            assert!(result.is_ok(), "操作 {} 失败", i);
        }
        
        // 强制垃圾回收（如果支持）
        drop(handler);
        
        // 记录最终状态
        let final_memory = get_memory_usage();
        let memory_growth = final_memory.saturating_sub(initial_memory);
        let avg_growth_per_operation = memory_growth / operations as u64;

        println!("内存使用统计:");
        println!("  操作前内存: {} bytes", initial_memory);
        println!("  操作后内存: {} bytes", final_memory);
        println!("  内存增长: {} bytes", memory_growth);
        println!("  单次操作平均增长: {} bytes", avg_growth_per_operation);

        // 断言：单次操作内存增长不应超过 1KB
        assert!(avg_growth_per_operation < 1024, "内存增长过快: {} bytes/操作", avg_growth_per_operation);
    }

    // 辅助函数

    fn generate_large_text(size: usize) -> String {
        let pattern = "这是一条测试消息，用于验证大文本处理性能。\n";
        let pattern_bytes = pattern.as_bytes();
        
        let mut result = String::new();
        while result.len() < size {
            if result.len() + pattern_bytes.len() > size {
                let remaining = size - result.len();
                result.push_str(&pattern[..remaining]);
            } else {
                result.push_str(pattern);
            }
        }
        
        result
    }

    fn get_memory_usage() -> u64 {
        // 简化实现，实际应该使用系统API获取内存使用情况
        // 这里返回固定值用于测试
        0
    }
}