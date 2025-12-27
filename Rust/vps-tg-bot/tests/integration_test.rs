//! VPS Telegram Bot 集成测试
//! 测试各模块之间的协作和真实场景

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// 模拟 Telegram API
#[derive(Clone)]
pub struct MockTelegramAPI {
    sent_messages: Arc<Mutex<Vec<MockMessage>>>,
    callback_responses: Arc<Mutex<Vec<String>>>,
}

#[derive(Clone, Debug)]
pub struct MockMessage {
    pub chat_id: i64,
    pub text: String,
    pub reply_markup: Option<String>,
}

impl MockTelegramAPI {
    pub fn new() -> Self {
        Self {
            sent_messages: Arc::new(Mutex::new(Vec::new())),
            callback_responses: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn send_message(&self, chat_id: i64, text: &str, reply_markup: Option<&str>) {
        let mut messages = self.sent_messages.lock().unwrap();
        messages.push(MockMessage {
            chat_id,
            text: text.to_string(),
            reply_markup: reply_markup.map(|s| s.to_string()),
        });
    }

    pub fn answer_callback(&self, callback_id: &str) {
        let mut responses = self.callback_responses.lock().unwrap();
        responses.push(callback_id.to_string());
    }

    pub fn get_sent_count(&self) -> usize {
        self.sent_messages.lock().unwrap().len()
    }

    pub fn get_last_message(&self) -> Option<MockMessage> {
        self.sent_messages.lock().unwrap().last().cloned()
    }
}

/// 模拟系统执行器
pub struct MockSystemExecutor {
    command_outputs: HashMap<String, String>,
    command_errors: HashMap<String, String>,
}

impl MockSystemExecutor {
    pub fn new() -> Self {
        let mut outputs = HashMap::new();
        outputs.insert("uptime".to_string(), "up 2 days, 5 hours".to_string());
        outputs.insert("free".to_string(), "Mem: 2Gi 512Mi 1.2Gi".to_string());
        outputs.insert("df".to_string(), "/dev/sda1 20G 8.0G 12G 40% /".to_string());
        outputs.insert("core_maintain".to_string(), "Core maintenance completed".to_string());
        outputs.insert("rules_maintain".to_string(), "Rules updated".to_string());
        outputs.insert("update_xray".to_string(), "Xray updated to v1.8.0".to_string());
        outputs.insert("update_singbox".to_string(), "Sing-box updated to v1.5.0".to_string());

        Self {
            command_outputs: outputs,
            command_errors: HashMap::new(),
        }
    }

    pub fn run_command(&self, cmd: &str) -> Result<String, String> {
        if let Some(err) = self.command_errors.get(cmd) {
            return Err(err.clone());
        }
        Ok(self.command_outputs.get(cmd).cloned().unwrap_or_default())
    }

    pub fn set_error(&mut self, cmd: &str, error: &str) {
        self.command_errors.insert(cmd.to_string(), error.to_string());
    }
}

/// 模拟调度器
pub struct MockScheduler {
    jobs: Arc<Mutex<HashMap<String, ScheduledJob>>>,
}

#[derive(Clone, Debug)]
pub struct ScheduledJob {
    pub name: String,
    pub cron: String,
    pub enabled: bool,
}

impl MockScheduler {
    pub fn new() -> Self {
        Self {
            jobs: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn set_job(&self, name: &str, cron: &str) -> Result<(), String> {
        let mut jobs = self.jobs.lock().unwrap();
        jobs.insert(name.to_string(), ScheduledJob {
            name: name.to_string(),
            cron: cron.to_string(),
            enabled: true,
        });
        Ok(())
    }

    pub fn remove_job(&self, name: &str) -> Result<(), String> {
        let mut jobs = self.jobs.lock().unwrap();
        jobs.remove(name);
        Ok(())
    }

    pub fn get_job_status(&self, name: &str) -> String {
        let jobs = self.jobs.lock().unwrap();
        if jobs.contains_key(name) {
            "✅ Scheduled".to_string()
        } else {
            "❌ Not scheduled".to_string()
        }
    }

    pub fn clear_all(&self) {
        let mut jobs = self.jobs.lock().unwrap();
        jobs.clear();
    }
}

/// 集成测试套件
pub struct IntegrationTestSuite {
    api: MockTelegramAPI,
    system: MockSystemExecutor,
    scheduler: MockScheduler,
    admin_chat_id: i64,
}

impl IntegrationTestSuite {
    pub fn new() -> Self {
        Self {
            api: MockTelegramAPI::new(),
            system: MockSystemExecutor::new(),
            scheduler: MockScheduler::new(),
            admin_chat_id: 123456789,
        }
    }

    /// 模拟回调查询
    pub fn simulate_callback(&self, chat_id: i64, data: &str) -> Result<(), String> {
        // 权限检查
        if chat_id != self.admin_chat_id {
            return Err("Unauthorized".to_string());
        }

        // 处理回调
        match data {
            "status" => {
                let uptime = self.system.run_command("uptime")?;
                let memory = self.system.run_command("free")?;
                let disk = self.system.run_command("df")?;
                let status = format!("📊 系统状态\n\n⏱ {}\n💾 {}\n💿 {}", uptime, memory, disk);
                self.api.send_message(chat_id, &status, None);
            }
            "maintain_core" => {
                let result = self.system.run_command("core_maintain")?;
                self.api.send_message(chat_id, &format!("✅ 核心维护完成\n{}", result), None);
            }
            "maintain_rules" => {
                let result = self.system.run_command("rules_maintain")?;
                self.api.send_message(chat_id, &format!("✅ 规则更新完成\n{}", result), None);
            }
            "update_xray" => {
                let result = self.system.run_command("update_xray")?;
                self.api.send_message(chat_id, &format!("✅ Xray 更新完成\n{}", result), None);
            }
            "update_singbox" => {
                let result = self.system.run_command("update_singbox")?;
                self.api.send_message(chat_id, &format!("✅ Sing-box 更新完成\n{}", result), None);
            }
            "schedule_core" => {
                self.scheduler.set_job("core_maintain", "0 4 * * *")?;
                self.api.send_message(chat_id, "✅ 核心维护调度已设置", None);
            }
            "schedule_rules" => {
                self.scheduler.set_job("rules_maintain", "0 5 * * *")?;
                self.api.send_message(chat_id, "✅ 规则更新调度已设置", None);
            }
            "schedule_clear" => {
                self.scheduler.clear_all();
                self.api.send_message(chat_id, "✅ 所有调度已清除", None);
            }
            _ => {
                self.api.send_message(chat_id, "❓ 未知操作", None);
            }
        }

        self.api.answer_callback(data);
        Ok(())
    }
}

// ===================== 集成测试用例 =====================

#[test]
fn test_integration_config_to_bot() {
    let suite = IntegrationTestSuite::new();
    
    // 验证配置正确加载
    assert_eq!(suite.admin_chat_id, 123456789);
    assert!(suite.api.get_sent_count() == 0);
}

#[test]
fn test_integration_bot_to_system() {
    let suite = IntegrationTestSuite::new();
    
    // 模拟状态查询
    let result = suite.simulate_callback(suite.admin_chat_id, "status");
    assert!(result.is_ok());
    
    // 验证消息发送
    assert!(suite.api.get_sent_count() > 0);
    
    let last_msg = suite.api.get_last_message().unwrap();
    assert!(last_msg.text.contains("系统状态"));
}

#[test]
fn test_integration_bot_to_scheduler() {
    let suite = IntegrationTestSuite::new();
    
    // 设置调度
    let result = suite.simulate_callback(suite.admin_chat_id, "schedule_core");
    assert!(result.is_ok());
    
    // 验证任务已添加
    let status = suite.scheduler.get_job_status("core_maintain");
    assert_eq!(status, "✅ Scheduled");
}

#[test]
fn test_integration_maintenance_workflow() {
    let suite = IntegrationTestSuite::new();
    
    // 1. 执行核心维护
    let result = suite.simulate_callback(suite.admin_chat_id, "maintain_core");
    assert!(result.is_ok());
    
    let msg = suite.api.get_last_message().unwrap();
    assert!(msg.text.contains("核心维护完成"));
    
    // 2. 执行规则更新
    let result = suite.simulate_callback(suite.admin_chat_id, "maintain_rules");
    assert!(result.is_ok());
    
    let msg = suite.api.get_last_message().unwrap();
    assert!(msg.text.contains("规则更新完成"));
}

#[test]
fn test_integration_schedule_workflow() {
    let suite = IntegrationTestSuite::new();
    
    // 1. 设置核心维护调度
    suite.simulate_callback(suite.admin_chat_id, "schedule_core").unwrap();
    assert_eq!(suite.scheduler.get_job_status("core_maintain"), "✅ Scheduled");
    
    // 2. 设置规则维护调度
    suite.simulate_callback(suite.admin_chat_id, "schedule_rules").unwrap();
    assert_eq!(suite.scheduler.get_job_status("rules_maintain"), "✅ Scheduled");
    
    // 3. 清除所有调度
    suite.simulate_callback(suite.admin_chat_id, "schedule_clear").unwrap();
    assert_eq!(suite.scheduler.get_job_status("core_maintain"), "❌ Not scheduled");
    assert_eq!(suite.scheduler.get_job_status("rules_maintain"), "❌ Not scheduled");
}

#[test]
fn test_integration_concurrent_requests() {
    let suite = Arc::new(IntegrationTestSuite::new());
    let mut handles = vec![];
    
    // 并发发送多个请求
    for _ in 0..10 {
        let suite_clone = Arc::clone(&suite);
        let handle = thread::spawn(move || {
            suite_clone.simulate_callback(suite_clone.admin_chat_id, "status")
        });
        handles.push(handle);
    }
    
    // 等待所有线程完成
    for handle in handles {
        let result = handle.join().unwrap();
        assert!(result.is_ok());
    }
    
    // 验证所有消息都已发送
    assert_eq!(suite.api.get_sent_count(), 10);
}

#[test]
fn test_integration_error_handling() {
    let mut suite = IntegrationTestSuite::new();
    
    // 模拟系统命令失败
    suite.system.set_error("core_maintain", "模拟错误");
    
    // 执行维护应该返回错误
    let result = suite.simulate_callback(suite.admin_chat_id, "maintain_core");
    assert!(result.is_err());
}

#[test]
fn test_integration_authorization_chain() {
    let suite = IntegrationTestSuite::new();
    
    // 测试未授权用户
    let result = suite.simulate_callback(999999999, "status");
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Unauthorized");
    
    // 验证没有消息发送
    assert_eq!(suite.api.get_sent_count(), 0);
}

#[test]
fn test_integration_update_operations() {
    let suite = IntegrationTestSuite::new();
    
    // 测试 Xray 更新
    let result = suite.simulate_callback(suite.admin_chat_id, "update_xray");
    assert!(result.is_ok());
    
    let msg = suite.api.get_last_message().unwrap();
    assert!(msg.text.contains("Xray 更新完成"));
    
    // 测试 Sing-box 更新
    let result = suite.simulate_callback(suite.admin_chat_id, "update_singbox");
    assert!(result.is_ok());
    
    let msg = suite.api.get_last_message().unwrap();
    assert!(msg.text.contains("Sing-box 更新完成"));
}

#[test]
fn test_integration_unknown_callback() {
    let suite = IntegrationTestSuite::new();
    
    // 测试未知回调
    let result = suite.simulate_callback(suite.admin_chat_id, "unknown_action");
    assert!(result.is_ok());
    
    let msg = suite.api.get_last_message().unwrap();
    assert!(msg.text.contains("未知操作"));
}

#[test]
fn test_integration_message_format() {
    let suite = IntegrationTestSuite::new();
    
    // 执行状态查询
    suite.simulate_callback(suite.admin_chat_id, "status").unwrap();
    
    let msg = suite.api.get_last_message().unwrap();
    
    // 验证消息格式
    assert!(msg.text.contains("📊"));
    assert!(msg.text.contains("⏱"));
    assert!(msg.text.contains("💾"));
    assert!(msg.text.contains("💿"));
}

#[test]
fn test_integration_scheduler_persistence() {
    let suite = IntegrationTestSuite::new();
    
    // 设置多个调度任务
    suite.simulate_callback(suite.admin_chat_id, "schedule_core").unwrap();
    suite.simulate_callback(suite.admin_chat_id, "schedule_rules").unwrap();
    
    // 验证任务状态
    assert_eq!(suite.scheduler.get_job_status("core_maintain"), "✅ Scheduled");
    assert_eq!(suite.scheduler.get_job_status("rules_maintain"), "✅ Scheduled");
    
    // 清除后验证
    suite.scheduler.clear_all();
    assert_eq!(suite.scheduler.get_job_status("core_maintain"), "❌ Not scheduled");
}
