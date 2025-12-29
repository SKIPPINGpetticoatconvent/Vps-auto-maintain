use anyhow::Result;
use sysinfo::{NetworksExt, System, SystemExt, CpuExt, DiskExt, NetworkExt};

#[derive(Debug, Clone)]
pub struct SystemStatus {
    pub cpu_usage: f32,
    pub memory_used: u64,
    pub memory_total: u64,
    pub disk_used: u64,
    pub disk_total: u64,
    pub network_rx: u64,
    pub network_tx: u64,
    pub uptime: u64,
}

pub fn get_system_status() -> Result<SystemStatus> {
    let mut system = System::new_all();
    system.refresh_all();

    let cpu_usage = system.global_cpu_info().cpu_usage();

    let memory_used = system.used_memory();
    let memory_total = system.total_memory();

    let disk_used = system.disks().iter().fold(0u64, |acc, disk| acc + disk.total_space() - disk.available_space());
    let disk_total = system.disks().iter().fold(0u64, |acc, disk| acc + disk.total_space());

    let network_rx = system.networks().iter().fold(0u64, |acc, (_, data): (&String, &sysinfo::NetworkData)| acc + data.received());
    let network_tx = system.networks().iter().fold(0u64, |acc, (_, data): (&String, &sysinfo::NetworkData)| acc + data.transmitted());

    let uptime = system.uptime();

    Ok(SystemStatus {
        cpu_usage,
        memory_used,
        memory_total,
        disk_used,
        disk_total,
        network_rx,
        network_tx,
        uptime,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sysinfo::{System, SystemExt, CpuExt, DiskExt, NetworkExt};

    #[test]
    fn test_system_status_creation() {
        let status = SystemStatus {
            cpu_usage: 25.5,
            memory_used: 1024 * 1024 * 1024, // 1GB
            memory_total: 4 * 1024 * 1024 * 1024, // 4GB
            disk_used: 50 * 1024 * 1024 * 1024, // 50GB
            disk_total: 100 * 1024 * 1024 * 1024, // 100GB
            network_rx: 1024 * 1024, // 1MB
            network_tx: 512 * 1024, // 512KB
            uptime: 3600, // 1小时
        };
        
        assert_eq!(status.cpu_usage, 25.5);
        assert_eq!(status.memory_used, 1024 * 1024 * 1024);
        assert_eq!(status.memory_total, 4 * 1024 * 1024 * 1024);
        assert_eq!(status.disk_used, 50 * 1024 * 1024 * 1024);
        assert_eq!(status.disk_total, 100 * 1024 * 1024 * 1024);
        assert_eq!(status.network_rx, 1024 * 1024);
        assert_eq!(status.network_tx, 512 * 1024);
        assert_eq!(status.uptime, 3600);
    }

    #[test]
    fn test_system_status_zero_values() {
        let status = SystemStatus {
            cpu_usage: 0.0,
            memory_used: 0,
            memory_total: 0,
            disk_used: 0,
            disk_total: 0,
            network_rx: 0,
            network_tx: 0,
            uptime: 0,
        };
        
        assert_eq!(status.cpu_usage, 0.0);
        assert_eq!(status.memory_used, 0);
        assert_eq!(status.memory_total, 0);
        assert_eq!(status.disk_used, 0);
        assert_eq!(status.disk_total, 0);
        assert_eq!(status.network_rx, 0);
        assert_eq!(status.network_tx, 0);
        assert_eq!(status.uptime, 0);
    }

    #[test]
    fn test_system_status_max_values() {
        let status = SystemStatus {
            cpu_usage: 100.0,
            memory_used: u64::MAX,
            memory_total: u64::MAX,
            disk_used: u64::MAX,
            disk_total: u64::MAX,
            network_rx: u64::MAX,
            network_tx: u64::MAX,
            uptime: u64::MAX,
        };
        
        assert_eq!(status.cpu_usage, 100.0);
        assert_eq!(status.memory_used, u64::MAX);
        assert_eq!(status.memory_total, u64::MAX);
        assert_eq!(status.disk_used, u64::MAX);
        assert_eq!(status.disk_total, u64::MAX);
        assert_eq!(status.network_rx, u64::MAX);
        assert_eq!(status.network_tx, u64::MAX);
        assert_eq!(status.uptime, u64::MAX);
    }

    #[test]
    fn test_system_status_memory_calculation() {
        // 测试内存使用率计算
        let status = SystemStatus {
            cpu_usage: 0.0,
            memory_used: 2 * 1024 * 1024 * 1024, // 2GB
            memory_total: 8 * 1024 * 1024 * 1024, // 8GB
            disk_used: 0,
            disk_total: 0,
            network_rx: 0,
            network_tx: 0,
            uptime: 0,
        };
        
        // 内存使用率应该是 25%
        let expected_usage_percent = (status.memory_used as f64 / status.memory_total as f64) * 100.0;
        assert_eq!(expected_usage_percent, 25.0);
    }

    #[test]
    fn test_system_status_disk_calculation() {
        // 测试磁盘使用率计算
        let status = SystemStatus {
            cpu_usage: 0.0,
            memory_used: 0,
            memory_total: 0,
            disk_used: 30 * 1024 * 1024 * 1024, // 30GB
            disk_total: 100 * 1024 * 1024 * 1024, // 100GB
            network_rx: 0,
            network_tx: 0,
            uptime: 0,
        };
        
        // 磁盘使用率应该是 30%
        let expected_usage_percent = (status.disk_used as f64 / status.disk_total as f64) * 100.0;
        assert_eq!(expected_usage_percent, 30.0);
    }

    #[test]
    fn test_system_status_network_calculation() {
        let status = SystemStatus {
            cpu_usage: 0.0,
            memory_used: 0,
            memory_total: 0,
            disk_used: 0,
            disk_total: 0,
            network_rx: 1024 * 1024 * 1024, // 1GB
            network_tx: 512 * 1024 * 1024, // 512MB
            uptime: 0,
        };
        
        // 验证网络数据存储正确
        assert_eq!(status.network_rx, 1024 * 1024 * 1024);
        assert_eq!(status.network_tx, 512 * 1024 * 1024);
        
        // 验证转换为MB的计算
        let rx_mb = status.network_rx / 1024 / 1024;
        let tx_mb = status.network_tx / 1024 / 1024;
        assert_eq!(rx_mb, 1024); // 1GB = 1024MB
        assert_eq!(tx_mb, 512); // 512MB = 512MB
    }

    #[test]
    fn test_system_status_uptime_calculation() {
        let status = SystemStatus {
            cpu_usage: 0.0,
            memory_used: 0,
            memory_total: 0,
            disk_used: 0,
            disk_total: 0,
            network_rx: 0,
            network_tx: 0,
            uptime: 86400, // 24小时
        };
        
        // 验证运行时间转换
        let uptime_hours = status.uptime / 3600;
        let uptime_days = uptime_hours / 24;
        assert_eq!(uptime_hours, 24);
        assert_eq!(uptime_days, 1);
    }

    #[test]
    fn test_system_status_debug_format() {
        let status = SystemStatus {
            cpu_usage: 50.0,
            memory_used: 2 * 1024 * 1024 * 1024,
            memory_total: 8 * 1024 * 1024 * 1024,
            disk_used: 50 * 1024 * 1024 * 1024,
            disk_total: 100 * 1024 * 1024 * 1024,
            network_rx: 1024 * 1024,
            network_tx: 512 * 1024,
            uptime: 7200,
        };
        
        let debug_str = format!("{:?}", status);
        
        // 验证调试字符串包含所有字段
        assert!(debug_str.contains("cpu_usage"));
        assert!(debug_str.contains("memory_used"));
        assert!(debug_str.contains("memory_total"));
        assert!(debug_str.contains("disk_used"));
        assert!(debug_str.contains("disk_total"));
        assert!(debug_str.contains("network_rx"));
        assert!(debug_str.contains("network_tx"));
        assert!(debug_str.contains("uptime"));
    }

    #[test]
    fn test_system_status_partial_equality() {
        let status1 = SystemStatus {
            cpu_usage: 25.0,
            memory_used: 1024 * 1024 * 1024,
            memory_total: 4 * 1024 * 1024 * 1024,
            disk_used: 50 * 1024 * 1024 * 1024,
            disk_total: 100 * 1024 * 1024 * 1024,
            network_rx: 1024 * 1024,
            network_tx: 512 * 1024,
            uptime: 3600,
        };
        
        let status2 = SystemStatus {
            cpu_usage: 25.0,
            memory_used: 1024 * 1024 * 1024,
            memory_total: 4 * 1024 * 1024 * 1024,
            disk_used: 50 * 1024 * 1024 * 1024,
            disk_total: 100 * 1024 * 1024 * 1024,
            network_rx: 1024 * 1024,
            network_tx: 512 * 1024,
            uptime: 3600,
        };
        
        let status3 = SystemStatus {
            cpu_usage: 30.0, // 不同值
            memory_used: 1024 * 1024 * 1024,
            memory_total: 4 * 1024 * 1024 * 1024,
            disk_used: 50 * 1024 * 1024 * 1024,
            disk_total: 100 * 1024 * 1024 * 1024,
            network_rx: 1024 * 1024,
            network_tx: 512 * 1024,
            uptime: 3600,
        };
        
        // 由于SystemStatus没有实现PartialEq，我们测试字段值
        assert_eq!(status1.cpu_usage, status2.cpu_usage);
        assert_ne!(status1.cpu_usage, status3.cpu_usage);
    }

    #[test]
    fn test_system_status_cloning() {
        let status1 = SystemStatus {
            cpu_usage: 25.0,
            memory_used: 1024 * 1024 * 1024,
            memory_total: 4 * 1024 * 1024 * 1024,
            disk_used: 50 * 1024 * 1024 * 1024,
            disk_total: 100 * 1024 * 1024 * 1024,
            network_rx: 1024 * 1024,
            network_tx: 512 * 1024,
            uptime: 3600,
        };
        
        let status2 = status1.clone();
        
        // 验证克隆后的值相同
        assert_eq!(status1.cpu_usage, status2.cpu_usage);
        assert_eq!(status1.memory_used, status2.memory_used);
        assert_eq!(status1.memory_total, status2.memory_total);
        assert_eq!(status1.disk_used, status2.disk_used);
        assert_eq!(status1.disk_total, status2.disk_total);
        assert_eq!(status1.network_rx, status2.network_rx);
        assert_eq!(status1.network_tx, status2.network_tx);
        assert_eq!(status1.uptime, status2.uptime);
    }

    #[test]
    fn test_system_status_memory_percentage_calculation() {
        let status = SystemStatus {
            cpu_usage: 0.0,
            memory_used: 3 * 1024 * 1024 * 1024, // 3GB
            memory_total: 8 * 1024 * 1024 * 1024, // 8GB
            disk_used: 0,
            disk_total: 0,
            network_rx: 0,
            network_tx: 0,
            uptime: 0,
        };
        
        // 内存使用率应该是 37.5%
        let expected_percentage = (status.memory_used as f64 / status.memory_total as f64) * 100.0;
        assert_eq!(expected_percentage, 37.5);
    }

    #[test]
    fn test_system_status_disk_percentage_calculation() {
        let status = SystemStatus {
            cpu_usage: 0.0,
            memory_used: 0,
            memory_total: 0,
            disk_used: 75 * 1024 * 1024 * 1024, // 75GB
            disk_total: 250 * 1024 * 1024 * 1024, // 250GB
            network_rx: 0,
            network_tx: 0,
            uptime: 0,
        };
        
        // 磁盘使用率应该是 30%
        let expected_percentage = (status.disk_used as f64 / status.disk_total as f64) * 100.0;
        assert_eq!(expected_percentage, 30.0);
    }

    #[test]
    fn test_system_status_network_data_conversion() {
        let status = SystemStatus {
            cpu_usage: 0.0,
            memory_used: 0,
            memory_total: 0,
            disk_used: 0,
            disk_total: 0,
            network_rx: 2048 * 1024 * 1024, // 2GB
            network_tx: 1536 * 1024 * 1024, // 1.5GB
            uptime: 0,
        };
        
        // 验证转换为MB的计算
        let rx_mb = status.network_rx / 1024 / 1024;
        let tx_mb = status.network_tx / 1024 / 1024;
        assert_eq!(rx_mb, 2048); // 2GB = 2048MB
        assert_eq!(tx_mb, 1536); // 1.5GB = 1536MB
        
        // 验证转换为GB的计算
        let rx_gb = status.network_rx / 1024 / 1024 / 1024;
        let tx_gb = status.network_tx / 1024 / 1024 / 1024;
        assert_eq!(rx_gb, 2);
        assert_eq!(tx_gb, 1);
    }

    #[test]
    fn test_system_status_uptime_human_readable() {
        let status = SystemStatus {
            cpu_usage: 0.0,
            memory_used: 0,
            memory_total: 0,
            disk_used: 0,
            disk_total: 0,
            network_rx: 0,
            network_tx: 0,
            uptime: 90000, // 25小时
        };
        
        // 验证运行时间转换
        let uptime_hours = status.uptime / 3600;
        let uptime_days = uptime_hours / 24;
        let remaining_hours = uptime_hours % 24;
        
        assert_eq!(uptime_hours, 25);
        assert_eq!(uptime_days, 1);
        assert_eq!(remaining_hours, 1);
    }

    #[test]
    fn test_system_status_cpu_usage_edge_cases() {
        // 测试CPU使用率的边界值
        let status_zero = SystemStatus {
            cpu_usage: 0.0,
            memory_used: 0,
            memory_total: 0,
            disk_used: 0,
            disk_total: 0,
            network_rx: 0,
            network_tx: 0,
            uptime: 0,
        };
        
        let status_full = SystemStatus {
            cpu_usage: 100.0,
            memory_used: 0,
            memory_total: 0,
            disk_used: 0,
            disk_total: 0,
            network_rx: 0,
            network_tx: 0,
            uptime: 0,
        };
        
        assert_eq!(status_zero.cpu_usage, 0.0);
        assert_eq!(status_full.cpu_usage, 100.0);
    }

    #[test]
    fn test_system_status_memory_edge_cases() {
        // 测试内存使用的边界值
        let status_no_memory = SystemStatus {
            cpu_usage: 0.0,
            memory_used: 0,
            memory_total: 8 * 1024 * 1024 * 1024, // 8GB
            disk_used: 0,
            disk_total: 0,
            network_rx: 0,
            network_tx: 0,
            uptime: 0,
        };
        
        let status_full_memory = SystemStatus {
            cpu_usage: 0.0,
            memory_used: 8 * 1024 * 1024 * 1024, // 8GB
            memory_total: 8 * 1024 * 1024 * 1024, // 8GB
            disk_used: 0,
            disk_total: 0,
            network_rx: 0,
            network_tx: 0,
            uptime: 0,
        };
        
        assert_eq!(status_no_memory.memory_used, 0);
        assert_eq!(status_full_memory.memory_used, status_full_memory.memory_total);
    }

    #[test]
    fn test_system_status_disk_edge_cases() {
        // 测试磁盘使用的边界值
        let status_empty_disk = SystemStatus {
            cpu_usage: 0.0,
            memory_used: 0,
            memory_total: 0,
            disk_used: 0,
            disk_total: 500 * 1024 * 1024 * 1024, // 500GB
            network_rx: 0,
            network_tx: 0,
            uptime: 0,
        };
        
        let status_full_disk = SystemStatus {
            cpu_usage: 0.0,
            memory_used: 0,
            memory_total: 0,
            disk_used: 500 * 1024 * 1024 * 1024, // 500GB
            disk_total: 500 * 1024 * 1024 * 1024, // 500GB
            network_rx: 0,
            network_tx: 0,
            uptime: 0,
        };
        
        assert_eq!(status_empty_disk.disk_used, 0);
        assert_eq!(status_full_disk.disk_used, status_full_disk.disk_total);
    }

    #[test]
    fn test_system_status_network_edge_cases() {
        // 测试网络数据的边界值
        let status_zero_network = SystemStatus {
            cpu_usage: 0.0,
            memory_used: 0,
            memory_total: 0,
            disk_used: 0,
            disk_total: 0,
            network_rx: 0,
            network_tx: 0,
            uptime: 0,
        };
        
        let status_large_network = SystemStatus {
            cpu_usage: 0.0,
            memory_used: 0,
            memory_total: 0,
            disk_used: 0,
            disk_total: 0,
            network_rx: u64::MAX / 2,
            network_tx: u64::MAX / 2,
            uptime: 0,
        };
        
        assert_eq!(status_zero_network.network_rx, 0);
        assert_eq!(status_zero_network.network_tx, 0);
        assert!(status_large_network.network_rx > 0);
        assert!(status_large_network.network_tx > 0);
    }

    #[test]
    fn test_system_status_uptime_edge_cases() {
        // 测试运行时间的边界值
        let status_zero_uptime = SystemStatus {
            cpu_usage: 0.0,
            memory_used: 0,
            memory_total: 0,
            disk_used: 0,
            disk_total: 0,
            network_rx: 0,
            network_tx: 0,
            uptime: 0,
        };
        
        let status_long_uptime = SystemStatus {
            cpu_usage: 0.0,
            memory_used: 0,
            memory_total: 0,
            disk_used: 0,
            disk_total: 0,
            network_rx: 0,
            network_tx: 0,
            uptime: u64::MAX,
        };
        
        assert_eq!(status_zero_uptime.uptime, 0);
        assert_eq!(status_long_uptime.uptime, u64::MAX);
        
        // 验证极长运行时间的计算不会溢出
        let days = status_long_uptime.uptime / (24 * 3600);
        assert!(days > 0);
    }

    #[test]
    fn test_system_status_typical_server_values() {
        // 测试典型服务器的值
        let typical_server = SystemStatus {
            cpu_usage: 45.7,
            memory_used: 16 * 1024 * 1024 * 1024, // 16GB
            memory_total: 32 * 1024 * 1024 * 1024, // 32GB
            disk_used: 200 * 1024 * 1024 * 1024, // 200GB
            disk_total: 500 * 1024 * 1024 * 1024, // 500GB
            network_rx: 1024 * 1024 * 1024 * 5, // 5GB
            network_tx: 1024 * 1024 * 1024 * 2, // 2GB
            uptime: 86400 * 15, // 15天
        };
        
        // 验证计算结果
        let memory_usage_percent = (typical_server.memory_used as f64 / typical_server.memory_total as f64) * 100.0;
        let disk_usage_percent = (typical_server.disk_used as f64 / typical_server.disk_total as f64) * 100.0;
        
        assert_eq!(memory_usage_percent, 50.0);
        assert_eq!(disk_usage_percent, 40.0);
        assert_eq!(typical_server.uptime / 86400, 15);
    }
}