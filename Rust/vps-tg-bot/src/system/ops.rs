use anyhow::{Context, Result};
use std::process::Command;

pub async fn perform_maintenance() -> Result<String> {
    let mut log = String::new();

    log.push_str("🔄 正在更新系统...\n");
    match run_command("apt-get", &["update"]).await {
        Ok(output) => log.push_str(&format!("✅ Apt 更新: 成功\n{}\n", output)),
        Err(e) => log.push_str(&format!("❌ Apt 更新: 失败 ({})\n", e)),
    }

    log.push_str("🔄 正在升级系统...\n");
    match run_command("apt-get", &["full-upgrade", "-y"]).await {
        Ok(output) => log.push_str(&format!("✅ Apt 完全升级: 成功\n{}\n", output)),
        Err(e) => log.push_str(&format!("❌ Apt 完全升级: 失败 ({})\n", e)),
    }

    log.push_str("🔄 正在清理不必要的软件包...\n");
    match run_command("apt-get", &["autoremove", "-y"]).await {
        Ok(output) => log.push_str(&format!("✅ Apt 自动移除: 成功\n{}\n", output)),
        Err(e) => log.push_str(&format!("❌ Apt 自动移除: 失败 ({})\n", e)),
    }

    log.push_str("🔄 正在清理缓存...\n");
    match run_command("apt-get", &["autoclean"]).await {
        Ok(output) => log.push_str(&format!("✅ Apt 自动清理: 成功\n{}\n", output)),
        Err(e) => log.push_str(&format!("❌ Apt 自动清理: 失败 ({})\n", e)),
    }

    Ok(log)
}

pub async fn check_security_updates() -> Result<bool> {
    let output = run_command("apt-get", &["upgrade", "-s"])
        .await
        .context("无法检查安全更新")?;
    Ok(output.contains("security"))
}

pub fn reboot_system() -> Result<()> {
    Command::new("reboot")
        .status()
        .context("无法重启系统")?;
    Ok(())
}

pub fn restart_service(service_name: &str) -> Result<()> {
    Command::new("systemctl")
        .args(["restart", service_name])
        .status()
        .context(format!("无法重启服务: {}", service_name))?;
    Ok(())
}

pub async fn update_xray() -> Result<String> {
    run_command("bash", &["-c", "bash -c $(curl -L https://github.com/XTLS/Xray-install/raw/main/install-release.sh) @ install"])
        .await
        .context("无法更新 Xray")
}

pub async fn update_singbox() -> Result<String> {
    run_command("bash", &["-c", "bash -c $(curl -L https://github.com/SagerNet/sing-box/raw/master/install.sh) @ install"])
        .await
        .context("无法更新 Sing-box")
}

pub async fn maintain_core() -> Result<String> {
    let mut log = String::new();

    log.push_str("🔄 正在执行核心维护...\n");
    match run_command("apt-get", &["update"]).await {
        Ok(output) => log.push_str(&format!("✅ Apt 更新: 成功\n{}\n", output)),
        Err(e) => log.push_str(&format!("❌ Apt 更新: 失败 ({})\n", e)),
    }

    log.push_str("🔄 正在升级系统...\n");
    match run_command("apt-get", &["full-upgrade", "-y"]).await {
        Ok(output) => log.push_str(&format!("✅ Apt 完全升级: 成功\n{}\n", output)),
        Err(e) => log.push_str(&format!("❌ Apt 完全升级: 失败 ({})\n", e)),
    }

    Ok(log)
}

pub async fn maintain_rules() -> Result<String> {
    run_command("bash", &["-c", "/usr/local/bin/vps-maintain-rules.sh"])
        .await
        .context("无法更新规则")
}

pub async fn get_system_logs(lines: usize) -> Result<String> {
    run_command("journalctl", &["-n", &lines.to_string(), "--no-pager"])
        .await
        .context("无法获取系统日志")
}

async fn run_command(command: &str, args: &[&str]) -> Result<String> {
    let output = tokio::process::Command::new(command)
        .args(args)
        .output()
        .await
        .context(format!("无法执行命令: {}", command))?;

    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "命令执行失败: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}