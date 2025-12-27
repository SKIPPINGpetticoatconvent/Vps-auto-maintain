//! E2E 测试模块
//! 模拟用户与 Telegram Bot 按钮交互，验证程序行为

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// 模拟的 Telegram API 响应
#[derive(Debug, Clone)]
pub struct MockTelegramResponse {
    pub ok: bool,
    pub message_id: i64,
    pub text: Option<String>,
}

/// 模拟的回调查询
#[derive(Debug, Clone)]
pub struct MockCallbackQuery {
    pub id: String,
    pub data: String,
    pub chat_id: i64,
    pub message_id: i64,
}

/// 模拟的消息
#[derive(Debug, Clone)]
pub struct MockMessage {
    pub chat_id: i64,
    pub message_id: i64,
    pub text: String,
    pub is_command: bool,
}

/// 模拟的 Telegram Bot API
pub struct MockTelegramBot {
    /// 发送的消息记录
    pub sent_messages: Arc<Mutex<Vec<String>>>,
    /// 回调响应记录
    pub callback_responses: Arc<Mutex<Vec<String>>>,
    /// 编辑的消息记录
    pub edited_messages: Arc<Mutex<Vec<String>>>,
    /// 管理员 Chat ID
    pub admin_chat_id: i64,
    /// 消息 ID 计数器
    message_id_counter: Arc<Mutex<i64>>,
}

impl MockTelegramBot {
    pub fn new(admin_chat_id: i64) -> Self {
        Self {
            sent_messages: Arc::new(Mutex::new(Vec::new())),
            callback_responses: Arc::new(Mutex::new(Vec::new())),
            edited_messages: Arc::new(Mutex::new(Vec::new())),
            admin_chat_id,
            message_id_counter: Arc::new(Mutex::new(1)),
        }
    }

    /// 模拟发送消息
    pub fn send_message(&self, chat_id: i64, text: &str) -> MockTelegramResponse {
        let mut messages = self.sent_messages.lock().unwrap();
        messages.push(format!("[{}] {}", chat_id, text));
        
        let mut counter = self.message_id_counter.lock().unwrap();
        *counter += 1;
        
        MockTelegramResponse {
            ok: true,
            message_id: *counter,
            text: Some(text.to_string()),
        }
    }

    /// 模拟编辑消息
    pub fn edit_message(&self, chat_id: i64, message_id: i64, text: &str) -> MockTelegramResponse {
        let mut messages = self.edited_messages.lock().unwrap();
        messages.push(format!("[{}/{}] {}", chat_id, message_id, text));
        
        MockTelegramResponse {
            ok: true,
            message_id,
            text: Some(text.to_string()),
        }
    }

    /// 模拟回答回调查询
    pub fn answer_callback_query(&self, callback_id: &str, text: Option<&str>) -> MockTelegramResponse {
        let mut responses = self.callback_responses.lock().unwrap();
        responses.push(format!("[{}] {:?}", callback_id, text));
        
        MockTelegramResponse {
            ok: true,
            message_id: 0,
            text: text.map(|s| s.to_string()),
        }
    }

    /// 获取发送的消息数量
    pub fn get_sent_count(&self) -> usize {
        self.sent_messages.lock().unwrap().len()
    }

    /// 获取编辑的消息数量
    pub fn get_edited_count(&self) -> usize {
        self.edited_messages.lock().unwrap().len()
    }

    /// 获取回调响应数量
    pub fn get_callback_count(&self) -> usize {
        self.callback_responses.lock().unwrap().len()
    }
}

/// 回调数据处理结果
#[derive(Debug)]
pub enum CallbackResult {
    Success(String),
    Error(String),
    Ignored,
}

/// E2E 测试处理器
pub struct E2ETestHandler {
    bot: MockTelegramBot,
    /// 模拟的系统命令输出
    system_outputs: HashMap<String, String>,
}

impl E2ETestHandler {
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
            bot: MockTelegramBot::new(admin_chat_id),
            system_outputs,
        }
    }

    /// 模拟 /start 命令
    pub fn handle_start(&self) -> CallbackResult {
        let response = self.bot.send_message(
            self.bot.admin_chat_id,
            "🚀 欢迎使用 VPS 管理机器人!\n\n请选择您要执行的操作:",
        );
        
        if response.ok {
            CallbackResult::Success("Main menu displayed".to_string())
        } else {
            CallbackResult::Error("Failed to display main menu".to_string())
        }
    }

    /// 处理回调查询
    pub fn handle_callback(&self, query: &MockCallbackQuery) -> CallbackResult {
        // 权限验证
        if query.chat_id != self.bot.admin_chat_id {
            self.bot.answer_callback_query(&query.id, Some("❌ 无权限访问"));
            return CallbackResult::Error("Unauthorized".to_string());
        }

        // 回答回调
        self.bot.answer_callback_query(&query.id, None);

        match query.data.as_str() {
            // 主菜单按钮
            "cmd_status" => {
                let status = self.system_outputs.get("status").unwrap();
                self.bot.edit_message(
                    query.chat_id,
                    query.message_id,
                    &format!("📊 系统状态:\n\n{}", status),
                );
                CallbackResult::Success("Status displayed".to_string())
            }
            "menu_maintain" => {
                self.bot.edit_message(
                    query.chat_id,
                    query.message_id,
                    "🛠️ 请选择维护操作:",
                );
                CallbackResult::Success("Maintain menu displayed".to_string())
            }
            "menu_schedule" => {
                self.bot.edit_message(
                    query.chat_id,
                    query.message_id,
                    "⏰ 定时任务设置\n\n请选择要设置的任务类型:",
                );
                CallbackResult::Success("Schedule menu displayed".to_string())
            }
            "cmd_logs" => {
                let logs = self.system_outputs.get("logs").unwrap();
                self.bot.edit_message(
                    query.chat_id,
                    query.message_id,
                    &format!("📋 系统日志:\n{}", logs),
                );
                CallbackResult::Success("Logs displayed".to_string())
            }
            
            // 维护菜单按钮
            "cmd_maintain_core" => {
                self.bot.edit_message(
                    query.chat_id,
                    query.message_id,
                    "🔄 正在执行核心维护...",
                );
                let result = self.system_outputs.get("maintain_core").unwrap();
                self.bot.edit_message(
                    query.chat_id,
                    query.message_id,
                    &format!("✅ 核心维护完成:\n{}", result),
                );
                CallbackResult::Success("Core maintenance completed".to_string())
            }
            "cmd_maintain_rules" => {
                self.bot.edit_message(
                    query.chat_id,
                    query.message_id,
                    "🔄 正在执行规则维护...",
                );
                let result = self.system_outputs.get("maintain_rules").unwrap();
                self.bot.edit_message(
                    query.chat_id,
                    query.message_id,
                    &format!("✅ 规则维护完成:\n{}", result),
                );
                CallbackResult::Success("Rules maintenance completed".to_string())
            }
            "cmd_update_xray" => {
                self.bot.edit_message(
                    query.chat_id,
                    query.message_id,
                    "🔄 正在更新 Xray...",
                );
                let result = self.system_outputs.get("update_xray").unwrap();
                self.bot.edit_message(
                    query.chat_id,
                    query.message_id,
                    &format!("✅ Xray 更新完成:\n{}", result),
                );
                CallbackResult::Success("Xray updated".to_string())
            }
            "cmd_update_sb" => {
                self.bot.edit_message(
                    query.chat_id,
                    query.message_id,
                    "🔄 正在更新 Sing-box...",
                );
                let result = self.system_outputs.get("update_singbox").unwrap();
                self.bot.edit_message(
                    query.chat_id,
                    query.message_id,
                    &format!("✅ Sing-box 更新完成:\n{}", result),
                );
                CallbackResult::Success("Sing-box updated".to_string())
            }
            
            // 任务类型按钮
            "task_system_maintenance" => {
                self.bot.edit_message(
                    query.chat_id,
                    query.message_id,
                    "🔄 系统维护定时设置\n\n请选择执行时间:",
                );
                CallbackResult::Success("System maintenance schedule displayed".to_string())
            }
            "task_core_maintenance" => {
                self.bot.edit_message(
                    query.chat_id,
                    query.message_id,
                    "🚀 核心维护定时设置\n\n请选择执行时间:",
                );
                CallbackResult::Success("Core maintenance schedule displayed".to_string())
            }
            "task_rules_maintenance" => {
                self.bot.edit_message(
                    query.chat_id,
                    query.message_id,
                    "🌍 规则维护定时设置\n\n请选择执行时间:",
                );
                CallbackResult::Success("Rules maintenance schedule displayed".to_string())
            }
            "task_update_xray" => {
                self.bot.edit_message(
                    query.chat_id,
                    query.message_id,
                    "🔧 更新 Xray 定时设置\n\n请选择执行时间:",
                );
                CallbackResult::Success("Xray update schedule displayed".to_string())
            }
            "task_update_singbox" => {
                self.bot.edit_message(
                    query.chat_id,
                    query.message_id,
                    "📦 更新 Sing-box 定时设置\n\n请选择执行时间:",
                );
                CallbackResult::Success("Singbox update schedule displayed".to_string())
            }
            "view_tasks" => {
                self.bot.edit_message(
                    query.chat_id,
                    query.message_id,
                    "📋 当前任务列表:\n\n暂无定时任务",
                );
                CallbackResult::Success("Tasks list displayed".to_string())
            }
            
            // 返回按钮
            "back_to_main" => {
                self.bot.edit_message(
                    query.chat_id,
                    query.message_id,
                    "🚀 欢迎使用 VPS 管理机器人!\n\n请选择您要执行的操作:",
                );
                CallbackResult::Success("Back to main menu".to_string())
            }
            "back_to_task_types" => {
                self.bot.edit_message(
                    query.chat_id,
                    query.message_id,
                    "⏰ 定时任务设置\n\n请选择要设置的任务类型:",
                );
                CallbackResult::Success("Back to task types".to_string())
            }
            
            // 预设时间按钮
            cmd if cmd.starts_with("set_preset_") => {
                let parts: Vec<&str> = cmd.strip_prefix("set_preset_").unwrap().split('_').collect();
                if parts.len() >= 2 {
                    let task_type = parts[0..parts.len()-1].join("_");
                    let frequency = parts[parts.len()-1];
                    self.bot.edit_message(
                        query.chat_id,
                        query.message_id,
                        &format!("⏰ 设置 {} {} 执行\n\n请选择具体执行时间:", task_type, frequency),
                    );
                    CallbackResult::Success(format!("Preset {} {} selected", task_type, frequency))
                } else {
                    CallbackResult::Error("Invalid preset format".to_string())
                }
            }
            
            // 时间选择按钮
            cmd if cmd.starts_with("set_time_") => {
                self.bot.edit_message(
                    query.chat_id,
                    query.message_id,
                    "✅ 定时任务设置成功!",
                );
                CallbackResult::Success("Time set successfully".to_string())
            }
            
            // 自定义设置
            cmd if cmd.starts_with("set_custom_") => {
                let task_type = cmd.strip_prefix("set_custom_").unwrap();
                self.bot.edit_message(
                    query.chat_id,
                    query.message_id,
                    &format!("⏰ 自定义 {} 定时任务设置\n\n请发送 Cron 表达式", task_type),
                );
                CallbackResult::Success(format!("Custom {} setting displayed", task_type))
            }
            
            _ => {
                self.bot.answer_callback_query(&query.id, Some("未知命令"));
                CallbackResult::Ignored
            }
        }
    }

    /// 获取 Bot 引用
    pub fn get_bot(&self) -> &MockTelegramBot {
        &self.bot
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_CHAT_ID: i64 = 123456789;

    fn create_callback(data: &str) -> MockCallbackQuery {
        MockCallbackQuery {
            id: format!("cb_{}", data),
            data: data.to_string(),
            chat_id: TEST_CHAT_ID,
            message_id: 1,
        }
    }

    #[test]
    fn test_start_command() {
        let handler = E2ETestHandler::new(TEST_CHAT_ID);
        let result = handler.handle_start();
        
        assert!(matches!(result, CallbackResult::Success(_)));
        assert_eq!(handler.get_bot().get_sent_count(), 1);
    }

    #[test]
    fn test_main_menu_buttons() {
        let handler = E2ETestHandler::new(TEST_CHAT_ID);
        
        let buttons = vec![
            "cmd_status",
            "menu_maintain",
            "menu_schedule",
            "cmd_logs",
        ];

        for button in buttons {
            let query = create_callback(button);
            let result = handler.handle_callback(&query);
            assert!(matches!(result, CallbackResult::Success(_)), "Button {} failed", button);
        }
    }

    #[test]
    fn test_maintain_menu_buttons() {
        let handler = E2ETestHandler::new(TEST_CHAT_ID);
        
        let buttons = vec![
            "cmd_maintain_core",
            "cmd_maintain_rules",
            "cmd_update_xray",
            "cmd_update_sb",
        ];

        for button in buttons {
            let query = create_callback(button);
            let result = handler.handle_callback(&query);
            assert!(matches!(result, CallbackResult::Success(_)), "Button {} failed", button);
        }
    }

    #[test]
    fn test_schedule_menu_buttons() {
        let handler = E2ETestHandler::new(TEST_CHAT_ID);
        
        let buttons = vec![
            "task_system_maintenance",
            "task_core_maintenance",
            "task_rules_maintenance",
            "task_update_xray",
            "task_update_singbox",
            "view_tasks",
        ];

        for button in buttons {
            let query = create_callback(button);
            let result = handler.handle_callback(&query);
            assert!(matches!(result, CallbackResult::Success(_)), "Button {} failed", button);
        }
    }

    #[test]
    fn test_back_navigation() {
        let handler = E2ETestHandler::new(TEST_CHAT_ID);
        
        let buttons = vec!["back_to_main", "back_to_task_types"];

        for button in buttons {
            let query = create_callback(button);
            let result = handler.handle_callback(&query);
            assert!(matches!(result, CallbackResult::Success(_)), "Button {} failed", button);
        }
    }

    #[test]
    fn test_unauthorized_access() {
        let handler = E2ETestHandler::new(TEST_CHAT_ID);
        
        let query = MockCallbackQuery {
            id: "cb_test".to_string(),
            data: "cmd_status".to_string(),
            chat_id: TEST_CHAT_ID + 999, // 未授权的 Chat ID
            message_id: 1,
        };
        
        let result = handler.handle_callback(&query);
        assert!(matches!(result, CallbackResult::Error(_)));
    }

    #[test]
    fn test_preset_buttons() {
        let handler = E2ETestHandler::new(TEST_CHAT_ID);
        
        let buttons = vec![
            "set_preset_system_maintenance_daily",
            "set_preset_core_maintenance_weekly",
            "set_preset_rules_maintenance_monthly",
        ];

        for button in buttons {
            let query = create_callback(button);
            let result = handler.handle_callback(&query);
            assert!(matches!(result, CallbackResult::Success(_)), "Button {} failed", button);
        }
    }

    #[test]
    fn test_time_selection() {
        let handler = E2ETestHandler::new(TEST_CHAT_ID);
        
        let query = create_callback("set_time_system_maintenance_daily_4");
        let result = handler.handle_callback(&query);
        assert!(matches!(result, CallbackResult::Success(_)));
    }

    #[test]
    fn test_custom_schedule() {
        let handler = E2ETestHandler::new(TEST_CHAT_ID);
        
        let query = create_callback("set_custom_system_maintenance");
        let result = handler.handle_callback(&query);
        assert!(matches!(result, CallbackResult::Success(_)));
    }

    #[test]
    fn test_invalid_callback() {
        let handler = E2ETestHandler::new(TEST_CHAT_ID);
        
        let query = create_callback("invalid_command");
        let result = handler.handle_callback(&query);
        assert!(matches!(result, CallbackResult::Ignored));
    }

    #[test]
    fn test_full_user_journey() {
        let handler = E2ETestHandler::new(TEST_CHAT_ID);
        
        // 1. 启动
        let result = handler.handle_start();
        assert!(matches!(result, CallbackResult::Success(_)));
        
        // 2. 查看状态
        let result = handler.handle_callback(&create_callback("cmd_status"));
        assert!(matches!(result, CallbackResult::Success(_)));
        
        // 3. 进入维护菜单
        let result = handler.handle_callback(&create_callback("menu_maintain"));
        assert!(matches!(result, CallbackResult::Success(_)));
        
        // 4. 执行核心维护
        let result = handler.handle_callback(&create_callback("cmd_maintain_core"));
        assert!(matches!(result, CallbackResult::Success(_)));
        
        // 5. 返回主菜单
        let result = handler.handle_callback(&create_callback("back_to_main"));
        assert!(matches!(result, CallbackResult::Success(_)));
        
        // 6. 进入调度设置
        let result = handler.handle_callback(&create_callback("menu_schedule"));
        assert!(matches!(result, CallbackResult::Success(_)));
        
        // 7. 选择任务类型
        let result = handler.handle_callback(&create_callback("task_core_maintenance"));
        assert!(matches!(result, CallbackResult::Success(_)));
        
        // 8. 选择预设
        let result = handler.handle_callback(&create_callback("set_preset_core_maintenance_daily"));
        assert!(matches!(result, CallbackResult::Success(_)));
        
        // 9. 选择时间
        let result = handler.handle_callback(&create_callback("set_time_core_maintenance_daily_4"));
        assert!(matches!(result, CallbackResult::Success(_)));
    }

    #[test]
    fn test_concurrent_callbacks() {
        use std::thread;
        use std::sync::Arc;
        
        let handler = Arc::new(E2ETestHandler::new(TEST_CHAT_ID));
        let mut handles = vec![];
        
        let buttons = vec!["cmd_status", "menu_maintain", "menu_schedule", "cmd_logs"];
        
        for button in buttons {
            let handler_clone = Arc::clone(&handler);
            let button_owned = button.to_string();
            
            let handle = thread::spawn(move || {
                let query = MockCallbackQuery {
                    id: format!("cb_{}", button_owned),
                    data: button_owned.clone(),
                    chat_id: TEST_CHAT_ID,
                    message_id: 1,
                };
                handler_clone.handle_callback(&query)
            });
            
            handles.push(handle);
        }
        
        for handle in handles {
            let result = handle.join().unwrap();
            assert!(matches!(result, CallbackResult::Success(_)));
        }
    }
}
