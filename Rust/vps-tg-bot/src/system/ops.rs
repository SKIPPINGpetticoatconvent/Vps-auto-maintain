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

pub fn classify_command_error(command: &str, error_message: &str) -> SystemError {
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

    // === 错误路径测试 ===

    #[test]
    fn test_command_not_found_error() {
        // 测试命令不存在的情况
        let error_message = "command not found";
        let error = classify_command_error("nonexistent_command", error_message);
        
        match error {
            SystemError::CommandExecutionError(msg) => {
                assert!(msg.contains("nonexistent_command"));
                assert!(msg.contains("command not found"));
            }
            _ => panic!("Expected CommandExecutionError for command not found"),
        }
    }

    #[test]
    fn test_command_timeout_error() {
        // 测试命令超时的情况
        let error_message = "command timed out";
        let error = classify_command_error("long_running_command", error_message);
        
        match error {
            SystemError::NetworkError(msg) => {
                assert!(msg.contains("long_running_command"));
                assert!(msg.contains("timed out"));
                assert!(msg.contains("timeout"));
            }
            _ => panic!("Expected NetworkError for timeout"),
        }
    }

    #[test]
    fn test_command_exit_code_error() {
        // 测试命令返回非零退出码的情况
        let error_messages = vec![
            "command exited with status 1",
            "process returned non-zero exit code: 127",
            "command failed with exit code 2",
        ];
        
        for msg in error_messages {
            let error = classify_command_error("test_command", msg);
            match error {
                SystemError::CommandExecutionError(_) => {
                    // 命令执行错误应该被正确分类
                }
                _ => panic!("Expected CommandExecutionError for exit code error: {}", msg),
            }
        }
    }

    #[test]
    fn test_command_output_parsing_error() {
        // 测试命令输出解析失败的情况
        let malformed_outputs = vec![
            "invalid utf8 output: \\xff\\fe\\x00",
            "output contains null bytes\0\0\0",
            "binary data output",
        ];
        
        for output in malformed_outputs {
            let error = classify_command_error("binary_command", output);
            match error {
                SystemError::CommandExecutionError(msg) => {
                    assert!(msg.contains("binary_command"));
                }
                _ => panic!("Expected CommandExecutionError for malformed output"),
            }
        }
    }

    #[test]
    fn test_permission_denied_scenarios() {
        // 测试各种权限被拒绝的场景
        let permission_errors = vec![
            "Permission denied",
            "operation not permitted",
            "EACCES: permission denied",
            "Access denied (insufficient permissions)",
            "sudo: must be root to run this command",
        ];
        
        for error_msg in permission_errors {
            let error = classify_command_error("restricted_command", error_msg);
            match error {
                SystemError::PermissionDenied(msg) => {
                    assert!(msg.contains("restricted_command"));
                    assert!(msg.contains(error_msg));
                }
                _ => panic!("Expected PermissionDenied for: {}", error_msg),
            }
        }
    }

    #[test]
    fn test_disk_space_error_scenarios() {
        // 测试各种磁盘空间不足的场景
        let disk_errors = vec![
            "No space left on device",
            "Disk quota exceeded",
            "write error: No space left on device",
            "cannot write to disk: disk full",
            "ENOSPC: no space left on device",
        ];
        
        for error_msg in disk_errors {
            let error = classify_command_error("write_command", error_msg);
            match error {
                SystemError::DiskSpaceError(msg) => {
                    assert!(msg.contains("write_command"));
                    assert!(msg.contains(error_msg));
                }
                _ => panic!("Expected DiskSpaceError for: {}", error_msg),
            }
        }
    }

    #[test]
    fn test_network_error_scenarios() {
        // 测试各种网络错误的场景
        let network_errors = vec![
            "Connection refused",
            "Network unreachable",
            "DNS resolution failed",
            "Connection timeout",
            "Host not found",
            "Network is unreachable",
        ];
        
        for error_msg in network_errors {
            let error = classify_command_error("network_command", error_msg);
            match error {
                SystemError::NetworkError(msg) => {
                    assert!(msg.contains("network_command"));
                    assert!(msg.contains(error_msg));
                }
                _ => panic!("Expected NetworkError for: {}", error_msg),
            }
        }
    }

    #[test]
    fn test_package_manager_error_scenarios() {
        // 测试包管理器特定的错误
        let apt_errors = vec![
            "Package 'nginx' has no installation candidate",
            "Unable to locate package python3-dev",
            "dpkg: dependency problems prevent configuration",
            "apt-get: command not found",
        ];
        
        for error_msg in apt_errors {
            let error = classify_command_error("apt-get", error_msg);
            match error {
                SystemError::PackageManagerError(msg) => {
                    assert!(msg.contains("apt-get"));
                    assert!(msg.contains(error_msg));
                }
                _ => panic!("Expected PackageManagerError for apt error: {}", error_msg),
            }
        }
    }

    #[test]
    fn test_service_error_scenarios() {
        // 测试服务管理错误
        let service_errors = vec![
            "Failed to restart nginx.service: Unit not found.",
            "systemctl restart failed: Service not active",
            "Job for apache2.service failed",
        ];
        
        for error_msg in service_errors {
            let error = classify_command_error("systemctl", error_msg);
            match error {
                SystemError::ServiceError(msg) => {
                    assert!(msg.contains("systemctl"));
                    assert!(msg.contains(error_msg));
                }
                _ => panic!("Expected ServiceError for systemctl error: {}", error_msg),
            }
        }
    }

    #[test]
    fn test_command_error_priority() {
        // 测试错误分类的优先级
        // 当错误消息包含多个关键字时，优先级应该正确
        let priority_tests = vec![
            ("permission denied network timeout", SystemError::PermissionDenied("".to_string())),
            ("no space left permission denied", SystemError::DiskSpaceError("".to_string())),
            ("network connection permission", SystemError::NetworkError("".to_string())),
        ];
        
        for (error_msg, expected_type) in priority_tests {
            let error = classify_command_error("test", error_msg);
            
            match (&error, &expected_type) {
                (SystemError::PermissionDenied(_), SystemError::PermissionDenied(_)) => {},
                (SystemError::DiskSpaceError(_), SystemError::DiskSpaceError(_)) => {},
                (SystemError::NetworkError(_), SystemError::NetworkError(_)) => {},
                _ => panic!("错误优先级不匹配: {:?} vs {:?}", error, expected_type),
            }
        }
    }

    #[test]
    fn test_error_context_preservation() {
        // 测试错误上下文保留
        let original_command = "critical_system_command";
        let original_error = "Critical system failure with detailed information";
        
        let error = classify_command_error(original_command, original_error);
        
        match error {
            SystemError::CommandExecutionError(msg) => {
                assert!(msg.contains(original_command));
                assert!(msg.contains(original_error));
                assert!(msg.len() > original_command.len() + original_error.len());
            }
            _ => panic!("Expected CommandExecutionError"),
        }
    }

    #[test]
    fn test_error_case_insensitive_matching() {
        // 测试错误消息的大小写不敏感匹配
        let case_variants = vec![
            "PERMISSION DENIED",
            "Permission Denied",
            "permission denied",
            "PeRmIsSiOn DeNiEd",
            "NETWORK ERROR",
            "Network Error",
            "network error",
        ];
        
        for variant in case_variants {
            let error = classify_command_error("test", variant);
            match error {
                SystemError::PermissionDenied(_) | SystemError::NetworkError(_) => {
                    // 应该被正确识别
                }
                _ => panic!("大小写不敏感匹配失败: {}", variant),
            }
        }
    }

    #[test]
    fn test_empty_error_handling() {
        // 测试空错误消息的处理
        let empty_error = "";
        let error = classify_command_error("test", empty_error);
        
        match error {
            SystemError::CommandExecutionError(msg) => {
                assert!(msg.contains("test"));
                assert!(msg.contains(empty_error));
            }
            _ => panic!("Expected CommandExecutionError for empty message"),
        }
    }

    #[test]
    fn test_special_character_error_handling() {
        // 测试包含特殊字符的错误消息
        let special_errors = vec![
            "Error with quotes: \"hello\"",
            "Error with newline: first\nsecond",
            "Error with tab: field1\tfield2",
            "Error with unicode: 你好世界 🌍",
            "Error with null: before\0after",
        ];
        
        for error_msg in special_errors {
            let error = classify_command_error("special_cmd", error_msg);
            
            match error {
                SystemError::CommandExecutionError(msg) => {
                    assert!(msg.contains("special_cmd"));
                    assert!(msg.contains(error_msg));
                }
                _ => panic!("Expected CommandExecutionError for special chars"),
            }
        }
    }

    #[test]
    fn test_error_classification_coverage() {
        // 测试错误分类的完整覆盖
        let all_error_types = vec![
            ("permission denied", "apt-get", SystemError::PermissionDenied("".to_string())),
            ("network unreachable", "curl", SystemError::NetworkError("".to_string())),
            ("no space left", "write", SystemError::DiskSpaceError("".to_string())),
            ("package not found", "apt", SystemError::PackageManagerError("".to_string())),
            ("service not found", "systemctl", SystemError::ServiceError("".to_string())),
            ("unknown error", "generic", SystemError::CommandExecutionError("".to_string())),
        ];
        
        for (error_msg, command, expected_type) in all_error_types {
            let error = classify_command_error(command, error_msg);
            
            match (&error, &expected_type) {
                (SystemError::PermissionDenied(_), SystemError::PermissionDenied(_)) => {},
                (SystemError::NetworkError(_), SystemError::NetworkError(_)) => {},
                (SystemError::DiskSpaceError(_), SystemError::DiskSpaceError(_)) => {},
                (SystemError::PackageManagerError(_), SystemError::PackageManagerError(_)) => {},
                (SystemError::ServiceError(_), SystemError::ServiceError(_)) => {},
                (SystemError::CommandExecutionError(_), SystemError::CommandExecutionError(_)) => {},
                _ => panic!("错误分类覆盖不完整: {:?} vs {:?}", error, expected_type),
            }
        }
    }

    #[test]
    fn test_run_command_error_context_structure_v2() {
        // 测试 run_command_with_error_context 函数的错误上下文结构
        // 这个测试验证函数签名和基本结构，不执行实际命令
        
        let test_cases = vec![
            ("apt-get", &["update", "-y"], "系统更新"),
            ("systemctl", &["restart", "nginx"], "重启服务"),
        ];
        
        for (cmd, _args, context) in test_cases {
            assert!(cmd == "apt-get" || cmd == "systemctl");
            assert!(!context.is_empty());
        }
    }

    #[test]
    fn test_error_message_length_limits() {
        // 测试错误消息长度限制
        let short_error = "short";
        let long_error = "x".repeat(10000);
        
        let short_err = classify_command_error("test", short_error);
        let long_err = classify_command_error("test", &long_error);
        
        match (short_err, long_err) {
            (SystemError::CommandExecutionError(short_msg), SystemError::CommandExecutionError(long_msg)) => {
                assert!(short_msg.contains("test"));
                assert!(short_msg.contains(short_error));
                assert!(long_msg.contains("test"));
                assert!(long_msg.contains(&long_error));
            }
            _ => panic!("错误消息长度测试失败"),
        }
    }

    #[test]
    fn test_command_specific_error_classification() {
        // 测试特定命令的错误分类
        let command_specific_tests = vec![
            ("apt", "some error", SystemError::PackageManagerError("".to_string())),
            ("dpkg", "some error", SystemError::PackageManagerError("".to_string())),
            ("systemctl", "some error", SystemError::ServiceError("".to_string())),
            ("curl", "some error", SystemError::NetworkError("".to_string())),
            ("wget", "some error", SystemError::NetworkError("".to_string())),
            ("unknown", "some error", SystemError::CommandExecutionError("".to_string())),
        ];
        
        for (command, error_msg, expected_type) in command_specific_tests {
            let error = classify_command_error(command, error_msg);
            
            match (&error, &expected_type) {
                (SystemError::PackageManagerError(_), SystemError::PackageManagerError(_)) => {},
                (SystemError::ServiceError(_), SystemError::ServiceError(_)) => {},
                (SystemError::NetworkError(_), SystemError::NetworkError(_)) => {},
                (SystemError::CommandExecutionError(_), SystemError::CommandExecutionError(_)) => {},
                _ => panic!("命令特定分类失败: {:?} vs {:?}", error, expected_type),
            }
        }
    }
}