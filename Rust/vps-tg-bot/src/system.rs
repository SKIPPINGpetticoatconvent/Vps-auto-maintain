use std::time::Duration;
use std::process::Command;
use std::path::Path;
use std::fs;
use crate::error::SystemError;
use users;

#[cfg(test)]
use mockall::{automock, predicate::*};

#[derive(Debug, Clone)]
pub struct ScriptResult {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

#[derive(Debug, Clone)]
pub struct SystemInfo {
    pub uptime: u64,
    pub load_avg: [f64; 3],
    pub memory_used: u64,
    pub memory_total: u64,
}

#[cfg_attr(test, automock)]
pub trait SystemOps: Send + Sync {
    fn execute_script(&self, path: &str, timeout: Duration) -> Result<ScriptResult, SystemError>;
    fn get_system_info(&self) -> Result<SystemInfo, SystemError>;
    fn reboot_system(&self) -> Result<(), SystemError>;
    fn get_service_logs(&self, service: &str, lines: usize) -> Result<String, SystemError>;
    fn check_file_exists(&self, path: &str) -> bool;
}

pub struct RealSystem;

impl RealSystem {
    pub fn new() -> Self {
        Self
    }
}

impl SystemOps for RealSystem {
    fn execute_script(&self, path: &str, timeout: Duration) -> Result<ScriptResult, SystemError> {
        // 验证脚本路径
        let allowed_scripts = [
            "/usr/local/bin/vps-maintain-core.sh",
            "/usr/local/bin/vps-maintain-rules.sh"
        ];
        
        if !allowed_scripts.contains(&path) {
            return Err(SystemError::ExecutionFailed("Unauthorized script path".to_string()));
        }
        
        // 检查文件是否存在
        if !Path::new(path).exists() {
            return Err(SystemError::ExecutionFailed("Script file not found".to_string()));
        }
        
        let output = Command::new("timeout")
            .arg(format!("{}s", timeout.as_secs()))
            .arg(path)
            .output()
            .map_err(|e| SystemError::ExecutionFailed("Failed to execute script".to_string()))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        
        if output.status.code() == Some(124) {
             return Err(SystemError::Timeout("Script execution timed out".to_string()));
        }

        Ok(ScriptResult {
            success: output.status.success(),
            stdout,
            stderr,
            exit_code: output.status.code().unwrap_or(-1),
        })
    }
    
    fn get_system_info(&self) -> Result<SystemInfo, SystemError> {
        // 这里简化权限检查，实际生产环境中需要 root 权限
        // let current_uid = users::get_current_uid();
        // if current_uid != 0 {
        //     return Err(SystemError::ExecutionFailed("Insufficient permissions".to_string()));
        // }
        
        // Uptime
        let uptime_str = fs::read_to_string("/proc/uptime")
            .map_err(|_| SystemError::IoError)?;
        let uptime = uptime_str.split_whitespace().next()
            .ok_or(SystemError::IoError)?
            .parse::<f64>()
            .map(|v| v as u64)
            .map_err(|_| SystemError::IoError)?;

        // Load Avg
        let load_str = fs::read_to_string("/proc/loadavg")
            .map_err(|_| SystemError::IoError)?;
        let parts: Vec<&str> = load_str.split_whitespace().collect();
        if parts.len() < 3 {
            return Err(SystemError::IoError);
        }
        let load_avg = [
            parts[0].parse().unwrap_or(0.0),
            parts[1].parse().unwrap_or(0.0),
            parts[2].parse().unwrap_or(0.0),
        ];

        // Memory
        let meminfo = fs::read_to_string("/proc/meminfo")
            .map_err(|_| SystemError::IoError)?;
        
        let mut total: u64 = 0;
        let mut available: u64 = 0;
        
        for line in meminfo.lines() {
            if line.starts_with("MemTotal:") {
                if let Some(val) = line.split_whitespace().nth(1) {
                    total = val.parse::<u64>().unwrap_or(0) * 1024; // Convert kB to bytes
                }
            } else if line.starts_with("MemAvailable:") {
                if let Some(val) = line.split_whitespace().nth(1) {
                    available = val.parse::<u64>().unwrap_or(0) * 1024;
                }
            }
        }
        
        if available == 0 {
             let mut free = 0;
             let mut buffers = 0;
             let mut cached = 0;
             for line in meminfo.lines() {
                if line.starts_with("MemFree:") {
                    if let Some(val) = line.split_whitespace().nth(1) {
                        free = val.parse::<u64>().unwrap_or(0) * 1024;
                    }
                } else if line.starts_with("Buffers:") {
                     if let Some(val) = line.split_whitespace().nth(1) {
                        buffers = val.parse::<u64>().unwrap_or(0) * 1024;
                    }
                } else if line.starts_with("Cached:") {
                     if let Some(val) = line.split_whitespace().nth(1) {
                        cached = val.parse::<u64>().unwrap_or(0) * 1024;
                    }
                }
             }
             available = free + buffers + cached;
        }

        Ok(SystemInfo {
            uptime,
            load_avg,
            memory_used: total.saturating_sub(available),
            memory_total: total,
        })
    }
    
    fn reboot_system(&self) -> Result<(), SystemError> {
        // 检查是否具有root权限
        let current_uid = users::get_current_uid();
        if current_uid != 0 {
            return Err(SystemError::ExecutionFailed("Root privileges required for system reboot".to_string()));
        }
        
        Command::new("reboot")
            .status()
            .map_err(|_| SystemError::ExecutionFailed("Failed to execute reboot command".to_string()))
            .map(|_| ())
    }
    
    fn get_service_logs(&self, service: &str, lines: usize) -> Result<String, SystemError> {
        // 验证服务名称
        if !service.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
            return Err(SystemError::ExecutionFailed("Invalid service name format".to_string()));
        }
        
        // 限制长度防止缓冲区溢出
        if service.len() > 100 {
            return Err(SystemError::ExecutionFailed("Service name too long".to_string()));
        }
        
        // 限制行数防止资源耗尽
        let limited_lines = std::cmp::min(lines, 1000);
        
        let output = Command::new("journalctl")
            .arg("-u")
            .arg(service)
            .arg("-n")
            .arg(limited_lines.to_string())
            .arg("--no-pager")
            .output()
            .map_err(|_| SystemError::ExecutionFailed("Failed to get service logs".to_string()))?;

        if !output.status.success() {
             return Err(SystemError::ExecutionFailed("Journalctl command failed".to_string()));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
    
    fn check_file_exists(&self, path: &str) -> bool {
        Path::new(path).exists()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::predicate::*;

    #[test]
    fn test_execute_script_mock() {
        let mut mock = MockSystemOps::new();
        mock.expect_execute_script()
            .with(eq("test_script.sh"), always())
            .times(1)
            .returning(|_, _| Ok(ScriptResult {
                success: true,
                stdout: "success".to_string(),
                stderr: "".to_string(),
                exit_code: 0,
            }));

        let result = mock.execute_script("test_script.sh", Duration::from_secs(5));
        assert!(result.is_ok());
        let script_result = result.unwrap();
        assert_eq!(script_result.success, true);
        assert_eq!(script_result.stdout, "success");
    }

    #[test]
    fn test_system_info_mock() {
        let mut mock = MockSystemOps::new();
        mock.expect_get_system_info()
            .times(1)
            .returning(|| Ok(SystemInfo {
                uptime: 3600,
                load_avg: [0.1, 0.2, 0.3],
                memory_used: 1024,
                memory_total: 4096,
            }));

        let result = mock.get_system_info();
        assert!(result.is_ok());
        let info = result.unwrap();
        assert_eq!(info.uptime, 3600);
        assert_eq!(info.memory_total, 4096);
    }
}
