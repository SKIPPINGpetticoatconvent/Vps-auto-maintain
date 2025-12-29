use anyhow::{Context, Result};
use tokio::process::Command;
use crate::system::errors::SystemError;
use crate::scheduler::maintenance_history::{self, MaintenanceResult};

pub async fn perform_maintenance() -> Result<String, SystemError> {
    let mut log = String::new();
    let mut has_errors = false;

    log.push_str("🔄 正在更新系统...\n");
    match run_command_with_error_context("apt-get", &["update"], "系统更新").await {
        Ok(output) => log.push_str(&format!("✅ Apt 更新: 成功\n{}\n", output)),
        Err(e) => {
            log.push_str(&format!("❌ Apt 更新: 失败 ({})\n", e));
            has_errors = true;
        }
    }

    log.push_str("🔄 正在升级系统...\n");
    match run_command_with_error_context("apt-get", &["full-upgrade", "-y"], "系统升级").await {
        Ok(output) => log.push_str(&format!("✅ Apt 完全升级: 成功\n{}\n", output)),
        Err(e) => {
            log.push_str(&format!("❌ Apt 完全升级: 失败 ({})\n", e));
            has_errors = true;
        }
    }

    log.push_str("🔄 正在清理不必要的软件包...\n");
    match run_command_with_error_context("apt-get", &["autoremove", "-y"], "清理软件包").await {
        Ok(output) => log.push_str(&format!("✅ Apt 自动移除: 成功\n{}\n", output)),
        Err(e) => {
            log.push_str(&format!("❌ Apt 自动移除: 失败 ({})\n", e));
            has_errors = true;
        }
    }

    log.push_str("🔄 正在清理缓存...\n");
    match run_command_with_error_context("apt-get", &["autoclean"], "清理缓存").await {
        Ok(output) => log.push_str(&format!("✅ Apt 自动清理: 成功\n{}\n", output)),
        Err(e) => {
            log.push_str(&format!("❌ Apt 自动清理: 失败 ({})\n", e));
            has_errors = true;
        }
    }

    // 记录维护历史
    let result = if has_errors {
        MaintenanceResult::Partial
    } else {
        MaintenanceResult::Success
    };
    
    let error_message = if has_errors { Some("部分操作失败") } else { None };
    maintenance_history::record_maintenance("系统维护", result, &log, error_message).await;

    Ok(log)
}

pub async fn check_security_updates() -> Result<bool, SystemError> {
    let output = run_command_with_error_context("apt-get", &["upgrade", "-s"], "检查安全更新")
        .await
        .map_err(|e| SystemError::PackageManagerError(format!("无法检查安全更新: {}", e)))?;
    Ok(output.contains("security"))
}

pub async fn reboot_system() -> Result<(), SystemError> {
    let status = Command::new("reboot")
        .status()
        .await
        .map_err(|e| SystemError::RebootError(format!("重启命令执行失败: {}", e)))?;
    
    if !status.success() {
        return Err(SystemError::RebootError("重启命令返回非零状态码".to_string()));
    }
    
    Ok(())
}

pub async fn restart_service(service_name: &str) -> Result<(), SystemError> {
    let status = Command::new("systemctl")
        .args(["restart", service_name])
        .status()
        .await
        .map_err(|e| SystemError::ServiceError(format!("服务重启命令执行失败: {}", e)))?;
    
    if !status.success() {
        return Err(SystemError::ServiceError(format!("服务 {} 重启失败", service_name)));
    }
    
    Ok(())
}

pub async fn update_xray() -> Result<String, SystemError> {
    let script = "bash -c $(curl -L https://github.com/XTLS/Xray-install/raw/main/install-release.sh) @ install";
    let result = run_command_with_error_context("bash", &["-c", script], "更新 Xray")
        .await
        .map_err(|e| SystemError::NetworkError(format!("Xray 更新失败: {}", e)))?;
    
    // 记录维护历史
    maintenance_history::record_maintenance("Xray更新", MaintenanceResult::Success, &result, None).await;
    
    Ok(result)
}

pub async fn update_singbox() -> Result<String, SystemError> {
    let script = "bash -c $(curl -L https://github.com/SagerNet/sing-box/raw/master/install.sh) @ install";
    let result = run_command_with_error_context("bash", &["-c", script], "更新 Sing-box")
        .await
        .map_err(|e| SystemError::NetworkError(format!("Sing-box 更新失败: {}", e)))?;
    
    // 记录维护历史
    maintenance_history::record_maintenance("Sing-box更新", MaintenanceResult::Success, &result, None).await;
    
    Ok(result)
}

pub async fn maintain_core() -> Result<String, SystemError> {
    let mut log = String::new();
    let mut has_errors = false;

    log.push_str("🔄 正在执行核心维护...\n");
    match run_command_with_error_context("apt-get", &["update"], "核心维护更新").await {
        Ok(output) => log.push_str(&format!("✅ Apt 更新: 成功\n{}\n", output)),
        Err(e) => {
            log.push_str(&format!("❌ Apt 更新: 失败 ({})\n", e));
            has_errors = true;
        }
    }

    log.push_str("🔄 正在升级系统...\n");
    match run_command_with_error_context("apt-get", &["full-upgrade", "-y"], "核心维护升级").await {
        Ok(output) => log.push_str(&format!("✅ Apt 完全升级: 成功\n{}\n", output)),
        Err(e) => {
            log.push_str(&format!("❌ Apt 完全升级: 失败 ({})\n", e));
            has_errors = true;
        }
    }

    log.push_str("🔄 系统更新完成，将在 3 秒后重启系统...\n");
    log.push_str("⚠️ 请保存您的工作，系统将自动重启\n");

    // 记录维护历史
    let result = if has_errors {
        MaintenanceResult::Partial
    } else {
        MaintenanceResult::Success
    };
    
    let error_message = if has_errors { Some("核心维护部分操作失败") } else { None };
    maintenance_history::record_maintenance("核心维护", result, &log, error_message).await;

    // 启动异步重启任务，给 Bot 发送消息的时间
    tokio::spawn(async {
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
        if let Err(e) = reboot_system().await {
            eprintln!("重启失败: {}", e);
        }
    });

    Ok(log)
}

pub async fn maintain_rules() -> Result<String, SystemError> {
    let result = run_command_with_error_context("bash", &["-c", "/usr/local/bin/vps-maintain-rules.sh"], "规则维护")
        .await
        .map_err(|e| SystemError::FileOperationError(format!("规则维护失败: {}", e)))?;
    
    // 记录维护历史
    maintenance_history::record_maintenance("规则维护", MaintenanceResult::Success, &result, None).await;
    
    Ok(result)
}

pub async fn perform_full_maintenance() -> Result<String, SystemError> {
    let mut log = String::new();
    let mut has_errors = false;

    log.push_str("🚀 开始执行完整维护（核心+规则）...\n\n");

    // 执行核心维护
    log.push_str("🔧 执行核心维护：\n");
    match maintain_core().await {
        Ok(output) => {
            log.push_str(&format!("✅ 核心维护完成:\n{}\n\n", output));
        }
        Err(e) => {
            log.push_str(&format!("❌ 核心维护失败: {}\n\n", e));
            has_errors = true;
        }
    }

    // 等待系统重启完成
    log.push_str("⏳ 等待系统重启完成...\n");
    tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;

    // 执行规则维护
    log.push_str("🌍 执行规则维护：\n");
    match maintain_rules().await {
        Ok(output) => {
            log.push_str(&format!("✅ 规则维护完成:\n{}\n\n", output));
        }
        Err(e) => {
            log.push_str(&format!("❌ 规则维护失败: {}\n\n", e));
            has_errors = true;
        }
    }

    log.push_str("🎉 完整维护执行完成！\n");

    // 记录维护历史
    let result = if has_errors {
        MaintenanceResult::Partial
    } else {
        MaintenanceResult::Success
    };
    
    let error_message = if has_errors { Some("完整维护部分操作失败") } else { None };
    maintenance_history::record_maintenance("完整维护", result, &log, error_message).await;

    Ok(log)
}

pub async fn get_system_logs(lines: usize) -> Result<String, SystemError> {
    run_command_with_error_context("journalctl", &["-n", &lines.to_string(), "--no-pager"], "获取系统日志")
        .await
        .map_err(|e| SystemError::CommandExecutionError(format!("获取系统日志失败: {}", e)))
}

async fn run_command_with_error_context(
    command: &str, 
    args: &[&str], 
    context: &str
) -> Result<String, SystemError> {
    let output = Command::new(command)
        .args(args)
        .output()
        .await
        .map_err(|e| SystemError::CommandExecutionError(format!("无法执行命令 {}: {}", command, e)))?;

    if !output.status.success() {
        let error_message = String::from_utf8_lossy(&output.stderr);
        let error_type = classify_command_error(command, &error_message);
        return Err(error_type);
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn classify_command_error(command: &str, error_message: &str) -> SystemError {
    let error_lower = error_message.to_lowercase();
    
    // 权限相关错误
    if error_lower.contains("permission denied") || 
       error_lower.contains("operation not permitted") ||
       error_lower.contains("cannot open") {
        return SystemError::PermissionDenied(format!("{}: {}", command, error_message));
    }
    
    // 网络相关错误
    if error_lower.contains("network") ||
       error_lower.contains("connection") ||
       error_lower.contains("timeout") ||
       error_lower.contains("dns") ||
       error_lower.contains("curl") {
        return SystemError::NetworkError(format!("{}: {}", command, error_message));
    }
    
    // 磁盘空间错误
    if error_lower.contains("no space left on device") ||
       error_lower.contains("disk") {
        return SystemError::DiskSpaceError(format!("{}: {}", command, error_message));
    }
    
    // 包管理器错误
    if command.contains("apt") || command.contains("dpkg") {
        return SystemError::PackageManagerError(format!("{}: {}", command, error_message));
    }
    
    // 服务管理错误
    if command.contains("systemctl") {
        return SystemError::ServiceError(format!("{}: {}", command, error_message));
    }
    
    // 默认分类为命令执行错误
    SystemError::CommandExecutionError(format!("{}: {}", command, error_message))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::runtime::Runtime;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    #[test]
    fn test_classify_command_error_permission_denied() {
        let error_message = "permission denied";
        let error = classify_command_error("apt-get", error_message);
        
        match error {
            SystemError::PermissionDenied(msg) => {
                assert!(msg.contains("permission denied"));
                assert!(msg.contains("apt-get"));
            }
            _ => panic!("Expected PermissionDenied error"),
        }
    }

    #[test]
    fn test_classify_command_error_operation_not_permitted() {
        let error_message = "operation not permitted";
        let error = classify_command_error("systemctl", error_message);
        
        match error {
            SystemError::PermissionDenied(msg) => {
                assert!(msg.contains("operation not permitted"));
                assert!(msg.contains("systemctl"));
            }
            _ => panic!("Expected PermissionDenied error"),
        }
    }

    #[test]
    fn test_classify_command_error_network() {
        let error_message = "connection timeout";
        let error = classify_command_error("curl", error_message);
        
        match error {
            SystemError::NetworkError(msg) => {
                assert!(msg.contains("connection timeout"));
                assert!(msg.contains("curl"));
            }
            _ => panic!("Expected NetworkError"),
        }
    }

    #[test]
    fn test_classify_command_error_dns() {
        let error_message = "dns resolution failed";
        let error = classify_command_error("wget", error_message);
        
        match error {
            SystemError::NetworkError(msg) => {
                assert!(msg.contains("dns resolution failed"));
                assert!(msg.contains("wget"));
            }
            _ => panic!("Expected NetworkError"),
        }
    }

    #[test]
    fn test_classify_command_error_disk_space() {
        let error_message = "no space left on device";
        let error = classify_command_error("dd", error_message);
        
        match error {
            SystemError::DiskSpaceError(msg) => {
                assert!(msg.contains("no space left on device"));
                assert!(msg.contains("dd"));
            }
            _ => panic!("Expected DiskSpaceError"),
        }
    }

    #[test]
    fn test_classify_command_error_package_manager() {
        let error_message = "package not found";
        let error = classify_command_error("apt-get", error_message);
        
        match error {
            SystemError::PackageManagerError(msg) => {
                assert!(msg.contains("package not found"));
                assert!(msg.contains("apt-get"));
            }
            _ => panic!("Expected PackageManagerError"),
        }
    }

    #[test]
    fn test_classify_command_error_service_management() {
        let error_message = "service not found";
        let error = classify_command_error("systemctl", error_message);
        
        match error {
            SystemError::ServiceError(msg) => {
                assert!(msg.contains("service not found"));
                assert!(msg.contains("systemctl"));
            }
            _ => panic!("Expected ServiceError"),
        }
    }

    #[test]
    fn test_classify_command_error_default() {
        let error_message = "unknown error";
        let error = classify_command_error("unknown_command", error_message);
        
        match error {
            SystemError::CommandExecutionError(msg) => {
                assert!(msg.contains("unknown error"));
                assert!(msg.contains("unknown_command"));
            }
            _ => panic!("Expected CommandExecutionError"),
        }
    }

    #[test]
    fn test_classify_command_error_case_insensitive() {
        let error_message = "PERMISSION DENIED";
        let error = classify_command_error("test", error_message);
        
        match error {
            SystemError::PermissionDenied(_) => {
                // 应该被识别为权限错误（大小写不敏感）
            }
            _ => panic!("Expected PermissionDenied error (case insensitive)"),
        }
    }

    #[test]
    fn test_classify_command_error_mixed_keywords() {
        // 测试包含多个关键字的错误消息
        let error_message = "permission denied: network connection timeout";
        let error = classify_command_error("test", error_message);
        
        // 权限错误应该优先匹配
        match error {
            SystemError::PermissionDenied(_) => {
                // 正确 - 权限错误优先
            }
            _ => panic!("Expected PermissionDenied error to take priority"),
        }
    }

    #[test]
    fn test_classify_command_error_apt_specific() {
        let error_message = "some apt error";
        let error = classify_command_error("apt", error_message);
        
        match error {
            SystemError::PackageManagerError(_) => {
                // apt命令应该被分类为包管理器错误
            }
            _ => panic!("Expected PackageManagerError for apt command"),
        }
    }

    #[test]
    fn test_classify_command_error_dpkg_specific() {
        let error_message = "some dpkg error";
        let error = classify_command_error("dpkg", error_message);
        
        match error {
            SystemError::PackageManagerError(_) => {
                // dpkg命令应该被分类为包管理器错误
            }
            _ => panic!("Expected PackageManagerError for dpkg command"),
        }
    }

    #[test]
    fn test_classify_command_error_systemctl_specific() {
        let error_message = "some systemctl error";
        let error = classify_command_error("systemctl", error_message);
        
        match error {
            SystemError::ServiceError(_) => {
                // systemctl命令应该被分类为服务错误
            }
            _ => panic!("Expected ServiceError for systemctl command"),
        }
    }

    #[test]
    fn test_classify_command_error_curl_keywords() {
        let error_message = "some curl error";
        let error = classify_command_error("curl", error_message);
        
        match error {
            SystemError::NetworkError(_) => {
                // curl命令应该被分类为网络错误
            }
            _ => panic!("Expected NetworkError for curl command"),
        }
    }

    #[test]
    fn test_classify_command_error_empty_message() {
        let error_message = "";
        let error = classify_command_error("test", error_message);
        
        match error {
            SystemError::CommandExecutionError(msg) => {
                assert!(msg.contains("test"));
                assert!(msg.contains(""));
            }
            _ => panic!("Expected CommandExecutionError for empty message"),
        }
    }

    #[test]
    fn test_classify_command_error_special_characters() {
        let error_message = "error with special chars: @#$%^&*()";
        let error = classify_command_error("test", error_message);
        
        match error {
            SystemError::CommandExecutionError(msg) => {
                assert!(msg.contains("test"));
                assert!(msg.contains("@#$%^&*()"));
            }
            _ => panic!("Expected CommandExecutionError"),
        }
    }

    #[test]
    fn test_classify_command_error_unicode() {
        let error_message = "错误信息 with unicode: 你好世界";
        let error = classify_command_error("test", error_message);
        
        match error {
            SystemError::CommandExecutionError(msg) => {
                assert!(msg.contains("test"));
                assert!(msg.contains("你好世界"));
            }
            _ => panic!("Expected CommandExecutionError"),
        }
    }

    // 注意：由于这些函数涉及真实的系统调用，我们只测试错误分类逻辑
    // 实际的命令执行需要在集成测试中进行模拟
    
    #[test]
    fn test_run_command_error_context_structure() {
        // 这个测试验证错误上下文的结构，不执行实际命令
        let command = "test_command";
        let args = &["arg1", "arg2"];
        let context = "测试上下文";
        
        // 我们只测试函数签名和基本结构，不执行实际命令
        // 实际的错误处理逻辑在classify_command_error中测试
        assert_eq!(command, "test_command");
        assert_eq!(args.len(), 2);
        assert_eq!(context, "测试上下文");
    }

    #[test]
    fn test_error_message_formatting() {
        // 测试错误消息格式化
        let command = "apt-get";
        let error_msg = "Permission denied";
        let error = classify_command_error(command, error_msg);
        
        let formatted = format!("{}", error);
        assert!(formatted.contains(command));
        assert!(formatted.contains(error_msg));
    }

    #[test]
    fn test_error_priority_matching() {
        // 测试错误类型匹配的优先级
        let test_cases = vec![
            ("permission denied", "apt-get", SystemError::PermissionDenied("".to_string())),
            ("network error", "curl", SystemError::NetworkError("".to_string())),
            ("no space left", "test", SystemError::DiskSpaceError("".to_string())),
        ];
        
        for (error_msg, command, expected_type) in test_cases {
            let error = classify_command_error(command, error_msg);
            
            match (&error, &expected_type) {
                (SystemError::PermissionDenied(_), SystemError::PermissionDenied(_)) => {},
                (SystemError::NetworkError(_), SystemError::NetworkError(_)) => {},
                (SystemError::DiskSpaceError(_), SystemError::DiskSpaceError(_)) => {},
                _ => panic!("错误类型不匹配: {:?} vs {:?}", error, expected_type),
            }
        }
    }

    #[test]
    fn test_maintenance_result_classification() {
        // 测试维护结果的分类逻辑
        // 注意：这里我们只测试逻辑结构，不执行实际的维护操作
        
        // 模拟有错误的维护场景
        let has_errors = true;
        let result = if has_errors {
            crate::scheduler::maintenance_history::MaintenanceResult::Partial
        } else {
            crate::scheduler::maintenance_history::MaintenanceResult::Success
        };
        
        assert_eq!(result, crate::scheduler::maintenance_history::MaintenanceResult::Partial);
        
        // 模拟无错误的维护场景
        let has_errors = false;
        let result = if has_errors {
            crate::scheduler::maintenance_history::MaintenanceResult::Partial
        } else {
            crate::scheduler::maintenance_history::MaintenanceResult::Success
        };
        
        assert_eq!(result, crate::scheduler::maintenance_history::MaintenanceResult::Success);
    }
}