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