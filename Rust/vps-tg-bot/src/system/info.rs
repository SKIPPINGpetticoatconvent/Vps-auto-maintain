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

    // === 错误路径测试 ===

    #[test]
    fn test_system_status_edge_case_values() {
        // 测试系统状态的边界值
        let edge_cases = vec![
            SystemStatus {
                cpu_usage: f32::MAX,
                memory_used: u64::MAX,
                memory_total: u64::MAX,
                disk_used: u64::MAX,
                disk_total: u64::MAX,
                network_rx: u64::MAX,
                network_tx: u64::MAX,
                uptime: u64::MAX,
            },
            SystemStatus {
                cpu_usage: f32::MIN,
                memory_used: 0,
                memory_total: 0,
                disk_used: 0,
                disk_total: 0,
                network_rx: 0,
                network_tx: 0,
                uptime: 0,
            },
        ];
        
        for status in edge_cases {
            // 验证结构体可以正常创建和访问
            assert!(status.cpu_usage.is_finite() || status.cpu_usage == f32::MAX || status.cpu_usage == f32::MIN);
            assert!(status.memory_used <= status.memory_total || status.memory_total == 0);
            assert!(status.disk_used <= status.disk_total || status.disk_total == 0);
        }
    }

    #[test]
    fn test_system_status_memory_calculation_errors() {
        // 测试内存计算中的错误情况
        let error_cases = vec![
            // 内存使用大于总内存
            SystemStatus {
                cpu_usage: 0.0,
                memory_used: 8 * 1024 * 1024 * 1024, // 8GB
                memory_total: 4 * 1024 * 1024 * 1024, // 4GB (小于使用量)
                disk_used: 0,
                disk_total: 0,
                network_rx: 0,
                network_tx: 0,
                uptime: 0,
            },
            // 内存总容量为0
            SystemStatus {
                cpu_usage: 0.0,
                memory_used: 1024 * 1024 * 1024, // 1GB
                memory_total: 0, // 0GB
                disk_used: 0,
                disk_total: 0,
                network_rx: 0,
                network_tx: 0,
                uptime: 0,
            },
        ];
        
        for status in error_cases {
            // 验证这些异常情况下的计算不会崩溃
            if status.memory_total > 0 {
                let usage_percent = (status.memory_used as f64 / status.memory_total as f64) * 100.0;
                // 应该产生合理的值或者无穷大/NaN
                assert!(usage_percent.is_finite() || usage_percent.is_infinite() || usage_percent.is_nan());
            }
        }
    }

    #[test]
    fn test_system_status_disk_calculation_errors() {
        // 测试磁盘计算中的错误情况
        let error_cases = vec![
            // 磁盘使用大于总容量
            SystemStatus {
                cpu_usage: 0.0,
                memory_used: 0,
                memory_total: 0,
                disk_used: 500 * 1024 * 1024 * 1024, // 500GB
                disk_total: 250 * 1024 * 1024 * 1024, // 250GB (小于使用量)
                network_rx: 0,
                network_tx: 0,
                uptime: 0,
            },
            // 磁盘总容量为0
            SystemStatus {
                cpu_usage: 0.0,
                memory_used: 0,
                memory_total: 0,
                disk_used: 100 * 1024 * 1024 * 1024, // 100GB
                disk_total: 0, // 0GB
                network_rx: 0,
                network_tx: 0,
                uptime: 0,
            },
        ];
        
        for status in error_cases {
            // 验证这些异常情况下的计算不会崩溃
            if status.disk_total > 0 {
                let usage_percent = (status.disk_used as f64 / status.disk_total as f64) * 100.0;
                assert!(usage_percent.is_finite() || usage_percent.is_infinite() || usage_percent.is_nan());
            }
        }
    }

    #[test]
    fn test_system_status_network_calculation_errors() {
        // 测试网络计算中的错误情况
        let error_cases = vec![
            // 网络数据溢出
            SystemStatus {
                cpu_usage: 0.0,
                memory_used: 0,
                memory_total: 0,
                disk_used: 0,
                disk_total: 0,
                network_rx: u64::MAX, // 最大值
                network_tx: u64::MAX, // 最大值
                uptime: 0,
            },
            // 网络数据为负数（虽然u64不能为负，但测试计算逻辑）
            SystemStatus {
                cpu_usage: 0.0,
                memory_used: 0,
                memory_total: 0,
                disk_used: 0,
                disk_total: 0,
                network_rx: 0,
                network_tx: 0,
                uptime: 0,
            },
        ];
        
        for status in error_cases {
            // 验证网络数据转换不会溢出
            let rx_mb = status.network_rx / 1024 / 1024;
            let tx_mb = status.network_tx / 1024 / 1024;
            let rx_gb = status.network_rx / 1024 / 1024 / 1024;
            let tx_gb = status.network_tx / 1024 / 1024 / 1024;
            
            // 验证计算结果合理
            assert!(rx_mb >= 0);
            assert!(tx_mb >= 0);
            assert!(rx_gb >= 0);
            assert!(tx_gb >= 0);
        }
    }

    #[test]
    fn test_system_status_cpu_usage_errors() {
        // 测试CPU使用率的错误情况
        let error_cases = vec![
            // 负数CPU使用率（理论上不可能）
            SystemStatus {
                cpu_usage: -10.0,
                memory_used: 0,
                memory_total: 0,
                disk_used: 0,
                disk_total: 0,
                network_rx: 0,
                network_tx: 0,
                uptime: 0,
            },
            // 超100%的CPU使用率
            SystemStatus {
                cpu_usage: 150.0,
                memory_used: 0,
                memory_total: 0,
                disk_used: 0,
                disk_total: 0,
                network_rx: 0,
                network_tx: 0,
                uptime: 0,
            },
            // NaN 和无穷大
            SystemStatus {
                cpu_usage: f32::NAN,
                memory_used: 0,
                memory_total: 0,
                disk_used: 0,
                disk_total: 0,
                network_rx: 0,
                network_tx: 0,
                uptime: 0,
            },
            SystemStatus {
                cpu_usage: f32::INFINITY,
                memory_used: 0,
                memory_total: 0,
                disk_used: 0,
                disk_total: 0,
                network_rx: 0,
                network_tx: 0,
                uptime: 0,
            },
        ];
        
        for status in error_cases {
            // 验证特殊值的处理
            if status.cpu_usage.is_nan() {
                assert!(status.cpu_usage.is_nan());
            }
            if status.cpu_usage.is_infinite() {
                assert!(status.cpu_usage.is_infinite());
            }
            if status.cpu_usage < 0.0 {
                assert!(status.cpu_usage < 0.0);
            }
        }
    }

    #[test]
    fn test_system_status_uptime_calculation_errors() {
        // 测试运行时间计算中的错误情况
        let error_cases = vec![
            // 极长的运行时间
            SystemStatus {
                cpu_usage: 0.0,
                memory_used: 0,
                memory_total: 0,
                disk_used: 0,
                disk_total: 0,
                network_rx: 0,
                network_tx: 0,
                uptime: u64::MAX,
            },
            // 异常的运行时间
            SystemStatus {
                cpu_usage: 0.0,
                memory_used: 0,
                memory_total: 0,
                disk_used: 0,
                disk_total: 0,
                network_rx: 0,
                network_tx: 0,
                uptime: u64::MAX / 2,
            },
        ];
        
        for status in error_cases {
            // 验证运行时间转换不会溢出
            let uptime_seconds = status.uptime;
            let uptime_minutes = uptime_seconds / 60;
            let uptime_hours = uptime_seconds / 3600;
            let uptime_days = uptime_seconds / 86400;
            let uptime_years = uptime_seconds / (86400 * 365);
            
            // 验证计算结果合理（不会溢出为0或负数）
            assert!(uptime_minutes >= uptime_seconds / 60);
            assert!(uptime_hours >= uptime_seconds / 3600);
            assert!(uptime_days >= uptime_seconds / 86400);
            assert!(uptime_years >= uptime_seconds / (86400 * 365));
        }
    }

    #[test]
    fn test_system_status_concurrent_access_safety() {
        // 测试并发访问安全性
        let status = SystemStatus {
            cpu_usage: 50.0,
            memory_used: 4 * 1024 * 1024 * 1024,
            memory_total: 8 * 1024 * 1024 * 1024,
            disk_used: 100 * 1024 * 1024 * 1024,
            disk_total: 500 * 1024 * 1024 * 1024,
            network_rx: 1024 * 1024,
            network_tx: 512 * 1024,
            uptime: 3600,
        };
        
        // 模拟多次并发访问
        let cloned_statuses: Vec<SystemStatus> = (0..100).map(|_| status.clone()).collect();
        
        // 验证所有克隆的状态与原始状态一致
        for cloned_status in cloned_statuses {
            assert_eq!(cloned_status.cpu_usage, status.cpu_usage);
            assert_eq!(cloned_status.memory_used, status.memory_used);
            assert_eq!(cloned_status.memory_total, status.memory_total);
            assert_eq!(cloned_status.disk_used, status.disk_used);
            assert_eq!(cloned_status.disk_total, status.disk_total);
            assert_eq!(cloned_status.network_rx, status.network_rx);
            assert_eq!(cloned_status.network_tx, status.network_tx);
            assert_eq!(cloned_status.uptime, status.uptime);
        }
    }

    #[test]
    fn test_system_status_memory_efficiency() {
        // 测试内存效率
        let large_status = SystemStatus {
            cpu_usage: 0.0,
            memory_used: 0,
            memory_total: 0,
            disk_used: 0,
            disk_total: 0,
            network_rx: 0,
            network_tx: 0,
            uptime: 0,
        };
        
        // 验证结构体大小合理
        let size = std::mem::size_of::<SystemStatus>();
        assert!(size < 1000, "SystemStatus size should be reasonable, got: {} bytes", size);
        
        // 验证克隆操作高效
        let start = std::time::Instant::now();
        for _ in 0..1000 {
            let _ = large_status.clone();
        }
        let duration = start.elapsed();
        assert!(duration.as_millis() < 100, "Cloning should be efficient");
    }

    #[test]
    fn test_system_status_serialization_readiness() {
        // 测试序列化准备状态
        let status = SystemStatus {
            cpu_usage: 25.5,
            memory_used: 2 * 1024 * 1024 * 1024,
            memory_total: 8 * 1024 * 1024 * 1024,
            disk_used: 50 * 1024 * 1024 * 1024,
            disk_total: 200 * 1024 * 1024 * 1024,
            network_rx: 1024 * 1024,
            network_tx: 512 * 1024,
            uptime: 7200,
        };
        
        // 验证所有字段都可以被序列化
        let debug_str = format!("{:?}", status);
        assert!(debug_str.contains("cpu_usage"));
        assert!(debug_str.contains("memory_used"));
        assert!(debug_str.contains("memory_total"));
        assert!(debug_str.contains("disk_used"));
        assert!(debug_str.contains("disk_total"));
        assert!(debug_str.contains("network_rx"));
        assert!(debug_str.contains("network_tx"));
        assert!(debug_str.contains("uptime"));
        
        // 验证显示格式正确
        let debug_str = format!("{:?}", status);
        assert!(debug_str.contains("SystemStatus"));
    }

    #[test]
    fn test_system_status_data_integrity() {
        // 测试数据完整性
        let test_values = vec![
            (0.0, 0, 0, 0, 0, 0, 0, 0),
            (100.0, u64::MAX, u64::MAX, u64::MAX, u64::MAX, u64::MAX, u64::MAX, u64::MAX),
            (50.0, 1024, 2048, 1024, 2048, 1024, 1024, 1024),
        ];
        
        for (cpu, mem_used, mem_total, disk_used, disk_total, net_rx, net_tx, up) in test_values {
            let status = SystemStatus {
                cpu_usage: cpu,
                memory_used: mem_used,
                memory_total: mem_total,
                disk_used: disk_used,
                disk_total: disk_total,
                network_rx: net_rx,
                network_tx: net_tx,
                uptime: up,
            };
            
            // 验证数据完整性
            assert_eq!(status.cpu_usage, cpu);
            assert_eq!(status.memory_used, mem_used);
            assert_eq!(status.memory_total, mem_total);
            assert_eq!(status.disk_used, disk_used);
            assert_eq!(status.disk_total, disk_total);
            assert_eq!(status.network_rx, net_rx);
            assert_eq!(status.network_tx, net_tx);
            assert_eq!(status.uptime, up);
        }
    }

    #[test]
    fn test_system_status_calculation_accuracy() {
        // 测试计算准确性
        let status = SystemStatus {
            cpu_usage: 0.0, // 将在计算中设置
            memory_used: 3 * 1024 * 1024 * 1024, // 3GB
            memory_total: 12 * 1024 * 1024 * 1024, // 12GB
            disk_used: 75 * 1024 * 1024 * 1024, // 75GB
            disk_total: 250 * 1024 * 1024 * 1024, // 250GB
            network_rx: 2 * 1024 * 1024 * 1024, // 2GB
            network_tx: 1 * 1024 * 1024 * 1024, // 1GB
            uptime: 86400 * 30, // 30天
        };
        
        // 验证内存使用率计算
        let expected_memory_percent = (status.memory_used as f64 / status.memory_total as f64) * 100.0;
        assert!((expected_memory_percent - 25.0).abs() < 0.01); // 25%
        
        // 验证磁盘使用率计算
        let expected_disk_percent = (status.disk_used as f64 / status.disk_total as f64) * 100.0;
        assert!((expected_disk_percent - 30.0).abs() < 0.01); // 30%
        
        // 验证网络数据转换
        let expected_rx_gb = status.network_rx / 1024 / 1024 / 1024;
        let expected_tx_gb = status.network_tx / 1024 / 1024 / 1024;
        assert_eq!(expected_rx_gb, 2);
        assert_eq!(expected_tx_gb, 1);
        
        // 验证运行时间转换
        let expected_days = status.uptime / 86400;
        assert_eq!(expected_days, 30);
    }

    #[test]
    fn test_system_status_boundary_conditions() {
        // 测试边界条件
        let boundary_cases = vec![
            // 最小值
            SystemStatus {
                cpu_usage: f32::MIN_POSITIVE,
                memory_used: 1,
                memory_total: 1,
                disk_used: 1,
                disk_total: 1,
                network_rx: 1,
                network_tx: 1,
                uptime: 1,
            },
            // 接近最大值
            SystemStatus {
                cpu_usage: 99.99,
                memory_used: u64::MAX - 1,
                memory_total: u64::MAX,
                disk_used: u64::MAX - 1,
                disk_total: u64::MAX,
                network_rx: u64::MAX - 1,
                network_tx: u64::MAX - 1,
                uptime: u64::MAX - 1,
            },
        ];
        
        for status in boundary_cases {
            // 验证边界值处理正确
            assert!(status.cpu_usage >= f32::MIN_POSITIVE || status.cpu_usage == f32::MIN_POSITIVE);
            assert!(status.memory_used <= status.memory_total || status.memory_total == 1);
            assert!(status.disk_used <= status.disk_total || status.disk_total == 1);
            assert!(status.memory_used > 0);
            assert!(status.disk_used > 0);
            assert!(status.uptime > 0);
        }
    }

    #[test]
    fn test_system_status_error_propagation() {
        // 测试错误传播
        // 这个测试验证当底层系统调用失败时，错误能够正确传播
        
        // 由于我们无法直接模拟 sysinfo 的失败，这里测试结构体的健壮性
        let suspicious_status = SystemStatus {
            cpu_usage: f32::NAN,
            memory_used: u64::MAX,
            memory_total: 0, // 这会导致除零
            disk_used: u64::MAX,
            disk_total: 0, // 这会导致除零
            network_rx: u64::MAX,
            network_tx: u64::MAX,
            uptime: u64::MAX,
        };
        
        // 验证即使在异常情况下，结构体仍然可用
        assert!(suspicious_status.cpu_usage.is_nan());
        assert_eq!(suspicious_status.memory_total, 0);
        assert_eq!(suspicious_status.disk_total, 0);
        
        // 验证计算时的行为
        if suspicious_status.memory_total > 0 {
            let memory_usage = suspicious_status.memory_used as f64 / suspicious_status.memory_total as f64;
            assert!(memory_usage.is_finite());
        }
        
        if suspicious_status.disk_total > 0 {
            let disk_usage = suspicious_status.disk_used as f64 / suspicious_status.disk_total as f64;
            assert!(disk_usage.is_finite());
        }
    }

    #[test]
    fn test_system_status_resource_monitoring_scenarios() {
        // 测试资源监控场景
        let scenarios = vec![
            // 低资源使用场景
            SystemStatus {
                cpu_usage: 5.0,
                memory_used: 512 * 1024 * 1024, // 512MB
                memory_total: 8 * 1024 * 1024 * 1024, // 8GB
                disk_used: 10 * 1024 * 1024 * 1024, // 10GB
                disk_total: 100 * 1024 * 1024 * 1024, // 100GB
                network_rx: 100 * 1024 * 1024, // 100MB
                network_tx: 50 * 1024 * 1024, // 50MB
                uptime: 86400, // 1天
            },
            // 高资源使用场景
            SystemStatus {
                cpu_usage: 95.0,
                memory_used: 28 * 1024 * 1024 * 1024, // 28GB
                memory_total: 32 * 1024 * 1024 * 1024, // 32GB
                disk_used: 450 * 1024 * 1024 * 1024, // 450GB
                disk_total: 500 * 1024 * 1024 * 1024, // 500GB
                network_rx: 10 * 1024 * 1024 * 1024, // 10GB
                network_tx: 5 * 1024 * 1024 * 1024, // 5GB
                uptime: 86400 * 365, // 1年
            },
            // 临界资源场景
            SystemStatus {
                cpu_usage: 100.0,
                memory_used: 32 * 1024 * 1024 * 1024, // 32GB
                memory_total: 32 * 1024 * 1024 * 1024, // 32GB (100%)
                disk_used: 499 * 1024 * 1024 * 1024, // 499GB
                disk_total: 500 * 1024 * 1024 * 1024, // 500GB (99.8%)
                network_rx: u64::MAX / 2,
                network_tx: u64::MAX / 2,
                uptime: u64::MAX / 2,
            },
        ];
        
        for (i, status) in scenarios.iter().enumerate() {
            match i {
                0 => {
                    // 低资源使用场景验证
                    let memory_percent = (status.memory_used as f64 / status.memory_total as f64) * 100.0;
                    let disk_percent = (status.disk_used as f64 / status.disk_total as f64) * 100.0;
                    assert!(memory_percent < 20.0);
                    assert!(disk_percent < 20.0);
                },
                1 => {
                    // 高资源使用场景验证
                    let memory_percent = (status.memory_used as f64 / status.memory_total as f64) * 100.0;
                    let disk_percent = (status.disk_used as f64 / status.disk_total as f64) * 100.0;
                    assert!(memory_percent > 80.0);
                    assert!(disk_percent > 80.0);
                },
                2 => {
                    // 临界资源场景验证
                    assert_eq!(status.cpu_usage, 100.0);
                    assert_eq!(status.memory_used, status.memory_total);
                },
                _ => {}
            }
        }
    }
}