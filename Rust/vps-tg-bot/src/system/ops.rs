use anyhow::{Context, Result};
use tokio::process::Command;
use crate::system::errors::SystemError;

pub async fn perform_maintenance() -> Result<String, SystemError> {
    let mut log = String::new();

    log.push_str("🔄 正在更新系统...\n");
    match run_command_with_error_context("apt-get", &["update"], "系统更新").await {
        Ok(output) => log.push_str(&format!("✅ Apt 更新: 成功\n{}\n", output)),
        Err(e) => log.push_str(&format!("❌ Apt 更新: 失败 ({})\n", e)),
    }

    log.push_str("🔄 正在升级系统...\n");
    match run_command_with_error_context("apt-get", &["full-upgrade", "-y"], "系统升级").await {
        Ok(output) => log.push_str(&format!("✅ Apt 完全升级: 成功\n{}\n", output)),
        Err(e) => log.push_str(&format!("❌ Apt 完全升级: 失败 ({})\n", e)),
    }

    log.push_str("🔄 正在清理不必要的软件包...\n");
    match run_command_with_error_context("apt-get", &["autoremove", "-y"], "清理软件包").await {
        Ok(output) => log.push_str(&format!("✅ Apt 自动移除: 成功\n{}\n", output)),
        Err(e) => log.push_str(&format!("❌ Apt 自动移除: 失败 ({})\n", e)),
    }

    log.push_str("🔄 正在清理缓存...\n");
    match run_command_with_error_context("apt-get", &["autoclean"], "清理缓存").await {
        Ok(output) => log.push_str(&format!("✅ Apt 自动清理: 成功\n{}\n", output)),
        Err(e) => log.push_str(&format!("❌ Apt 自动清理: 失败 ({})\n", e)),
    }

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
    run_command_with_error_context("bash", &["-c", script], "更新 Xray")
        .await
        .map_err(|e| SystemError::NetworkError(format!("Xray 更新失败: {}", e)))
}

pub async fn update_singbox() -> Result<String, SystemError> {
    let script = "bash -c $(curl -L https://github.com/SagerNet/sing-box/raw/master/install.sh) @ install";
    run_command_with_error_context("bash", &["-c", script], "更新 Sing-box")
        .await
        .map_err(|e| SystemError::NetworkError(format!("Sing-box 更新失败: {}", e)))
}

pub async fn maintain_core() -> Result<String, SystemError> {
    let mut log = String::new();

    log.push_str("🔄 正在执行核心维护...\n");
    match run_command_with_error_context("apt-get", &["update"], "核心维护更新").await {
        Ok(output) => log.push_str(&format!("✅ Apt 更新: 成功\n{}\n", output)),
        Err(e) => log.push_str(&format!("❌ Apt 更新: 失败 ({})\n", e)),
    }

    log.push_str("🔄 正在升级系统...\n");
    match run_command_with_error_context("apt-get", &["full-upgrade", "-y"], "核心维护升级").await {
        Ok(output) => log.push_str(&format!("✅ Apt 完全升级: 成功\n{}\n", output)),
        Err(e) => log.push_str(&format!("❌ Apt 完全升级: 失败 ({})\n", e)),
    }

    log.push_str("🔄 系统更新完成，将在 3 秒后重启系统...\n");
    log.push_str("⚠️ 请保存您的工作，系统将自动重启\n");

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
    run_command_with_error_context("bash", &["-c", "/usr/local/bin/vps-maintain-rules.sh"], "规则维护")
        .await
        .map_err(|e| SystemError::FileOperationError(format!("规则维护失败: {}", e)))
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