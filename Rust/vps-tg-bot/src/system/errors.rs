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
}