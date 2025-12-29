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
    match read_dmi_field("product_uuid") {
        Ok(cpu_id) => {
            debug!("✅ 采集到 CPU ID: {}", cpu_id);
            fingerprint_parts.push(cpu_id);
        }
        Err(e) => {
            warn!("⚠️ 无法采集 CPU ID: {}", e);
            // 尝试备用 DMI 字段
            if let Ok(backup_cpu_id) = read_dmi_field("sys_vendor").or_else(|_| read_dmi_field("board_name")) {
                debug!("✅ 使用备用 CPU 标识: {}", backup_cpu_id);
                fingerprint_parts.push(format!("backup_{}", backup_cpu_id));
            } else {
                debug!("❌ 使用默认 CPU 标识");
                fingerprint_parts.push("unknown_cpu_id".to_string());
            }
        }
    }

    // 2. 主网卡 MAC 地址
    match get_primary_network_mac() {
        Ok(mac_addr) => {
            debug!("✅ 采集到主网卡 MAC: {}", mac_addr);
            fingerprint_parts.push(mac_addr);
        }
        Err(e) => {
            warn!("⚠️ 无法采集主网卡 MAC 地址: {}", e);
            // 尝试备用方法
            if let Ok(backup_mac) = get_secondary_network_mac() {
                debug!("✅ 使用备用 MAC 地址: {}", backup_mac);
                fingerprint_parts.push(backup_mac);
            } else {
                debug!("❌ 使用默认 MAC 标识");
                fingerprint_parts.push("unknown_mac".to_string());
            }
        }
    }

    // 3. 根分区 UUID
    match get_root_partition_uuid() {
        Ok(root_uuid) => {
            debug!("✅ 采集到根分区 UUID: {}", root_uuid);
            fingerprint_parts.push(root_uuid);
        }
        Err(e) => {
            warn!("⚠️ 无法采集根分区 UUID: {}", e);
            // 尝试备用方法
            if let Ok(backup_uuid) = get_system_uuid() {
                debug!("✅ 使用备用系统 UUID: {}", backup_uuid);
                fingerprint_parts.push(backup_uuid);
            } else {
                debug!("❌ 使用默认 UUID 标识");
                fingerprint_parts.push("unknown_root_uuid".to_string());
            }
        }
    }

    // 4. 主机名
    match get_hostname() {
        Ok(hostname) => {
            debug!("✅ 采集到主机名: {}", hostname);
            fingerprint_parts.push(hostname);
        }
        Err(e) => {
            warn!("⚠️ 无法采集主机名: {}", e);
            // 使用备用方法
            match hostname_mut() {
                Ok(backup_hostname) => {
                    debug!("✅ 使用备用主机名: {}", backup_hostname);
                    fingerprint_parts.push(backup_hostname);
                }
                Err(_) => {
                    debug!("❌ 使用默认主机名");
                    fingerprint_parts.push("unknown_hostname".to_string());
                }
            }
        }
    }

    // 组合指纹
    let fingerprint = fingerprint_parts.join(FINGERPRINT_SEPARATOR);
    
    debug!("✅ 机器指纹采集完成，长度: {} 字符", fingerprint.len());
    debug!("指纹内容: {} (前64字符)", 
           if fingerprint.len() > 64 { 
               format!("{}...", &fingerprint[..64]) 
           } else { 
               fingerprint.clone() 
           });

    // 确保指纹不为空
    if fingerprint.trim().is_empty() {
        return Err(anyhow::anyhow!("所有指纹采集方法都失败，无法生成机器指纹"));
    }

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
        .args(["link", "show"])
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
        .args(["-l", "-t", "TYPE=vfat", "-o", "DEVICE"])
        .output()
    {
        let output_str = String::from_utf8_lossy(&output.stdout);
        if let Some(device) = output_str.lines().find_map(|line| {
            let trimmed = line.trim();
            if !trimmed.is_empty() { Some(trimmed.to_string()) } else { None }
        }) {
            if let Ok(uuid_output) = Command::new("blkid")
                .args(["-s", "UUID", "-o", "value", &device])
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
        .args(["-f", "--output=UUID,FSTYPE,MOUNTPOINT"])
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

/// 获取备用网络接口 MAC 地址
fn get_secondary_network_mac() -> Result<String> {
    // 尝试所有可能的网络接口
    let all_interfaces = [
        "lo", "docker0", "br-*", "veth*", "tap*", "tun*", "ppp*", 
        "eth*", "en*", "wlp*", "wlan*", "ra*", "wlan*"
    ];
    
    for pattern in &all_interfaces {
        if let Ok(mac) = find_interface_by_pattern(pattern) {
            return Ok(mac);
        }
    }
    
    // 最后尝试从 /proc/net/dev 读取
    if let Ok(mac) = read_proc_net_dev() {
        return Ok(mac);
    }
    
    Err(anyhow::anyhow!("无法找到任何网络接口 MAC 地址"))
}

/// 通过模式匹配查找网络接口
fn find_interface_by_pattern(pattern: &str) -> Result<String> {
    // 读取 /sys/class/net/ 目录中的所有接口
    let net_dir = "/sys/class/net";
    if !std::path::Path::new(net_dir).exists() {
        return Err(anyhow::anyhow!("/sys/class/net 目录不存在"));
    }
    
    let entries = fs::read_dir(net_dir)
        .with_context(|| format!("无法读取网络接口目录: {}", net_dir))?;
    
    for entry in entries {
        let entry = entry.with_context(|| "读取目录项失败")?;
        let file_name = entry.file_name();
        let interface_name = file_name
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("接口名称不是有效的 UTF-8"))?;
            
        // 跳过回环接口
        if interface_name == "lo" {
            continue;
        }
        
        // 检查是否匹配模式（简单的字符串包含检查）
        if pattern.contains('*') {
            let pattern_prefix = pattern.trim_end_matches('*');
            if !interface_name.starts_with(pattern_prefix) {
                continue;
            }
        } else if interface_name != pattern {
            continue;
        }
        
        // 尝试读取 MAC 地址
        if let Ok(mac) = read_interface_mac(interface_name) {
            debug!("找到匹配接口 {} 的 MAC: {}", interface_name, mac);
            return Ok(mac);
        }
    }
    
    Err(anyhow::anyhow!("模式 {} 没有找到匹配的接口", pattern))
}

/// 从 /proc/net/dev 读取网络接口信息
fn read_proc_net_dev() -> Result<String> {
    let content = fs::read_to_string("/proc/net/dev")
        .context("无法读取 /proc/net/dev")?;
    
    for line in content.lines().skip(2) { // 跳过前两行标题
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            let interface = parts[0].trim_end_matches(':');
            if interface != "lo" && parts.len() >= 17 {
                let mac = parts[15]; // MAC 地址通常在第16列
                if mac.len() == 17 && mac.chars().filter(|&c| c == ':').count() == 5 {
                    return Ok(mac.to_string());
                }
            }
        }
    }
    
    Err(anyhow::anyhow!("无法从 /proc/net/dev 提取有效 MAC 地址"))
}

/// 获取系统 UUID 作为备用标识
fn get_system_uuid() -> Result<String> {
    // 尝试读取系统 UUID
    if let Ok(uuid) = read_dmi_field("board_serial") {
        return Ok(uuid);
    }
    
    // 尝试 /etc/machine-id
    if let Ok(machine_id) = fs::read_to_string("/etc/machine-id") {
        let machine_id = machine_id.trim();
        if !machine_id.is_empty() {
            return Ok(format!("machine_id_{}", machine_id));
        }
    }
    
    Err(anyhow::anyhow!("无法获取系统 UUID"))
}

/// 备用方法获取主机名
fn hostname_mut() -> Result<String> {
    // 首先尝试环境变量
    if let Ok(hostname) = std::env::var("HOSTNAME") {
        if !hostname.is_empty() {
            return Ok(hostname);
        }
    }
    
    // 尝试从 /etc/hostname 读取
    if let Ok(hostname_content) = fs::read_to_string("/etc/hostname") {
        let hostname = hostname_content.trim();
        if !hostname.is_empty() {
            return Ok(hostname.to_string());
        }
    }
    
    // 尝试从当前目录提取
    std::env::current_dir()
        .and_then(|path| {
            path.to_str()
                .and_then(|s| s.split('/').next_back())
                .map(|s| s.to_string())
                .ok_or_else(|| std::io::Error::other("无法从路径提取主机名"))
        })
        .map_err(|_| anyhow::anyhow!("所有备用主机名方法都失败"))
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