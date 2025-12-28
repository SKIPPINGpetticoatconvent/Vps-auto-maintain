//! Rust VPS Telegram Bot - 安全测试模块
//! 测试系统的安全性，包括命令注入、路径遍历、XSS攻击等

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::thread;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::path::Path;

/// 安全测试处理器
pub struct SecurityTestHandler {
    /// 模拟的 Telegram Bot
    bot: Arc<MockTelegramBot>,
    /// 系统命令输出映射（安全过滤）
    safe_system_outputs: HashMap<String, String>,
    /// 危险命令黑名单
    dangerous_commands: Vec<String>,
    /// 安全测试指标
    security_metrics: Arc<Mutex<SecurityMetrics>>,
}

#[derive(Debug, Clone)]
pub struct SecurityMetrics {
    pub command_injection_attempts: usize,
    pub path_traversal_attempts: usize,
    pub xss_attempts: usize,
    pub malicious_input_attempts: usize,
    pub blocked_requests: usize,
    pub allowed_requests: usize,
}

impl SecurityTestHandler {
    pub fn new(admin_chat_id: i64) -> Self {
        let mut safe_system_outputs = HashMap::new();
        
        // 设置安全的系统命令输出
        safe_system_outputs.insert("status".to_string(), "CPU: 10%, Memory: 50%, Disk: 40%".to_string());
        safe_system_outputs.insert("logs".to_string(), "Dec 27 10:00:00 vps Bot started".to_string());

        // 定义危险命令黑名单
        let dangerous_commands = vec![
            "rm".to_string(),
            "del".to_string(),
            "format".to_string(),
            "shutdown".to_string(),
            "reboot".to_string(),
            "kill".to_string(),
            "pkill".to_string(),
            "killall".to_string(),
            "sudo".to_string(),
            "su".to_string(),
            "chmod".to_string(),
            "chown".to_string(),
            "cat".to_string(),
            "less".to_string(),
            "more".to_string(),
            "head".to_string(),
            "tail".to_string(),
            "find".to_string(),
            "grep".to_string(),
            "sed".to_string(),
            "awk".to_string(),
            "curl".to_string(),
            "wget".to_string(),
            "nc".to_string(),
            "netcat".to_string(),
            "telnet".to_string(),
            "ssh".to_string(),
            "ftp".to_string(),
            "echo".to_string(),
            "eval".to_string(),
            "exec".to_string(),
            "system".to_string(),
            "shell_exec".to_string(),
            "passthru".to_string(),
            "proc_open".to_string(),
            "popen".to_string(),
        ];

        Self {
            bot: Arc::new(MockTelegramBot::new(admin_chat_id)),
            safe_system_outputs,
            dangerous_commands,
            security_metrics: Arc::new(Mutex::new(SecurityMetrics {
                command_injection_attempts: 0,
                path_traversal_attempts: 0,
                xss_attempts: 0,
                malicious_input_attempts: 0,
                blocked_requests: 0,
                allowed_requests: 0,
            })),
        }
    }

    /// 安全处理回调查询
    pub fn handle_callback_securely(&self, query: &MockCallbackQuery) -> Result<String, String> {
        // 权限验证
        if query.chat_id != self.bot.admin_chat_id {
            self.bot.answer_callback_query(&query.id, Some("❌ 无权限访问"));
            return Err("Unauthorized".to_string());
        }

        // 安全检查
        if let Err(security_error) = self.perform_security_checks(&query.data) {
            self.increment_metric("blocked_requests");
            return Err(security_error);
        }

        self.increment_metric("allowed_requests");
        self.bot.answer_callback_query(&query.id, None);

        // 安全处理业务逻辑
        let result = self.handle_safe_callback(query);
        result
    }

    /// 执行安全检查
    fn perform_security_checks(&self, input: &str) -> Result<(), String> {
        // 检查命令注入
        if self.detect_command_injection(input) {
            self.increment_metric("command_injection_attempts");
            return Err("命令注入攻击被阻止".to_string());
        }

        // 检查路径遍历
        if self.detect_path_traversal(input) {
            self.increment_metric("path_traversal_attempts");
            return Err("路径遍历攻击被阻止".to_string());
        }

        // 检查XSS
        if self.detect_xss(input) {
            self.increment_metric("xss_attempts");
            return Err("XSS攻击被阻止".to_string());
        }

        // 检查恶意输入
        if self.detect_malicious_input(input) {
            self.increment_metric("malicious_input_attempts");
            return Err("恶意输入被阻止".to_string());
        }

        Ok(())
    }

    /// 检测命令注入
    fn detect_command_injection(&self, input: &str) -> bool {
        let injection_patterns = [
            ";", "&&", "||", "|", "`", "$(",
            "$(", "$(", "${", "$(", "$(",
            "$(", "$(", "wget", "curl", "nc",
            "bash", "sh", "cmd", "powershell",
            "nc", "netcat", "telnet", "ssh",
        ];

        for pattern in &injection_patterns {
            if input.contains(pattern) {
                return true;
            }
        }

        // 检查是否有连续的特殊字符
        let special_chars: Vec<char> = input.chars()
            .filter(|c| matches!(c, ';' | '&' | '|' | '`' | '$' | '(' | ')'))
            .collect();
        
        if special_chars.len() > 2 {
            return true;
        }

        false
    }

    /// 检测路径遍历
    fn detect_path_traversal(&self, input: &str) -> bool {
        let path_patterns = [
            "../", "..\\", "....//", "....\\\\",
            "%2e%2e%2f", "%2e%2e%5c",
            "..%252f", "..%255c",
            "/../../../", "\\\\..\\..\\..\\",
            "....%2F", "....%5C",
        ];

        for pattern in &path_patterns {
            if input.contains(pattern) {
                return true;
            }
        }

        false
    }

    /// 检测XSS攻击
    fn detect_xss(&self, input: &str) -> bool {
        let xss_patterns = [
            "<script", "</script", "javascript:",
            "onerror=", "onload=", "onclick=",
            "<img", "<svg", "<iframe", "<object",
            "<embed", "<link", "<style", "<meta",
            "<body", "<html", "<head",
            "alert(", "confirm(", "prompt(",
            "eval(", "document.cookie",
            "document.location", "window.location",
        ];

        for pattern in &xss_patterns {
            if input.to_lowercase().contains(pattern) {
                return true;
            }
        }

        false
    }

    /// 检测恶意输入
    fn detect_malicious_input(&self, input: &str) -> bool {
        // 检查长度
        if input.len() > 10000 {
            return true;
        }

        // 检查空字节
        if input.contains('\x00') {
            return true;
        }

        // 检查控制字符
        if input.chars().any(|c| c.is_control() && !c.is_whitespace()) {
            return true;
        }

        // 检查重复字符模式
        let mut char_counts = HashMap::new();
        for c in input.chars() {
            *char_counts.entry(c).or_insert(0) += 1;
        }

        // 如果某个字符重复超过100次，可能是有问题的输入
        for count in char_counts.values() {
            if *count > 100 {
                return true;
            }
        }

        // 检查无效UTF-8（简化实现）
        if !input.is_empty() && input.chars().next().is_none() {
            return true;
        }

        false
    }

    /// 安全处理业务逻辑
    fn handle_safe_callback(&self, query: &MockCallbackQuery) -> Result<String, String> {
        match query.data.as_str() {
            // 主菜单按钮
            "cmd_status" => {
                let status = self.safe_system_outputs.get("status").unwrap();
                self.bot.edit_message(
                    query.chat_id,
                    query.message_id,
                    &format!("📊 系统状态:\n\n{}", self.escape_html(status)),
                );
                Ok("Status displayed".to_string())
            }
            "menu_maintain" => {
                self.bot.edit_message(
                    query.chat_id,
                    query.message_id,
                    "🛠️ 请选择维护操作:",
                );
                Ok("Maintain menu displayed".to_string())
            }
            "menu_schedule" => {
                self.bot.edit_message(
                    query.chat_id,
                    query.message_id,
                    "⏰ 定时任务设置\n\n请选择要设置的任务类型:",
                );
                Ok("Schedule menu displayed".to_string())
            }
            "cmd_logs" => {
                let logs = self.safe_system_outputs.get("logs").unwrap();
                self.bot.edit_message(
                    query.chat_id,
                    query.message_id,
                    &format!("📋 系统日志:\n{}", self.escape_html(logs)),
                );
                Ok("Logs displayed".to_string())
            }
            
            // 维护菜单按钮
            "cmd_maintain_core" => {
                self.bot.edit_message(
                    query.chat_id,
                    query.message_id,
                    "🔄 正在执行核心维护...",
                );
                // 模拟安全的维护操作
                thread::sleep(Duration::from_millis(100));
                self.bot.edit_message(
                    query.chat_id,
                    query.message_id,
                    "✅ 核心维护完成",
                );
                Ok("Core maintenance completed".to_string())
            }
            "cmd_maintain_rules" => {
                self.bot.edit_message(
                    query.chat_id,
                    query.message_id,
                    "🔄 正在执行规则维护...",
                );
                thread::sleep(Duration::from_millis(100));
                self.bot.edit_message(
                    query.chat_id,
                    query.message_id,
                    "✅ 规则维护完成",
                );
                Ok("Rules maintenance completed".to_string())
            }
            
            // 其他按钮（简化实现）
            "back_to_main" => {
                self.bot.edit_message(
                    query.chat_id,
                    query.message_id,
                    "🚀 欢迎使用 VPS 管理机器人!\n\n请选择您要执行的操作:",
                );
                Ok("Back to main menu".to_string())
            }
            
            _ => {
                self.bot.answer_callback_query(&query.id, Some("未知命令"));
                Ok("Unknown command".to_string())
            }
        }
    }

    /// HTML转义
    fn escape_html(&self, text: &str) -> String {
        text.replace("&", "&")
            .replace("<", "<")
            .replace(">", ">")
            .replace("\"", """)
            .replace("'", "&#x27;")
    }

    /// 增加安全指标
    fn increment_metric(&self, metric_type: &str) {
        if let Ok(mut metrics) = self.security_metrics.lock() {
            match metric_type {
                "command_injection_attempts" => metrics.command_injection_attempts += 1,
                "path_traversal_attempts" => metrics.path_traversal_attempts += 1,
                "xss_attempts" => metrics.xss_attempts += 1,
                "malicious_input_attempts" => metrics.malicious_input_attempts += 1,
                "blocked_requests" => metrics.blocked_requests += 1,
                "allowed_requests" => metrics.allowed_requests += 1,
                _ => {}
            }
        }
    }

    /// 获取安全指标
    pub fn get_security_metrics(&self) -> SecurityMetrics {
        if let Ok(metrics) = self.security_metrics.lock() {
            metrics.clone()
        } else {
            SecurityMetrics {
                command_injection_attempts: 0,
                path_traversal_attempts: 0,
                xss_attempts: 0,
                malicious_input_attempts: 0,
                blocked_requests: 0,
                allowed_requests: 0,
            }
        }
    }
}

#[cfg(test)]
mod security_tests {
    use super::*;
    use std::time::Duration;

    const TEST_CHAT_ID: i64 = 123456789;

    fn create_callback(data: &str) -> MockCallbackQuery {
        MockCallbackQuery {
            id: format!("cb_sec_{}", data),
            data: data.to_string(),
            chat_id: TEST_CHAT_ID,
            message_id: 1,
        }
    }

    #[test]
    fn test_command_injection_protection() {
        let handler = SecurityTestHandler::new(TEST_CHAT_ID);
        
        let injection_payloads = vec![
            "status; cat /etc/passwd",
            "status && rm -rf /",
            "status| whoami",
            "status `id`",
            "status$(cat /etc/shadow)",
            "status && echo 'hacked'",
            "status; sleep 5",
            "status|| id",
        ];

        let mut blocked_count = 0;
        
        for payload in injection_payloads {
            let query = create_callback(payload);
            let result = handler.handle_callback_securely(&query);
            
            match result {
                Err(_) => blocked_count += 1,
                Ok(_) => {
                    // 检查是否有敏感输出
                    println!("警告: 命令注入可能被成功执行: {}", payload);
                }
            }
        }

        let metrics = handler.get_security_metrics();
        println!("命令注入防护测试结果:");
        println!("  总攻击尝试: {}", injection_payloads.len());
        println!("  被阻止: {}", metrics.command_injection_attempts);
        println!("  成功率: {}%", blocked_count * 100 / injection_payloads.len());

        assert!(metrics.command_injection_attempts >= injection_payloads.len() / 2, 
                "命令注入防护失败");
    }

    #[test]
    fn test_path_traversal_protection() {
        let handler = SecurityTestHandler::new(TEST_CHAT_ID);
        
        let path_payloads = vec![
            "logs_../../../etc/passwd",
            "logs_..\\..\\..\\windows\\system32",
            "logs_....//....//....//etc/passwd",
            "logs_%2e%2e%2f%2e%2e%2f%2e%2e%2fetc%2fpasswd",
            "logs_..%252f..%252f..%252fetc%252fpasswd",
            "logs_/../../../etc/passwd",
        ];

        let mut blocked_count = 0;
        
        for payload in path_payloads {
            let query = create_callback(payload);
            let result = handler.handle_callback_securely(&query);
            
            match result {
                Err(_) => blocked_count += 1,
                Ok(_) => {
                    println!("警告: 路径遍历可能被成功执行: {}", payload);
                }
            }
        }

        let metrics = handler.get_security_metrics();
        println!("路径遍历防护测试结果:");
        println!("  总攻击尝试: {}", path_payloads.len());
        println!("  被阻止: {}", metrics.path_traversal_attempts);

        assert!(metrics.path_traversal_attempts >= path_payloads.len() / 2, 
                "路径遍历防护失败");
    }

    #[test]
    fn test_xss_protection() {
        let handler = SecurityTestHandler::new(TEST_CHAT_ID);
        
        let xss_payloads = vec![
            "<script>alert('XSS')</script>",
            "javascript:alert('XSS')",
            "<img src=x onerror=alert('XSS')>",
            "<svg onload=alert('XSS')>",
            "'<script>alert('XSS')</script>",
            "<iframe src=javascript:alert('XSS')>",
            "<body onload=alert('XSS')>",
        ];

        let mut blocked_count = 0;
        
        for payload in xss_payloads {
            let query = create_callback(&format!("logs_{}", payload));
            let result = handler.handle_callback_securely(&query);
            
            match result {
                Err(_) => blocked_count += 1,
                Ok(_) => {
                    println!("警告: XSS攻击可能被成功执行: {}", payload);
                }
            }
        }

        let metrics = handler.get_security_metrics();
        println!("XSS防护测试结果:");
        println!("  总攻击尝试: {}", xss_payloads.len());
        println!("  被阻止: {}", metrics.xss_attempts);

        assert!(metrics.xss_attempts >= xss_payloads.len() / 2, 
                "XSS防护失败");
    }

    #[test]
    fn test_malicious_input_handling() {
        let handler = SecurityTestHandler::new(TEST_CHAT_ID);
        
        let malicious_inputs = vec![
            "", // 空输入
            "A".repeat(15000), // 超长输入
            "test\x00null", // Null字节
            "test\x01\x02\x03\x04\x05", // 控制字符
            "A".repeat(200), // 重复字符
            "test!@#$%^&*()_+-={}[]|\\:;\"'<>?,./", // 特殊符号
        ];

        let mut blocked_count = 0;
        
        for (i, input) in malicious_inputs.iter().enumerate() {
            let query = create_callback(&format!("test_{}_{}", i, input));
            let result = handler.handle_callback_securely(&query);
            
            match result {
                Err(_) => blocked_count += 1,
                Ok(_) => {
                    println!("警告: 恶意输入未被阻止: {:?}", input);
                }
            }
        }

        let metrics = handler.get_security_metrics();
        println!("恶意输入处理测试结果:");
        println!("  总输入数: {}", malicious_inputs.len());
        println!("  被阻止: {}", metrics.malicious_input_attempts);

        assert!(metrics.malicious_input_attempts >= malicious_inputs.len() / 2, 
                "恶意输入处理失败");
    }

    #[test]
    fn test_concurrent_security_attacks() {
        let handler = Arc::new(SecurityTestHandler::new(TEST_CHAT_ID));
        let attack_payloads = vec![
            "status; cat /etc/passwd",
            "logs_../../../etc/passwd",
            "<script>alert('XSS')</script>",
            "A".repeat(20000),
        ];

        let concurrency = 20;
        let iterations_per_thread = 10;
        let blocked_count = Arc::new(AtomicUsize::new(0));
        
        let start = Instant::now();
        
        let mut handles = vec![];
        
        for i in 0..concurrency {
            let handler_clone = Arc::clone(&handler);
            let blocked_count_clone = Arc::clone(&blocked_count);
            
            let handle = thread::spawn(move || {
                for j in 0..iterations_per_thread {
                    let payload = &attack_payloads[(i + j) % attack_payloads.len()];
                    let query = create_callback(payload);
                    
                    match handler_clone.handle_callback_securely(&query) {
                        Err(_) => blocked_count_clone.fetch_add(1, Ordering::SeqCst),
                        Ok(_) => {
                            println!("警告: 并发攻击未被阻止: {}", payload);
                        }
                    }
                    
                    // 短暂延迟
                    thread::sleep(Duration::from_millis(1));
                }
            });
            
            handles.push(handle);
        }
        
        for handle in handles {
            handle.join().unwrap();
        }
        
        let total_duration = start.elapsed();
        let total_blocked = blocked_count.load(Ordering::SeqCst);
        let total_attempts = concurrency * iterations_per_thread;
        let block_rate = total_blocked as f64 / total_attempts as f64 * 100.0;

        println!("并发安全攻击测试结果:");
        println!("  并发数: {}", concurrency);
        println!("  每线程迭代: {}", iterations_per_thread);
        println!("  总攻击尝试: {}", total_attempts);
        println!("  被阻止: {}", total_blocked);
        println!("  阻止率: {:.2}%", block_rate);
        println!("  测试时长: {:?}", total_duration);

        assert!(block_rate >= 80.0, "并发攻击阻止率过低: {:.2}%", block_rate);
    }

    #[test]
    fn test_resource_exhaustion_protection() {
        let handler = Arc::new(SecurityTestHandler::new(TEST_CHAT_ID));
        
        // 快速连续请求测试
        let request_count = 100;
        let blocked_count = Arc::new(AtomicUsize::new(0));
        
        let start = Instant::now();
        
        let mut handles = vec![];
        
        for i in 0..request_count {
            let handler_clone = Arc::clone(&handler);
            let blocked_count_clone = Arc::clone(&blocked_count);
            
            let handle = thread::spawn(move || {
                let query = create_callback("status");
                
                // 设置超时
                let result = thread::spawn(move || {
                    handler_clone.handle_callback_securely(&query)
                });
                
                match result.join() {
                    Ok(Ok(_)) => {},
                    Ok(Err(_)) => blocked_count_clone.fetch_add(1, Ordering::SeqCst),
                    Err(_) => {
                        println!("线程 panicked");
                    }
                }
            });
            
            handles.push(handle);
        }
        
        for handle in handles {
            handle.join().unwrap();
        }
        
        let total_duration = start.elapsed();
        let total_blocked = blocked_count.load(Ordering::SeqCst);
        let requests_per_second = request_count as f64 / total_duration.as_secs_f64();

        println!("资源耗尽防护测试结果:");
        println!("  总请求数: {}", request_count);
        println!("  被阻止: {}", total_blocked);
        println!("  总耗时: {:?}", total_duration);
        println!("  平均QPS: {:.2}", requests_per_second);

        // 断言：系统应该能够处理大量请求而不崩溃
        assert!(total_duration < Duration::from_secs(10), "处理时间过长");
    }

    #[test]
    fn test_unicode_security() {
        let handler = SecurityTestHandler::new(TEST_CHAT_ID);
        
        let unicode_payloads = vec![
            "test\u200B\u200C\u200D\uFEFF", // 零宽字符
            "Hello世界Приветこんにちは", // 混合语言
            "test\u202A\u202B\u202C\u202D\u202E", // RTL字符
            "a\u0301\u0302\u0303\u0304\u0305", // 组合字符
            "\u0000", // Null字符
        ];

        let mut handled_count = 0;
        
        for payload in &unicode_payloads {
            let query = create_callback(payload);
            let result = handler.handle_callback_securely(&query);
            
            match result {
                Ok(_) => handled_count += 1,
                Err(_) => {
                    println!("Unicode输入被阻止: {:?}", payload);
                }
            }
        }

        println!("Unicode安全测试结果:");
        println!("  总输入数: {}", unicode_payloads.len());
        println!("  成功处理: {}", handled_count);

        // Unicode输入应该被安全处理，而不是崩溃
        assert!(handled_count >= unicode_payloads.len() / 2, "Unicode处理失败");
    }

    #[test]
    fn test_html_escaping() {
        let handler = SecurityTestHandler::new(TEST_CHAT_ID);
        
        let test_inputs = vec![
            "<script>alert('test')</script>",
            "test & test",
            "test \"quotes\" test",
            "test 'single' test",
            "test < > test",
        ];

        for input in &test_inputs {
            let escaped = handler.escape_html(input);
            
            // 检查HTML转义是否正确
            assert!(!escaped.contains("<script>"), "HTML未正确转义");
            assert!(escaped.contains("<"), "HTML实体转义缺失");
            
            println!("输入: {}", input);
            println!("转义后: {}", escaped);
        }
    }
}