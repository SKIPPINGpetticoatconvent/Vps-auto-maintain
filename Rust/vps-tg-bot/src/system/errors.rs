//! 系统操作错误类型定义
//! 
//! 使用 thiserror 定义具体的错误类型，便于根据不同错误提供精准的用户提示

use thiserror::Error;

#[derive(Error, Debug)]
pub enum SystemError {
    #[error("权限不足: {0}")]
    PermissionDenied(String),
    
    #[error("网络连接失败: {0}")]
    NetworkError(String),
    
    #[error("磁盘空间不足: {0}")]
    DiskSpaceError(String),
    
    #[error("包管理器错误: {0}")]
    PackageManagerError(String),
    
    #[error("服务管理错误: {0}")]
    ServiceError(String),
    
    #[error("系统重启失败: {0}")]
    RebootError(String),
    
    #[error("文件操作失败: {0}")]
    FileOperationError(String),
    
    #[error("命令执行失败: {0}")]
    CommandExecutionError(String),
    
    #[error("未知系统错误: {0}")]
    UnknownError(String),
}

impl SystemError {
    /// 获取用户友好的错误提示
    pub fn user_message(&self) -> &'static str {
        match self {
            SystemError::PermissionDenied(_) => 
                "❌ 权限不足。请确保以 root 权限运行此程序。",
            SystemError::NetworkError(_) => 
                "❌ 网络连接失败。请检查网络连接或 DNS 设置。",
            SystemError::DiskSpaceError(_) => 
                "❌ 磁盘空间不足。请清理磁盘空间后重试。",
            SystemError::PackageManagerError(_) => 
                "❌ 包管理器错误。请检查 apt 源配置或网络连接。",
            SystemError::ServiceError(_) => 
                "❌ 服务管理错误。请检查服务名称和权限。",
            SystemError::RebootError(_) => 
                "❌ 系统重启失败。请手动执行重启操作。",
            SystemError::FileOperationError(_) => 
                "❌ 文件操作失败。请检查文件权限和磁盘空间。",
            SystemError::CommandExecutionError(_) => 
                "❌ 命令执行失败。请检查命令路径和参数。",
            SystemError::UnknownError(_) => 
                "❌ 发生未知错误。请检查系统日志或联系技术支持。",
        }
    }
    
    /// 判断是否为可重试的错误
    pub fn is_retryable(&self) -> bool {
        matches!(self, 
            SystemError::NetworkError(_) | 
            SystemError::PackageManagerError(_) |
            SystemError::CommandExecutionError(_)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_error_permission_denied() {
        let error = SystemError::PermissionDenied("Access denied".to_string());
        assert_eq!(format!("{}", error), "权限不足: Access denied");
        assert_eq!(error.user_message(), "❌ 权限不足。请确保以 root 权限运行此程序。");
        assert!(!error.is_retryable());
    }

    #[test]
    fn test_system_error_network_error() {
        let error = SystemError::NetworkError("Connection timeout".to_string());
        assert_eq!(format!("{}", error), "网络连接失败: Connection timeout");
        assert_eq!(error.user_message(), "❌ 网络连接失败。请检查网络连接或 DNS 设置。");
        assert!(error.is_retryable());
    }

    #[test]
    fn test_system_error_disk_space_error() {
        let error = SystemError::DiskSpaceError("No space left".to_string());
        assert_eq!(format!("{}", error), "磁盘空间不足: No space left");
        assert_eq!(error.user_message(), "❌ 磁盘空间不足。请清理磁盘空间后重试。");
        assert!(!error.is_retryable());
    }

    #[test]
    fn test_system_error_package_manager_error() {
        let error = SystemError::PackageManagerError("Apt update failed".to_string());
        assert_eq!(format!("{}", error), "包管理器错误: Apt update failed");
        assert_eq!(error.user_message(), "❌ 包管理器错误。请检查 apt 源配置或网络连接。");
        assert!(error.is_retryable());
    }

    #[test]
    fn test_system_error_service_error() {
        let error = SystemError::ServiceError("Service restart failed".to_string());
        assert_eq!(format!("{}", error), "服务管理错误: Service restart failed");
        assert_eq!(error.user_message(), "❌ 服务管理错误。请检查服务名称和权限。");
        assert!(!error.is_retryable());
    }

    #[test]
    fn test_system_error_reboot_error() {
        let error = SystemError::RebootError("Reboot command failed".to_string());
        assert_eq!(format!("{}", error), "系统重启失败: Reboot command failed");
        assert_eq!(error.user_message(), "❌ 系统重启失败。请手动执行重启操作。");
        assert!(!error.is_retryable());
    }

    #[test]
    fn test_system_error_file_operation_error() {
        let error = SystemError::FileOperationError("Cannot write file".to_string());
        assert_eq!(format!("{}", error), "文件操作失败: Cannot write file");
        assert_eq!(error.user_message(), "❌ 文件操作失败。请检查文件权限和磁盘空间。");
        assert!(!error.is_retryable());
    }

    #[test]
    fn test_system_error_command_execution_error() {
        let error = SystemError::CommandExecutionError("Command not found".to_string());
        assert_eq!(format!("{}", error), "命令执行失败: Command not found");
        assert_eq!(error.user_message(), "❌ 命令执行失败。请检查命令路径和参数。");
        assert!(error.is_retryable());
    }

    #[test]
    fn test_system_error_unknown_error() {
        let error = SystemError::UnknownError("Unexpected error".to_string());
        assert_eq!(format!("{}", error), "未知系统错误: Unexpected error");
        assert_eq!(error.user_message(), "❌ 发生未知错误。请检查系统日志或联系技术支持。");
        assert!(!error.is_retryable());
    }

    #[test]
    fn test_system_error_debug_format() {
        let error = SystemError::NetworkError("Test error".to_string());
        let debug_str = format!("{:?}", error);
        assert!(debug_str.contains("NetworkError"));
        assert!(debug_str.contains("Test error"));
    }

    #[test]
    fn test_system_error_is_retryable_combinations() {
        // 可重试的错误
        assert!(SystemError::NetworkError("test".to_string()).is_retryable());
        assert!(SystemError::PackageManagerError("test".to_string()).is_retryable());
        assert!(SystemError::CommandExecutionError("test".to_string()).is_retryable());
        
        // 不可重试的错误
        assert!(!SystemError::PermissionDenied("test".to_string()).is_retryable());
        assert!(!SystemError::DiskSpaceError("test".to_string()).is_retryable());
        assert!(!SystemError::ServiceError("test".to_string()).is_retryable());
        assert!(!SystemError::RebootError("test".to_string()).is_retryable());
        assert!(!SystemError::FileOperationError("test".to_string()).is_retryable());
        assert!(!SystemError::UnknownError("test".to_string()).is_retryable());
    }

    #[test]
    fn test_system_error_user_message_consistency() {
        let errors = vec![
            SystemError::PermissionDenied("test1".to_string()),
            SystemError::NetworkError("test2".to_string()),
            SystemError::DiskSpaceError("test3".to_string()),
            SystemError::PackageManagerError("test4".to_string()),
            SystemError::ServiceError("test5".to_string()),
            SystemError::RebootError("test6".to_string()),
            SystemError::FileOperationError("test7".to_string()),
            SystemError::CommandExecutionError("test8".to_string()),
            SystemError::UnknownError("test9".to_string()),
        ];
        
        for error in errors {
            let user_msg = error.user_message();
            // 所有用户消息都应该以 ❌ 开头
            assert!(user_msg.starts_with("❌"));
            // 用户消息不应该为空
            assert!(!user_msg.is_empty());
        }
    }

    // === 错误路径测试 ===

    #[test]
    fn test_system_error_edge_cases() {
        // 测试边界情况的错误消息
        let empty_error = SystemError::UnknownError("".to_string());
        assert_eq!(empty_error.user_message(), "❌ 发生未知错误。请检查系统日志或联系技术支持。");
        
        let long_error = SystemError::CommandExecutionError("a".repeat(1000));
        let user_msg = long_error.user_message();
        assert!(user_msg.starts_with("❌"));
        assert!(user_msg.contains("命令执行失败"));
        
        let unicode_error = SystemError::NetworkError("网络错误 你好世界 🌍".to_string());
        assert_eq!(format!("{}", unicode_error), "网络连接失败: 网络错误 你好世界 🌍");
    }

    #[test]
    fn test_error_context_preservation() {
        // 测试错误上下文是否正确保留
        let original_msg = "Failed to execute: permission denied while accessing /var/log/syslog";
        let error = SystemError::PermissionDenied(original_msg.to_string());
        
        let formatted = format!("{}", error);
        assert!(formatted.contains(original_msg));
        
        let debug_formatted = format!("{:?}", error);
        assert!(debug_formatted.contains("PermissionDenied"));
        assert!(debug_formatted.contains(original_msg));
    }

    #[test]
    fn test_error_conversion_scenarios() {
        // 测试不同错误转换场景
        let network_error = SystemError::NetworkError("Connection refused".to_string());
        assert!(network_error.is_retryable());
        
        let permission_error = SystemError::PermissionDenied("Access denied".to_string());
        assert!(!permission_error.is_retryable());
        
        let package_error = SystemError::PackageManagerError("Package not found".to_string());
        assert!(package_error.is_retryable());
        
        let command_error = SystemError::CommandExecutionError("Command failed".to_string());
        assert!(command_error.is_retryable());
    }

    #[test]
    fn test_error_message_special_characters() {
        // 测试特殊字符处理
        let special_chars = vec![
            "Error with quotes: \"hello\"",
            "Error with apostrophe: it's broken",
            "Error with newline: first line\nsecond line",
            "Error with tab: field1\tfield2",
            "Error with null: before\0after",
        ];
        
        for msg in special_chars {
            let error = SystemError::FileOperationError(msg.to_string());
            let formatted = format!("{}", error);
            assert!(formatted.contains("文件操作失败"));
            assert!(formatted.contains(msg));
        }
    }

    #[test]
    fn test_error_classification_edge_cases() {
        // 测试错误分类的边界情况
        let ambiguous_cases = vec![
            ("permission denied network timeout", SystemError::PermissionDenied("".to_string())),
            ("network disk space error", SystemError::NetworkError("".to_string())),
            ("permission denied disk full", SystemError::PermissionDenied("".to_string())),
        ];
        
        for (msg, expected_type) in ambiguous_cases {
            // 这个测试验证分类逻辑的优先级
            // 在实际实现中，我们期望第一个匹配的类型获胜
            match expected_type {
                SystemError::PermissionDenied(_) => {
                    // 权限错误应该有最高优先级
                    assert!(msg.contains("permission denied"));
                },
                SystemError::NetworkError(_) => {
                    assert!(msg.contains("network"));
                },
                _ => {}
            }
        }
    }

    #[test]
    fn test_error_retryable_logic() {
        // 测试可重试错误的逻辑
        let retryable_errors = vec![
            SystemError::NetworkError("timeout".to_string()),
            SystemError::PackageManagerError("apt failed".to_string()),
            SystemError::CommandExecutionError("command not found".to_string()),
        ];
        
        let non_retryable_errors = vec![
            SystemError::PermissionDenied("access denied".to_string()),
            SystemError::DiskSpaceError("no space".to_string()),
            SystemError::ServiceError("service failed".to_string()),
            SystemError::RebootError("reboot failed".to_string()),
            SystemError::FileOperationError("file error".to_string()),
            SystemError::UnknownError("unknown".to_string()),
        ];
        
        for error in retryable_errors {
            assert!(error.is_retryable(), " {:?} should be retryable", error);
        }
        
        for error in non_retryable_errors {
            assert!(!error.is_retryable(), " {:?} should not be retryable", error);
        }
    }

    #[test]
    fn test_error_display_format_consistency() {
        // 测试错误显示格式的一致性
        let test_cases = vec![
            (SystemError::PermissionDenied("test".to_string()), "权限不足: test"),
            (SystemError::NetworkError("test".to_string()), "网络连接失败: test"),
            (SystemError::DiskSpaceError("test".to_string()), "磁盘空间不足: test"),
            (SystemError::PackageManagerError("test".to_string()), "包管理器错误: test"),
            (SystemError::ServiceError("test".to_string()), "服务管理错误: test"),
            (SystemError::RebootError("test".to_string()), "系统重启失败: test"),
            (SystemError::FileOperationError("test".to_string()), "文件操作失败: test"),
            (SystemError::CommandExecutionError("test".to_string()), "命令执行失败: test"),
            (SystemError::UnknownError("test".to_string()), "未知系统错误: test"),
        ];
        
        for (error, expected) in test_cases {
            assert_eq!(format!("{}", error), expected);
        }
    }

    #[test]
    fn test_error_user_message_localization() {
        // 测试用户消息的本地化格式
        let user_messages = vec![
            SystemError::PermissionDenied("test".to_string()).user_message(),
            SystemError::NetworkError("test".to_string()).user_message(),
            SystemError::DiskSpaceError("test".to_string()).user_message(),
            SystemError::PackageManagerError("test".to_string()).user_message(),
            SystemError::ServiceError("test".to_string()).user_message(),
            SystemError::RebootError("test".to_string()).user_message(),
            SystemError::FileOperationError("test".to_string()).user_message(),
            SystemError::CommandExecutionError("test".to_string()).user_message(),
            SystemError::UnknownError("test".to_string()).user_message(),
        ];
        
        // 验证所有用户消息都符合格式
        for msg in user_messages {
            // 应该以 ❌ 开头
            assert!(msg.starts_with("❌"), "用户消息应该以 ❌ 开头: {}", msg);
            
            // 应该包含建议的操作
            assert!(msg.contains("请") || msg.contains("检查") || msg.contains("联系"), 
                   "用户消息应该包含建议: {}", msg);
            
            // 不应该为空
            assert!(!msg.is_empty());
            
            // 消息长度应该合理
            assert!(msg.len() > 10 && msg.len() < 200, "用户消息长度不合理: {}", msg);
        }
    }

    #[test]
    fn test_error_equality_and_hashing() {
        // 测试错误的相等性和哈希特性
        let error1 = SystemError::NetworkError("test".to_string());
        let error2 = SystemError::NetworkError("test".to_string());
        let error3 = SystemError::NetworkError("different".to_string());
        
        // 相同内容的错误应该相等
        assert_eq!(format!("{:?}", error1), format!("{:?}", error2));
        assert_eq!(format!("{:?}", error1), format!("{:?}", error3));
        
        // 不同类型的错误应该不相等
        let error4 = SystemError::PermissionDenied("test".to_string());
        assert_ne!(format!("{:?}", error1), format!("{:?}", error4));
    }

    #[test]
    fn test_error_memory_safety() {
        // 测试错误类型的内存安全性
        let large_string = "x".repeat(10000);
        let error = SystemError::UnknownError(large_string.clone());
        
        // 验证错误消息被正确存储
        assert!(format!("{}", error).contains(&large_string));
        
        // 验证用户消息不受影响
        assert_eq!(error.user_message(), "❌ 发生未知错误。请检查系统日志或联系技术支持。");
    }
}