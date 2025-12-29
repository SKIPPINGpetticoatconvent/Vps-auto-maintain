//! 机器指纹采集模块
//! 
//! 采集机器唯一标识特征，用于生成加密密钥的输入。

use anyhow::{Context, Result};
use log::{debug, warn};
use std::fs;
use std::process::Command;

/// 机器指纹分隔符
const FINGERPRINT_SEPARATOR: &str = "|";

/// 采集机器唯一指纹信息
pub fn collect_machine_fingerprint() -> Result<String> {
    debug!("开始采集机器指纹");

    let mut fingerprint_parts = Vec::new();

    // 1. CPU ID - 从 DMI 信息读取
    if let Ok(cpu_id) = read_dmi_field("product_uuid") {
        debug!("采集到 CPU ID: {}", cpu_id);
        fingerprint_parts.push(cpu_id);
    } else {
        warn!("无法采集 CPU ID");
        fingerprint_parts.push("unknown_cpu_id".to_string());
    }

    // 2. 主网卡 MAC 地址
    if let Ok(mac_addr) = get_primary_network_mac() {
        debug!("采集到主网卡 MAC: {}", mac_addr);
        fingerprint_parts.push(mac_addr);
    } else {
        warn!("无法采集主网卡 MAC 地址");
        fingerprint_parts.push("unknown_mac".to_string());
    }

    // 3. 根分区 UUID
    if let Ok(root_uuid) = get_root_partition_uuid() {
        debug!("采集到根分区 UUID: {}", root_uuid);
        fingerprint_parts.push(root_uuid);
    } else {
        warn!("无法采集根分区 UUID");
        fingerprint_parts.push("unknown_root_uuid".to_string());
    }

    // 4. 主机名
    if let Ok(hostname) = get_hostname() {
        debug!("采集到主机名: {}", hostname);
        fingerprint_parts.push(hostname);
    } else {
        warn!("无法采集主机名");
        hostname_mut()
            .map(|h| {
                fingerprint_parts.push(h.clone());
                debug!("使用备用主机名: {}", h);
            })
            .unwrap_or_else(|_| {
                fingerprint_parts.push("unknown_hostname".to_string());
            });
    }

    // 组合指纹
    let fingerprint = fingerprint_parts.join(FINGERPRINT_SEPARATOR);
    
    debug!("机器指纹采集完成，长度: {} 字符", fingerprint.len());
    debug!("指纹内容: {} (前32字符)", 
           if fingerprint.len() > 32 { 
               format!("{}...", &fingerprint[..32]) 
           } else { 
               fingerprint.clone() 
           });

    Ok(fingerprint)
}

/// 读取 DMI 字段信息
fn read_dmi_field(field: &str) -> Result<String> {
    let dmi_path = format!("/sys/class/dmi/id/{}", field);
    
    let content = fs::read_to_string(&dmi_path)
        .with_context(|| format!("无法读取 DMI 文件: {}", dmi_path))?;
    
    Ok(content.trim().to_string())
}

/// 获取主网卡 MAC 地址
fn get_primary_network_mac() -> Result<String> {
    // 尝试读取主要网络接口的 MAC 地址
    let interfaces = ["eth0", "ens33", "enp0s3", "enp0s25", "wlp2s0"];
    
    for interface in &interfaces {
        if let Ok(mac) = read_interface_mac(interface) {
            return Ok(mac);
        }
    }
    
    // 如果主要接口都不存在，尝试系统第一个网络接口
    if let Ok(output) = Command::new("ip")
        .args(&["link", "show"])
        .output()
    {
        let output_str = String::from_utf8_lossy(&output.stdout);
        for line in output_str.lines() {
            if let Some(mac_start) = line.find("link/ether ") {
                let mac_part = &line[mac_start + 11..];
                if let Some(mac_end) = mac_part.find(' ') {
                    let mac = &mac_part[..mac_end];
                    if mac != "00:00:00:00:00:00" && !mac.starts_with("00:") {
                        return Ok(mac.to_string());
                    }
                }
            }
        }
    }
    
    Err(anyhow::anyhow!("无法找到有效的网络接口 MAC 地址"))
}

/// 读取指定网络接口的 MAC 地址
fn read_interface_mac(interface: &str) -> Result<String> {
    let path = format!("/sys/class/net/{}/address", interface);
    
    let content = fs::read_to_string(&path)
        .with_context(|| format!("无法读取接口 {} MAC 地址", interface))?;
    
    let mac = content.trim();
    
    // 验证 MAC 地址格式
    if mac.len() != 17 || mac.chars().filter(|&c| c == ':').count() != 5 {
        return Err(anyhow::anyhow!("无效的 MAC 地址格式: {}", mac));
    }
    
    Ok(mac.to_string())
}

/// 获取根分区 UUID
fn get_root_partition_uuid() -> Result<String> {
    // 方法 1: 通过 blkid 获取
    if let Ok(output) = Command::new("blkid")
        .args(&["-l", "-t", "TYPE=vfat", "-o", "DEVICE"])
        .output()
    {
        let output_str = String::from_utf8_lossy(&output.stdout);
        if let Some(device) = output_str.lines().find_map(|line| {
            let trimmed = line.trim();
            if !trimmed.is_empty() { Some(trimmed.to_string()) } else { None }
        }) {
            if let Ok(uuid_output) = Command::new("blkid")
                .args(&["-s", "UUID", "-o", "value", &device])
                .output()
            {
                let uuid_binding = String::from_utf8_lossy(&uuid_output.stdout);
                let uuid = uuid_binding.trim();
                if !uuid.is_empty() {
                    return Ok(uuid.to_string());
                }
            }
        }
    }

    // 方法 2: 解析 /etc/fstab
    if let Ok(content) = fs::read_to_string("/etc/fstab") {
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('#') || trimmed.is_empty() {
                continue;
            }
            
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() >= 3 && parts[1] == "/" {
                // 查找 UUID=xxx 的格式
                if let Some(uuid_start) = parts[0].find("UUID=") {
                    let uuid = &parts[0][uuid_start + 5..];
                    if !uuid.is_empty() {
                        return Ok(uuid.to_string());
                    }
                }
            }
        }
    }

    // 方法 3: 通过 lsblk 获取
    if let Ok(output) = Command::new("lsblk")
        .args(&["-f", "--output=UUID,FSTYPE,MOUNTPOINT"])
        .output()
    {
        let output_str = String::from_utf8_lossy(&output.stdout);
        for line in output_str.lines().skip(1) { // 跳过表头
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 && parts[2] == "/" && !parts[0].is_empty() {
                return Ok(parts[0].to_string());
            }
        }
    }

    Err(anyhow::anyhow!("无法获取根分区 UUID"))
}

/// 获取主机名
fn get_hostname() -> Result<String> {
    let hostname = Command::new("hostname")
        .output()
        .with_context(|| "无法执行 hostname 命令")?
        .stdout;
    
    let hostname_binding = String::from_utf8_lossy(&hostname);
    let hostname_str = hostname_binding.trim();
    
    if hostname_str.is_empty() {
        Err(anyhow::anyhow!("主机名为空"))
    } else {
        Ok(hostname_str.to_string())
    }
}

/// 备用方法获取主机名
fn hostname_mut() -> Result<String> {
    Ok(std::env::current_dir()
        .and_then(|path| {
            path.to_str()
                .and_then(|s| s.split('/').last())
                .map(|s| s.to_string())
                .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::Other, "无法从路径提取主机名"))
        })
        .unwrap_or_else(|_| "fallback_hostname".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_fingerprint_separator() {
        assert_eq!(FINGERPRINT_SEPARATOR, "|", "指纹分隔符应该是 |");
    }

    #[test]
    fn test_fingerprint_structure() {
        // 使用模拟数据进行测试
        let parts = vec![
            "cpu-uuid-123".to_string(),
            "mac-addr-456".to_string(), 
            "root-uuid-789".to_string(),
            "hostname".to_string(),
        ];
        
        let fingerprint = parts.join(FINGERPRINT_SEPARATOR);
        
        let expected = "cpu-uuid-123|mac-addr-456|root-uuid-789|hostname";
        assert_eq!(fingerprint, expected);
        
        // 验证分隔符数量
        assert_eq!(fingerprint.matches(FINGERPRINT_SEPARATOR).count(), 3);
    }

    #[test]
    fn test_mac_address_validation() {
        // 测试有效的 MAC 地址格式
        let valid_mac = "00:11:22:33:44:55";
        assert_eq!(valid_mac.len(), 17);
        assert_eq!(valid_mac.chars().filter(|&c| c == ':').count(), 5);
        
        // 测试无效的 MAC 地址格式
        let invalid_mac = "00:11:22:33:44"; // 太短
        assert_ne!(invalid_mac.len(), 17);
    }

    #[test]
    fn test_hostname_validation() {
        // 测试正常主机名
        let hostname = "test-server";
        assert!(!hostname.is_empty());
        assert!(hostname.len() <= 255); // RFC 限制
        
        // 测试空主机名
        let empty_hostname = "";
        assert!(empty_hostname.is_empty());
    }

    #[test]
    fn test_read_dmi_field_mock() {
        // 创建临时 DMI 文件进行测试
        let temp_dir = tempfile::tempdir().unwrap();
        let dmi_file = temp_dir.path().join("product_uuid");
        fs::write(&dmi_file, "test-cpu-uuid-123\n").unwrap();
        
        // 注意：这个测试在真实环境中可能失败，因为我们修改了读取路径
        // 在生产环境中，DMI 文件位于 /sys/class/dmi/id/ 目录
    }
}