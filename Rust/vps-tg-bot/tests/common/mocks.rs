//! 共享的 Mock 类型定义
//! 用于 E2E 测试中模拟 Telegram Bot API

use std::collections::HashMap;

/// 模拟的回调查询结构
#[derive(Debug, Clone)]
pub struct MockCallbackQuery {
    pub id: String,
    pub data: String,
    pub chat_id: i64,
    pub message_id: i32,
}

/// 模拟的 Telegram Bot API
pub struct MockTelegramBot {
    /// 发送的消息记录
    pub sent_messages: HashMap<(i64, i32), String>,
    /// 回调查询回答记录
    pub callback_answers: Vec<(String, Option<String>)>,
    /// 管理员聊天 ID
    pub admin_chat_id: i64,
    /// 消息编辑记录
    pub edited_messages: Vec<(i64, i32, String)>,
}

impl MockTelegramBot {
    pub fn new(admin_chat_id: i64) -> Self {
        Self {
            sent_messages: HashMap::new(),
            callback_answers: Vec::new(),
            admin_chat_id,
            edited_messages: Vec::new(),
        }
    }

    /// 模拟回答回调查询
    pub fn answer_callback_query(&mut self, query_id: &str, text: Option<&str>) {
        self.callback_answers
            .push((query_id.to_string(), text.map(|s| s.to_string())));
    }

    /// 模拟编辑消息
    pub fn edit_message(&mut self, chat_id: i64, message_id: i32, text: &str) {
        self.edited_messages
            .push((chat_id, message_id, text.to_string()));
        self.sent_messages
            .insert((chat_id, message_id), text.to_string());
    }

    /// 模拟发送消息
    pub fn send_message(&mut self, chat_id: i64, text: &str) -> i32 {
        let message_id = self.sent_messages.len() as i32 + 1;
        self.sent_messages.insert((chat_id, message_id), text.to_string());
        message_id
    }

    /// 获取发送的消息
    pub fn get_sent_messages(&self) -> &HashMap<(i64, i32), String> {
        &self.sent_messages
    }

    /// 获取回调查询回答
    pub fn get_callback_answers(&self) -> &Vec<(String, Option<String>)> {
        &self.callback_answers
    }

    /// 获取编辑的消息
    pub fn get_edited_messages(&self) -> &Vec<(i64, i32, String)> {
        &self.edited_messages
    }

    /// 清空所有记录
    pub fn clear(&mut self) {
        self.sent_messages.clear();
        self.callback_answers.clear();
        self.edited_messages.clear();
    }

    /// 检查是否发送了包含特定文本的消息
    pub fn contains_message_with_text(&self, text: &str) -> bool {
        self.sent_messages.values().any(|msg| msg.contains(text))
    }

    /// 获取最后一条消息
    pub fn get_last_message(&self) -> Option<&String> {
        self.sent_messages.values().last()
    }

    /// 检查是否回答了特定的回调查询
    pub fn has_callback_answer(&self, query_id: &str) -> bool {
        self.callback_answers.iter().any(|(id, _)| id == query_id)
    }

    /// 获取特定查询的回答文本
    pub fn get_callback_answer_text(&self, query_id: &str) -> Option<&String> {
        self.callback_answers
            .iter()
            .find(|(id, _)| id == query_id)
            .and_then(|(_, text)| text.as_ref())
    }
}

impl Clone for MockTelegramBot {
    fn clone(&self) -> Self {
        Self {
            sent_messages: self.sent_messages.clone(),
            callback_answers: self.callback_answers.clone(),
            admin_chat_id: self.admin_chat_id,
            edited_messages: self.edited_messages.clone(),
        }
    }
}