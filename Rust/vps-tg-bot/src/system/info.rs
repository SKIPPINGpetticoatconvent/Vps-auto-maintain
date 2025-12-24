use anyhow::Result;
use sysinfo::{NetworksExt, System, SystemExt, CpuExt, DiskExt, NetworkExt};

#[derive(Debug)]
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